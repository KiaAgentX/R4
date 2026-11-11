import { describe, expect, it } from 'vitest';
import { DEFAULT_LOCALE, dirOf, isRtl, localeFromLanguageCode, localeName, oppositeLocale, type Locale } from '../src/locale.js';
import { escrowStateLabel, en as enDict, fa as faDict, t } from '../src/dict.js';
import {
  CLIENT_ERROR_CODES,
  DOMAIN_ERROR_CODES,
  WIRE_ERROR_CODES,
  errorMessage,
} from '../src/errors.js';

/** The 20 codes exactly as the Rust bot test enumerates them (i18n.rs §tests). */
const BOT_DOMAIN_CODES = [
  'NON_INTEGER_AMOUNT',
  'NEGATIVE_AMOUNT',
  'AMOUNT_OUT_OF_RANGE',
  'BPS_OUT_OF_RANGE',
  'TOO_FEW_POSTINGS',
  'UNBALANCED_TRANSACTION',
  'UNKNOWN_ACCOUNT',
  'INVALID_ACCOUNT_KIND',
  'DUPLICATE_ACCOUNT',
  'ACCOUNT_OVERDRAFT',
  'INVALID_TRANSITION',
  'ESCROW_NOT_FOUND',
  'ESCROW_ALREADY_EXISTS',
  'REQUIRES_HUMAN_APPROVAL',
  'MINT_CAP_EXCEEDED',
  'HITL_NOT_FOUND',
  'HITL_NOT_REQUIRED',
  'HITL_ALREADY_DECIDED',
  'HITL_EXPIRED',
  'PRODUCT_NOT_FOUND',
] as const;

describe('locale — mirrors bot Locale semantics', () => {
  it('language_code mapping', () => {
    expect(localeFromLanguageCode('fa')).toBe('fa');
    expect(localeFromLanguageCode('en-US')).toBe('en');
    expect(localeFromLanguageCode(undefined)).toBe('fa'); // پیش‌فرض fa
    expect(localeFromLanguageCode('de')).toBe('fa');
    expect(DEFAULT_LOCALE).toBe('fa');
  });

  it('opposite + name + rtl', () => {
    expect(oppositeLocale('fa')).toBe('en');
    expect(oppositeLocale('en')).toBe('fa');
    expect(localeName('fa')).toBe('فارسی');
    expect(localeName('en')).toBe('English');
    expect(isRtl('fa')).toBe(true);
    expect(dirOf('fa')).toBe('rtl');
    expect(dirOf('en')).toBe('ltr');
  });
});

describe('dict parity — fa/en expose the exact same key set', () => {
  it('same keys, same order-independent set', () => {
    const faKeys = Object.keys(faDict).sort();
    const enKeys = Object.keys(enDict).sort();
    expect(enKeys).toEqual(faKeys);
  });

  it('every value is a non-empty string', () => {
    for (const dict of [faDict, enDict]) {
      for (const [k, v] of Object.entries(dict)) {
        expect(typeof v).toBe('string');
        expect(v.length).toBeGreaterThan(0);
      }
    }
  });

  it('fa and en messages are distinct where they must be (same rule as bot test)', () => {
    for (const key of Object.keys(faDict)) {
      const k = key as keyof typeof faDict;
      // appName is brand-identical by design; everything else must differ.
      if (k !== 'appName') expect(faDict[k]).not.toBe(enDict[k]);
    }
  });

  it('t() resolves and escrowStateLabel maps all 8 states', () => {
    expect(t('fa', 'splitPreview')).toContain('۹۰/۱۰');
    expect(t('en', 'splitPreview')).toContain('90/10');
    for (const state of ['CREATED', 'FUNDED', 'DELIVERED', 'RELEASED', 'DISPUTED', 'CANCELLED', 'EXPIRED', 'REFUNDED']) {
      expect(escrowStateLabel('fa', state)).not.toBe(state);
      expect(escrowStateLabel('en', state)).not.toBe(state);
      expect(escrowStateLabel('fa', state).length).toBeGreaterThan(0);
      expect(escrowStateLabel('en', state).length).toBeGreaterThan(0);
    }
    expect(escrowStateLabel('en', 'SOMETHING_ELSE')).toBe('SOMETHING_ELSE');
  });
});

describe('error dictionary — D8 mirror of bot i18n.rs', () => {
  const locales: Locale[] = ['fa', 'en'];

  it('covers every one of the 20 domain codes with distinct non-empty fa/en', () => {
    expect(DOMAIN_ERROR_CODES).toHaveLength(20);
    expect([...DOMAIN_ERROR_CODES].sort()).toEqual([...BOT_DOMAIN_CODES].sort());
    for (const code of DOMAIN_ERROR_CODES) {
      for (const locale of locales) {
        const msg = errorMessage(locale, code);
        expect(msg.length).toBeGreaterThan(0);
        expect(msg).not.toBe(code); // raw code never leaks
      }
      expect(errorMessage('fa', code)).not.toBe(errorMessage('en', code));
    }
  });

  it('mirrors the exact bot wording for representative codes', () => {
    expect(errorMessage('fa', 'REQUIRES_HUMAN_APPROVAL')).toBe(
      '🛡 این مبلغ بالای آستانهٔ ۵۰٬۰۰۰ Đ است — باید از صف HITL تأیید ادمین بگذرد.',
    );
    expect(errorMessage('en', 'REQUIRES_HUMAN_APPROVAL')).toBe(
      '🛡 This amount is above the 50,000 Đ threshold — it must pass the HITL queue.',
    );
    expect(errorMessage('fa', 'ACCOUNT_OVERDRAFT')).toBe('💸 موجودی کافی نیست — کیف هرگز منفی نمی‌شود.');
    expect(errorMessage('en', 'HITL_ALREADY_DECIDED')).toBe(
      '🚫 This request was already decided — a second decision is void (zero refund).',
    );
  });

  it('covers every server wire code + client codes', () => {
    expect(WIRE_ERROR_CODES.length).toBeGreaterThanOrEqual(20);
    for (const code of [...WIRE_ERROR_CODES, ...CLIENT_ERROR_CODES]) {
      for (const locale of locales) {
        const msg = errorMessage(locale, code);
        expect(msg.length).toBeGreaterThan(0);
        expect(msg).not.toBe(code);
      }
      expect(errorMessage('fa', code)).not.toBe(errorMessage('en', code));
    }
  });

  it('unknown codes fall back to the bot catch-all (never the raw code)', () => {
    for (const locale of locales) {
      const msg = errorMessage(locale, 'TOTALLY_UNKNOWN_CODE');
      expect(msg).not.toContain('TOTALLY_UNKNOWN_CODE');
      expect(msg.length).toBeGreaterThan(0);
    }
    expect(errorMessage('en', 'TOTALLY_UNKNOWN_CODE')).toBe(
      '🚨 Internal error — this should never reach the user; logged to audit.',
    );
  });
});
