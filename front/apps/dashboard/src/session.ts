/**
 * Admin session — the X-KIA-Key storage layer (STEP1-AUDIT §4).
 *
 * The key stays in the ADMIN'S BROWSER ONLY (dict: adminKeyPrompt). It is
 * kept in localStorage under the SAME key the strategic shell reads for role
 * detection (`ao-admin-key`, apps/shell/src/detect.ts) — one store, one truth.
 * All access is fail-closed: storage disabled (private mode) never crashes the
 * dashboard, it just degrades to a session-only key held in React state.
 */

export const ADMIN_KEY_STORE = 'ao-admin-key';
export const LOCALE_STORE = 'ao-locale';

function safeGet(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null; // storage disabled → no stored key
  }
}

/** Stored admin key, trimmed; null when absent or blank. */
export function loadAdminKey(): string | null {
  const raw = safeGet(ADMIN_KEY_STORE);
  const trimmed = raw?.trim();
  return trimmed !== undefined && trimmed.length > 0 ? trimmed : null;
}

/**
 * Persist the key. Returns false when storage is unavailable — the caller
 * then keeps the key in memory only (still a valid session, never persisted).
 */
export function saveAdminKey(key: string): boolean {
  const trimmed = key.trim();
  if (trimmed.length === 0) return false;
  try {
    localStorage.setItem(ADMIN_KEY_STORE, trimmed);
    return true;
  } catch {
    return false; // storage disabled → session-only key
  }
}

/** Forget the stored key (sign-out). Storage-disabled stays silent. */
export function clearAdminKey(): void {
  try {
    localStorage.removeItem(ADMIN_KEY_STORE);
  } catch {
    /* storage disabled → nothing stored anyway */
  }
}

/** Initial locale — stored choice > navigator > fa (the bot's rule). */
export function initialLocale(): 'fa' | 'en' {
  const stored = safeGet(LOCALE_STORE);
  if (stored === 'fa' || stored === 'en') return stored;
  try {
    const nav = navigator?.language;
    return nav !== undefined && nav.toLowerCase().startsWith('en') ? 'en' : 'fa';
  } catch {
    return 'fa';
  }
}
