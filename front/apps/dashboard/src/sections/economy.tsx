/**
 * Section: Economy — `GET /economy` transparency + treasury ops
 * (`POST /wallet/mint`, `POST /wallet/burn` — the admin money mutations,
 * replay-safe via client-generated idempotency keys, decision D7).
 * Money renders through the exact domain mirror; amounts ≥ 50,000 Đ get the
 * art. 7 HITL color.
 */
import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t, type Locale } from '@agent-one/i18n/dashboard';
import { assertDin } from '@agent-one/ui';
import type { AgentOneClient } from '@agent-one/api-client';
import type { SectionProps } from '../props.js';
import { Money, Card, EmptyState, ErrorState, Meter, PendingState, Tile, errorText } from '../ui.js';
import { settleAction, treasuryActionId } from '../money.js';

export async function fetchEconomy(client: AgentOneClient) {
  return unwrap(await client.api.GET('/economy'));
}
export type EconomyData = Awaited<ReturnType<typeof fetchEconomy>>;

export interface TreasuryInput {
  accountId: string;
  amount: number;
  memo?: string;
}

interface TreasuryResult {
  txId: string;
  accountId: string;
  balance: number;
  replayed: boolean;
}

/** Shared mutation body for mint/burn — replay-safe per D7 (see money.ts). */
function treasuryMutation(client: AgentOneClient, kind: 'mint' | 'burn') {
  return async (input: TreasuryInput): Promise<TreasuryResult> => {
    const actionId = treasuryActionId(kind, input.accountId, input.amount);
    const body = {
      accountId: input.accountId,
      amount: input.amount,
      ...(input.memo !== undefined ? { memo: input.memo } : {}),
    };
    try {
      const res =
        kind === 'mint'
          ? await client.money.mint(body, actionId)
          : await client.money.burn(body, actionId);
      const data = unwrap(res);
      client.idempotency.release(actionId); // settled successfully
      return data;
    } catch (e) {
      settleAction(client, actionId, e); // release on terminal errors, keep on lost responses
      throw e;
    }
  };
}

function ResultRow({ locale, result }: { locale: Locale; result: TreasuryResult }) {
  return (
    <div className="aur-card__footer" role="status">
      {t(locale, 'txRecorded')} · {t(locale, 'txIdLabel')}:{' '}
      <code className="text-xs" dir="ltr">
        {result.txId}
      </code>{' '}
      · {t(locale, 'newBalanceLabel')}: <Money amount={result.balance} />
      {result.replayed && (
        <span className="aur-badge aur-badge--info">{t(locale, 'replayedBadge')}</span>
      )}
    </div>
  );
}

export function EconomyView({
  locale,
  economy,
}: {
  locale: Locale;
  economy: EconomyData;
}) {
  return (
    <Card title={t(locale, 'economyTitle')}>
      <div className="aur-tiles">
        <Tile label={t(locale, 'economyCirculating')} value={<Money amount={economy.circulatingSupply} />} />
        <Tile label={t(locale, 'economyTreasury')} value={<Money amount={economy.treasuryBalance} />} />
        <Tile
          label={t(locale, 'economyEscrowLocked')}
          value={<Money amount={economy.escrowLocked} />}
          sub={t(locale, 'economyOpenEscrows') + ': ' + String(economy.openEscrows)}
        />
        <Tile label={t(locale, 'economyMintedAllTime')} value={<Money amount={economy.mintedAllTime} />} />
        <Tile label={t(locale, 'economyBurnedAllTime')} value={<Money amount={economy.burnedAllTime} />} />
        <Tile label={t(locale, 'economyGenesis')} value={<Money amount={economy.genesisSupply} />} />
        <Tile
          label={t(locale, 'economyTrialBalance')}
          value={<Money amount={economy.trialBalanceSum} />}
          sub={t(locale, 'economyAccounts') + ': ' + String(economy.accountCount)}
        />
        <Tile
          label={t(locale, 'economyBalanced')}
          value={
            economy.balanced ? (
              <span className="aur-badge aur-badge--brand">{t(locale, 'economyBalanced')}</span>
            ) : (
              <span className="aur-badge aur-badge--danger">{t(locale, 'economyUnbalanced')}</span>
            )
          }
        />
      </div>
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between gap-2 text-sm">
          <span>
            {t(locale, 'economyMintedThisYear')}: <Money amount={economy.mintedThisYear} />
          </span>
          <span className="text-aur-muted">
            {t(locale, 'economyYearlyCap')}: <Money amount={economy.yearlyMintCap} hitl={economy.yearlyMintCap >= 50_000} />
          </span>
        </div>
        <Meter value={economy.mintedThisYear} max={economy.yearlyMintCap} label={t(locale, 'economyYearlyCap')} />
      </div>
    </Card>
  );
}

