import { describe, expect, it } from 'vitest';
import {
  DIN_SYMBOL,
  DinRangeError,
  HITL_THRESHOLD,
  MAX_DIN,
  assertDin,
  escrowSplit,
  formatDin,
  formatSignedDin,
  requiresHumanApproval,
} from '../src/money.js';

// Mirror of the Rust domain test `formats_with_the_dinar_symbol` (money.rs).
describe('formatDin — mirror of dropagentx-domain money.rs', () => {
  it('formats with the dinar symbol', () => {
    expect(formatDin(187_672)).toBe('187,672 \u{0110}');
    expect(formatDin(0)).toBe('0 \u{0110}');
    expect(formatDin(1_234_567)).toBe('1,234,567 \u{0110}');
  });

  it('symbol is U+0110 D WITH STROKE', () => {
    expect(DIN_SYMBOL).toBe('\u{0110}');
    expect(DIN_SYMBOL).toHaveLength(1);
  });

  it('accepts every safe non-negative integer', () => {
    expect(() => assertDin(0)).not.toThrow();
    expect(() => assertDin(1)).not.toThrow();
    expect(() => assertDin(MAX_DIN)).not.toThrow();
  });

  it('rejects negative amounts', () => {
    const e = (() => {
      try {
        formatDin(-1);
        return null;
      } catch (err) {
        return err;
      }
    })() as DinRangeError;
    expect(e).toBeInstanceOf(DinRangeError);
    expect(e.code).toBe('NEGATIVE_AMOUNT');
  });

  it('rejects amounts beyond the safe integer range', () => {
    expect(() => assertDin(9_007_199_254_740_992)).toThrowError(DinRangeError);
    expect(() => assertDin(MAX_DIN + 1)).toThrowError(/safe integer range/);
  });

  it('rejects floats (NON_INTEGER_AMOUNT)', () => {
    expect(() => formatDin(187_672.5)).toThrowError(DinRangeError);
    expect(() => formatDin(Number.NaN)).toThrowError(DinRangeError);
    expect(() => formatDin(Number.POSITIVE_INFINITY)).toThrowError(DinRangeError);
  });

  it('signed history lines match the bot semantics (+ / − U+2212, abs)', () => {
    expect(formatSignedDin(54_000)).toBe('+54,000 \u{0110}');
    expect(formatSignedDin(-6_000)).toBe('\u{2212}6,000 \u{0110}');
  });
});

describe('escrowSplit — 90/10 preview (art. 2 + art. 8, mul_bps floor)', () => {
  it('splits 60,000 exactly 90/10', () => {
    expect(escrowSplit(60_000)).toEqual({ sellerNet: 54_000, tax: 6_000 });
  });

  it('floors the tax like mul_bps (55 → tax 5)', () => {
    expect(escrowSplit(55)).toEqual({ sellerNet: 50, tax: 5 });
  });

  it('partitions exactly for arbitrary amounts', () => {
    for (const amount of [0, 1, 10, 999, 4_000, 49_999, 187_672]) {
      const { sellerNet, tax } = escrowSplit(amount);
      expect(sellerNet + tax).toBe(amount);
    }
  });

  it('HITL threshold is 50,000 and applies at the exact boundary', () => {
    expect(HITL_THRESHOLD).toBe(50_000);
    expect(requiresHumanApproval(49_999)).toBe(false);
    expect(requiresHumanApproval(50_000)).toBe(true);
    expect(requiresHumanApproval(60_000)).toBe(true);
  });
});
