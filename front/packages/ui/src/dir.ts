/**
 * Aurora direction & language handling — RTL is first-class (UX rule §6):
 * fa is the default locale of the nation and the whole component layer is
 * written with logical CSS properties, so flipping `dir` is the entire job.
 */

export type AurDir = 'rtl' | 'ltr';

/** fa (and other RTL scripts) → rtl; everything else → ltr. */
export function dirOf(locale: string): AurDir {
  return locale.toLowerCase().startsWith('fa') ? 'rtl' : 'ltr';
}

/** Applies lang + dir to <html> — call once per locale change. */
export function applyLocale(locale: string): void {
  if (typeof document === 'undefined') return;
  document.documentElement.lang = locale.toLowerCase();
  document.documentElement.dir = dirOf(locale);
}
