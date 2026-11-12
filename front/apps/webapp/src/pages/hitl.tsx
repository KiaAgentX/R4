/**
 * Page 13 — HITL status (map §2.4 row 13). The queue itself is a staff
 * surface on the frozen contract (`GET /hitl` is admin-keyed), so the page
 * follows the D14 ladder honestly:
 *   staff key → the real queue view (art. 7 rows with the 90/10 preview)
 *   citizen   → guidance + "ask the assistant" (the chat can report status)
 *   guest     → honest read-only note
 */
import { useQuery } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t } from '@agent-one/i18n/webapp';
import { requiresHumanApproval } from '@agent-one/ui';
import type { PageCtx } from '../ctx.js';
import { formatDate } from '../format.js';
import { pageHref } from '../router.js';
import { Card, EmptyState, ErrorState, Kv, Money, PendingState, SplitPreview } from '../ui.js';

interface HitlRow {
  id: string;
  kind: 'ESCROW_RELEASE';
  escrowId: string;
  amount: number;
  requesterId: string;
  note: string | null;
  status: 'PENDING' | 'APPROVED' | 'REJECTED' | 'EXPIRED';
  createdAt: string;
  expiresAt: string;
  decidedBy: string | null;
  decidedAt: string | null;
  decisionNote: string | null;
}

const STATUS_LABEL = {
  PENDING: 'approvalPending',
  APPROVED: 'approvalApproved',
  REJECTED: 'approvalRejected',
  EXPIRED: 'approvalExpired',
} as const satisfies Record<HitlRow['status'], Parameters<typeof t>[1]>;

export default function HitlStatusPage({ ctx }: { ctx: PageCtx }) {
  const { client, locale, mode, openChat } = ctx;
  const staff = mode.kind === 'staff';
  const queue = useQuery({
    queryKey: ['w-hitl'],
    queryFn: async () => unwrap(await client.api.GET('/hitl', { params: { query: { status: 'PENDING' } } })),
    enabled: staff,
    retry: 0,
    staleTime: 15_000,
  });

  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'hitlStatusTitle')} note={t(locale, 'hitlBody')}>
        {!staff && (
          <>
            <span className="text-sm text-aur-muted">{t(locale, 'hitlCitizenNote')}</span>
            {mode.kind === 'citizen' && (
              <button type="button" className="aur-btn aur-btn--primary" onClick={() => openChat(t(locale, 'hitlAskChat'))}>
                {t(locale, 'hitlAskChat')}
              </button>
            )}
          </>
        )}
      </Card>

      {staff && (
        <Card title={t(locale, 'hitlStaffView')}>
          {queue.isPending && <PendingState locale={locale} />}
          {queue.isError && <ErrorState locale={locale} error={queue.error} onRetry={() => void queue.refetch()} />}
          {queue.isSuccess &&
            (queue.data.items.length === 0 ? (
              <EmptyState locale={locale} />
            ) : (
              <div className="flex flex-col gap-3">
                {queue.data.items.map((h: HitlRow, i) => (
                  <div key={h.id} className="aur-receipt">
                    <span className="aur-receipt__head">
                      <span>
                        {t(locale, 'hitlPosition')}: {i + 1}
                      </span>
                      <span className="aur-badge">{t(locale, STATUS_LABEL[h.status])}</span>
                    </span>
                    <Kv label={t(locale, 'colId')} ltr>
                      <code className="text-xs">{h.id}</code>
                    </Kv>
                    <Kv label={t(locale, 'escrowRefLabel')} ltr>
                      <a href={pageHref('escrow', [h.escrowId])}>
                        <code className="text-xs">{h.escrowId}</code>
                      </a>
                    </Kv>
                    <Kv label={t(locale, 'colAmount')}>
                      <Money amount={h.amount} hitl={requiresHumanApproval(h.amount)} />
                    </Kv>
                    {h.status === 'PENDING' && <SplitPreview locale={locale} amount={h.amount} />}
                    <Kv label={t(locale, 'requesterLabel')} ltr>
                      <code className="text-xs">{h.requesterId}</code>
                    </Kv>
                    <Kv label={t(locale, 'expiresLabel')}>{formatDate(h.expiresAt, locale)}</Kv>
                  </div>
                ))}
              </div>
            ))}
        </Card>
      )}
    </div>
  );
}
