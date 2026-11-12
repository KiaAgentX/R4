/**
 * Client-generated idempotency keys — decision D7 (STEP1-AUDIT §8, §12):
 * every money mutation carries a client-generated key, created ONCE per
 * logical action, persisted in the queue store, and REUSED across retries.
 * Replay hits the backend's `Tx.n` UNIQUE semantics and returns the original
 * transaction — double-click / reconnect replay can never double-pay.
 *
 * The key format is ULID (26 Crockford chars ≥ the wire's 8-char minimum)
 * generated locally — identity/entropy never depends on the server.
 */

/** Crockford base32 alphabet (ULID spec) — no I, L, O, U. */
const ALPHABET = '0123456789ABCDEFGHJKMNPQRSTVWXYZ' as const;

const at = (i: number): string => ALPHABET[i] ?? '0';

let lastRandomPart = ''; // monotonic guard within the same millisecond

function encodeTime(time: number): string {
  let out = '';
  for (let i = 9; i >= 0; i--) {
    out = at(time % 32) + out;
    time = Math.floor(time / 32);
  }
  return out;
}

/** 80 bits of CSPRNG randomness → 16 Crockford chars. */
function randomPart(): string {
  const bytes = new Uint8Array(10);
  globalThis.crypto.getRandomValues(bytes);
  let value = 0n;
  for (const b of bytes) value = (value << 8n) | BigInt(b);
  let out = '';
  for (let i = 0; i < 16; i++) {
    out = at(Number(value & 31n)) + out;
    value >>= 5n;
  }
  return out;
}

/** Fresh ULID — 26 chars, monotonic within the same millisecond. */
export function newIdempotencyKey(): string {
  const now = Date.now();
  let random = encodeTime(now) + randomPart();
  if (random <= lastRandomPart) {
    // same-ms collision → bump the random half (ULID monotonic rule)
    const timeHalf = random.slice(0, 10);
    const rest = random.slice(10);
    const restChars = rest.split('');
    for (let i = restChars.length - 1; i >= 0; i--) {
      const idx = ALPHABET.indexOf(restChars[i]!);
      if (idx < 31) {
        restChars[i] = at(idx + 1);
        break;
      }
      restChars[i] = at(0);
    }
    random = timeHalf + restChars.join('');
  }
  lastRandomPart = random;
  return random;
}

export interface IdempotencyStore {
  /** Returns the stable key for a logical action, creating it if needed. */
  for(actionId: string): string;
  /** Existing key without creating — used for offline-queue inspection. */
  peek(actionId: string): string | null;
  /** Drop a key once its action has settled (success or terminal error). */
  release(actionId: string): void;
}

/** In-memory store — tests, SSR, and non-persistent contexts. */
export class MemoryIdempotencyStore implements IdempotencyStore {
  private readonly keys = new Map<string, string>();

  for(actionId: string): string {
    let key = this.keys.get(actionId);
    if (key === undefined) {
      key = newIdempotencyKey();
      this.keys.set(actionId, key);
    }
    return key;
  }

  peek(actionId: string): string | null {
    return this.keys.get(actionId) ?? null;
  }

  release(actionId: string): void {
    this.keys.delete(actionId);
  }
}

/** localStorage-backed store — persists across reconnects and reloads. */
export class LocalStorageIdempotencyStore implements IdempotencyStore {
  private readonly prefix: string;

  constructor(prefix = 'ao-idem:') {
    this.prefix = prefix;
  }

  for(actionId: string): string {
    const k = this.prefix + actionId;
    const existing = this.safeGet(k);
    if (existing !== null) return existing;
    const key = newIdempotencyKey();
    this.safeSet(k, key);
    return key;
  }

  peek(actionId: string): string | null {
    return this.safeGet(this.prefix + actionId);
  }

  release(actionId: string): void {
    try {
      localStorage.removeItem(this.prefix + actionId);
    } catch {
      /* storage unavailable → in-flight keys die with the page */
    }
  }

  private safeGet(k: string): string | null {
    try {
      return localStorage.getItem(k);
    } catch {
      return null;
    }
  }

  private safeSet(k: string, v: string): void {
    try {
      localStorage.setItem(k, v);
    } catch {
      /* storage disabled → keys degrade to per-page stability */
    }
  }
}

/** The right store for the current runtime. */
export function defaultIdempotencyStore(): IdempotencyStore {
  try {
    if (typeof localStorage !== 'undefined') return new LocalStorageIdempotencyStore();
  } catch {
    /* fall through */
  }
  return new MemoryIdempotencyStore();
}
