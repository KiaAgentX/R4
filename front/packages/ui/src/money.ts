/**
 * Aurora money display — exact mirror of `dropagentx-domain` money.rs.
 *
 * Invariants (phase 1 tests, mirrored 1:1 here):
 *  • integer Din only — `Number.isSafeInteger` = the 2^53−1 ceiling (MAX_DIN)
 *  • `format_din`: en-US 3-digit grouping + `" Đ"` (U+0110) suffix
 *  • `187,672 Đ` / `0 Đ` / `1,234,567 Đ`
 *  • negative values never enter formatting — callers pass `|amount|`
 *    and render the sign separately, exactly like the bot's history lines
 *    (`+` / `−` U+2212).
 *
 * UX rule §6: fees/tax never hidden → `escrowSplit` exposes the 90/10 split
 * (art. 2 / art. 8: tax = floor(amount/10), floor semantics of `mul_bps`).
 */

/** `Number.MAX_SAFE_INTEGER` — the deliberate wire ceiling (money.rs). */
export const MAX_DIN = 9_007_199_254_740_991;

/** The Din symbol — U+0110 LATIN CAPITAL LETTER D WITH STROKE. */
export const DIN_SYMBOL = '\u{0110}';

export class DinRangeError extends RangeError {
  constructor(
    message: string,
    /** 'NEGATIVE_AMOUNT' | 'AMOUNT_OUT_OF_RANGE' — mirrors domain codes. */
    readonly code: 'NEGATIVE_AMOUNT' | 'AMOUNT_OUT_OF_RANGE' | 'NON_INTEGER_AMOUNT',
  ) {
    super(message);
    this.name = 'DinRangeError';
  }
}

/** Validates like `Din::new` in the domain — fail-closed at the edge. */
export function assertDin(amount: number): void {
  if (!Number.isInteger(amount)) {
    throw new DinRangeError(`Money must be an integer, got ${amount}`, 'NON_INTEGER_AMOUNT');
  }
  if (amount < 0) {
    throw new DinRangeError(`Money cannot be negative, got ${amount}`, 'NEGATIVE_AMOUNT');
  }
  if (amount > MAX_DIN) {
    throw new DinRangeError(`Money exceeds the safe integer range, got ${amount}`, 'AMOUNT_OUT_OF_RANGE');
  }
}

/**
 * `format_din` mirror: `187,672 Đ`. Zero is valid and renders `0 Đ`.
 * @throws DinRangeError on float / negative / > 2^53−1 (never silently shows
 *         a wrong number — honest fail-closed, same as the domain).
 */
export function formatDin(amount: number): string {
  assertDin(amount);
  const digits = amount.toString();
  let out = '';
  for (let i = 0; i < digits.length; i++) {
    if (i > 0 && (digits.length - i) % 3 === 0) out += ',';
    out += digits[i];
  }
  return `${out} ${DIN_SYMBOL}`;
}

/**
 * Signed history line — exact semantics of the bot's `/history` rows:
 * `+54,000 Đ` / `−6,000 Đ` (U+2212 minus, abs() formatted).
 */
export function formatSignedDin(amount: number): string {
  const sign = amount < 0 ? '\u{2212}' : '+';
  return `${sign}${formatDin(Math.abs(amount))}`;
}

/**
 * 90/10 settlement preview (art. 2 + art. 8) — mirrors the domain:
 * tax = `amount.mul_bps(1_000)` (integer floor), seller keeps the rest.
 * `escrowSplit(60_000)` → `{ sellerNet: 54_000, tax: 6_000 }`.
 */
export function escrowSplit(amount: number): { sellerNet: number; tax: number } {
  assertDin(amount);
  const tax = Math.floor((amount * 1_000) / 10_000);
  return { sellerNet: amount - tax, tax };
}

/** True when the amount must pass the HITL queue (charter art. 7). */
export function requiresHumanApproval(amount: number): boolean {
  return amount >= 50_000;
}

/** The HITL threshold in Din — charter art. 7. */
export const HITL_THRESHOLD = 50_000;
