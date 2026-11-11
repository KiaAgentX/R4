/**
 * Display formatting for the dashboard — every localized/numeric shape the
 * admin surfaces need. Pure functions, fully testable offline.
 *  • dates — locale-aware (fa → Persian calendar via Intl, en → Gregorian)
 *  • gen2 order vocabulary — the mapping the audit requires to be preserved
 *  • bytes — backup size display
 */
import { t, type Locale } from '@agent-one/i18n';

/** Locale-aware short date-time. Invalid input degrades to the raw string. */
export function formatDate(iso: string, locale: Locale): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  try {
    return new Intl.DateTimeFormat(locale === 'fa' ? 'fa' : 'en', {
      dateStyle: 'short',
      timeStyle: 'short',
    }).format(d);
  } catch {
    return iso;
  }
}

/** Human bytes — 1536 → `1.5 KB` (decimal units, one fraction digit). */
export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return '—';
  if (n < 1024) return `${n} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = n;
  let unit = 'B';
  for (const u of units) {
    if (value < 1024) break;
    value /= 1024;
    unit = u;
  }
  return `${value >= 100 ? Math.round(value) : Math.round(value * 10) / 10} ${unit}`;
}

/** The gen2 order vocabulary — preserved verbatim (STEP1-AUDIT §2.5). */
export const ORDER_STATUS_FILTERS = [
  'all',
  'paid',
  'pending',
  'delivered',
  'refunded',
  'cancelled',
] as const;
export type OrderStatusFilter = (typeof ORDER_STATUS_FILTERS)[number];

/** Localized order status; unknown values fall back to the raw word (data,
 *  not an error code — the never-raw rule applies to error codes). */
export function orderStatusLabel(locale: Locale, status: string): string {
  const cap = status.charAt(0).toUpperCase() + status.slice(1).toLowerCase();
  const key = `orderStatus${cap}` as Parameters<typeof t>[1];
  const value: unknown = t(locale, key);
  return typeof value === 'string' && value.length > 0 ? value : status;
}

/** The wire's order-action verbs (POST /orders/{id}/status). */
export const ORDER_ACTIONS = ['deliver', 'cancel', 'refund'] as const;
export type OrderAction = (typeof ORDER_ACTIONS)[number];

export function orderActionLabel(locale: Locale, action: OrderAction): string {
  switch (action) {
    case 'deliver':
      return t(locale, 'orderActionDeliver');
    case 'cancel':
      return t(locale, 'orderActionCancel');
    case 'refund':
      return t(locale, 'orderActionRefund');
  }
}

/** gen2 approval statuses + the HITL queue's four wire states. */
export function approvalStatusLabel(locale: Locale, status: string): string {
  switch (status.toUpperCase()) {
    case 'PENDING':
      return t(locale, 'approvalPending');
    case 'APPROVED':
      return t(locale, 'approvalApproved');
    case 'REJECTED':
      return t(locale, 'approvalRejected');
    case 'EXPIRED':
      return t(locale, 'approvalExpired');
    default:
      return status;
  }
}

/** Approval risk level → localized badge label. */
export function riskLabel(locale: Locale, risk: string): string {
  switch (risk) {
    case 'low':
      return t(locale, 'riskLow');
    case 'medium':
      return t(locale, 'riskMedium');
    case 'high':
      return t(locale, 'riskHigh');
    default:
      return risk;
  }
}

/** gen2 skill kind → localized badge. */
export function skillKindLabel(locale: Locale, kind: string): string {
  switch (kind) {
    case 'read':
      return t(locale, 'skillKindRead');
    case 'write':
      return t(locale, 'skillKindWrite');
    case 'money':
      return t(locale, 'skillKindMoney');
    case 'staff':
      return t(locale, 'skillKindStaff');
    default:
      return kind;
  }
}
