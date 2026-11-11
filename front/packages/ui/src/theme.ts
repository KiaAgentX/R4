/**
 * Aurora theme handling — tokens live in tokens.css; this module only flips
 * the `data-theme` attribute and persists the explicit choice.
 *
 * Precedence (mirrors tokens.css):
 *   explicit data-theme  >  prefers-color-scheme (system default)
 * `system` clears the attribute so the CSS media query decides.
 */

export type AuroraTheme = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'aurora-theme';

export function isBrowser(): boolean {
  return typeof document !== 'undefined';
}

export function getStoredTheme(): AuroraTheme {
  if (!isBrowser()) return 'system';
  const v = document.documentElement.dataset.theme;
  if (v === 'light' || v === 'dark') return v;
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === 'light' || stored === 'dark') return stored;
  } catch {
    /* storage disabled (private mode) → system */
  }
  return 'system';
}

/** Applies the theme to <html> and persists the explicit choice. */
export function applyTheme(theme: AuroraTheme): void {
  if (!isBrowser()) return;
  if (theme === 'system') {
    delete document.documentElement.dataset.theme;
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      /* ignore */
    }
  } else {
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem(STORAGE_KEY, theme);
    } catch {
      /* ignore */
    }
  }
}

/** Effective theme as painted right now. */
export function effectiveTheme(): 'light' | 'dark' {
  const explicit = document.documentElement.dataset.theme;
  if (explicit === 'light' || explicit === 'dark') return explicit;
  if (typeof matchMedia !== 'undefined' && matchMedia('(prefers-color-scheme: dark)').matches) {
    return 'dark';
  }
  return 'light';
}

/** Toggle between explicit light/dark (used by theme buttons). */
export function toggleTheme(): AuroraTheme {
  const next: AuroraTheme = effectiveTheme() === 'dark' ? 'light' : 'dark';
  applyTheme(next);
  return next;
}
