/**
 * Section: Orders — `GET /orders` + `POST /orders/{id}/status` with the gen2
 * order vocabulary preserved verbatim (STEP1-AUDIT §2.5), plus the Users and
 * Wallets views (`GET /users`, `GET /wallets`). Order actions are two-step
 * armed — a recorded status change must never be a one-click accident.
 */
import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t, type Locale } from '@agent-one/i18n/dashboard';
import type { AgentOneClient } from '@agent-one/api-client';
import type { SectionProps } from '../props.js';
import {
  ORDER_ACTIONS,
  ORDER_STATUS_FILTERS,
  formatDate,
  orderActionLabel,
  orderStatusLabel,
  type OrderAction,
  type OrderStatusFilter,
} from '../format.js';
import {
  ArmButton,
  Card,
  EmptyState,
  ErrorState,
  FilterChips,
  Money,
  PendingState,
  errorText,
} from '../ui.js';

async function fetchOrders(client: AgentOneClient, status: OrderStatusFilter) {
  return unwrap(
    await client.api.GET('/orders', { params: { query: { status, limit: 50 } } }),
  );
}

function OrderActions({
  locale,
  orderId,
  busy,
  onAction,
}: {
  locale: Locale;
  orderId: string;
  busy: boolean;
  onAction: (orderId: string, action: OrderAction) => void;
}) {
  return (
    <div className="flex flex-wrap gap-2">
      {ORDER_ACTIONS.map((action) => (
        <ArmButton
          key={action}
          label={orderActionLabel(locale, action)}
          armedLabel={t(locale, 'orderActionConfirm')}
          variant={action === 'refund' ? 'danger' : 'ghost'}
          disabled={busy}
          ariaLabel={`${orderActionLabel(locale, action)}: ${orderId}`}
          onConfirm={() => onAction(orderId, action)}
        />
      ))}
    </div>
  );
}

export default function OrdersSection({ client, locale }: SectionProps) {
  const queryClient = useQueryClient();
  const [status, setStatus] = useState<OrderStatusFilter>('all');

  const orders = useQuery({
    queryKey: ['dash-orders', status],
    queryFn: () => fetchOrders(client, status),
    staleTime: 15_000,
  });
  const users = useQuery({
    queryKey: ['dash-users'],
    queryFn: async () => unwrap(await client.api.GET('/users', { params: { query: { limit: 100 } } })),
    staleTime: 60_000,
  });
  const wallets = useQuery({
    queryKey: ['dash-wallets'],
    queryFn: async () => unwrap(await client.api.GET('/wallets', { params: { query: { limit: 100 } } })),
    staleTime: 60_000,
  });

  const action = useMutation({
    mutationFn: async (input: { id: string; action: OrderAction }) =>
      unwrap(
        await client.api.POST('/orders/{id}/status', {
          params: { path: { id: input.id } },
          body: { action: input.action },
        }),
      ),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['dash-orders'] });
      void queryClient.invalidateQueries({ queryKey: ['dash-overview'] });
    },
  });

  if (orders.isPending) return <PendingState locale={locale} />;
  if (orders.isError) {
    return <ErrorState locale={locale} error={orders.error} onRetry={() => { void orders.refetch(); }} />;
  }

  const items = orders.data.data.items;
  const userList = users.data?.data.items ?? [];
  const walletList = wallets.data?.data.items ?? [];

  return (
    <div className="flex flex-col gap-4">
      <Card
        title={t(locale, 'navOrders')}
        note={action.error !== undefined ? errorText(locale, action.error) : undefined}
      >
        <FilterChips
          options={ORDER_STATUS_FILTERS}
          value={status}
          onChange={setStatus}
          labels={(s) => (s === 'all' ? t(locale, 'filterAll') : orderStatusLabel(locale, s))}
        />

        {items.length === 0 ? (
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
                  <th scope="col">{t(locale, 'colActions')}</th>
                </tr>
              </thead>
              <tbody>
                {items.map((o) => (
                  <tr key={o.id}>
                    <td dir="ltr">
                      <code className="text-xs">{o.id}</code>
                    </td>
                    <td>{o.product}</td>
                    <td>{o.buyer.name ?? o.buyer.username ?? '—'}</td>
                    <td>
                      <span className="aur-money">
                        {o.totalStars} ★ / {o.totalCredits} cr
                      </span>
                    </td>
                    <td>
                      <span className="aur-badge">{orderStatusLabel(locale, o.status)}</span>
                    </td>
                    <td>{formatDate(o.createdAt, locale)}</td>
                    <td>
                      <OrderActions
                        locale={locale}
                        orderId={o.id}
                        busy={action.isPending}
                        onAction={(id, act) => action.mutate({ id, action: act })}
                      />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>

      <Card title={t(locale, 'usersCard')}>
        {userList.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table className="aur-table">
              <thead>
                <tr>
                  <th scope="col">{t(locale, 'colId')}</th>
                  <th scope="col">{t(locale, 'colName')}</th>
                  <th scope="col">{t(locale, 'colRole')}</th>
                  <th scope="col">{t(locale, 'colLanguage')}</th>
                  <th scope="col">{t(locale, 'colDate')}</th>
                </tr>
              </thead>
              <tbody>
                {userList.map((u) => (
                  <tr key={u.id}>
                    <td dir="ltr">
                      <code className="text-xs">{u.id}</code>
                    </td>
                    <td>
                      {u.name}
                      {u.username !== null && <span className="text-aur-faint text-xs"> @{u.username}</span>}
                    </td>
                    <td>
                      <span className="aur-badge">{u.role}</span>
                    </td>
                    <td>{u.language ?? '—'}</td>
                    <td>{formatDate(u.createdAt, locale)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>

      <Card title={t(locale, 'walletsCard')}>
        {walletList.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table className="aur-table">
              <thead>
                <tr>
                  <th scope="col">{t(locale, 'colName')}</th>
                  <th scope="col">{t(locale, 'colRole')}</th>
                  <th scope="col">{t(locale, 'colBalance')}</th>
                  <th scope="col">{t(locale, 'colLocked')}</th>
                  <th scope="col">{t(locale, 'colCredits')}</th>
                </tr>
              </thead>
              <tbody>
                {walletList.map((w) => (
                  <tr key={`${w.name}-${w.username ?? ''}`}>
                    <td>
                      {w.name}
                      {w.username !== null && <span className="text-aur-faint text-xs"> @{w.username}</span>}
                    </td>
                    <td>
                      <span className="aur-badge">{w.role}</span>
                    </td>
                    <td>
                      <Money amount={w.balance} />
                    </td>
                    <td>
                      <Money amount={w.locked} />
                    </td>
                    <td>{w.credits}</td>
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
