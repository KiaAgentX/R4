/**
 * Shared dashboard UI on Aurora — hook-free presentational primitives.
 * Every route ships pending / error / empty / data states (UX rule §6);
 * money renders through the exact domain mirror (formatDin); raw wire error
 * codes never reach the user (D8 — localized dictionary text only).
 */
import { useState } from 'react';
import type { ReactNode } from 'react';
import { errorMessage, t, type Locale } from '@agent-one/i18n/dashboard';
import { DinRangeError, formatDin } from '@agent-one/ui';
import { clientErrorCode } from '@agent-one/api-client/errors';
import { dirOf } from '@agent-one/ui/dir';

/* ── Error text (D8 — codes never reach the user) ─────────────────────── */

/**
 * Localized message for any failure: API envelopes via the code map,
 * client-side Din validation via its domain-mirroring codes, everything
 * else via the network/abort client codes.
 */
export function errorText(locale: Locale, error: unknown): string {
  if (error instanceof DinRangeError) return errorMessage(locale, error.code);
  return errorMessage(locale, clientErrorCode(error));
}

/* ── Money ────────────────────────────────────────────────────────────── */

/** `187,672 Đ` — LTR-isolated, tabular; hitl amounts get the art. 7 color. */
export function Money({ amount, hitl = false }: { amount: number; hitl?: boolean }) {
  return (
    <span className="aur-money" data-hitl={hitl ? 'true' : undefined} aria-label={formatDin(amount)}>
      {formatDin(amount)}
    </span>
  );
}

/* ── Stat tiles ───────────────────────────────────────────────────────── */

export function Tile({
  label,
  value,
  sub,
}: {
  label: string;
  value: ReactNode;
  sub?: string | undefined;
}) {
  return (
    <div className="aur-tile">
      <span className="aur-tile__label">{label}</span>
      <span className="aur-tile__value">{value}</span>
      {sub !== undefined && <span className="aur-tile__sub">{sub}</span>}
    </div>
  );
}

/* ── Meter (mint cap / AI usage) ──────────────────────────────────────── */

export function Meter({
  value,
  max,
  label,
}: {
  value: number;
  max: number;
  label: string;
}) {
  const pct = max > 0 ? Math.min(100, Math.round((value / max) * 100)) : 0;
  return (
    <div
      className="aur-meter"
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={max}
      aria-valuenow={value}
      aria-label={label}
    >
      <div className="aur-meter__bar" style={{ inlineSize: `${pct}%` }} />
    </div>
  );
}

/* ── Simple CSS bar chart (no library, honest data) ───────────────────── */

export function BarChart({
  points,
  ariaLabel,
}: {
  points: { label: string; value: number }[];
  ariaLabel: string;
}) {
  const max = Math.max(1, ...points.map((p) => p.value));
  return (
    <div className="aur-bars" role="img" aria-label={ariaLabel}>
      {points.map((p) => (
        <div key={p.label} className="aur-bars__col" title={`${p.label}: ${p.value}`}>
          <div
            className="aur-bars__bar"
            style={{ blockSize: `${Math.round((p.value / max) * 100)}%` }}
          />
          <span className="aur-bars__label">{p.label}</span>
        </div>
      ))}
    </div>
  );
}

/* ── State surfaces (§6: pending / error / empty on every route) ──────── */

export function PendingState({ locale }: { locale: Locale }) {
  return (
    <div className="aur-empty" role="status" aria-label={t(locale, 'loading')}>
      <div className="aur-spinner" aria-hidden="true" />
      <span>{t(locale, 'loading')}</span>
    </div>
  );
}

export function ErrorState({
  locale,
  error,
  onRetry,
}: {
  locale: Locale;
  error: unknown;
  onRetry?: (() => void) | undefined;
}) {
  // The CODE is audit-only; the user always sees the localized message (D8).
  const message = errorText(locale, error);
  return (
    <div className="aur-error-state" role="alert">
      <strong>{t(locale, 'errorTitle')}</strong>
      <span>{message}</span>
      {onRetry !== undefined && (
        <button type="button" className="aur-btn aur-btn--ghost" onClick={onRetry}>
          {t(locale, 'retry')}
        </button>
      )}
    </div>
  );
}

export function EmptyState({ locale }: { locale: Locale }) {
  return (
    <div className="aur-empty">
      <strong>{t(locale, 'emptyTitle')}</strong>
    </div>
  );
}

/* ── Two-step armed action (money-honesty confirm) ────────────────────── */

/**
 * Click once → arms with the honest warning; click again → executes.
 * No window.confirm dialogs, no accidental one-click money moves.
 */
export function ArmButton({
  label,
  armedLabel,
  onConfirm,
  disabled = false,
  variant = 'primary',
  ariaLabel,
}: {
  label: string;
  armedLabel: string;
  onConfirm: () => void;
  disabled?: boolean | undefined;
  variant?: 'primary' | 'ghost' | 'danger' | undefined;
  ariaLabel?: string | undefined;
}) {
  const [armed, setArmed] = useState(false);
  if (!armed) {
    return (
      <button
        type="button"
        className={`aur-btn aur-btn--${variant}`}
        disabled={disabled}
        aria-label={ariaLabel}
        onClick={() => setArmed(true)}
      >
        {label}
      </button>
    );
  }
  return (
    <span className="aur-arm">
      <span className="aur-arm__label" role="status">
        {armedLabel}
      </span>
      <button
        type="button"
        className="aur-btn aur-btn--danger aur-btn--sm"
        onClick={() => {
          setArmed(false);
          onConfirm();
        }}
      >
        ✓
      </button>
      <button
        type="button"
        className="aur-btn aur-btn--ghost aur-btn--sm"
        onClick={() => setArmed(false)}
      >
        ✕
      </button>
    </span>
  );
}

