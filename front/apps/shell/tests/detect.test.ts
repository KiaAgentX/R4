import { describe, expect, it } from 'vitest';
import {
  ADMIN_KEY_STORE,
  detectShell,
  LOCALE_STORE,
  primaryArea,
  type DetectionWindow,
} from '../src/detect.js';

function win(overrides: Partial<DetectionWindow> = {}): DetectionWindow {
  return { innerWidth: 1024, ...overrides };
}

describe('detectShell — Telegram detection (identity never faked)', () => {
  it('no Telegram object → guest', () => {
    const d = detectShell(win());
    expect(d.inTelegram).toBe(false);
    expect(d.role).toBe('guest');
    expect(d.telegramPlatform).toBeNull();
  });

  it('empty initData is NOT Telegram (fail-closed)', () => {
    const d = detectShell(win({ Telegram: { WebApp: { initData: '', platform: 'android' } } }));
    expect(d.inTelegram).toBe(false);
    expect(d.role).toBe('guest');
    expect(d.telegramPlatform).toBeNull();
  });

  it('non-empty initData → citizen + platform surfaced', () => {
    const d = detectShell(win({ Telegram: { WebApp: { initData: 'query_id=AA&hash=x', platform: 'ios' } } }));
    expect(d.inTelegram).toBe(true);
    expect(d.role).toBe('citizen');
    expect(d.telegramPlatform).toBe('ios');
  });
});

describe('detectShell — role precedence (admin key > telegram > guest)', () => {
  const store = (entries: Record<string, string>) => ({
    getItem: (k: string) => entries[k] ?? null,
  });

  it('stored admin key → admin even inside Telegram', () => {
    const d = detectShell(
      win({
        localStorage: store({ [ADMIN_KEY_STORE]: 'kia-admin-secret-0123456789' }),
        Telegram: { WebApp: { initData: 'q=1' } },
      }),
    );
    expect(d.role).toBe('admin');
    expect(d.inTelegram).toBe(true);
  });

  it('empty stored key is not a key', () => {
    const d = detectShell(win({ localStorage: store({ [ADMIN_KEY_STORE]: '' }) }));
    expect(d.role).toBe('guest');
  });

  it('storage throwing (private mode) → guest, no crash', () => {
    const d = detectShell(
      win({
        localStorage: {
          getItem: () => {
            throw new Error('SecurityError');
          },
        },
      }),
    );
    expect(d.role).toBe('guest');
  });
});

describe('detectShell — language precedence (stored > telegram hint > navigator > fa)', () => {
  const store = (entries: Record<string, string>) => ({ getItem: (k: string) => entries[k] ?? null });

  it('explicit stored choice wins', () => {
    const d = detectShell(
      win({
        localStorage: store({ [LOCALE_STORE]: 'en' }),
        Telegram: { WebApp: { initData: 'q=1', initDataUnsafe: { user: { language_code: 'fa' } } } },
      }),
    );
    expect(d.locale).toBe('en');
  });

  it('telegram language_code hint — en-US → en', () => {
    const d = detectShell(
      win({ Telegram: { WebApp: { initData: 'q=1', initDataUnsafe: { user: { language_code: 'en-US' } } } } }),
    );
    expect(d.locale).toBe('en');
  });

  it('telegram hint fa → fa (default of the nation)', () => {
    const d = detectShell(
      win({ Telegram: { WebApp: { initData: 'q=1', initDataUnsafe: { user: { language_code: 'fa' } } } } }),
    );
    expect(d.locale).toBe('fa');
  });

  it('no hints anywhere → fa (bot rule)', () => {
    expect(detectShell(win()).locale).toBe('fa');
    expect(detectShell(win({ navigator: { language: 'de' } })).locale).toBe('fa');
  });

  it('navigator fallback en', () => {
    expect(detectShell(win({ navigator: { language: 'en-GB' } })).locale).toBe('en');
  });
});

describe('detectShell — device (coarse pointer or narrow viewport)', () => {
  it('coarse pointer → mobile', () => {
    expect(detectShell(win({ matchMedia: (q) => ({ matches: q.includes('coarse') }) })).device).toBe('mobile');
  });

  it('narrow viewport → mobile', () => {
    expect(detectShell(win({ innerWidth: 480 })).device).toBe('mobile');
  });

  it('desktop default', () => {
    expect(detectShell(win()).device).toBe('desktop');
  });
});

describe('primaryArea — role → area mapping', () => {
  it('admin → /admin/, citizen → /app/, guest → /site/', () => {
    expect(primaryArea('admin')).toBe('/admin/');
    expect(primaryArea('citizen')).toBe('/app/');
    expect(primaryArea('guest')).toBe('/site/');
  });
});
