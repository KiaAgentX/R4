/**
 * Pages 5–12 — the escrow lifecycle (map §2.4 rows 5–12): created/pending-fund,
 * my escrows, the 8-state detail timeline, and the five money confirms
 * (fund / deliver / release / dispute / cancel).
 *
 * Money honesty (§6): the 90/10 preview ALWAYS shows on the release confirm,
 * HITL amounts (≥ 50,000 Đ) carry the art. 7 badge, confirmations are
 * two-step, and every action goes through the D14 ladder — staff key →
 * direct idempotency-keyed transition; Telegram citizen → assistant; guest →
 * the exact bot command. No page ever fakes a mutation.
 */
import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t } from '@agent-one/i18n/webapp';
import type { PageCtx } from '../ctx.js';
import { routeAction, type EscrowAction } from '../actions.js';
import { escrowStateLabel, formatDate, STATE_ACTIONS, type EscrowActionUi, type EscrowState } from '../format.js';
import { pageHref } from '../router.js';
import {
  Card,
  EmptyState,
  ErrorState,
  Field,
  Kv,
  LadderView,
  Money,
  PendingState,
  SplitPreview,
  StateBadge,
  StateTimeline,
} from '../ui.js';

/** Wire shape of one escrow (generated schema, quoted here for props). */
export interface EscrowRow {
  id: string;
  buyerId: string;
  sellerId: string;
  amount: number;
  productRef: string | null;
  status: EscrowState;
  createdAt: string;
  fundedTxId: string | null;
  settleTxId: string | null;
  disputeReason: string | null;
  deliveryRef: string | null;
  expiresAt: string | null;
}

interface TransitionResult {
  escrow: EscrowRow;
  txId: string | null;
  replayed: boolean;
}

export function useEscrow(client: PageCtx['client'], id: string | undefined) {
  return useQuery({
    queryKey: ['w-escrow', id],
    queryFn: async () => unwrap<EscrowRow>(await client.reads.escrow(id as string)),
    enabled: id !== undefined,
    retry: 0,
    staleTime: 10_000,
  });
}

/* ── Page 6 — my escrows / public escrows ─────────────────────────────── */

/**
 * Registry truth: the escrow LIST is an admin route; only the single
 * `GET /escrows/{id}` is public. So the page follows the D14 ladder:
 * staff → the real filtered list; citizen → "ask the assistant" (the
 * escrow.status skill lists their escrows) + an id lookup; guest → the
 * honest id lookup + bot handoff. Nothing is faked either way.
 */
