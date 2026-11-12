import { describe, expect, it } from 'vitest';
import {
  appendActionLog,
  clearActionLog,
  clearLocalData,
  initialLocale,
  loadActionLog,
  loadAdminKey,
  loadBotHandle,
  saveAdminKey,
  saveBotHandle,
  saveLocale,
  telegramInitData,
  telegramUser,
  userAccount,
  type ActionLogEntry,
  type WebappWindow,
} from '../src/session.js';

/** A controllable fake window — happy-dom provides the real one; this makes
 *  the fail-closed storage matrix explicit and offline. */
function fakeWindow(opts: {
  data?: Record<string, string>;
  broken?: boolean;
  initData?: string;
  tgUser?: { id: number | string; first_name?: string; last_name?: string; username?: string; language_code?: string };
  navigatorLanguage?: string;
} = {}): WebappWindow {
  const data = { ...opts.data };
  const w: WebappWindow = {
    localStorage: {
      getItem: (key) => {
        if (opts.broken === true) throw new Error('storage disabled');
        return data[key] ?? null;
      },
      setItem: (key, value) => {
        if (opts.broken === true) throw new Error('storage disabled');
        data[key] = value;
      },
      removeItem: (key) => {
        if (opts.broken === true) throw new Error('storage disabled');
        delete data[key];
      },
    },
  };
  if (opts.initData !== undefined || opts.tgUser !== undefined) {
    w.Telegram = {
      WebApp: {
        ...(opts.initData !== undefined ? { initData: opts.initData } : {}),
        ...(opts.tgUser !== undefined ? { initDataUnsafe: { user: opts.tgUser } } : {}),
      },
    };
  }
  if (opts.navigatorLanguage !== undefined) w.navigator = { language: opts.navigatorLanguage };
  return w;
}

describe('session — locale (the bot rule)', () => {
  it('stored choice beats the Telegram hint and navigator', () => {
    const w = fakeWindow({ data: { 'ao-locale': 'en' }, tgUser: { id: 1, language_code: 'fa' }, navigatorLanguage: 'fa' });
    expect(initialLocale(w)).toBe('en');
  });

  it('Telegram hint → en* rule; default fa', () => {
    expect(initialLocale(fakeWindow({ tgUser: { id: 1, language_code: 'en-US' } }))).toBe('en');
    expect(initialLocale(fakeWindow({ tgUser: { id: 1, language_code: 'de' } }))).toBe('fa');
    expect(initialLocale(fakeWindow({ navigatorLanguage: 'en' }))).toBe('en');
    expect(initialLocale(fakeWindow())).toBe('fa');
  });

  it('save persists; broken storage degrades silently (session-only)', () => {
    expect(saveLocale('en', fakeWindow())).toBe(true);
    expect(saveLocale('en', fakeWindow({ broken: true }))).toBe(false);
  });
});

describe('session — identity (never faked, never persisted)', () => {
  it('initData counts ONLY when a non-empty string (D14)', () => {
    expect(telegramInitData(fakeWindow({ initData: 'query_id=aa' }))).toBe('query_id=aa');
    expect(telegramInitData(fakeWindow({ initData: '' }))).toBeNull();
    expect(telegramInitData(fakeWindow())).toBeNull();
  });

  it('display user reads the public fields; no user → null', () => {
    const u = telegramUser(fakeWindow({ tgUser: { id: 42, first_name: 'Kia', last_name: 'X', username: 'kia', language_code: 'fa' } }));
    expect(u).toEqual({ id: '42', name: 'Kia X', username: 'kia', languageCode: 'fa' });
    expect(telegramUser(fakeWindow())).toBeNull();
    const nameless = telegramUser(fakeWindow({ tgUser: { id: 7 } }));
    expect(nameless?.name).toBe('tg:7');
  });

  it('account convention is Phase 2\u2019s user:tg:<id>', () => {
    expect(userAccount('42')).toBe('user:tg:42');
  });
});

describe('session — staff key and bot handle (fail-closed)', () => {
  it('admin key trims; blank is null; broken storage is null', () => {
    const w = fakeWindow({ data: { 'ao-admin-key': '  kia-key  ' } });
    expect(loadAdminKey(w)).toBe('kia-key');
    expect(loadAdminKey(fakeWindow())).toBeNull();
    expect(saveAdminKey('k', fakeWindow({ broken: true }))).toBe(false);
    expect(loadAdminKey(fakeWindow({ broken: true }))).toBeNull();
  });

  it('bot handle strips the @; empty after strip is false', () => {
    const w = fakeWindow();
    expect(saveBotHandle('@drop_bot', w)).toBe(true);
    expect(loadBotHandle(w)).toBe('drop_bot');
    expect(saveBotHandle('@@', w)).toBe(false);
    expect(loadBotHandle(fakeWindow())).toBeNull();
  });
});

describe('session — the local action log (notifications receipts)', () => {
  const entry: Omit<ActionLogEntry, 'at'> = { kind: 'staff', action: 'escrow.release', target: 'esc-1', txId: 'tx-1', replayed: false };

  it('appends newest-first and caps at 50 entries', () => {
    const w = fakeWindow();
    for (let i = 0; i < 60; i++) appendActionLog({ ...entry, action: `a${i}` }, w);
    const log = loadActionLog(w);
    expect(log.length).toBe(50);
    expect(log[0]?.action).toBe('a59');
    expect(log[0]?.at).toBeTruthy();
  });

  it('corrupted JSON degrades to an honest empty log (never crashes)', () => {
    const w = fakeWindow({ data: { 'ao-webapp-log': '{oops' } });
    expect(loadActionLog(w)).toEqual([]);
  });

  it('clear removes the log and clearLocalData wipes every webapp key', () => {
    const w = fakeWindow();
    appendActionLog(entry, w);
    saveLocale('en', w);
    saveBotHandle('bot', w);
    clearActionLog(w);
    expect(loadActionLog(w)).toEqual([]);
    expect(saveAdminKey('k', w)).toBe(true);
    clearLocalData(w);
    expect(loadAdminKey(w)).toBeNull();
    expect(loadBotHandle(w)).toBeNull();
    expect(initialLocale(w)).toBe('fa');
  });

  it('broken storage never crashes the append (log stays page-local)', () => {
    expect(() => appendActionLog(entry, fakeWindow({ broken: true }))).not.toThrow();
    expect(loadActionLog(fakeWindow({ broken: true }))).toEqual([]);
  });
});
