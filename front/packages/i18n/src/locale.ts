/**
 * Locale handling — mirrors crates/bot/src/i18n.rs Locale:
 *   • fa is the default of the nation (bot: `from_language_code(None) → Fa`)
 *   • `en*` → En, everything else → Fa
 */

export const LOCALES = ['fa', 'en'] as const;
export type Locale = (typeof LOCALES)[number];

/** The nation's default language — same as the bot. */
export const DEFAULT_LOCALE: Locale = 'fa';

/** Bot rule: `en*` → en, everything else (incl. none) → fa. */
export function localeFromLanguageCode(code?: string | null): Locale {
  const c = code?.trim().toLowerCase();
  return c?.startsWith('en') ? 'en' : 'fa';
}

/** The other locale (bot `/lang` toggle semantics). */
export function oppositeLocale(locale: Locale): Locale {
  return locale === 'fa' ? 'en' : 'fa';
}

export function isRtl(locale: Locale): boolean {
  return locale === 'fa';
}

export function dirOf(locale: Locale): 'rtl' | 'ltr' {
  return isRtl(locale) ? 'rtl' : 'ltr';
}

/** Native language name — bot `Locale::name`. */
export function localeName(locale: Locale): string {
  return locale === 'fa' ? 'فارسی' : 'English';
}
