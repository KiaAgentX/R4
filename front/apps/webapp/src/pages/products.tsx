/**
 * Pages 2–4 — Products list, product detail, buy flow (map §2.4 rows 2–4).
 * The list/detail are pure public reads (GET /products). The buy flow is the
 * honest D14 page: the buyer picks the Din amount, ALWAYS sees the 90/10
 * preview + HITL badge, and the action goes through the ladder (staff key →
 * direct create; Telegram → assistant; guest → the bot's /buy grammar).
 * Staff preview must name an explicit buyer — the webapp never invents one.
 */
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t } from '@agent-one/i18n/webapp';
import { assertDin, DinRangeError } from '@agent-one/ui';
import type { PageCtx } from '../ctx.js';
import { routeAction } from '../actions.js';
import { pageHref } from '../router.js';
import { Card, EmptyState, ErrorState, Field, HitlBadge, LadderView, Money, PendingState, SplitPreview } from '../ui.js';

export default function ProductsListPage({ ctx }: { ctx: PageCtx }) {
  const { client, locale } = ctx;
  const [q, setQ] = useState('');
  const products = useQuery({
    queryKey: ['w-products', q],
    queryFn: async () => unwrap(await client.reads.products()),
    staleTime: 30_000,
  });

  const items = useMemo(() => {
    const all = products.data?.data.items ?? [];
    const needle = q.trim().toLowerCase();
    return needle.length === 0
      ? all.filter((p) => p.status === 'active')
      : all.filter(
          (p) =>
            p.status === 'active' &&
            (p.title.toLowerCase().includes(needle) ||
              p.description.toLowerCase().includes(needle) ||
              p.category.toLowerCase().includes(needle)),
        );
  }, [products.data, q]);

  if (products.isPending) return <PendingState locale={locale} />;
  if (products.isError) return <ErrorState locale={locale} error={products.error} onRetry={() => void products.refetch()} />;

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <div className="aur-field">
          <label className="aur-label" htmlFor="w-product-q">
            {t(locale, 'searchProducts')}
          </label>
          <input id="w-product-q" className="aur-input" type="search" value={q} onChange={(e) => setQ(e.target.value)} />
        </div>
      </Card>
      <Card title={t(locale, 'navStore')}>
        {items.length === 0 ? (
          <EmptyState locale={locale} note={t(locale, 'productsEmpty')} />
        ) : (
          <div className="flex flex-col gap-2">
            {items.map((p) => (
              <a key={p.id} className="aur-kv" href={pageHref('product', [p.id])}>
                <span className="aur-kv__label">
                  {p.title}
                  {p.featured && (
                    <span className="aur-badge aur-badge--brand" style={{ marginInlineStart: '0.5rem' }}>
                      {t(locale, 'fieldFeatured')}
                    </span>
                  )}
                </span>
                <span className="aur-kv__value">
                  {p.priceStars} ★ · {p.priceCredits} ¢
                </span>
              </a>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}

/** One product read from the public list (the contract has no single GET). */
export function useProduct(client: PageCtx['client'], id: string | undefined) {
  return useQuery({
    queryKey: ['w-products', 'for', id],
    queryFn: async () => unwrap(await client.reads.products()),
    staleTime: 30_000,
    select: (res) => res.data.items.find((p) => p.id === id) ?? null,
    enabled: id !== undefined,
  });
}

export function ProductDetailPage({ ctx, id }: { ctx: PageCtx; id?: string | undefined }) {
  const { client, locale } = ctx;
  const product = useProduct(client, id);
  if (product.isPending) return <PendingState locale={locale} />;
  if (product.isError) return <ErrorState locale={locale} error={product.error} onRetry={() => void product.refetch()} />;
  const p = product.data;
  if (p === null) {
    return (
      <Card title={t(locale, 'productDetailTitle')}>
        <EmptyState locale={locale} note={t(locale, 'productNotFound')} />
      </Card>
    );
  }
  return (
    <div className="flex flex-col gap-4">
      <Card title={p.title} note={p.description}>
        <div className="aur-kv">
          <span className="aur-kv__label">{t(locale, 'fieldPriceStars')}</span>
          <span className="aur-kv__value">{p.priceStars} ★</span>
        </div>
        <div className="aur-kv">
          <span className="aur-kv__label">{t(locale, 'fieldPriceCredits')}</span>
          <span className="aur-kv__value">{p.priceCredits} ¢</span>
        </div>
        <div className="aur-kv">
          <span className="aur-kv__label">{t(locale, 'productSeller')}</span>
          <span className="aur-kv__value" dir="ltr">
            <code className="text-xs">{p.sellerId}</code>
          </span>
        </div>
        <div className="aur-kv">
          <span className="aur-kv__label">{t(locale, 'colStock')}</span>
          <span className="aur-kv__value">{p.stock < 0 ? '∞' : p.stock}</span>
        </div>
      </Card>
      <a className="aur-btn aur-btn--primary" href={pageHref('buy', [p.id])}>
        {t(locale, 'productBuyCta')}
      </a>
    </div>
  );
}

/** Persian-digit tolerant integer parser — fail-closed like the domain.
 *  Accepts fa/en grouping separators (`,`, `٬` U+066C) and both digit sets. */
export function parseDinInput(raw: string): number | null {
  const normalized = raw
    .replace(/[,\s_٬]/g, '')
    .replace(/[۰-۹]/g, (d) => String('۰۱۲۳۴۵۶۷۸۹'.indexOf(d)))
    .replace(/[٠-٩]/g, (d) => String('٠١٢٣٤٥٦٧٨٩'.indexOf(d)));
  if (!/^\d+$/.test(normalized)) return null;
  const value = Number(normalized);
  try {
    assertDin(value);
  } catch (e) {
    if (e instanceof DinRangeError) return null;
    throw e;
  }
  return value;
}

export function BuyFlowPage({ ctx, id }: { ctx: PageCtx; id?: string | undefined }) {
  const { client, locale, mode, botHandle, initData, openChat, logAction } = ctx;
  const queryClient = useQueryClient();
  const product = useProduct(client, id);
  const [amountRaw, setAmountRaw] = useState('');
  const [seller, setSeller] = useState('');
  const [buyer, setBuyer] = useState(''); // staff preview only — never invented
  const [receipt, setReceipt] = useState<{ escrowId: string; txId: string | null; replayed: boolean } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const amount = parseDinInput(amountRaw);
  const effectiveSeller = seller.trim() !== '' ? seller.trim() : (product.data?.sellerId ?? '');
  const sellerOk = effectiveSeller.length > 3;
  const buyerOk = mode.kind !== 'staff' || buyer.trim().length > 3;
  const hitl = amount !== null && amount >= 50_000;

  const create = useMutation({
    mutationFn: async () => {
      if (amount === null || !sellerOk || !buyerOk) throw new Error('invalid');
      const res = await client.money.createEscrow(
        {
          buyerId: buyer.trim(),
          sellerId: effectiveSeller,
          amount,
          ...(product.data !== null && product.data !== undefined ? { productRef: product.data.id } : {}),
        },
        `webapp-buy-${product.data?.id ?? 'p'}-${amount}-${effectiveSeller}`,
      );
      return unwrap(res);
    },
    onSuccess: (data) => {
      setReceipt({ escrowId: data.escrow.id, txId: data.txId, replayed: data.replayed });
      logAction({ kind: 'staff', action: 'escrow.create', target: data.escrow.id, txId: data.txId, replayed: data.replayed });
      void queryClient.invalidateQueries({ queryKey: ['w-escrows'] });
    },
  });

  if (product.isPending) return <PendingState locale={locale} />;
  if (product.isError) return <ErrorState locale={locale} error={product.error} onRetry={() => void product.refetch()} />;

  const p = product.data;
  const route =
    amount !== null && sellerOk && buyerOk
      ? routeAction({
          mode,
          action: 'create',
          locale,
          seller: effectiveSeller,
          amount: amount,
          ...(p !== null ? { productId: p.id } : {}),
        })
      : null;

  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'buyTitle')} note={t(locale, 'buyExplain')}>
        {p !== null ? (
          <div className="aur-kv">
            <span className="aur-kv__label">{t(locale, 'colProduct')}</span>
            <span className="aur-kv__value">{p.title}</span>
          </div>
        ) : (
          <EmptyState locale={locale} note={t(locale, 'productNotFound')} />
        )}
        <Field id="w-buy-amount" label={t(locale, 'buyAmountLabel')} hint={t(locale, 'buyAmountHint')}>
          <input
            id="w-buy-amount"
            className="aur-input"
            inputMode="numeric"
            dir="ltr"
            value={amountRaw}
            onChange={(e) => setAmountRaw(e.target.value)}
          />
        </Field>
        <Field id="w-buy-seller" label={t(locale, 'buySellerIdLabel')} hint={t(locale, 'buySellerIdHint')}>
          <input id="w-buy-seller" className="aur-input" dir="ltr" value={seller} placeholder={p?.sellerId ?? ''} onChange={(e) => setSeller(e.target.value)} />
        </Field>
        {mode.kind === 'staff' && (
          <Field id="w-buy-buyer" label={t(locale, 'accountIdLabel')} hint={t(locale, 'staffPreviewNote')}>
            <input id="w-buy-buyer" className="aur-input" dir="ltr" value={buyer} onChange={(e) => setBuyer(e.target.value)} />
          </Field>
        )}
      </Card>

      {amount !== null && (
        <Card title={t(locale, 'buySummaryTitle')} note={t(locale, 'splitAlways')}>
          <div className="aur-kv">
            <span className="aur-kv__label">{t(locale, 'colAmount')}</span>
            <span className="aur-kv__value">
              <Money amount={amount} locale={locale} />
            </span>
          </div>
          <SplitPreview locale={locale} amount={amount} />
          {hitl && (
            <div role="note" className="text-xs font-bold text-aur-warning">
              <HitlBadge locale={locale} />
              <div>{t(locale, 'hitlBody')}</div>
            </div>
          )}
        </Card>
      )}

      {route !== null && (
        <Card title={t(locale, 'howToActTitle')}>
          {error !== null && (
            <div role="alert" className="aur-error-state">
              {t(locale, 'actionFailed')}
            </div>
          )}
          {receipt !== null ? (
            <div className="aur-receipt" role="status">
              <span className="aur-receipt__head">
                <span>{t(locale, 'actionReceipt')}</span>
                <span>{t(locale, 'actionDone')}</span>
              </span>
              <code className="text-xs" dir="ltr">
                {receipt.escrowId}
              </code>
              {receipt.replayed && <span className="aur-badge aur-badge--info">{t(locale, 'replayedBadge')}</span>}
              <a className="aur-btn aur-btn--sm" href={pageHref('escrowNew', [receipt.escrowId])}>
                {t(locale, 'buyCreatedGoto')}
              </a>
            </div>
          ) : (
            <LadderView
              locale={locale}
              route={route}
              botHandle={botHandle}
              busy={busy || create.isPending}
              onStaffConfirm={() => {
                setBusy(true);
                setError(null);
                create.mutate(undefined, {
                  onSettled: () => setBusy(false),
                  onError: (e) => setError(e),
                });
              }}
              onChat={(prefill) => openChat(prefill)}
            />
          )}
          {mode.kind !== 'staff' && initData === null && botHandle === null && (
            <span className="text-aur-faint text-xs">{t(locale, 'botOpenHint')}</span>
          )}
        </Card>
      )}

      {amountRaw.trim() !== '' && amount === null && (
        <div role="alert" className="aur-error-state">
          {t(locale, 'invalidNumber')}
        </div>
      )}
    </div>
  );
}


