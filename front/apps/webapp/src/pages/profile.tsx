/**
 * Pages 19–21 — profile, settings, language (map §2.4 rows 19–21).
 * Identity is DISPLAY-ONLY here (initDataUnsafe public fields) and everything
 * persists in the browser only — money and identity never leave the device
 * except through the verified contract paths (settings dict promise).
 */
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { applyLocale } from '@agent-one/ui/dir';
import { effectiveTheme, toggleTheme } from '@agent-one/ui/theme';
import { localeName, t, type Locale } from '@agent-one/i18n/webapp';
import type { PageCtx } from '../ctx.js';
import { clearAdminKey, clearLocalData, saveAdminKey, saveBotHandle, saveLocale, userAccount } from '../session.js';
import { Card, Kv, Money, Tile } from '../ui.js';

/* ── Page 19 — profile ────────────────────────────────────────────────── */

export default function ProfilePage({ ctx }: { ctx: PageCtx }) {
  const { locale, tgUser, mode } = ctx;
  const account = tgUser !== null ? userAccount(tgUser.id) : null;
  const balance = useQuery({
    queryKey: ['w-balance', account],
    queryFn: async () => unwrap(await ctx.client.reads.balance(account as string)),
    enabled: account !== null,
    retry: 0,
    staleTime: 15_000,
  });

  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'profileTitle')}>
        {tgUser === null ? (
          <span className="text-sm text-aur-muted">{t(locale, 'profileGuest')}</span>
        ) : (
          <>
            <Kv label={t(locale, 'profileName')}>{tgUser.name}</Kv>
            {tgUser.username !== null && (
              <Kv label={t(locale, 'profileUsername')} ltr>
                @{tgUser.username}
              </Kv>
            )}
            <Kv label={t(locale, 'profileTelegramId')} ltr>
              <code className="text-xs">{tgUser.id}</code>
            </Kv>
            <Kv label={t(locale, 'profileAccount')} ltr>
              <code className="text-xs">{account}</code>
            </Kv>
          </>
        )}
      </Card>
      {account !== null && (
        <Card>
          {balance.isPending && <span className="text-aur-muted text-sm">{t(locale, 'loading')}</span>}
          {balance.isSuccess && (
            <Tile label={t(locale, 'colBalance')} value={<Money amount={balance.data.balance} hitl={false} />} />
          )}
        </Card>
      )}
      {mode.kind === 'staff' && (
        <Card note={t(locale, 'staffPreviewNote')}>
          <span className="aur-badge aur-badge--brand">{t(locale, 'staffPreviewBadge')}</span>
        </Card>
      )}
    </div>
  );
}

/* ── Page 20 — settings (browser-only) ────────────────────────────────── */

export function SettingsPage({ ctx }: { ctx: PageCtx }) {
  const { locale } = ctx;
  const [bot, setBot] = useState(ctx.botHandle ?? '');
  const [key, setKey] = useState('');
  const [saved, setSaved] = useState<string | null>(null);

  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'settingsTitle')} note={t(locale, 'settingsLocalNote')}>
        <div className="aur-field">
          <label className="aur-label" htmlFor="w-bot-handle">
            {t(locale, 'settingsBotUsername')}
          </label>
          <input id="w-bot-handle" className="aur-input" dir="ltr" value={bot} placeholder="DropAgentX_bot" onChange={(e) => setBot(e.target.value)} />
          <div className="text-aur-faint text-xs">{t(locale, 'settingsBotUsernameHint')}</div>
        </div>
        <button
          type="button"
          className="aur-btn aur-btn--primary"
          onClick={() => {
            const ok = saveBotHandle(bot);
            setSaved(ok ? t(locale, 'settingsSaved') : t(locale, 'settingsCleared'));
          }}
        >
          {t(locale, 'save')}
        </button>

        <div className="aur-field">
          <label className="aur-label" htmlFor="w-admin-key">
            {t(locale, 'settingsAdminKey')}
          </label>
          <input id="w-admin-key" className="aur-input" type="password" autoComplete="off" dir="ltr" value={key} onChange={(e) => setKey(e.target.value)} />
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="aur-btn aur-btn--ghost"
            disabled={key.trim().length === 0}
            onClick={() => {
              saveAdminKey(key);
              ctx.client.setAdminKey(key.trim());
              setSaved(t(locale, 'settingsSaved'));
            }}
          >
            {t(locale, 'save')}
          </button>
          <button
            type="button"
            className="aur-btn aur-btn--ghost"
            onClick={() => {
              clearAdminKey();
              ctx.client.setAdminKey(undefined);
              setSaved(t(locale, 'settingsSaved'));
            }}
          >
            {t(locale, 'signOut')}
          </button>
        </div>

        <div className="flex flex-wrap gap-2">
          <button type="button" className="aur-btn aur-btn--ghost" onClick={() => toggleTheme()}>
            {t(locale, 'theme')}: {effectiveTheme() === 'dark' ? t(locale, 'themeDark') : t(locale, 'themeLight')}
          </button>
          <button
            type="button"
            className="aur-btn aur-btn--danger"
            onClick={() => {
              clearLocalData();
              setSaved(t(locale, 'settingsCleared'));
            }}
          >
            {t(locale, 'settingsDangerZone')}
          </button>
        </div>
        {saved !== null && (
          <div role="status" className="text-aur-muted text-sm">
            {saved}
          </div>
        )}
      </Card>
    </div>
  );
}

/* ── Page 21 — language ───────────────────────────────────────────────── */

export function LanguagePage({ ctx }: { ctx: PageCtx }) {
  const { locale } = ctx;
  const pick = (next: Locale): void => {
    saveLocale(next);
    applyLocale(next);
    // A full reload is the honest way to re-render every dictionary surface.
    window.location.reload();
  };
  return (
    <Card title={t(locale, 'languageTitle')} note={t(locale, 'languageNote')}>
      <button type="button" className="aur-btn aur-btn--ghost" aria-pressed={locale === 'fa'} onClick={() => pick('fa')}>
        {t(locale, 'languageFa')} · {localeName('fa')}
      </button>
      <button type="button" className="aur-btn aur-btn--ghost" aria-pressed={locale === 'en'} onClick={() => pick('en')}>
        {t(locale, 'languageEn')} · {localeName('en')}
      </button>
      <span className="text-aur-faint text-xs">
        {t(locale, 'language')}: {localeName(locale)}
      </span>
    </Card>
  );
}
