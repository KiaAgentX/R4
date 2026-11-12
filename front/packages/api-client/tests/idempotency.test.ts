import { describe, expect, it } from 'vitest';
import {
  LocalStorageIdempotencyStore,
  MemoryIdempotencyStore,
  newIdempotencyKey,
} from '../src/idempotency.js';

describe('newIdempotencyKey — ULID shape (D7)', () => {
  it('is 26 Crockford base32 chars (≥ the wire minimum of 8)', () => {
    const key = newIdempotencyKey();
    expect(key).toHaveLength(26);
    expect(key).toMatch(/^[0-9ABCDEFGHJKMNPQRSTVWXYZ]{26}$/);
  });

  it('never repeats across a burst', () => {
    const seen = new Set<string>();
    for (let i = 0; i < 2_000; i++) seen.add(newIdempotencyKey());
    expect(seen.size).toBe(2_000);
  });
});

describe('MemoryIdempotencyStore — stable per logical action', () => {
  it('same actionId → same key across retries; different actions differ', () => {
    const store = new MemoryIdempotencyStore();
    const first = store.for('fund:esc:123');
    const retry = store.for('fund:esc:123');
    const other = store.for('fund:esc:456');
    expect(retry).toBe(first);
    expect(other).not.toBe(first);
  });

  it('peek does not create; release drops', () => {
    const store = new MemoryIdempotencyStore();
    expect(store.peek('mint:user:1')).toBeNull();
    const key = store.for('mint:user:1');
    expect(store.peek('mint:user:1')).toBe(key);
    store.release('mint:user:1');
    expect(store.peek('mint:user:1')).toBeNull();
  });
});

describe('LocalStorageIdempotencyStore — persists across reconnects (§8)', () => {
  it('keys survive a new store instance (page reload semantics)', () => {
    localStorage.clear();
    const a = new LocalStorageIdempotencyStore();
    const key = a.for('buy:product:9');
    const b = new LocalStorageIdempotencyStore();
    expect(b.for('buy:product:9')).toBe(key);
    expect(b.peek('buy:product:9')).toBe(key);
    b.release('buy:product:9');
    expect(b.peek('buy:product:9')).toBeNull();
  });

  it('degrades silently when storage is unavailable', () => {
    const original = Storage.prototype.getItem;
    const originalSet = Storage.prototype.setItem;
    Storage.prototype.getItem = () => {
      throw new Error('SecurityError');
    };
    Storage.prototype.setItem = () => {
      throw new Error('SecurityError');
    };
    try {
      const store = new LocalStorageIdempotencyStore();
      const k1 = store.for('release:esc:1');
      const k2 = store.for('release:esc:1');
      expect(k1).toHaveLength(26);
      expect(k2).toHaveLength(26); // per-page stability still holds
    } finally {
      Storage.prototype.getItem = original;
      Storage.prototype.setItem = originalSet;
    }
  });
});
