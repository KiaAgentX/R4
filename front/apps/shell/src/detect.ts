/**
 * Shell context detection — pure, DOM-guarded, fully testable.
 * The strategic index detects: Telegram presence, role, language, device —
 * then routes to the right area (STEP1-AUDIT §7 apps/shell).
 *
 * Rules (honest, fail-closed):
 *  • Telegram  ⇔ non-empty `Telegram.WebApp.initData` (identity is never faked)
 *  • role      = admin if a stored admin key exists, else citizen in Telegram,
 *                else guest
 *  • language  = explicit stored choice > Telegram `language_code` hint >
 *                navigator > fa (the nation's default — same rule as the bot)
 *  • device    = coarse pointer or narrow viewport → mobile
 */
import { localeFromLanguageCode, type Locale } from '@agent-one/i18n';

export interface TelegramWebApp {
  initData?: string;
  initDataUnsafe?: { user?: { language_code?: string } };
  platform?: string;
}

/** The slice of `window` the detection actually reads. */
export interface DetectionWindow {
  Telegram?: { WebApp?: TelegramWebApp };
  navigator?: { language?: string; languages?: readonly string[] };
  localStorage?: { getItem(key: string): string | null };
  matchMedia?: (query: string) => { matches: boolean };
  innerWidth?: number;
}

export type ShellRole = 'admin' | 'citizen' | 'guest';
export type ShellDevice = 'mobile' | 'desktop';

export interface Detection {
  inTelegram: boolean;
  telegramPlatform: string | null;
  role: ShellRole;
  locale: Locale;
  device: ShellDevice;
}

export const ADMIN_KEY_STORE = 'ao-admin-key';
export const LOCALE_STORE = 'ao-locale';

function safeGet(storage: DetectionWindow['localStorage'], key: string): string | null {
  try {
    return storage?.getItem(key) ?? null;
  } catch {
    return null; // storage disabled (private mode) → hint ignored
  }
}

export function detectShell(win: DetectionWindow): Detection {
  const webApp = win.Telegram?.WebApp;
  const initData = webApp?.initData;
  const inTelegram = typeof initData === 'string' && initData.length > 0;

  const storedLocale = safeGet(win.localStorage, LOCALE_STORE);
  const locale: Locale =
    storedLocale === 'fa' || storedLocale === 'en'
      ? storedLocale
      : localeFromLanguageCode(webApp?.initDataUnsafe?.user?.language_code ?? win.navigator?.language);

  const hasAdminKey = (safeGet(win.localStorage, ADMIN_KEY_STORE) ?? '').length > 0;
  const role: ShellRole = hasAdminKey ? 'admin' : inTelegram ? 'citizen' : 'guest';

  const coarse = win.matchMedia?.('(pointer: coarse)')?.matches ?? false;
  const narrow = (win.innerWidth ?? 1024) < 768;
  const device: ShellDevice = coarse || narrow ? 'mobile' : 'desktop';

  return {
    inTelegram,
    telegramPlatform: inTelegram ? (webApp?.platform ?? null) : null,
    role,
    locale,
    device,
  };
}

/** The area the detected context should land in first. */
export function primaryArea(role: ShellRole): '/admin/' | '/app/' | '/site/' {
  switch (role) {
    case 'admin':
      return '/admin/';
    case 'citizen':
      return '/app/';
    case 'guest':
      return '/site/';
  }
}
