/**
 * Pages 24–26 — quests, bookings, leaderboard (map §2.4 rows 24–26).
 * Read-only gen2 views on PUBLIC routes — raw shapes rendered honestly
 * (gen2RawNote), nothing fabricated; the leaderboard gets the localized
 * xp/sales/refs filter.
 */
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t } from '@agent-one/i18n/webapp';
import type { PageCtx } from '../ctx.js';
import { Card, EmptyState, ErrorState, PendingState, RawItemsTable } from '../ui.js';

type LeaderboardType = 'xp' | 'sales' | 'ref';

export function QuestsPage({ ctx }: { ctx: PageCtx }) {
  const { client, locale } = ctx;
  const quests = useQuery({ queryKey: ['w-quests'], queryFn: async () => unwrap(await client.reads.quests()), staleTime: 30_000 });
  if (quests.isPending) return <PendingState locale={locale} />;
  if (quests.isError) return <ErrorState locale={locale} error={quests.error} onRetry={() => void quests.refetch()} />;
  return (
    <Card title={t(locale, 'questsTitle')} note={t(locale, 'gen2RawNote')}>
      {quests.data.data.items.length === 0 ? (
        <EmptyState locale={locale} />
      ) : (
        <RawItemsTable items={quests.data.data.items} />
      )}
    </Card>
  );
}

export function BookingsPage({ ctx }: { ctx: PageCtx }) {
  const { client, locale } = ctx;
  const bookings = useQuery({ queryKey: ['w-bookings'], queryFn: async () => unwrap(await client.reads.bookings()), staleTime: 30_000 });
  if (bookings.isPending) return <PendingState locale={locale} />;
  if (bookings.isError) return <ErrorState locale={locale} error={bookings.error} onRetry={() => void bookings.refetch()} />;
  return (
    <Card title={t(locale, 'bookingsTitle')} note={t(locale, 'gen2RawNote')}>
      {bookings.data.data.items.length === 0 ? (
        <EmptyState locale={locale} />
      ) : (
        <RawItemsTable items={bookings.data.data.items} />
      )}
    </Card>
  );
}

export function LeaderboardPage({ ctx }: { ctx: PageCtx }) {
  const { client, locale } = ctx;
  const [type, setType] = useState<LeaderboardType>('xp');
  const board = useQuery({
    queryKey: ['w-leaderboard', type],
    queryFn: async () => unwrap(await client.reads.leaderboard()),
    staleTime: 30_000,
  });
  if (board.isPending) return <PendingState locale={locale} />;
  if (board.isError) return <ErrorState locale={locale} error={board.error} onRetry={() => void board.refetch()} />;
  const items = board.data.data.items;
  const sorted = [...items].sort((a, b) => (type === 'xp' ? b.xp - a.xp : type === 'sales' ? b.sales - a.sales : b.refs - a.refs)).slice(0, 20);
  return (
    <Card title={t(locale, 'leaderboardTitle')} note={t(locale, 'gen2RawNote')}>
      <div className="aur-filters" role="group" aria-label={t(locale, 'leaderboardTitle')}>
        {(['xp', 'sales', 'ref'] as const).map((k) => (
          <button key={k} type="button" className="aur-nav__link" aria-pressed={type === k} onClick={() => setType(k)}>
            {t(locale, k === 'xp' ? 'lbTypeXp' : k === 'sales' ? 'lbTypeSales' : 'lbTypeRef')}
          </button>
        ))}
      </div>
      {sorted.length === 0 ? (
        <EmptyState locale={locale} />
      ) : (
        <div style={{ overflowX: 'auto' }}>
          <table className="aur-table">
            <thead>
              <tr>
                <th scope="col">{t(locale, 'colRank')}</th>
                <th scope="col">{t(locale, 'colName')}</th>
                <th scope="col">{t(locale, 'colXp')}</th>
                <th scope="col">{t(locale, 'salesCount')}</th>
                <th scope="col">{t(locale, 'colRefs')}</th>
              </tr>
            </thead>
            <tbody>
              {sorted.map((r) => (
                <tr key={`${r.rank}-${r.name}`}>
                  <td>{r.rank}</td>
                  <td>
                    {r.name}
                    {r.username !== null && <span className="text-aur-faint text-xs" dir="ltr"> @{r.username}</span>}
                  </td>
                  <td>{r.xp}</td>
                  <td>{r.sales}</td>
                  <td>{r.refs}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </Card>
  );
}
