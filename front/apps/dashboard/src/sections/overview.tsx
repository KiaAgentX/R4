/**
 * Section: Overview — `GET /overview` + `GET /brand` (STEP1-AUDIT §2.5).
 * Command-center tiles, the 7-day revenue series, top products and the
 * recent-orders feed. Read-only — the overview is the server's truth (D6).
 */
import { useQuery } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t, type Locale } from '@agent-one/i18n';
import type { AgentOneClient } from '@agent-one/api-client';
import type { SectionProps } from '../props.js';
import { formatDate, orderStatusLabel } from '../format.js';
import { BarChart, Card, EmptyState, ErrorState, PendingState, Tile } from '../ui.js';

export async function fetchOverview(client: AgentOneClient) {
  return unwrap(await client.api.GET('/overview'));
}

export type OverviewData = Awaited<ReturnType<typeof fetchOverview>>['data'];

export function OverviewView({
  locale,
  data,
  brandName,
}: {
  locale: Locale;
  data: OverviewData;
  brandName: string | null;
}) {
  return (
    <div className="flex flex-col gap-4">
      {brandName !== null && brandName.length > 0 && (
        <p className="text-aur-faint text-sm">
          {brandName} — {t(locale, 'dashboardSubtitle')}
        </p>
      )}
      <div className="aur-tiles">
        <Tile label={t(locale, 'tileUsers')} value={data.users} />
        <Tile label={t(locale, 'tileProducts')} value={data.products} />
        <Tile label={t(locale, 'tileOrders')} value={data.orders} />
        <Tile label={t(locale, 'tilePendingApprovals')} value={data.pendingApprovals} />
        <Tile label={t(locale, 'tileAiCalls')} value={data.aiCalls} />
        <Tile label={t(locale, 'revenueStars')} value={data.revenueStars} />
        <Tile label={t(locale, 'revenueCredits')} value={data.revenueCredits} />
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <Card title={t(locale, 'series7d')}>
          <BarChart
            points={data.series.map((s) => ({ label: s.date.length >= 10 ? s.date.slice(5) : s.date, value: s.revenue }))}
            ariaLabel={t(locale, 'seriesRevenue')}
          />
        </Card>
        <Card title={t(locale, 'topProducts')}>
          {data.topProducts.length === 0 ? (
            <EmptyState locale={locale} />
          ) : (
            <table className="aur-table">
              <thead>
                <tr>
                  <th scope="col">{t(locale, 'colProduct')}</th>
                  <th scope="col">{t(locale, 'salesCount')}</th>
                </tr>
              </thead>
              <tbody>
                {data.topProducts.map((p) => (
                  <tr key={p.title}>
                    <td>{p.title}</td>
                    <td>{p.salesCount}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </Card>
      </div>

      <Card title={t(locale, 'recentOrders')}>
        {data.recentOrders.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table className="aur-table">
              <thead>
                <tr>
                  <th scope="col">{t(locale, 'colId')}</th>
                  <th scope="col">{t(locale, 'colProduct')}</th>
                  <th scope="col">{t(locale, 'colBuyer')}</th>
                  <th scope="col">{t(locale, 'colTotal')}</th>
                  <th scope="col">{t(locale, 'colStatus')}</th>
                  <th scope="col">{t(locale, 'colDate')}</th>
                </tr>
              </thead>
              <tbody>
                {data.recentOrders.map((o) => (
                  <tr key={o.id}>
                    <td dir="ltr">
                      <code className="text-xs">{o.id}</code>
                    </td>
                    <td>{o.title ?? '—'}</td>
                    <td>{o.buyer.name ?? o.buyer.username ?? '—'}</td>
                    <td>
                      <span className="aur-money">
                        {o.totalStars} / {o.totalCredits}
                      </span>
                    </td>
                    <td>
                      <span className="aur-badge">{orderStatusLabel(locale, o.status)}</span>
                    </td>
                    <td>{formatDate(o.createdAt, locale)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </div>
  );
}

export default function OverviewSection({ client, locale }: SectionProps) {
  const overview = useQuery({
    queryKey: ['dash-overview'],
    queryFn: () => fetchOverview(client),
    staleTime: 15_000,
  });
  const brand = useQuery({
    queryKey: ['dash-brand'],
    queryFn: async () => unwrap(await client.reads.brand()),
    staleTime: 60_000,
  });

  if (overview.isPending) return <PendingState locale={locale} />;
  if (overview.isError) {
    return <ErrorState locale={locale} error={overview.error} onRetry={() => { void overview.refetch(); }} />;
  }
  const brandName = brand.data?.data.name ?? null;
  return <OverviewView locale={locale} data={overview.data.data} brandName={brandName} />;
}
