/**
 * Section: Approvals — the two human-approval surfaces (STEP1-AUDIT §2.5):
 *  • gen2 admin approvals — `GET /approvals` + `POST /approvals/{id}/decide`
 *  • the charter's HITL queue (art. 7) — `GET /hitl`, `GET /hitl/{id}`,
 *    `POST /hitl/{id}/approve|reject`, `POST /hitl/sweep`
 * Money honesty (§6): for a pending ESCROW_RELEASE the 90/10 settlement
 * preview (seller net / treasury tax, floor semantics) is ALWAYS visible in
 * the row — the admin decides with the real split in front of them.
 */
import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t, type Locale } from '@agent-one/i18n/dashboard';
import { escrowSplit, requiresHumanApproval } from '@agent-one/ui';
import type { AgentOneClient } from '@agent-one/api-client';
import type { SectionProps } from '../props.js';
import { approvalStatusLabel, formatDate, riskLabel } from '../format.js';
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

/** The dashboard's actor id for HITL decisions (server tests: `admin:<name>`). */
const HITL_DECIDER = 'admin:dashboard';

const APPROVAL_FILTERS = ['pending', 'approved', 'rejected', 'expired', 'all'] as const;
type ApprovalFilter = (typeof APPROVAL_FILTERS)[number];

const HITL_FILTERS = ['PENDING', 'APPROVED', 'REJECTED', 'EXPIRED'] as const;
type HitlFilter = (typeof HITL_FILTERS)[number];

type ApprovalItem = {
  id: string;
  type: string;
  riskLevel: 'low' | 'medium' | 'high';
  summary: string;
  note: string | null;
  amount: number;
  status: string;
  createdAt: string;
  expiresAt: string | null;
};

function riskBadgeClass(risk: 'low' | 'medium' | 'high'): string {
  switch (risk) {
    case 'low':
      return 'aur-badge aur-badge--brand';
    case 'medium':
      return 'aur-badge aur-badge--warning';
    case 'high':
      return 'aur-badge aur-badge--danger';
  }
}

/** The 90/10 split line — always shown for a pending release decision (§6). */
function SplitPreview({ locale, amount }: { locale: Locale; amount: number }) {
  const { sellerNet, tax } = escrowSplit(amount);
  return (
    <span className="aur-money__split">
      <span>
        {t(locale, 'sellerShare')}: <Money amount={sellerNet} />
      </span>
      <span>
        {t(locale, 'treasuryTax')}: <Money amount={tax} />
      </span>
    </span>
  );
}

