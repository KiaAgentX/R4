/**
 * Webapp session — locale, optional staff key, Telegram identity, bot handle.
 *
 * Storage discipline (settings dict: settingsLocalNote):
 *  • `ao-locale`     — language choice, SAME store the shell + dashboard read
 *  • `ao-admin-key`  — OPTIONAL staff preview (same store as the dashboard);
 *                      when set, money actions may run directly (staff mode)
 *  • `ao-bot-handle` — the bot username for "open in Telegram" handoffs
 *  • identity NEVER persists — Telegram initData is read live per session
 * All storage access is fail-closed: disabled storage (private mode) never
 * crashes the app, it just degrades to session-only values.
 */

import { localeFromLanguageCode, type Locale } from '@agent-one/i18n/webapp';

export const LOCALE_STORE = 'ao-locale';
export const ADMIN_KEY_STORE = 'ao-admin-key';
export const BOT_HANDLE_STORE = 'ao-bot-handle';

/** A client-side Telegram context — DISPLAY ONLY. Identity-critical calls
 *  always carry the raw `initData` string so the SERVER verifies it (D14). */
export interface TelegramUser {
  id: string;
  name: string;
  username: string | null;
  languageCode: string | null;
}

export interface WebappWindow {
  Telegram?: {
    WebApp?: {
      initData?: string;
      initDataUnsafe?: {
        user?: {
          id?: number | string;
          first_name?: string;
          last_name?: string;
          username?: string;
          language_code?: string;
        };
      };
    };
  };
  navigator?: { language?: string };
  localStorage?: {
    getItem(key: string): string | null;
    setItem(key: string, value: string): void;
    removeItem(key: string): void;
  };
}

function safeGet(storage: WebappWindow['localStorage'], key: string): string | null {
  try {
    return storage?.getItem(key) ?? null;
  } catch {
    return null; // storage disabled → no stored value
  }
}

function safeSet(storage: WebappWindow['localStorage'], key: string, value: string): boolean {
  try {
    storage?.setItem(key, value);
    return true;
  } catch {
    return false; // storage disabled → session-only value
  }
}

function safeRemove(storage: WebappWindow['localStorage'], key: string): void {
  try {
    storage?.removeItem(key);
  } catch {
    /* storage disabled → nothing stored anyway */
  }
}

/** Stored locale (bot rule), else Telegram hint, else navigator, else fa. */
export function initialLocale(win: WebappWindow = window): Locale {
  const stored = safeGet(win.localStorage, LOCALE_STORE);
  if (stored === 'fa' || stored === 'en') return stored;
  const hint = win.Telegram?.WebApp?.initDataUnsafe?.user?.language_code ?? win.navigator?.language;
  return localeFromLanguageCode(hint);
}

export function saveLocale(locale: Locale, win: WebappWindow = window): boolean {
  return safeSet(win.localStorage, LOCALE_STORE, locale);
}

/** The stored OPTIONAL staff key — trimmed; null when absent or blank. */
export function loadAdminKey(win: WebappWindow = window): string | null {
  const raw = safeGet(win.localStorage, ADMIN_KEY_STORE)?.trim();
  return raw !== undefined && raw.length > 0 ? raw : null;
}

export function saveAdminKey(key: string, win: WebappWindow = window): boolean {
  const trimmed = key.trim();
  if (trimmed.length === 0) return false;
  return safeSet(win.localStorage, ADMIN_KEY_STORE, trimmed);
}

export function clearAdminKey(win: WebappWindow = window): void {
  safeRemove(win.localStorage, ADMIN_KEY_STORE);
}

/** Bot handle without the leading `@`s; null when unset or blank. */
export function loadBotHandle(win: WebappWindow = window): string | null {
  const raw = safeGet(win.localStorage, BOT_HANDLE_STORE)?.trim().replace(/^@+/, '');
  return raw !== undefined && raw.length > 0 ? raw : null;
}

export function saveBotHandle(handle: string, win: WebappWindow = window): boolean {
  const trimmed = handle.trim().replace(/^@+/, '');
  if (trimmed.length === 0) return false;
  return safeSet(win.localStorage, BOT_HANDLE_STORE, trimmed);
}

/** Non-empty raw initData — the ONLY identity source (never faked, D14). */
export function telegramInitData(win: WebappWindow = window): string | null {
  const data = win.Telegram?.WebApp?.initData;
  return typeof data === 'string' && data.length > 0 ? data : null;
}

/** Display-only Telegram user — derived from `initDataUnsafe` (public fields);
 *  anything identity-critical goes through the verified initData path. */
export function telegramUser(win: WebappWindow = window): TelegramUser | null {
  const u = win.Telegram?.WebApp?.initDataUnsafe?.user;
  if (u?.id === undefined) return null;
  const name = [u.first_name, u.last_name].filter((p) => typeof p === 'string' && p.length > 0).join(' ');
  return {
    id: String(u.id),
    name: name.length > 0 ? name : `tg:${String(u.id)}`,
    username: typeof u.username === 'string' && u.username.length > 0 ? u.username : null,
    languageCode: typeof u.language_code === 'string' ? u.language_code : null,
  };
}

/** The nation account convention of Phase 2 — `user:tg:<id>`. */
export function userAccount(telegramId: string): string {
  return `user:tg:${telegramId}`;
}

/** Every stored webapp value — one switch for the "clear local data" action. */
export function clearLocalData(win: WebappWindow = window): void {
  safeRemove(win.localStorage, LOCALE_STORE);
  safeRemove(win.localStorage, ADMIN_KEY_STORE);
  safeRemove(win.localStorage, BOT_HANDLE_STORE);
  safeRemove(win.localStorage, ACTION_LOG_STORE);
}

/** Local action log (notifications page) — receipts of THIS device only. */
export const ACTION_LOG_STORE = 'ao-webapp-log';
export interface ActionLogEntry {
  at: string;
  kind: 'staff' | 'chat' | 'bot' | 'readonly';
  action: string;
  target: string | null;
  txId: string | null;
  replayed: boolean;
}

export function loadActionLog(win: WebappWindow = window): ActionLogEntry[] {
  const raw = safeGet(win.localStorage, ACTION_LOG_STORE);
  if (raw === null) return [];
  try {
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as ActionLogEntry[]) : [];
  } catch {
    return []; // corrupted log → honest empty, never crash
  }
}

export function appendActionLog(entry: Omit<ActionLogEntry, 'at'>, win: WebappWindow = window): void {
  const stamped: ActionLogEntry = { ...entry, at: new Date().toISOString() };
  const next = [stamped, ...loadActionLog(win)].slice(0, 50); // bounded, local-only
  try {
    win.localStorage?.setItem(ACTION_LOG_STORE, JSON.stringify(next));
  } catch {
    /* storage disabled → log stays in-memory of the page */
  }
}

export function clearActionLog(win: WebappWindow = window): void {
  safeRemove(win.localStorage, ACTION_LOG_STORE);
}
