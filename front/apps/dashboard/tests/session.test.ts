import { afterEach, describe, expect, it, vi } from 'vitest';
import { ADMIN_KEY_STORE, clearAdminKey, loadAdminKey, saveAdminKey } from '../src/session.js';

afterEach(() => {
  vi.unstubAllGlobals();
  try {
    localStorage.removeItem(ADMIN_KEY_STORE);
  } catch {
    /* ignore */
  }
});

describe('session — admin key storage (X-KIA-Key, browser-only)', () => {
  it('returns null when nothing is stored', () => {
    expect(loadAdminKey()).toBeNull();
  });

  it('saves trimmed, loads trimmed, and clears', () => {
    expect(saveAdminKey('  kia-secret-123  ')).toBe(true);
    expect(loadAdminKey()).toBe('kia-secret-123');
    clearAdminKey();
    expect(loadAdminKey()).toBeNull();
  });

  it('rejects blank keys without touching storage', () => {
    expect(saveAdminKey('   ')).toBe(false);
    expect(loadAdminKey()).toBeNull();
  });

  it('fails closed when storage throws (private mode) — no crash, session-only key', () => {
    vi.stubGlobal('localStorage', {
      getItem: () => {
        throw new Error('storage blocked');
      },
      setItem: () => {
        throw new Error('storage blocked');
      },
      removeItem: () => {
        throw new Error('storage blocked');
      },
    });
    expect(loadAdminKey()).toBeNull();
    // save reports "not persisted" — the App then keeps the key in memory
    expect(saveAdminKey('kia-secret-123')).toBe(false);
    expect(() => clearAdminKey()).not.toThrow();
  });
});
