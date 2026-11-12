/**
 * Pages 14–16 — wallet, transaction history, transaction detail
 * (map §2.4 rows 14–16). All three ride PUBLIC transparency routes only:
 *  • wallet — `GET /accounts/{id}/balance` for any account id; inside
 *    Telegram the id defaults to the account convention `user:tg:<id>`
 *  • history — derived from the PUBLIC escrow list (funded/settle tx ids);
 *    the contract has no direct citizen tx route, and the page SAYS so
 *    (txDerivedNote) instead of pretending otherwise
 *  • detail — one escrow's transaction ids with the settlement context
 */
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t } from '@agent-one/i18n/webapp';
import type { PageCtx } from '../ctx.js';
import { userAccount } from '../session.js';
import { formatDate } from '../format.js';
import { pageHref } from '../router.js';
import { Card, EmptyState, ErrorState, Field, Kv, Money, PendingState, SplitPreview, StateBadge } from '../ui.js';

export default function WalletPage({ ctx }: { ctx: PageCtx }) {
  const { client, locale, tgUser } = ctx;
  const defaultAccount = tgUser !== null ? userAccount(tgUser.id) : '';
  const [account, setAccount] = useState(defaultAccount);
  const [lookup, setLookup] = useState<string | null>(defaultAccount !== '' ? defaultAccount : null);
  const balance = useQuery({
    queryKey: ['w-balance', lookup],
    queryFn: async () => unwrap(await client.reads.balance(lookup as string)),
    enabled: lookup !== null && lookup.length > 3,
    retry: 0,
    staleTime: 15_000,
  });

  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'walletTitle')}>
        <Field
          id="w-wallet-account"
          label={tgUser !== null ? t(locale, 'walletYourAccount') : t(locale, 'walletAnyAccount')}
          hint={tgUser !== null ? t(locale, 'walletFromTelegram') : t(locale, 'buySellerIdHint')}
        >
          <input
            id="w-wallet-account"
            className="aur-input"
            dir="ltr"
            value={account}
            placeholder="user:tg:123"
            onChange={(e) => setAccount(e.target.value)}
          />
        </Field>
        <button type="button" className="aur-btn aur-btn--primary" disabled={account.trim().length <= 3} onClick={() => setLookup(account.trim())}>
          {t(locale, 'walletLookup')}
        </button>
      </Card>
      {lookup !== null && (
        <Card>
          {balance.isPending && <PendingState locale={locale} />}
          {balance.isError && <ErrorState locale={locale} error={balance.error} onRetry={() => void balance.refetch()} />}
          {balance.isSuccess && (
            <Kv label={t(locale, 'colBalance')} ltr>
              <span dir="ltr">
                <code className="text-xs">{balance.data.accountId}</code> ·{' '}
                <Money amount={balance.data.balance} hitl={false} />
              </span>
            </Kv>
          )}
        </Card>
      )}
    </div>
  );
}

interface TxRow {
  escrowId: string;
  kind: 'fund' | 'settle';
  txId: string;
  amount: number;
  status: string;
  createdAt: string;
}

/** Derived rows — public escrows are the only public tx source; honest. */
function deriveTxRows(escrows: { id: string; amount: number; status: string; createdAt: string; fundedTxId: string | null; settleTxId: string | null }[]): TxRow[] {
  const rows: TxRow[] = [];
  for (const e of escrows) {
    if (e.fundedTxId !== null) {
      rows.push({ escrowId: e.id, kind: 'fund', txId: e.fundedTxId, amount: e.amount, status: e.status, createdAt: e.createdAt });
    }
    if (e.settleTxId !== null) {
      rows.push({ escrowId: e.id, kind: 'settle', txId: e.settleTxId, amount: e.amount, status: e.status, createdAt: e.createdAt });
    }
  }
  return rows;
}

