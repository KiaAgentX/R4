/**
 * Pages 17–18 — my orders + order detail (map §2.4 rows 17–18). Gen2 orders
 * live on ADMIN routes of the frozen contract, so the D14 ladder applies:
 * with a staff key the real orders render (gen2 vocabulary preserved);
 * without one the page says so honestly and links to the admin dashboard —
 * it never renders fake orders.
 */
import { useQuery } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t } from '@agent-one/i18n/webapp';
import type { PageCtx } from '../ctx.js';
import { formatDate, orderStatusLabel } from '../format.js';
import { pageHref } from '../router.js';
import { Card, EmptyState, ErrorState, Kv, PendingState } from '../ui.js';

interface OrderRow {
  id: string;
  product: string;
  buyer: { name: string | null; username: string | null };
  totalStars: number;
  totalCredits: number;
  status: string;
  createdAt: string;
}

function useOrders(ctx: PageCtx, enabled: boolean) {
  return useQuery({
    queryKey: ['w-orders'],
    queryFn: async () => unwrap(await ctx.client.api.GET('/orders')),
    enabled,
    retry: 0,
    staleTime: 15_000,
  });
}

export function OrdersStaffList({ ctx }: { ctx: PageCtx }) {
  const { locale } = ctx;
  const orders = useOrders(ctx, ctx.mode.kind === 'staff');
  if (orders.isPending) return <PendingState locale={locale} />;
  if (orders.isError) return <ErrorState locale={locale} error={orders.error} onRetry={() => void orders.refetch()} />;
  const items = orders.data.data.items as OrderRow[];
  if (items.length === 0) return <EmptyState locale={locale} note={t(locale, 'ordersEmptyStaff')} />;
  return (
    <div className="flex flex-col gap-2">
      {items.map((o) => (
        <a key={o.id} className="aur-kv" href={pageHref('order', [o.id])}>
          <span className="aur-kv__label">
            {o.product} · <span className="aur-badge">{orderStatusLabel(locale, o.status)}</span>
          </span>
          <span className="aur-kv__value">
            {o.totalStars} ★ · {o.totalCredits} ¢
          </span>
        </a>
      ))}
    </div>
  );
}

export default function OrdersPage({ ctx }: { ctx: PageCtx }) {
  const { locale, mode } = ctx;
  return (
    <div className="flex flex-col gap-4">
      {mode.kind === 'staff' ? (
        <Card title={t(locale, 'navOrders')}>
          <OrdersStaffList ctx={ctx} />
        </Card>
      ) : (
        <Card title={t(locale, 'navOrders')} note={t(locale, 'ordersStaffNote')}>
          <span className="text-sm text-aur-muted">{t(locale, 'ordersStaffNote')}</span>
          <a className="aur-btn aur-btn--ghost" href="/admin/">
            {t(locale, 'orderOpenDashboard')}
          </a>
        </Card>
      )}
    </div>
  );
}

export function OrderDetailPage({ ctx, id }: { ctx: PageCtx; id?: string | undefined }) {
  const { locale, mode } = ctx;
  const orders = useOrders(ctx, mode.kind === 'staff');
  if (mode.kind !== 'staff') {
    return (
      <Card title={t(locale, 'orderDetailTitle')} note={t(locale, 'ordersStaffNote')}>
        <span className="text-sm text-aur-muted">{t(locale, 'ordersStaffNote')}</span>
        <a className="aur-btn aur-btn--ghost" href="/admin/">
          {t(locale, 'orderOpenDashboard')}
        </a>
      </Card>
    );
  }
  if (orders.isPending) return <PendingState locale={locale} />;
  if (orders.isError) return <ErrorState locale={locale} error={orders.error} onRetry={() => void orders.refetch()} />;
  const o = (orders.data.data.items as OrderRow[]).find((x) => x.id === id);
  if (o === undefined) {
    return (
      <Card title={t(locale, 'orderDetailTitle')}>
        <EmptyState locale={locale} note={t(locale, 'ordersEmptyStaff')} />
      </Card>
    );
  }
  return (
    <Card title={t(locale, 'orderDetailTitle')}>
      <Kv label={t(locale, 'colId')} ltr>
        <code className="text-xs">{o.id}</code>
      </Kv>
      <Kv label={t(locale, 'colProduct')}>{o.product}</Kv>
      <Kv label={t(locale, 'colBuyer')}>{o.buyer.name ?? o.buyer.username ?? '—'}</Kv>
      <Kv label={t(locale, 'revenueStars')}>{o.totalStars} ★</Kv>
      <Kv label={t(locale, 'revenueCredits')}>{o.totalCredits} ¢</Kv>
      <Kv label={t(locale, 'colStatus')}>{orderStatusLabel(locale, o.status)}</Kv>
      <Kv label={t(locale, 'colDate')}>{formatDate(o.createdAt, locale)}</Kv>
    </Card>
  );
}