export default function EscrowsListPage({ ctx }: { ctx: PageCtx }) {
  const { client, locale, tgUser, mode, openChat } = ctx;
  const [mine, setMine] = useState(true);
  const [status, setStatus] = useState<string>('all');
  const [lookup, setLookup] = useState('');
  const staff = mode.kind === 'staff';
  const escrows = useQuery({
    queryKey: ['w-escrows', status],
    queryFn: async () => unwrap(await client.reads.escrows()),
    enabled: staff,
    retry: 0,
    staleTime: 15_000,
  });

  const LookupCard = (
    <Card note={t(locale, 'escrowsLookupNote')}>
      <div className="flex gap-2">
        <input
          className="aur-input"
          dir="ltr"
          aria-label={t(locale, 'hitlIdInput')}
          placeholder="esc:…"
          value={lookup}
          onChange={(e) => setLookup(e.target.value)}
        />
        <button
          type="button"
          className="aur-btn aur-btn--primary"
          disabled={lookup.trim().length < 3}
          onClick={() => ctx.navigate('escrow', [lookup.trim()])}
        >
          {t(locale, 'productVisit')}
        </button>
      </div>
      {!staff && mode.kind === 'citizen' && (
        <button type="button" className="aur-btn aur-btn--ghost" onClick={() => openChat(t(locale, 'hitlAskChat'))}>
          {t(locale, 'hitlAskChat')}
        </button>
      )}
    </Card>
  );

  if (!staff) {
    return (
      <div className="flex flex-col gap-4">
        <Card title={t(locale, 'escrowsMine')} note={mode.kind === 'guest' ? t(locale, 'escrowsStaffNote') : t(locale, 'escrowsMineNote')}>
          <span className="text-sm text-aur-muted">{t(locale, 'escrowsStaffNote')}</span>
        </Card>
        {LookupCard}
      </div>
    );
  }

  if (escrows.isPending) return <PendingState locale={locale} />;
  if (escrows.isError) return <ErrorState locale={locale} error={escrows.error} onRetry={() => void escrows.refetch()} />;

  const myAccount = tgUser !== null ? `user:tg:${tgUser.id}` : null;
  let items = escrows.data.items as EscrowRow[];
  if (mine && myAccount !== null) items = items.filter((e) => e.buyerId === myAccount || e.sellerId === myAccount);
  if (mine && myAccount === null) items = [];
  if (status !== 'all') items = items.filter((e) => e.status === status);

  return (
    <div className="flex flex-col gap-4">
      <Card note={mine ? t(locale, 'escrowsMineNote') : undefined}>
        <div className="aur-filters" role="group" aria-label={t(locale, 'escrowsMine')}>
          <button type="button" className="aur-nav__link" aria-pressed={mine} onClick={() => setMine(true)}>
            {t(locale, 'mineFilter')}
          </button>
          <button type="button" className="aur-nav__link" aria-pressed={!mine} onClick={() => setMine(false)}>
            {t(locale, 'escrowsAllPublic')}
          </button>
        </div>
        <div className="aur-filters" role="group" aria-label={t(locale, 'statusFilter')}>
          {(['all', 'CREATED', 'FUNDED', 'DELIVERED', 'RELEASED', 'DISPUTED', 'REFUNDED', 'CANCELLED', 'EXPIRED'] as const).map((s) => (
            <button key={s} type="button" className="aur-nav__link" aria-pressed={status === s} onClick={() => setStatus(s)}>
              {s === 'all' ? t(locale, 'filterAll') : escrowStateLabel(locale, s)}
            </button>
          ))}
        </div>
      </Card>
      <Card title={mine ? t(locale, 'escrowsMine') : t(locale, 'escrowsAllPublic')}>
        {items.length === 0 ? (
          <EmptyState locale={locale} note={mine && myAccount === null ? t(locale, 'profileGuest') : undefined} />
        ) : (
          <div className="flex flex-col gap-2">
            {items.map((e) => (
              <a key={e.id} className="aur-kv" href={pageHref('escrow', [e.id])}>
                <span className="aur-kv__label">
                  <code className="text-xs" dir="ltr">
                    {e.id}
                  </code>{' '}
                  <StateBadge locale={locale} state={e.status} />
                </span>
                <span className="aur-kv__value">
                  <Money amount={e.amount} />
                </span>
              </a>
            ))}
          </div>
        )}
      </Card>
      {LookupCard}
    </div>
  );
}

/* ── Escrow summary rows (shared by detail + confirms) ────────────────── */

function EscrowSummary({ locale, e }: { locale: PageCtx['locale']; e: EscrowRow }) {
  return (
    <>
      <Kv label={t(locale, 'colAmount')}>
        <Money amount={e.amount} locale={locale} />
      </Kv>
      <Kv label={t(locale, 'escrowBuyerLabel')} ltr>
        <code className="text-xs">{e.buyerId}</code>
      </Kv>
      <Kv label={t(locale, 'escrowSellerLabel')} ltr>
        <code className="text-xs">{e.sellerId}</code>
      </Kv>
      {e.productRef !== null && (
        <Kv label={t(locale, 'escrowProductRefLabel')} ltr>
          <code className="text-xs">{e.productRef}</code>
        </Kv>
      )}
      <Kv label={t(locale, 'colDate')}>{formatDate(e.createdAt, locale)}</Kv>
      {e.expiresAt !== null && (
        <Kv label={t(locale, 'expiresLabel')}>{formatDate(e.expiresAt, locale)}</Kv>
      )}
      {e.disputeReason !== null && (
        <Kv label={t(locale, 'escrowDisputeReasonLabel')}>{e.disputeReason}</Kv>
      )}
      {e.deliveryRef !== null && (
        <Kv label={t(locale, 'escrowDeliveryRefLabel')} ltr>
          <code className="text-xs">{e.deliveryRef}</code>
        </Kv>
      )}
      {e.fundedTxId !== null && (
        <Kv label={t(locale, 'txFundLabel')} ltr>
          <code className="text-xs">{e.fundedTxId}</code>
        </Kv>
      )}
      {e.settleTxId !== null && (
        <Kv label={t(locale, 'txSettleLabel')} ltr>
          <code className="text-xs">{e.settleTxId}</code>
        </Kv>
      )}
    </>
  );
}

/* ── The ladder for one escrow action (shared by all money pages) ─────── */

const ACTION_LABEL: Record<EscrowActionUi, Parameters<typeof t>[1]> = {
  fund: 'actFund',
  deliver: 'actDeliver',
  release: 'actRelease',
  dispute: 'actDispute',
  cancel: 'actCancel',
};

