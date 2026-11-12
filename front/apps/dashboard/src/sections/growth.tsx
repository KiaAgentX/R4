/**
 * Section: Growth — read-only gen2 views (STEP1-AUDIT §2.5):
 * `GET /quests`, `GET /bookings`, `GET /leaderboard`. Quests/bookings have
 * opaque `{[key]: unknown}` contract shapes — they render RAW (nothing
 * fabricated); the leaderboard is fully typed (xp / sales / ref tabs).
 */
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t, type Locale } from '@agent-one/i18n/dashboard';
import type { AgentOneClient } from '@agent-one/api-client';
import type { SectionProps } from '../props.js';
import { Card, EmptyState, ErrorState, FilterChips, PendingState, RawItemsTable } from '../ui.js';

type LeaderboardType = 'xp' | 'sales' | 'ref';
const LEADERBOARD_TYPES: readonly LeaderboardType[] = ['xp', 'sales', 'ref'];

async function fetchRawItems(client: AgentOneClient, path: '/quests' | '/bookings') {
  return unwrap(await client.api.GET(path));
}

export function LeaderboardView({
  locale,
  type,
  items,
  onTypeChange,
}: {
  locale: Locale;
  type: LeaderboardType;
  items: {
    rank: number;
    name: string;
    username: string | null;
    xp: number;
    sales: number;
    refs: number;
  }[];
  onTypeChange: (next: LeaderboardType) => void;
}) {
  return (
    <Card title={t(locale, 'leaderboardTitle')}>
      <FilterChips
        options={LEADERBOARD_TYPES}
        value={type}
        onChange={onTypeChange}
        labels={(option) =>
          option === 'xp' ? t(locale, 'lbTypeXp') : option === 'sales' ? t(locale, 'lbTypeSales') : t(locale, 'lbTypeRef')
        }
      />
      {items.length === 0 ? (
        <EmptyState locale={locale} />
      ) : (
        <div style={{ overflowX: 'auto' }}>
          <table className="aur-table">
            <thead>
              <tr>
                <th scope="col">{t(locale, 'colRank')}</th>
                <th scope="col">{t(locale, 'colName')}</th>
                <th scope="col">{t(locale, 'colXp')}</th>
                <th scope="col">{t(locale, 'lbTypeSales')}</th>
                <th scope="col">{t(locale, 'colRefs')}</th>
              </tr>
            </thead>
            <tbody>
              {items.map((row) => (
                <tr key={`${row.rank}-${row.name}`}>
                  <td>
                    <span className="aur-badge">{row.rank}</span>
                  </td>
                  <td>
                    {row.name}
                    {row.username !== null && <span className="text-aur-faint text-xs"> @{row.username}</span>}
                  </td>
                  <td>{row.xp}</td>
                  <td>{row.sales}</td>
                  <td>{row.refs}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </Card>
  );
}

export default function GrowthSection({ client, locale }: SectionProps) {
  const [lbType, setLbType] = useState<LeaderboardType>('xp');

  const quests = useQuery({
    queryKey: ['dash-quests'],
    queryFn: () => fetchRawItems(client, '/quests'),
    staleTime: 60_000,
  });
  const bookings = useQuery({
    queryKey: ['dash-bookings'],
    queryFn: () => fetchRawItems(client, '/bookings'),
    staleTime: 60_000,
  });
  const leaderboard = useQuery({
    queryKey: ['dash-leaderboard', lbType],
    queryFn: async () =>
      unwrap(await client.api.GET('/leaderboard', { params: { query: { type: lbType, limit: 10 } } })),
    staleTime: 60_000,
  });

  if (quests.isError) {
    return <ErrorState locale={locale} error={quests.error} onRetry={() => { void quests.refetch(); }} />;
  }
  if (bookings.isError) {
    return <ErrorState locale={locale} error={bookings.error} onRetry={() => { void bookings.refetch(); }} />;
  }
  if (leaderboard.isError) {
    return <ErrorState locale={locale} error={leaderboard.error} onRetry={() => { void leaderboard.refetch(); }} />;
  }
  if (quests.isPending || bookings.isPending || leaderboard.isPending) {
    return <PendingState locale={locale} />;
  }

  return (
    <div className="flex flex-col gap-4">
      <LeaderboardView locale={locale} type={lbType} items={leaderboard.data.data.items} onTypeChange={setLbType} />

      <Card title={t(locale, 'questsTitle')} note={t(locale, 'gen2RawNote')}>
        {quests.data.data.items.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <RawItemsTable items={quests.data.data.items as Record<string, unknown>[]} />
        )}
      </Card>

      <Card title={t(locale, 'bookingsTitle')} note={t(locale, 'gen2RawNote')}>
        {bookings.data.data.items.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <RawItemsTable items={bookings.data.data.items as Record<string, unknown>[]} />
        )}
      </Card>
    </div>
  );
}
