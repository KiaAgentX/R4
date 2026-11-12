/**
 * Page 1 — Storefront home (map §2.4 row 1): featured products, the economy
 * at a glance (art. 6 transparency), server health chip and the identity
 * state — all from PUBLIC routes only; nothing fabricated while offline.
 */
import { useQuery } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t } from '@agent-one/i18n/webapp';
import { formatDin } from '@agent-one/ui';
import type { PageCtx } from '../ctx.js';
import { pageHref } from '../router.js';
import { Card, EmptyState, ErrorState, Money, PendingState, Tile } from '../ui.js';

export default function HomePage({ ctx }: { ctx: PageCtx }) {
  const { client, locale, tgUser, mode } = ctx;
  const products = useQuery({
    queryKey: ['w-products'],
    queryFn: async () => unwrap(await client.reads.products()),
    staleTime: 30_000,
  });
  const economy = useQuery({
    queryKey: ['w-economy'],
    queryFn: async () => unwrap(await client.reads.economy()),
    staleTime: 30_000,
  });
  const health = useQuery({
    queryKey: ['w-health'],
    queryFn: async () => unwrap(await client.reads.health()),
    staleTime: 30_000,
  });

  if (products.isPending || economy.isPending) return <PendingState locale={locale} />;
  if (products.isError) return <ErrorState locale={locale} error={products.error} onRetry={() => void products.refetch()} />;

  const featured = products.data.data.items.filter((p) => p.featured && p.status === 'active').slice(0, 4);
  const eco = economy.isSuccess ? economy.data : null;

  return (
    <div className="flex flex-col gap-4">
      <div className="aur-hero">
        <span className="aur-hero__title">{t(locale, 'homeHero')}</span>
        <span className="text-sm opacity-90">{t(locale, 'webappSubtitle')}</span>
        {health.isSuccess && (
          <span className="text-xs opacity-90">
            {t(locale, 'backendOk')} · v{health.data.version}
          </span>
        )}
        <a className="aur-btn aur-btn--sm aur-btn--ghost" href={pageHref('products')} style={{ alignSelf: 'flex-start' }}>
          {t(locale, 'homeExplore')}
        </a>
      </div>

      {tgUser !== null ? (
        <Card title={t(locale, 'homeIdentity')}>
          <div className="text-sm">
            {tgUser.name} · <code dir="ltr">user:tg:{tgUser.id}</code>
          </div>
        </Card>
      ) : (
        <Card note={mode.kind === 'guest' ? t(locale, 'profileGuest') : undefined}>
          <span className="text-sm text-aur-muted">{t(locale, 'roleGuest')}: {t(locale, 'profileGuest')}</span>
        </Card>
      )}

      <Card title={t(locale, 'homeFeatured')}>
        {featured.length === 0 ? (
          <EmptyState locale={locale} note={t(locale, 'productsEmpty')} />
        ) : (
          <div className="flex flex-col gap-2">
            {featured.map((p) => (
              <a key={p.id} className="aur-kv" href={pageHref('product', [p.id])}>
                <span className="aur-kv__label">{p.title}</span>
                <span className="aur-kv__value">
                  {p.priceStars} ★ / {p.priceCredits} ¢
                </span>
              </a>
            ))}
          </div>
        )}
      </Card>

      {eco !== null && (
        <Card title={t(locale, 'economyTitle')} note={t(locale, 'economyIntro')}>
          <div className="aur-tiles">
            <Tile label={t(locale, 'economyCirculating')} value={<Money amount={eco.circulatingSupply} hitl={false} />} />
            <Tile label={t(locale, 'economyTreasury')} value={<Money amount={eco.treasuryBalance} hitl={false} />} />
            <Tile label={t(locale, 'economyEscrowLocked')} value={<Money amount={eco.escrowLocked} hitl={false} />} />
            <Tile label={t(locale, 'economyOpenEscrows')} value={eco.openEscrows} />
          </div>
          <div className="text-aur-faint text-xs" title={formatDin(eco.yearlyMintCap)}>
            {t(locale, 'economyMintedThisYear')}: {formatDin(eco.mintedThisYear)} / {t(locale, 'economyYearlyCap')}:{' '}
            {formatDin(eco.yearlyMintCap)}
          </div>
          <div className={eco.balanced ? 'text-aur-faint text-xs' : 'text-xs font-bold text-aur-danger'}>
            {eco.balanced ? t(locale, 'economyBalanced') : t(locale, 'economyUnbalanced')} · {t(locale, 'economyTrialBalance')}:{' '}
            {formatDin(Math.abs(eco.trialBalanceSum))}
          </div>
        </Card>
      )}
    </div>
  );
}