/* ── Filter chip row ──────────────────────────────────────────────────── */

export function FilterChips<T extends string>({
  options,
  value,
  onChange,
  labels,
}: {
  options: readonly T[];
  value: T;
  onChange: (next: T) => void;
  labels: (option: T) => string;
}) {
  return (
    <div className="aur-filters" role="group">
      {options.map((option) => (
        <button
          key={option}
          type="button"
          className="aur-nav__link"
          aria-pressed={option === value}
          onClick={() => onChange(option)}
        >
          {labels(option)}
        </button>
      ))}
    </div>
  );
}

/* ── Generic raw-item table (gen2 opaque shapes — honest, nothing fabricated) */

function cellText(value: unknown): string {
  if (value === null || value === undefined) return '—';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return JSON.stringify(value);
}

export function RawItemsTable({ items }: { items: Record<string, unknown>[] }) {
  const columns: string[] = [];
  for (const item of items) {
    for (const k of Object.keys(item)) {
      if (!columns.includes(k)) columns.push(k);
    }
  }
  return (
    <div style={{ overflowX: 'auto' }}>
      <table className="aur-table">
        <thead>
          <tr>
            {columns.map((c) => (
              <th key={c} scope="col">
                {c}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {items.map((item, i) => (
            <tr key={i}>
              {columns.map((c) => (
                <td key={c} dir="auto">
                  {cellText(item[c])}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/* ── Section card ─────────────────────────────────────────────────────── */

export function Card({
  title,
  children,
  note,
}: {
  title: string;
  children: ReactNode;
  note?: string | undefined;
}) {
  return (
    <section className="aur-card">
      <div className="aur-card__header">{title}</div>
      <div className="aur-card__body flex flex-col gap-4">{children}</div>
      {note !== undefined && <div className="aur-card__footer">{note}</div>}
    </section>
  );
}

/* ── Admin gate (X-KIA-Key entry — STEP1-AUDIT §4) ────────────────────── */

export type GateState =
  | { kind: 'checking' }
  | { kind: 'form' }
  | { kind: 'invalid' }
  | { kind: 'disabled' }
  | { kind: 'error'; error: unknown };

/**
 * The sign-in card shown until the admin key is verified against a real
 * admin route. 401 → invalid; 503 ADMIN_DISABLED → disabled (honest);
 * anything else → error state with retry. The key NEVER leaves the browser
 * except as the X-KIA-Key header of dashboard requests.
 */
export function AdminGateView({
  locale,
  gate,
  onSubmit,
  onRetry,
}: {
  locale: Locale;
  gate: GateState;
  onSubmit: (key: string) => void;
  onRetry: () => void;
}) {
  const [value, setValue] = useState('');
  const ready = value.trim().length > 0;
  return (
    <div
      className="min-h-dvh flex items-center justify-center p-6"
      dir={dirOf(locale) === 'rtl' ? 'rtl' : 'ltr'}
    >
      <form
        className="aur-card w-full max-w-md"
        onSubmit={(e) => {
          e.preventDefault();
          if (ready && gate.kind !== 'checking') onSubmit(value);
        }}
      >
        <div className="aur-card__header">{t(locale, 'adminKeyTitle')}</div>
        <div className="aur-card__body flex flex-col gap-4">
          <p className="text-aur-muted text-sm">{t(locale, 'adminKeyPrompt')}</p>
          <div className="aur-field">
            <label className="aur-label" htmlFor="ao-admin-key">
              {t(locale, 'adminKeyPlaceholder')}
            </label>
            <input
              id="ao-admin-key"
              className="aur-input"
              type="password"
              autoComplete="off"
              dir="ltr"
              value={value}
              aria-invalid={gate.kind === 'invalid'}
              onChange={(e) => setValue(e.target.value)}
            />
          </div>
          {gate.kind === 'checking' && (
            <div className="flex items-center gap-2 text-aur-muted text-sm">
              <span className="aur-spinner aur-spinner--inline" aria-hidden="true" />
              {t(locale, 'gateChecking')}
            </div>
          )}
          {gate.kind === 'invalid' && (
            <div role="alert" className="aur-error-state">
              {t(locale, 'adminKeyInvalid')}
            </div>
          )}
          {gate.kind === 'disabled' && (
            <div role="alert" className="aur-error-state">
              {errorMessage(locale, 'ADMIN_DISABLED')}
            </div>
          )}
          {gate.kind === 'error' && <ErrorState locale={locale} error={gate.error} onRetry={onRetry} />}
          <button
            type="submit"
            className="aur-btn aur-btn--primary"
            disabled={!ready || gate.kind === 'checking'}
          >
            {t(locale, 'adminKeySubmit')}
          </button>
        </div>
      </form>
    </div>
  );
}
