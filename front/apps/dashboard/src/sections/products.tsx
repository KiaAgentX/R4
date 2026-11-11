/**
 * Section: Products — `GET/POST /products`, `PATCH/DELETE /products/{id}`
 * (STEP1-AUDIT §2.5). Archive = soft delete (DELETE endpoint) and is always
 * a two-step armed action. Create/edit share one form; stars/credits are the
 * gen2 price counters — Din is NOT used there (server contract is frozen).
 */
import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t, type Locale } from '@agent-one/i18n';
import type { AgentOneClient } from '@agent-one/api-client';
import type { SectionProps } from '../props.js';
import { ArmButton, Card, EmptyState, ErrorState, PendingState, errorText } from '../ui.js';

export interface DashProduct {
  id: string;
  sellerId: string;
  title: string;
  description: string;
  category: string;
  priceStars: number;
  priceCredits: number;
  stock: number;
  salesCount: number;
  featured: boolean;
  status: 'active' | 'archived';
  createdAt: string;
}

export interface ProductFormBody {
  title: string;
  category?: string;
  priceStars?: number;
  priceCredits?: number;
  stock?: number;
  featured?: boolean;
  description?: string;
}

/** PATCH allows every field optional (frozen schema, updateDashProduct). */
export type ProductPatchBody = Omit<ProductFormBody, 'title'> & {
  title?: string;
  active?: boolean;
};

async function fetchProducts(client: AgentOneClient, q: string) {
  const query =
    q.trim().length > 0 ? { limit: 50, q: q.trim() } : { limit: 50 };
  return unwrap(await client.api.GET('/products', { params: { query } }));
}

/* ── Create / edit form ───────────────────────────────────────────────── */

export function ProductForm({
  locale,
  heading,
  initial,
  busy,
  error,
  onSubmit,
}: {
  locale: Locale;
  heading: string;
  /** Prefill for edit mode; omit for create. */
  initial?: DashProduct | undefined;
  busy: boolean;
  error: unknown;
  onSubmit: (body: ProductFormBody) => void;
}) {
  const [title, setTitle] = useState(initial?.title ?? '');
  const [category, setCategory] = useState(initial?.category ?? '');
  const [priceStars, setPriceStars] = useState(
    initial !== undefined ? String(initial.priceStars) : '',
  );
  const [priceCredits, setPriceCredits] = useState(
    initial !== undefined ? String(initial.priceCredits) : '',
  );
  const [stock, setStock] = useState(initial !== undefined ? String(initial.stock) : '');
  const [featured, setFeatured] = useState(initial?.featured ?? false);
  const [description, setDescription] = useState(initial?.description ?? '');
  const [localError, setLocalError] = useState<string | null>(null);

  function optionalInt(raw: string): { value?: number; invalid: boolean } {
    const trimmed = raw.trim();
    if (trimmed.length === 0) return { invalid: false };
    const n = Number(trimmed);
    if (!Number.isInteger(n) || n < 0) return { invalid: true };
    return { value: n, invalid: false };
  }

  function submit(): void {
    const trimmedTitle = title.trim();
    if (trimmedTitle.length === 0) {
      setLocalError(t(locale, 'amountRequired'));
      return;
    }
    const stars = optionalInt(priceStars);
    const credits = optionalInt(priceCredits);
    const stockN = optionalInt(stock);
    if (stars.invalid || credits.invalid || stockN.invalid) {
      setLocalError(t(locale, 'invalidNumber'));
      return;
    }
    setLocalError(null);
    const trimmedCategory = category.trim();
    const trimmedDescription = description.trim();
    const body: ProductFormBody = {
      title: trimmedTitle,
      ...(trimmedCategory.length > 0 ? { category: trimmedCategory } : {}),
      ...(stars.value !== undefined ? { priceStars: stars.value } : {}),
      ...(credits.value !== undefined ? { priceCredits: credits.value } : {}),
      ...(stockN.value !== undefined ? { stock: stockN.value } : {}),
      featured,
      ...(trimmedDescription.length > 0 ? { description: trimmedDescription } : {}),
    };
    onSubmit(body);
  }

  return (
    <Card title={heading}>
      <div className="grid gap-3 sm:grid-cols-2">
        <div className="aur-field sm:col-span-2">
          <label className="aur-label" htmlFor="ao-product-title">
            {t(locale, 'fieldTitle')}
          </label>
          <input
            id="ao-product-title"
            className="aur-input"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
          />
        </div>
        <div className="aur-field">
          <label className="aur-label" htmlFor="ao-product-category">
            {t(locale, 'fieldCategory')}
          </label>
          <input
            id="ao-product-category"
            className="aur-input"
            value={category}
            onChange={(e) => setCategory(e.target.value)}
          />
        </div>
        <div className="aur-field">
          <label className="aur-label" htmlFor="ao-product-stock">
            {t(locale, 'fieldStock')}
          </label>
          <input
            id="ao-product-stock"
            className="aur-input aur-input--money"
            type="number"
            min={0}
            step={1}
            value={stock}
            onChange={(e) => setStock(e.target.value)}
          />
        </div>
        <div className="aur-field">
          <label className="aur-label" htmlFor="ao-product-stars">
            {t(locale, 'fieldPriceStars')}
          </label>
          <input
            id="ao-product-stars"
            className="aur-input aur-input--money"
            type="number"
            min={0}
            step={1}
            value={priceStars}
            onChange={(e) => setPriceStars(e.target.value)}
          />
        </div>
        <div className="aur-field">
          <label className="aur-label" htmlFor="ao-product-credits">
            {t(locale, 'fieldPriceCredits')}
          </label>
          <input
            id="ao-product-credits"
            className="aur-input aur-input--money"
            type="number"
            min={0}
            step={1}
            value={priceCredits}
            onChange={(e) => setPriceCredits(e.target.value)}
          />
        </div>
        <div className="aur-field sm:col-span-2">
          <label className="aur-label" htmlFor="ao-product-description">
            {t(locale, 'fieldDescription')}
          </label>
          <textarea
            id="ao-product-description"
            className="aur-input"
            rows={3}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
        </div>
        <label className="aur-field flex-row items-center gap-2 sm:col-span-2">
          <input
            type="checkbox"
            checked={featured}
            onChange={(e) => setFeatured(e.target.checked)}
          />
          <span className="text-sm">{t(locale, 'fieldFeatured')}</span>
        </label>
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

      <div>
        <button type="button" className="aur-btn aur-btn--primary" disabled={busy} onClick={submit}>
          {t(locale, 'save')}
        </button>
      </div>
    </Card>
  );
}