export function EscrowLadder({
  ctx,
  e,
  action,
  withReason = false,
}: {
  ctx: PageCtx;
  e: EscrowRow;
  action: EscrowActionUi;
  withReason?: boolean | undefined;
}) {
  const { client, locale, mode, botHandle, openChat, logAction } = ctx;
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<TransitionResult | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [reason, setReason] = useState('');

  const escrowAction: EscrowAction = action === 'cancel' ? 'cancel' : action;
  const route = routeAction({ mode, action: escrowAction, locale, escrowId: e.id, reason: reason.trim() });

  const mutate = useMutation({
    mutationFn: async (): Promise<TransitionResult> => {
      const key = `webapp-${action}-${e.id}`;
      switch (action) {
        case 'fund':
          return unwrap<TransitionResult>(await client.money.fund(e.id, {}, key));
        case 'deliver':
          return unwrap<TransitionResult>(await client.money.deliver(e.id, {}, key));
        case 'release':
          return unwrap<TransitionResult>(await client.money.release(e.id, { approvedByAdmin: true }, key));
        case 'dispute':
          return unwrap<TransitionResult>(await client.money.dispute(e.id, { reason: reason.trim().length > 0 ? reason.trim() : 'buyer dispute' }, key));
        case 'cancel':
          return unwrap<TransitionResult>(await client.money.cancel(e.id, {}, key));
      }
    },
    onSuccess: (data) => {
      setResult(data);
      logAction({ kind: 'staff', action: `escrow.${action}`, target: e.id, txId: data.txId, replayed: data.replayed });
      void queryClient.invalidateQueries({ queryKey: ['w-escrow', e.id] });
      void queryClient.invalidateQueries({ queryKey: ['w-escrows'] });
    },
  });

  return (
    <Card title={t(locale, 'howToActTitle')} note={action === 'release' ? t(locale, 'splitAlways') : undefined}>
      {action === 'release' && <SplitPreview locale={locale} amount={e.amount} />}
      {result !== null ? (
        <div className="aur-receipt" role="status">
          <span className="aur-receipt__head">
            <span>{t(locale, 'actionReceipt')}</span>
            <span>
              {t(locale, 'actionDone')} · <StateBadge locale={locale} state={result.escrow.status} />
            </span>
          </span>
          {result.txId !== null && (
            <code className="text-xs" dir="ltr">
              {result.txId}
            </code>
          )}
          {result.replayed && <span className="aur-badge aur-badge--info">{t(locale, 'replayedBadge')}</span>}
          <a className="aur-btn aur-btn--sm" href={pageHref('escrow', [e.id])}>
            {t(locale, 'escrowDetailTitle')}
          </a>
        </div>
      ) : (
        <>
          {error !== null && (
            <div role="alert" className="aur-error-state">
              {t(locale, 'actionFailed')}
            </div>
          )}
          {withReason && (
            <Field id="w-dispute-reason" label={t(locale, 'confirmDisputeReason')}>
              <input id="w-dispute-reason" className="aur-input" value={reason} onChange={(ev) => setReason(ev.target.value)} />
            </Field>
          )}
          <LadderView
            locale={locale}
            route={route}
            botHandle={botHandle}
            busy={busy || mutate.isPending}
            onStaffConfirm={() => {
              setBusy(true);
              setError(null);
              mutate.mutate(undefined, {
                onSettled: () => setBusy(false),
                onError: (err) => setError(err),
              });
            }}
            onChat={(prefill) => openChat(prefill)}
          />
        </>
      )}
    </Card>
  );
}

/* ── Page 5 — escrow created, pending fund ────────────────────────────── */

export function EscrowNewPage({ ctx, id }: { ctx: PageCtx; id?: string | undefined }) {
  const { client, locale } = ctx;
  const escrow = useEscrow(client, id);
  if (escrow.isPending) return <PendingState locale={locale} />;
  if (escrow.isError) return <ErrorState locale={locale} error={escrow.error} onRetry={() => void escrow.refetch()} />;
  const e = escrow.data;
  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'escrowNewTitle')} note={t(locale, 'escrowNewBody')}>
        <EscrowSummary locale={locale} e={e} />
        <StateTimeline locale={locale} state={e.status} />
      </Card>
      {STATE_ACTIONS[e.status].includes('fund') ? (
        <EscrowLadder ctx={ctx} e={e} action="fund" />
      ) : (
        <Card>
          <a className="aur-btn aur-btn--ghost" href={pageHref('escrow', [e.id])}>
            {t(locale, 'escrowDetailTitle')}
          </a>
        </Card>
      )}
    </div>
  );
}

