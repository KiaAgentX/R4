/**
 * Citizen webapp UI on Aurora — hook-free presentational primitives.
 * Every page ships pending / error / empty / data states (UX rule §6);
 * money renders through the exact domain mirror (formatDin); the 90/10 split
 * shows BEFORE any release confirm (fees never hidden); HITL amounts carry
 * the art. 7 badge; raw wire error codes never reach the user (D8).
 */
import { useState, useEffect, type ReactNode } from 'react';
import { errorMessage, t, type Locale } from '@agent-one/i18n/webapp';
import { DinRangeError, escrowSplit, formatDin, requiresHumanApproval } from '@agent-one/ui';
import { clientErrorCode } from '@agent-one/api-client/errors';
import { dirOf } from '@agent-one/ui/dir';
import type { ActionRoute } from './actions.js';
import {
  ESCROW_STATES,
  ESCROW_TRANSITIONS,
  escrowStateLabel,
  isTerminalState,
  type EscrowState,
} from './format.js';

/* ── Error text (D8 — codes never reach the user) ─────────────────────── */

export function errorText(locale: Locale, error: unknown): string {
  if (error instanceof DinRangeError) return errorMessage(locale, error.code);
  return errorMessage(locale, clientErrorCode(error));
}

/* ── Money ────────────────────────────────────────────────────────────── */

/** `187,672 Đ` — LTR-isolated, tabular; ≥ 50,000 Đ gets the art. 7 mark. */
export function Money({ amount, hitl, locale }: { amount: number; hitl?: boolean | undefined; locale?: Locale | undefined }) {
  const hitlMark = hitl ?? requiresHumanApproval(amount);
  return (
    <span className="aur-money" data-hitl={hitlMark ? 'true' : undefined} aria-label={formatDin(amount)}>
      {formatDin(amount)}
      {hitlMark && locale !== undefined && <HitlBadge locale={locale} />}
    </span>
  );
}

/** The charter art. 7 badge — same voice as the bot's HITL wording. */
export function HitlBadge({ locale }: { locale: Locale }) {
  return (
    <span className="aur-badge aur-badge--warning aur-badge--hitl" title={t(locale, 'hitlBadge')}>
      {t(locale, 'hitlBadge')}
    </span>
  );
}

/** The 90/10 settlement preview — ALWAYS on release/confirm surfaces (§6). */
export function SplitPreview({ locale, amount }: { locale: Locale; amount: number }) {
  const { sellerNet, tax } = escrowSplit(amount);
  return (
    <span className="aur-money__split">
      <span>
        {t(locale, 'sellerShare')}: <Money amount={sellerNet} hitl={false} />
      </span>
      <span>
        {t(locale, 'treasuryTax')}: <Money amount={tax} hitl={false} />
      </span>
    </span>
  );
}

/* ── State surfaces (§6: pending / error / empty on every page) ───────── */

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
  return (
    <div className="aur-error-state" role="alert">
      <strong>{t(locale, 'errorTitle')}</strong>
      <span>{errorText(locale, error)}</span>
      {onRetry !== undefined && (
        <button type="button" className="aur-btn aur-btn--ghost" onClick={onRetry}>
          {t(locale, 'retry')}
        </button>
      )}
    </div>
  );
}

export function EmptyState({ locale, note }: { locale: Locale; note?: string | undefined }) {
  return (
    <div className="aur-empty">
      <strong>{t(locale, 'emptyTitle')}</strong>
      {note !== undefined && <span>{note}</span>}
    </div>
  );
}

/* ── Two-step armed action (money-honesty confirm) ────────────────────── */

/**
 * Click once → arms with the honest warning; click again → executes.
 * Money NEVER moves on a single tap (§6) — same pattern as the dashboard.
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

/* ── Page card / tile / meter ─────────────────────────────────────────── */

export function Card({
  title,
  children,
  note,
}: {
  title?: string | undefined;
  children: ReactNode;
  note?: string | undefined;
}) {
  return (
    <section className="aur-card">
      {title !== undefined && <div className="aur-card__header">{title}</div>}
      <div className="aur-card__body flex flex-col gap-4">{children}</div>
      {note !== undefined && <div className="aur-card__footer">{note}</div>}
    </section>
  );
}

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

export function Field({
  id,
  label,
  hint,
  children,
}: {
  id: string;
  label: string;
  hint?: string | undefined;
  children: ReactNode;
}) {
  return (
    <div className="aur-field">
      <label className="aur-label" htmlFor={id}>
        {label}
      </label>
      {children}
      {hint !== undefined && <div className="text-aur-faint text-xs">{hint}</div>}
    </div>
  );
}

/* ── 8-state timeline (AURORA) ────────────────────────────────────────── */

/**
 * The escrow 8-state machine as a vertical timeline. The current state is
 * highlighted; reachable next states (domain transition table) show a hint;
 * terminal states close the line. Localized labels — never raw wire words.
 */