/* ── Section ──────────────────────────────────────────────────────────── */

export default function ProductsSection({ client, locale }: SectionProps) {
  const queryClient = useQueryClient();
  const [q, setQ] = useState('');
  const [editing, setEditing] = useState<DashProduct | null>(null);
  const [creating, setCreating] = useState(false);

  const products = useQuery({
    queryKey: ['dash-products', q],
    queryFn: () => fetchProducts(client, q),
    staleTime: 15_000,
  });

  function invalidate(): void {
    void queryClient.invalidateQueries({ queryKey: ['dash-products'] });
    void queryClient.invalidateQueries({ queryKey: ['dash-overview'] });
  }

  const create = useMutation({
    mutationFn: async (body: ProductFormBody) =>
      unwrap(await client.api.POST('/products', { body })),
    onSuccess: () => {
      setCreating(false);
      invalidate();
    },
  });
  const patch = useMutation({
    mutationFn: async (input: { id: string; body: ProductPatchBody }) =>
      unwrap(
        await client.api.PATCH('/products/{id}', {
          params: { path: { id: input.id } },
          body: input.body,
        }),
      ),
    onSuccess: () => {
      setEditing(null);
      invalidate();
    },
  });
  const archive = useMutation({
    mutationFn: async (id: string) =>
      unwrap(await client.api.DELETE('/products/{id}', { params: { path: { id } } })),
    onSuccess: invalidate,
  });

  const busy = create.isPending || patch.isPending || archive.isPending;
  const items = products.data?.data.items ?? [];

  if (products.isPending) return <PendingState locale={locale} />;
  if (products.isError) {
    return <ErrorState locale={locale} error={products.error} onRetry={() => { void products.refetch(); }} />;
  }

  return (
    <div className="flex flex-col gap-4">
      {creating && (
        <ProductForm
          locale={locale}
          heading={t(locale, 'createProduct')}
          busy={busy}
          error={create.error}
          onSubmit={(body) => create.mutate(body)}
        />
      )}
      {editing !== null && (
        <ProductForm
          locale={locale}
          heading={`${t(locale, 'editProduct')} — ${editing.title}`}
          initial={editing}
          busy={busy}
          error={patch.error}
          onSubmit={(body) => patch.mutate({ id: editing.id, body })}
        />
      )}

      <Card
        title={t(locale, 'navProducts')}
        note={`${items.length} / ${String(products.data?.data.total ?? items.length)}`}
      >
        <div className="aur-field">
          <label className="aur-label" htmlFor="ao-product-search">
            {t(locale, 'search')}
          </label>
          <input
            id="ao-product-search"
            className="aur-input"
            type="search"
            placeholder={t(locale, 'searchProducts')}
            value={q}
            onChange={(e) => setQ(e.target.value)}
          />
        </div>

        {items.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table className="aur-table">
              <thead>
                <tr>
                  <th scope="col">{t(locale, 'colProduct')}</th>
                  <th scope="col">{t(locale, 'colPrice')}</th>
                  <th scope="col">{t(locale, 'colStock')}</th>
                  <th scope="col">{t(locale, 'colStatus')}</th>
                  <th scope="col">{t(locale, 'colActions')}</th>
                </tr>
              </thead>
              <tbody>
                {items.map((p) => (
                  <tr key={p.id}>
                    <td>
                      <strong>{p.title}</strong>
                      <div className="text-aur-faint text-xs">
                        {p.category} · {t(locale, 'salesCount')}: {p.salesCount}
                      </div>
                    </td>
                    <td>
                      <span className="aur-money">
                        {p.priceStars} ★ / {p.priceCredits} cr
                      </span>
                    </td>
                    <td>{p.stock}</td>
                    <td>
                      {p.status === 'active' ? (
                        <span className="aur-badge aur-badge--brand">{t(locale, 'productActive')}</span>
                      ) : (
                        <span className="aur-badge">{t(locale, 'productArchived')}</span>
                      )}
                      {p.featured && (
                        <span className="aur-badge aur-badge--info">★ {t(locale, 'fieldFeatured')}</span>
                      )}
                    </td>
                    <td>
                      <div className="flex flex-wrap gap-2">
                        {p.status === 'active' && (
                          <>
                            <button
                              type="button"
                              className="aur-btn aur-btn--ghost aur-btn--sm"
                              onClick={() => {
                                setCreating(false);
                                setEditing(p);
                              }}
                            >
                              {t(locale, 'edit')}
                            </button>
                            <button
                              type="button"
                              className="aur-btn aur-btn--ghost aur-btn--sm"
                              disabled={busy}
                              onClick={() => patch.mutate({ id: p.id, body: { featured: !p.featured } })}
                              aria-label={p.featured ? t(locale, 'featureOff') : t(locale, 'featureOn')}
                            >
                              {p.featured ? t(locale, 'featureOff') : t(locale, 'featureOn')}
                            </button>
                            <ArmButton
                              label={t(locale, 'archiveProduct')}
                              armedLabel={t(locale, 'orderActionConfirm')}
                              variant="danger"
                              disabled={busy}
                              ariaLabel={`${t(locale, 'archiveProduct')}: ${p.title}`}
                              onConfirm={() => archive.mutate(p.id)}
                            />
                          </>
                        )}
                        {p.status === 'archived' && (
                          <button
                            type="button"
                            className="aur-btn aur-btn--ghost aur-btn--sm"
                            disabled={busy}
                            onClick={() => patch.mutate({ id: p.id, body: { active: true } })}
                          >
                            {t(locale, 'restoreProduct')}
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        <div className="flex flex-wrap gap-3">
          <button
            type="button"
            className="aur-btn aur-btn--primary"
            disabled={creating}
            onClick={() => {
              setEditing(null);
              setCreating(true);
            }}
          >
            + {t(locale, 'createProduct')}
          </button>
        </div>
      </Card>
    </div>
  );
}