export function ApprovalsView({
  locale,
  approvals,
  hitl,
  busy,
  note,
  onNote,
  onDecide,
  onHitlDecide,
  onSweep,
  sweepResult,
  settleResult,
}: {
  locale: Locale;
  approvals: ApprovalItem[];
  hitl: {
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
  }[];
  busy: boolean;
  note: string;
  onNote: (next: string) => void;
  onDecide: (id: string, approve: boolean) => void;
  onHitlDecide: (id: string, approve: boolean) => void;
  onSweep: () => void;
  sweepResult: number | null;
  settleResult: { escrowId: string | null; txId: string | null; replayed: boolean } | null;
}) {
  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'approvalsTitle')}>
        {approvals.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table className="aur-table">
              <thead>
                <tr>
                  <th scope="col">{t(locale, 'colType')}</th>
                  <th scope="col">{t(locale, 'colSummary')}</th>
                  <th scope="col">{t(locale, 'colAmount')}</th>
                  <th scope="col">{t(locale, 'colStatus')}</th>
                  <th scope="col">{t(locale, 'expiresLabel')}</th>
                  <th scope="col">{t(locale, 'colActions')}</th>
                </tr>
              </thead>
              <tbody>
                {approvals.map((a) => (
                  <tr key={a.id}>
                    <td>
                      <span className={`aur-badge ${riskBadgeClass(a.riskLevel)}`}>
                        {riskLabel(locale, a.riskLevel)}
                      </span>
                      <div className="text-aur-faint text-xs" dir="ltr">
                        {a.type}
                      </div>
                    </td>
                    <td>
                      {a.summary}
                      {a.note !== null && <div className="text-aur-faint text-xs">{a.note}</div>}
                    </td>
                    <td>
                      <Money amount={a.amount} hitl={requiresHumanApproval(a.amount)} />
                      {a.type === 'ESCROW_RELEASE' && a.status.toUpperCase() === 'PENDING' && (
                        <div>
                          <SplitPreview locale={locale} amount={a.amount} />
                        </div>
                      )}
                    </td>
                    <td>
                      <span className="aur-badge">{approvalStatusLabel(locale, a.status)}</span>
                    </td>
                    <td>{a.expiresAt !== null ? formatDate(a.expiresAt, locale) : '—'}</td>
                    <td>
                      {a.status.toUpperCase() === 'PENDING' ? (
                        <div className="flex flex-wrap gap-2">
                          <ArmButton
                            label={t(locale, 'confirm')}
                            armedLabel={t(locale, 'orderActionConfirm')}
                            disabled={busy}
                            ariaLabel={`approve: ${a.id}`}
                            onConfirm={() => onDecide(a.id, true)}
                          />
                          <ArmButton
                            label={t(locale, 'decideReject')}
                            armedLabel={t(locale, 'orderActionConfirm')}
                            variant="ghost"
                            disabled={busy}
                            ariaLabel={`reject: ${a.id}`}
                            onConfirm={() => onDecide(a.id, false)}
                          />
                        </div>
                      ) : (
                        <span className="text-aur-faint text-xs">—</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        <div className="aur-field">
          <label className="aur-label" htmlFor="ao-decision-note">
            {t(locale, 'noteLabel')}
          </label>
          <input
            id="ao-decision-note"
            className="aur-input"
            type="text"
            value={note}
            onChange={(e) => onNote(e.target.value)}
          />
        </div>
      </Card>

      <Card
        title={t(locale, 'queueTitle')}
        note={t(locale, 'hitlBody')}
      >
        {sweepResult !== null && (
          <div role="status" className="aur-card__footer">
            {t(locale, 'sweepDone')}: {sweepResult}
          </div>
        )}
        {settleResult !== null && (
          <div role="status" className="aur-card__footer">
            {t(locale, 'decidedSettle')}
            {settleResult.escrowId !== null && (
              <>
                {' '}
                · {t(locale, 'escrowRefLabel')}:{' '}
                <code className="text-xs" dir="ltr">
                  {settleResult.escrowId}
                </code>
              </>
            )}
            {settleResult.txId !== null && (
              <>
                {' '}
                · {t(locale, 'txIdLabel')}:{' '}
                <code className="text-xs" dir="ltr">
                  {settleResult.txId}
                </code>
              </>
            )}
            {settleResult.replayed && (
              <span className="aur-badge aur-badge--info">{t(locale, 'replayedBadge')}</span>
            )}
          </div>
        )}

        {hitl.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table className="aur-table">
              <thead>
                <tr>
                  <th scope="col">{t(locale, 'colId')}</th>
                  <th scope="col">{t(locale, 'escrowRefLabel')}</th>
                  <th scope="col">{t(locale, 'colAmount')}</th>
                  <th scope="col">{t(locale, 'requesterLabel')}</th>
                  <th scope="col">{t(locale, 'colStatus')}</th>
                  <th scope="col">{t(locale, 'expiresLabel')}</th>
                  <th scope="col">{t(locale, 'colActions')}</th>
                </tr>
              </thead>
              <tbody>
                {hitl.map((h) => (
                  <tr key={h.id}>
                    <td dir="ltr">
                      <code className="text-xs">{h.id}</code>
                      {h.decisionNote !== null && (
                        <div className="text-aur-faint text-xs">
                          {t(locale, 'decisionNoteLabel')}: {h.decisionNote}
                        </div>
                      )}
                    </td>
                    <td dir="ltr">
                      <code className="text-xs">{h.escrowId}</code>
                    </td>
                    <td>
                      <Money amount={h.amount} hitl={requiresHumanApproval(h.amount)} />
                      {h.status === 'PENDING' && (
                        <div>
                          <SplitPreview locale={locale} amount={h.amount} />
                        </div>
                      )}
                    </td>
                    <td dir="ltr">
                      <code className="text-xs">{h.requesterId}</code>
                    </td>
                    <td>
                      <span className="aur-badge">{approvalStatusLabel(locale, h.status)}</span>
                      {h.decidedBy !== null && (
                        <div className="text-aur-faint text-xs" dir="ltr">
                          {t(locale, 'decidedByLabel')}: {h.decidedBy}
                        </div>
                      )}
                    </td>
                    <td>{formatDate(h.expiresAt, locale)}</td>
                    <td>
                      {h.status === 'PENDING' ? (
                        <div className="flex flex-wrap gap-2">
                          <ArmButton
                            label={t(locale, 'confirm')}
                            armedLabel={t(locale, 'orderActionConfirm')}
                            disabled={busy}
                            ariaLabel={`hitl-approve: ${h.id}`}
                            onConfirm={() => onHitlDecide(h.id, true)}
                          />
                          <ArmButton
                            label={t(locale, 'decideReject')}
                            armedLabel={t(locale, 'orderActionConfirm')}
                            variant="ghost"
                            disabled={busy}
                            ariaLabel={`hitl-reject: ${h.id}`}
                            onConfirm={() => onHitlDecide(h.id, false)}
                          />
                        </div>
                      ) : (
                        <span className="text-aur-faint text-xs">—</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        <div className="flex flex-wrap gap-3">
          <ArmButton
            label={t(locale, 'sweepExpired')}
            armedLabel={t(locale, 'orderActionConfirm')}
            variant="ghost"
            disabled={busy}
            onConfirm={onSweep}
          />
        </div>
        <div className="aur-field">
          <label className="aur-label" htmlFor="ao-hitl-note">
            {t(locale, 'noteLabel')}
          </label>
          <input
            id="ao-hitl-note"
            className="aur-input"
            type="text"
            value={note}
            onChange={(e) => onNote(e.target.value)}
          />
        </div>
      </Card>
    </div>
  );
}

export default function ApprovalsSection({ client, locale }: SectionProps) {
  const queryClient = useQueryClient();
  const [approvalFilter, setApprovalFilter] = useState<ApprovalFilter>('pending');
  const [hitlFilter, setHitlFilter] = useState<HitlFilter>('PENDING');
  const [note, setNote] = useState('');
  const [sweepResult, setSweepResult] = useState<number | null>(null);
  const [settleResult, setSettleResult] = useState<{
    escrowId: string | null;
    txId: string | null;
    replayed: boolean;
  } | null>(null);

  const approvals = useQuery({
    queryKey: ['dash-approvals', approvalFilter],
    queryFn: async () =>
      unwrap(
        await client.api.GET('/approvals', {
          params: { query: { status: approvalFilter } },
        }),
      ),
    staleTime: 15_000,
  });
  const hitl = useQuery({
    queryKey: ['dash-hitl', hitlFilter],
    queryFn: async () =>
      unwrap(
        await client.api.GET('/hitl', { params: { query: { status: hitlFilter } } }),
      ),
    staleTime: 15_000,
  });

  function invalidate(): void {
    void queryClient.invalidateQueries({ queryKey: ['dash-approvals'] });
    void queryClient.invalidateQueries({ queryKey: ['dash-hitl'] });
    void queryClient.invalidateQueries({ queryKey: ['dash-overview'] });
    void queryClient.invalidateQueries({ queryKey: ['dash-economy'] });
  }

  const decide = useMutation({
    mutationFn: async (input: { id: string; approve: boolean }) =>
      unwrap(
        await client.api.POST('/approvals/{id}/decide', {
          params: { path: { id: input.id } },
          body: {
            approve: input.approve,
            ...(note.trim().length > 0 ? { note: note.trim() } : {}),
          },
        }),
      ),
    onSuccess: invalidate,
  });

  const hitlDecide = useMutation({
    mutationFn: async (input: { id: string; approve: boolean }) => {
      const body = {
        decidedBy: HITL_DECIDER,
        ...(note.trim().length > 0 ? { note: note.trim() } : {}),
      };
      const res =
        input.approve === true
          ? await client.api.POST('/hitl/{id}/approve', {
              params: { path: { id: input.id } },
              body,
            })
          : await client.api.POST('/hitl/{id}/reject', {
              params: { path: { id: input.id } },
              body,
            });
      return unwrap(res);
    },
    onSuccess: (data) => {
      setSettleResult({
        escrowId: data.escrow?.id ?? null,
        txId: data.txId,
        replayed: data.replayed,
      });
      invalidate();
    },
  });

  const sweep = useMutation({
    mutationFn: async () => unwrap(await client.api.POST('/hitl/sweep')),
    onSuccess: (data) => setSweepResult(data.expired),
  });

  if (approvals.isPending || hitl.isPending) return <PendingState locale={locale} />;
  if (approvals.isError) {
    return <ErrorState locale={locale} error={approvals.error} onRetry={() => { void approvals.refetch(); }} />;
  }
  if (hitl.isError) {
    return <ErrorState locale={locale} error={hitl.error} onRetry={() => { void hitl.refetch(); }} />;
  }

  const busy = decide.isPending || hitlDecide.isPending || sweep.isPending;

  return (
    <div className="flex flex-col gap-4">
      <FilterChips
        options={APPROVAL_FILTERS}
        value={approvalFilter}
        onChange={setApprovalFilter}
        labels={(s) => (s === 'all' ? t(locale, 'filterAll') : approvalStatusLabel(locale, s))}
      />
      <ApprovalsView
        locale={locale}
        approvals={approvals.data.data.items}
        hitl={hitl.data.items}
        busy={busy}
        note={note}
        onNote={setNote}
        onDecide={(id, approve) => decide.mutate({ id, approve })}
        onHitlDecide={(id, approve) => hitlDecide.mutate({ id, approve })}
        onSweep={() => sweep.mutate()}
        sweepResult={sweepResult}
        settleResult={settleResult}
      />
    </div>
  );
}
