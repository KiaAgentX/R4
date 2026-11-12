/**
 * Pages 29–33 — notifications, help/FAQ, offline, 404, about
 * (map §2.4 rows 29–33). Honesty rules:
 *  • notifications = THIS DEVICE's receipts only (no server push exists)
 *  • help = the bot's own commands + FAQ (mirrors the bot's /help voice)
 *  • offline = real navigator.onLine state + "no money moves offline"
 *  • 404 = every unknown hash, with the way back
 *  • about = live brand + server version from PUBLIC routes
 */
import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t } from '@agent-one/i18n/webapp';
import type { PageCtx } from '../ctx.js';
import { BOT_ADMIN_COMMANDS, BOT_COMMANDS, FAQ, botCommandHelp } from '../content.js';
import { formatDate } from '../format.js';
import { pageHref } from '../router.js';
import {
  clearActionLog,
  loadActionLog,
  type ActionLogEntry,
} from '../session.js';
import { Card, EmptyState, Kv, useOnline } from '../ui.js';

/* ── Page 29 — notifications (local receipts) ─────────────────────────── */

export function NotificationsView({ entries, locale }: { entries: ActionLogEntry[]; locale: PageCtx['locale'] }) {
  if (entries.length === 0) return <EmptyState locale={locale} note={t(locale, 'notifEmpty')} />;
  return (
    <div className="flex flex-col gap-3">
      {entries.map((e, i) => (
        <div key={`${e.at}-${i}`} className="aur-receipt">
          <span className="aur-receipt__head">
            <span>
              <span className="aur-badge">{e.kind}</span> {e.action}
            </span>
            <span>{formatDate(e.at, locale)}</span>
          </span>
          {e.target !== null && (
            <code className="text-xs" dir="ltr">
              {e.target}
            </code>
          )}
          <span className="aur-receipt__head">
            <span>{e.txId !== null ? `${t(locale, 'notifReceipt')}: ${e.txId}` : t(locale, 'notifQueued')}</span>
            {e.replayed && <span className="aur-badge aur-badge--info">{t(locale, 'notifReplay')}</span>}
          </span>
        </div>
      ))}
    </div>
  );
}

export function NotificationsPage({ ctx }: { ctx: PageCtx }) {
  const { locale } = ctx;
  const [entries, setEntries] = useState<ActionLogEntry[]>(() => loadActionLog());
  useEffect(() => {
    const tick = (): void => setEntries(loadActionLog());
    window.addEventListener('focus', tick);
    window.addEventListener('storage', tick);
    return () => {
      window.removeEventListener('focus', tick);
      window.removeEventListener('storage', tick);
    };
  }, []);
  return (
    <Card title={t(locale, 'notifTitle')} note={t(locale, 'notifLocalNote')}>
      <NotificationsView entries={entries} locale={locale} />
      <button type="button" className="aur-btn aur-btn--ghost aur-btn--sm" disabled={entries.length === 0} onClick={() => { clearActionLog(); setEntries([]); }}>
        {t(locale, 'notifClear')}
      </button>
    </Card>
  );
}

/* ── Page 30 — help / FAQ ─────────────────────────────────────────────── */

export function HelpPage({ ctx }: { ctx: PageCtx }) {
  const { locale, mode } = ctx;
  const staff = mode.kind === 'staff';
  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'helpBotCommands')}>
        <div className="flex flex-col gap-1">
          {BOT_COMMANDS.map((c) => (
            <Kv key={c.cmd} label={c.cmd} ltr>
              {botCommandHelp(c, locale)}
            </Kv>
          ))}
        </div>
      </Card>
      {staff && (
        <Card title={t(locale, 'helpBotAdminCommands')}>
          <div className="flex flex-col gap-1">
            {BOT_ADMIN_COMMANDS.map((c) => (
              <Kv key={c.cmd} label={c.cmd} ltr>
                {botCommandHelp(c, locale)}
              </Kv>
            ))}
          </div>
        </Card>
      )}
      <Card title={t(locale, 'helpFaq')}>
        <div className="flex flex-col gap-3">
          {FAQ.map((item, i) => (
            <div key={i} className="aur-receipt">
              <strong>{locale === 'fa' ? item.q.fa : item.q.en}</strong>
              <span className="text-sm text-aur-muted">{locale === 'fa' ? item.a.fa : item.a.en}</span>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}

/* ── Page 31 — offline shell state (hook lives in ui.tsx) ─────────────── */

export function OfflinePage({ ctx }: { ctx: PageCtx }) {
  const { locale } = ctx;
  const online = useOnline();
  return (
    <Card title={online ? t(locale, 'onlineBack') : t(locale, 'offlineTitle')} note={t(locale, 'offlineHint')}>
      <span className="text-sm text-aur-muted">{online ? t(locale, 'onlineBack') : t(locale, 'offlineBody')}</span>
      {!online && (
        <button type="button" className="aur-btn aur-btn--primary" onClick={() => window.location.reload()}>
          {t(locale, 'offlineRetry')}
        </button>
      )}
    </Card>
  );
}

/* ── Page 32 — error / 404 ────────────────────────────────────────────── */

export function NotFoundPage({ ctx, hash }: { ctx: PageCtx; hash?: string | undefined }) {
  const { locale } = ctx;
  return (
    <Card title={t(locale, 'notFoundTitle')}>
      <span className="text-sm text-aur-muted">{t(locale, 'notFoundBody')}</span>
      {hash !== undefined && hash !== '' && (
        <code className="text-xs" dir="ltr">
          {hash}
        </code>
      )}
      <a className="aur-btn aur-btn--primary" href={pageHref('home')}>
        {t(locale, 'notFoundHome')}
      </a>
    </Card>
  );
}

/* ── Page 33 — about / brand ──────────────────────────────────────────── */

export function AboutPage({ ctx }: { ctx: PageCtx }) {
  const { client, locale } = ctx;
  const brand = useQuery({ queryKey: ['w-brand'], queryFn: async () => unwrap(await client.reads.brand()), staleTime: 60_000 });
  const health = useQuery({ queryKey: ['w-health'], queryFn: async () => unwrap(await client.reads.health()), staleTime: 60_000 });
  const name = brand.isSuccess ? brand.data.data.name : t(locale, 'appName');
  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'aboutTitle')}>
        <span className="text-sm">{t(locale, 'aboutBody')}</span>
        {brand.isSuccess && <span className="text-sm text-aur-muted">{brand.data.data.tagline}</span>}
        {health.isSuccess && (
          <Kv label={t(locale, 'aboutServerVersion')} ltr>
            <code className="text-xs">{health.data.version}</code>
          </Kv>
        )}
        <span className="text-xs text-aur-faint">{t(locale, 'aboutDesign')}</span>
        <div className="flex flex-wrap gap-2">
          <a className="aur-btn aur-btn--ghost" href={pageHref('charter')}>
            {t(locale, 'aboutCharterLink')}
          </a>
          <a className="aur-btn aur-btn--ghost" href="/admin/">
            {t(locale, 'aboutOpenDashboard')}
          </a>
          <a className="aur-btn aur-btn--ghost" href="/site/">
            {t(locale, 'aboutOpenSite')}
          </a>
        </div>
        <span className="text-xs text-aur-faint">{name}</span>
      </Card>
    </div>
  );
}
