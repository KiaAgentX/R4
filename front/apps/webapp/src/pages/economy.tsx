/**
 * Pages 27–28 — economy transparency + charter (map §2.4 rows 27–28).
 * The economy page renders the PUBLIC art. 6 numbers with the art. 3 mint-cap
 * meter; the charter page prints the 10 articles VERBATIM from the domain
 * (content.ts mirrors crates/domain/src/charter.rs) — one nation, one law.
 */
import { useQuery } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t } from '@agent-one/i18n/webapp';
import { formatDin } from '@agent-one/ui';
import type { PageCtx } from '../ctx.js';
import { CHARTER_ARTICLES, charterTitle, charterValue } from '../content.js';
import { Card, ErrorState, Meter, Money, PendingState, Tile } from '../ui.js';

export function EconomyPage({ ctx }: { ctx: PageCtx }) {
  const { client, locale } = ctx;
  const economy = useQuery({ queryKey: ['w-economy'], queryFn: async () => unwrap(await client.reads.economy()), staleTime: 30_000 });
  if (economy.isPending) return <PendingState locale={locale} />;
  if (economy.isError) return <ErrorState locale={locale} error={economy.error} onRetry={() => void economy.refetch()} />;
  const eco = economy.data;
  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'economyTitle')} note={t(locale, 'economyIntro')}>
        <div className="aur-tiles">
          <Tile label={t(locale, 'economyCirculating')} value={<Money amount={eco.circulatingSupply} hitl={false} />} />
          <Tile label={t(locale, 'economyTreasury')} value={<Money amount={eco.treasuryBalance} hitl={false} />} />
          <Tile label={t(locale, 'economyEscrowLocked')} value={<Money amount={eco.escrowLocked} hitl={false} />} />
          <Tile label={t(locale, 'economyGenesis')} value={<Money amount={eco.genesisSupply} hitl={false} />} />
          <Tile label={t(locale, 'economyMintedAllTime')} value={<Money amount={eco.mintedAllTime} hitl={false} />} />
          <Tile label={t(locale, 'economyBurnedAllTime')} value={<Money amount={eco.burnedAllTime} hitl={false} />} />
          <Tile label={t(locale, 'economyAccounts')} value={eco.accountCount} />
          <Tile label={t(locale, 'economyOpenEscrows')} value={eco.openEscrows} />
        </div>
        <div className="aur-field">
          <span className="aur-label">
            {t(locale, 'economyYearlyCap')}: {formatDin(eco.mintedThisYear)} / {formatDin(eco.yearlyMintCap)}
          </span>
          <Meter value={eco.mintedThisYear} max={eco.yearlyMintCap} label={t(locale, 'economyYearlyCap')} />
        </div>
        <div className={eco.balanced ? 'text-aur-faint text-xs' : 'text-xs font-bold text-aur-danger'} role={eco.balanced ? undefined : 'alert'}>
          {eco.balanced ? t(locale, 'economyBalanced') : t(locale, 'economyUnbalanced')} · {t(locale, 'economyTrialBalance')}:{' '}
          {formatDin(Math.abs(eco.trialBalanceSum))}
        </div>
      </Card>
    </div>
  );
}

export function CharterPage({ ctx }: { ctx: PageCtx }) {
  const { locale } = ctx;
  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'charterTitle')} note={t(locale, 'charterIntro')}>
        <ol className="flex flex-col gap-3">
          {CHARTER_ARTICLES.map((a) => (
            <li key={a.article} className="aur-receipt">
              <span className="aur-receipt__head">
                <span>
                  <span className="aur-badge aur-badge--brand">{a.article}</span>{' '}
                  <strong>{charterTitle(a, locale)}</strong>
                </span>
              </span>
              <span className="text-sm">{charterValue(a, locale)}</span>
            </li>
          ))}
        </ol>
      </Card>
    </div>
  );
}