export function TxHistoryPage({ ctx }: { ctx: PageCtx }) {
  const { client, locale, tgUser, mode, openChat } = ctx;
  const [mine, setMine] = useState(true);
  const [lookup, setLookup] = useState('');
  const staff = mode.kind === 'staff';
  const escrows = useQuery({
    queryKey: ['w-escrows', 'tx-derive'],
    queryFn: async () => unwrap(await client.reads.escrows()),
    enabled: staff,
    retry: 0,
    staleTime: 15_000,
  });

  const LookupCard = (
    <Card note={t(locale, 'txLookupNote')}>
      <div className="flex gap-2">
        <input
          className="aur-input"
          dir="ltr"
          aria-label={t(locale, 'escrowRefLabel')}
          placeholder="esc:…"
          value={lookup}
          onChange={(e) => setLookup(e.target.value)}
        />
        <button
          type="button"
          className="aur-btn aur-btn--primary"
          disabled={lookup.trim().length < 3}
          onClick={() => ctx.navigate('txDetail', [lookup.trim()])}
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
        <Card title={t(locale, 'txTitle')} note={t(locale, 'txDerivedNote')}>
          <span className="text-sm text-aur-muted">{t(locale, 'escrowsStaffNote')}</span>
        </Card>
        {LookupCard}
      </div>
    );
  }

  if (escrows.isPending) return <PendingState locale={locale} />;
  if (escrows.isError) return <ErrorState locale={locale} error={escrows.error} onRetry={() => void escrows.refetch()} />;

  const myAccount = tgUser !== null ? userAccount(tgUser.id) : null;
  let list = escrows.data.items;
  if (mine && myAccount !== null) list = list.filter((e) => e.buyerId === myAccount || e.sellerId === myAccount);
  if (mine && myAccount === null) list = [];
  const rows = deriveTxRows(list);

  return (
    <div className="flex flex-col gap-4">
      <Card note={t(locale, 'txDerivedNote')}>
        <div className="aur-filters" role="group">
          <button type="button" className="aur-nav__link" aria-pressed={mine} onClick={() => setMine(true)}>
            {t(locale, 'mineFilter')}
          </button>
          <button type="button" className="aur-nav__link" aria-pressed={!mine} onClick={() => setMine(false)}>
            {t(locale, 'filterAll')}
          </button>
        </div>
      </Card>
      <Card title={t(locale, 'txTitle')}>
        {rows.length === 0 ? (
          <EmptyState locale={locale} note={t(locale, 'txEmpty')} />
        ) : (
          <div className="flex flex-col gap-2">
            {rows.map((r) => (
              <a key={`${r.escrowId}-${r.kind}`} className="aur-kv" href={pageHref('txDetail', [r.escrowId])}>
                <span className="aur-kv__label">
                  <span className="aur-badge">{r.kind === 'fund' ? t(locale, 'txFundLabel') : t(locale, 'txSettleLabel')}</span>{' '}
                  <code className="text-xs" dir="ltr">
                    {r.txId}
                  </code>
                </span>
                <span className="aur-kv__value">
                  <Money amount={r.amount} hitl={false} />
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

export function TxDetailPage({ ctx, id }: { ctx: PageCtx; id?: string | undefined }) {
  const { client, locale } = ctx;
  const escrow = useQuery({
    queryKey: ['w-escrow', id, 'tx-detail'],
    queryFn: async () => unwrap(await client.reads.escrow(id as string)),
    enabled: id !== undefined,
    retry: 0,
    staleTime: 10_000,
  });
  if (escrow.isPending) return <PendingState locale={locale} />;
  if (escrow.isError) return <ErrorState locale={locale} error={escrow.error} onRetry={() => void escrow.refetch()} />;
  const e = escrow.data;
  const rows = deriveTxRows([e]).filter((r) => r.escrowId === e.id);
  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'txDetailTitle')}>
        <Kv label={t(locale, 'txEscrowContext')} ltr>
          <a href={pageHref('escrow', [e.id])}>
            <code className="text-xs">{e.id}</code>
          </a>{' '}
          <StateBadge locale={locale} state={e.status} />
        </Kv>
        {rows.length === 0 ? (
          <EmptyState locale={locale} note={t(locale, 'txEmpty')} />
        ) : (
          rows.map((r) => (
            <div key={r.txId} className="aur-receipt">
              <span className="aur-receipt__head">
                <span>{r.kind === 'fund' ? t(locale, 'txFundLabel') : t(locale, 'txSettleLabel')}</span>
                <span>{formatDate(r.createdAt, locale)}</span>
              </span>
              <Kv label={t(locale, 'txIdLabel')} ltr>
                <code className="text-xs">{r.txId}</code>
              </Kv>
              <Kv label={t(locale, 'colAmount')}>
                <Money amount={r.amount} hitl={false} />
              </Kv>
              {r.kind === 'settle' && (e.status === 'RELEASED' || e.status === 'DELIVERED') && (
                <SplitPreview locale={locale} amount={r.amount} />
              )}
            </div>
          ))
        )}
      </Card>
    </div>
  );
}
