/**
 * Display formatting for the citizen webapp — pure functions, testable
 * offline. Dates are locale-aware; the escrow state vocabulary and the gen2
 * order vocabulary come from the shared dictionary (one nation, one voice).
 */
import { escrowStateLabel, t, type Locale } from '@agent-one/i18n/webapp';

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

/** A `user:tg:<id>` / `esc:<id>` style id for display — keeps LTR direction. */
export function isAccountId(id: string): boolean {
  return /^(user|esc|tx|wallet|treasury|mint|burn)[:_]/.test(id);
}

/** The gen2 order status vocabulary — localized (dashboard parity). */
export const ORDER_STATUSES = ['paid', 'pending', 'delivered', 'refunded', 'cancelled'] as const;
export type OrderStatus = (typeof ORDER_STATUSES)[number];

export function orderStatusLabel(locale: Locale, status: string): string {
  const cap = status.charAt(0).toUpperCase() + status.slice(1).toLowerCase();
  const key = `orderStatus${cap}` as Parameters<typeof t>[1];
  const value: unknown = t(locale, key);
  return typeof value === 'string' && value.length > 0 ? value : status;
}

export { escrowStateLabel };

/** The 8 escrow states in machine order — the timeline spine. */
export const ESCROW_STATES = [
  'CREATED',
  'FUNDED',
  'DELIVERED',
  'RELEASED',
  'DISPUTED',
  'REFUNDED',
  'CANCELLED',
  'EXPIRED',
] as const;
export type EscrowState = (typeof ESCROW_STATES)[number];

/**
 * The domain transition table (crates/domain/src/escrow.rs — escrow_transitions)
 * mirrored for the timeline: which states a state can reach.
 */
export const ESCROW_TRANSITIONS: Record<EscrowState, readonly EscrowState[]> = {
  CREATED: ['FUNDED', 'CANCELLED'],
  FUNDED: ['DELIVERED', 'DISPUTED', 'EXPIRED'],
  DELIVERED: ['RELEASED', 'DISPUTED'],
  DISPUTED: ['REFUNDED', 'RELEASED'],
  RELEASED: [],
  REFUNDED: [],
  CANCELLED: [],
  EXPIRED: [],
};

/** Terminal states get a distinct visual (no further transitions). */
export function isTerminalState(state: EscrowState): boolean {
  return ESCROW_TRANSITIONS[state].length === 0;
}

/** Which actions are legally offered per state (money actions the UI shows). */
export type EscrowActionUi = 'fund' | 'deliver' | 'release' | 'dispute' | 'cancel';

export const STATE_ACTIONS: Record<EscrowState, readonly EscrowActionUi[]> = {
  CREATED: ['fund', 'cancel'],
  FUNDED: ['deliver', 'dispute'], // expiry is the automatic TTL sweep, not a button
  DELIVERED: ['release', 'dispute'],
  DISPUTED: [], // refund/release are arbitration (staff/bot) decisions
  RELEASED: [],
  REFUNDED: [],
  CANCELLED: [],
  EXPIRED: [],
};