export function TreasuryForm({
  locale,
  users,
  busy,
  error,
  result,
  onMint,
  onBurn,
}: {
  locale: Locale;
  users: { id: string; name: string }[];
  busy: boolean;
  error: unknown;
  result: TreasuryResult | null;
  onMint: (input: TreasuryInput) => void;
  onBurn: (input: TreasuryInput) => void;
}) {
  // Local state + validation — fail-closed at the edge (mirrors Din::new).
  const [accountId, setAccountId] = useState('');
  const [amountRaw, setAmountRaw] = useState('');
  const [memo, setMemo] = useState('');
  const [localError, setLocalError] = useState<string | null>(null);

  function parse(): TreasuryInput | null {
    const id = accountId.trim();
    const n = Number(amountRaw.trim());
    if (id.length === 0 || amountRaw.trim().length === 0) {
      setLocalError(t(locale, 'amountRequired'));
      return null;
    }
    if (!Number.isInteger(n) || n < 1) {
      setLocalError(t(locale, 'invalidNumber'));
      return null;
    }
    try {
      assertDin(n);
    } catch (e) {
      setLocalError(errorText(locale, e));
      return null;
    }
    setLocalError(null);
    const trimmedMemo = memo.trim();
    return {
      accountId: id,
      amount: n,
      ...(trimmedMemo.length > 0 ? { memo: trimmedMemo } : {}),
    };
  }

  function submit(kind: 'mint' | 'burn'): void {
    const input = parse();
    if (input === null) return;
    if (kind === 'mint') onMint(input);
    else onBurn(input);
  }

  return (
    <Card title={t(locale, 'treasuryOps')} note={t(locale, 'treasuryOpsNote')}>
      <div className="grid gap-3 sm:grid-cols-2">
        <div className="aur-field">
          <label className="aur-label" htmlFor="ao-treasury-account">
            {t(locale, 'accountIdLabel')}
          </label>
          <input
            id="ao-treasury-account"
            className="aur-input"
            dir="ltr"
            list="ao-treasury-users"
            value={accountId}
            onChange={(e) => setAccountId(e.target.value)}
          />
          <datalist id="ao-treasury-users">
            {users.map((u) => (
              <option key={u.id} value={u.id}>
                {u.name}
              </option>
            ))}
          </datalist>
          <span className="text-aur-faint text-xs">{t(locale, 'accountPickHint')}</span>
        </div>
        <div className="aur-field">
          <label className="aur-label" htmlFor="ao-treasury-amount">
            {t(locale, 'amountLabel')}
          </label>
          <input
            id="ao-treasury-amount"
            className="aur-input aur-input--money"
            type="number"
            min={1}
            step={1}
            inputMode="numeric"
            value={amountRaw}
            onChange={(e) => setAmountRaw(e.target.value)}
          />
        </div>
        <div className="aur-field sm:col-span-2">
          <label className="aur-label" htmlFor="ao-treasury-memo">
            {t(locale, 'memoLabel')} <span className="text-aur-faint">({t(locale, 'memoOptional')})</span>
          </label>
          <input
            id="ao-treasury-memo"
            className="aur-input"
            value={memo}
            onChange={(e) => setMemo(e.target.value)}
          />
        </div>
      </div>

      {localError !== null && (
        <div role="alert" className="aur-error-state">
          {localError}
        </div>
      )}
      {error !== undefined && (
        <div role="alert" className="aur-error-state">
          {errorText(locale, error)}
        </div>
      )}

      <div className="flex flex-wrap gap-3">
        <button
          type="button"
          className="aur-btn aur-btn--primary"
          disabled={busy}
          aria-label={t(locale, 'mintAction')}
          onClick={() => submit('mint')}
        >
          {t(locale, 'mintAction')}
        </button>
        <button
          type="button"
          className="aur-btn aur-btn--danger"
          disabled={busy}
          aria-label={t(locale, 'burnAction')}
          onClick={() => submit('burn')}
        >
          {t(locale, 'burnAction')}
        </button>
      </div>

      {result !== null && <ResultRow locale={locale} result={result} />}
    </Card>
  );
}

export default function EconomySection({ client, locale }: SectionProps) {
  const queryClient = useQueryClient();
  const economy = useQuery({
    queryKey: ['dash-economy'],
    queryFn: () => fetchEconomy(client),
    staleTime: 15_000,
  });
  const users = useQuery({
    queryKey: ['dash-users'],
    queryFn: async () => unwrap(await client.api.GET('/users', { params: { query: { limit: 100 } } })),
    staleTime: 60_000,
  });

  const [lastResult, setLastResult] = useState<TreasuryResult | null>(null);

  const mint = useMutation({ mutationFn: treasuryMutation(client, 'mint') });
  const burn = useMutation({ mutationFn: treasuryMutation(client, 'burn') });

  function onTreasurySuccess(data: TreasuryResult): void {
    setLastResult(data);
    void queryClient.invalidateQueries({ queryKey: ['dash-economy'] });
    void queryClient.invalidateQueries({ queryKey: ['dash-overview'] });
  }

  if (economy.isPending) return <PendingState locale={locale} />;
  if (economy.isError) {
    return <ErrorState locale={locale} error={economy.error} onRetry={() => { void economy.refetch(); }} />;
  }

  const userList = users.data?.data.items ?? [];
  const busy = mint.isPending || burn.isPending;

  return (
    <div className="flex flex-col gap-4">
      <EconomyView locale={locale} economy={economy.data} />
      {userList.length === 0 ? (
        <Card title={t(locale, 'treasuryOps')} note={t(locale, 'treasuryOpsNote')}>
          <EmptyState locale={locale} />
        </Card>
      ) : (
        <TreasuryForm
          locale={locale}
          users={userList.map((u) => ({ id: u.id, name: u.name }))}
          busy={busy}
          error={mint.error ?? burn.error}
          result={lastResult}
          onMint={(input) => {
            mint.mutate(input, { onSuccess: onTreasurySuccess });
          }}
          onBurn={(input) => {
            burn.mutate(input, { onSuccess: onTreasurySuccess });
          }}
        />
      )}
    </div>
  );
}