export function StateTimeline({ locale, state }: { locale: Locale; state: EscrowState }) {
  const idx = ESCROW_STATES.indexOf(state);
  const next = ESCROW_TRANSITIONS[state];
  return (
    <ol className="aur-timeline" aria-label={t(locale, 'timelineTitle')}>
      {ESCROW_STATES.map((s, i) => {
        const done = idx >= 0 && i < idx;
        const current = s === state;
        return (
          <li
            key={s}
            className="aur-timeline__row"
            data-state={done ? 'done' : current ? 'current' : 'todo'}
            data-terminal={isTerminalState(s) ? 'true' : undefined}
          >
            <span className="aur-timeline__dot" aria-hidden="true" />
            <span className="aur-timeline__label">
              {escrowStateLabel(locale, s)}
              {current && (
                <span className="aur-badge aur-badge--brand aur-timeline__now">
                  {t(locale, 'colStatus')}
                </span>
              )}
            </span>
            {current && next.length > 0 && (
              <span className="aur-timeline__next">
                → {next.map((n) => escrowStateLabel(locale, n)).join(' · ')}
              </span>
            )}
          </li>
        );
      })}
    </ol>
  );
}

/* ── The D14 action ladder renderer ───────────────────────────────────── */

/**
 * Renders ONE honest way to perform the action (the ladder):
 *  staff → the armed primary button (direct, idempotency-keyed)
 *  chat  → "open the assistant with this sentence" (prefill carried in hash)
 *  bot   → open the Telegram bot + the exact command to run (copyable);
 *          without a bot handle the honest hint shows instead of a dead link
 *  readonly → the honest read-only note (no fake button anywhere)
 */
export function LadderView({
  locale,
  route,
  botHandle,
  busy,
  onStaffConfirm,
  onChat,
}: {
  locale: Locale;
  route: ActionRoute;
  botHandle: string | null;
  busy: boolean;
  onStaffConfirm: () => void;
  onChat: (prefill: string) => void;
}) {
  if (route.via === 'readonly') {
    return (
      <div className="aur-ladder" role="note">
        <strong>{t(locale, 'howToActTitle')}</strong>
        <span>{t(locale, 'actReadonly')}</span>
      </div>
    );
  }
  if (route.via === 'staff') {
    return (
      <div className="aur-ladder">
        <ArmButton
          label={t(locale, 'actViaStaff')}
          armedLabel={t(locale, 'orderActionConfirm')}
          disabled={busy}
          onConfirm={onStaffConfirm}
        />
      </div>
    );
  }
  if (route.via === 'chat') {
    return (
      <div className="aur-ladder">
        <button type="button" className="aur-btn aur-btn--primary" disabled={busy} onClick={() => onChat(route.prefill)}>
          {t(locale, 'actViaChat')}
        </button>
        <span className="aur-ladder__prefill" dir="auto">
          {route.prefill}
        </span>
      </div>
    );
  }
  const link = botHandle !== null ? `https://t.me/${botHandle}` : null;
  return (
    <div className="aur-ladder">
      {link !== null ? (
        <a className="aur-btn aur-btn--primary" href={link} target="_blank" rel="noreferrer">
          {t(locale, 'actViaBot')}
        </a>
      ) : (
        <span className="text-aur-muted text-sm">{t(locale, 'botOpenHint')}</span>
      )}
      <div className="aur-ladder__command" dir="ltr">
        <code>{route.command}</code>
        <CopyBtn locale={locale} text={route.command} />
      </div>
    </div>
  );
}

/** Copy-to-clipboard with an honest fallback (no clipboard API → the text is selectable). */
export function CopyBtn({ locale, text }: { locale: Locale; text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      className="aur-btn aur-btn--ghost aur-btn--sm"
      aria-label={t(locale, 'copy')}
      onClick={() => {
        try {
          void navigator.clipboard?.writeText(text);
          setCopied(true);
          setTimeout(() => setCopied(false), 1500);
        } catch {
          setCopied(false); // clipboard unavailable → the code is selectable
        }
      }}
    >
      {copied ? t(locale, 'copied') : t(locale, 'copy')}
    </button>
  );
}

/* ── Key/value rows (detail pages) ────────────────────────────────────── */

export function Kv({ label, children, ltr }: { label: string; children: ReactNode; ltr?: boolean | undefined }) {
  return (
    <div className="aur-kv">
      <span className="aur-kv__label">{label}</span>
      <span className="aur-kv__value" dir={ltr === true ? 'ltr' : undefined}>
        {children}
      </span>
    </div>
  );
}

/* ── Generic raw-item table (gen2 opaque shapes — honest, nothing fabricated) ── */

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

/** Localized escrow state badge. */
export function StateBadge({ locale, state }: { locale: Locale; state: string }) {
  return <span className="aur-badge">{escrowStateLabel(locale, state)}</span>;
}

/** Honest direction helper for app shells. */
export function dirFor(locale: Locale): 'rtl' | 'ltr' {
  return dirOf(locale) === 'rtl' ? 'rtl' : 'ltr';
}

/* ── Online/offline (real browser events, offline page + frame banner) ── */

export function useOnline(): boolean {
  const [online, setOnline] = useState(() => (typeof navigator !== 'undefined' ? navigator.onLine : true));
  useEffect(() => {
    const up = (): void => setOnline(true);
    const down = (): void => setOnline(false);
    window.addEventListener('online', up);
    window.addEventListener('offline', down);
    return () => {
      window.removeEventListener('online', up);
      window.removeEventListener('offline', down);
    };
  }, []);
  return online;
}