/* ── Page 7 — escrow detail with the 8-state timeline ─────────────────── */

/** Presentational detail view — SSR-testable without any query. */
export function EscrowDetailView({ locale, e }: { locale: PageCtx['locale']; e: EscrowRow }) {
  const actions = STATE_ACTIONS[e.status];
  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'escrowDetailTitle')}>
        <Kv label={t(locale, 'colId')} ltr>
          <code className="text-xs">{e.id}</code>
        </Kv>
        <EscrowSummary locale={locale} e={e} />
        <StateTimeline locale={locale} state={e.status} />
      </Card>
      {actions.length > 0 && (
        <Card note={e.status === 'FUNDED' ? t(locale, 'actExpireNote') : undefined}>
          <div className="flex flex-wrap gap-2">
            {actions.map((a) => (
              <a
                key={a}
                className="aur-btn aur-btn--ghost"
                href={pageHref(
                  a === 'fund' ? 'escrowFund' : a === 'deliver' ? 'escrowDeliver' : a === 'release' ? 'escrowRelease' : a === 'dispute' ? 'escrowDispute' : 'escrowCancel',
                  [e.id],
                )}
              >
                {t(locale, ACTION_LABEL[a])}
              </a>
            ))}
          </div>
        </Card>
      )}
      <Card>
        <a className="aur-btn aur-btn--ghost" href={pageHref('tx', [e.id])}>
          {t(locale, 'txDetailTitle')}
        </a>
      </Card>
    </div>
  );
}

export function EscrowDetailPage({ ctx, id }: { ctx: PageCtx; id?: string | undefined }) {
  const { client, locale } = ctx;
  const escrow = useEscrow(client, id);
  if (escrow.isPending) return <PendingState locale={locale} />;
  if (escrow.isError) return <ErrorState locale={locale} error={escrow.error} onRetry={() => void escrow.refetch()} />;
  return <EscrowDetailView locale={locale} e={escrow.data} />;
}

/* ── Pages 8–12 — the five money confirms (one factory, five routes) ──── */

export interface ConfirmSpec {
  titleKey: Parameters<typeof t>[1];
  bodyKey: Parameters<typeof t>[1];
  action: EscrowActionUi;
  withReason?: boolean | undefined;
}

/** Presentational confirm view — SSR-testable; the ladder stays inside. */
export function EscrowConfirmView({ ctx, e, spec }: { ctx: PageCtx; e: EscrowRow; spec: ConfirmSpec }) {
  const { locale } = ctx;
  const legal = STATE_ACTIONS[e.status].includes(spec.action);
  return (
    <div className="flex flex-col gap-4">
      <Card title={`${t(locale, spec.titleKey)}`}>
        <span className="text-sm text-aur-muted">{t(locale, spec.bodyKey)}</span>
        <EscrowSummary locale={locale} e={e} />
        <StateTimeline locale={locale} state={e.status} />
      </Card>
      {legal ? (
        <EscrowLadder ctx={ctx} e={e} action={spec.action} withReason={spec.withReason === true} />
      ) : (
        <Card note={t(locale, 'actExpireNote')}>
          <EmptyState locale={locale} note={t(locale, 'actReadonly')} />
        </Card>
      )}
    </div>
  );
}

export function EscrowConfirmPage({
  ctx,
  id,
  spec,
}: {
  ctx: PageCtx;
  id?: string | undefined;
  spec: ConfirmSpec;
}) {
  const { client, locale } = ctx;
  const escrow = useEscrow(client, id);
  if (escrow.isPending) return <PendingState locale={locale} />;
  if (escrow.isError) return <ErrorState locale={locale} error={escrow.error} onRetry={() => void escrow.refetch()} />;
  return <EscrowConfirmView ctx={ctx} e={escrow.data} spec={spec} />;
}

export const CONFIRM_SPECS = {
  fund: { titleKey: 'confirmFundTitle', bodyKey: 'confirmFundBody', action: 'fund' },
  deliver: { titleKey: 'confirmDeliverTitle', bodyKey: 'confirmDeliverBody', action: 'deliver' },
  release: { titleKey: 'confirmReleaseTitle', bodyKey: 'confirmReleaseBody', action: 'release' },
  dispute: { titleKey: 'confirmDisputeTitle', bodyKey: 'confirmDisputeBody', action: 'dispute', withReason: true },
  cancel: { titleKey: 'confirmCancelTitle', bodyKey: 'confirmCancelBody', action: 'cancel' },
} as const satisfies Record<string, ConfirmSpec>;
