import { describe, expect, it } from 'vitest';
import { BOT_ADMIN_COMMANDS, BOT_COMMANDS, CHARTER_ARTICLES, charterTitle, charterValue, FAQ } from '../src/content.js';
import { ESCROW_STATES, ESCROW_TRANSITIONS, isTerminalState, STATE_ACTIONS } from '../src/format.js';

describe('content — the charter mirrors crates/domain/src/charter.rs', () => {
  it('ships exactly 10 articles, numbered 1..10', () => {
    expect(CHARTER_ARTICLES.length).toBe(10);
    CHARTER_ARTICLES.forEach((a, i) => expect(a.article).toBe(i + 1));
  });

  it('keeps the Persian text VERBATIM from the domain source', () => {
    // Spot checks against charter.rs CHARTER_ARTICLES (fa preserved 1:1)
    expect(CHARTER_ARTICLES[0]).toMatchObject({ faTitle: 'واحد پول', faValue: 'دینار دیجیتال (Đ / DNX) — عدد صحیح، بدون اعشار' });
    expect(CHARTER_ARTICLES[1]).toMatchObject({ faValue: '۱۰٪ هر تسویه اسکرو به خزانه می‌رود' });
    expect(CHARTER_ARTICLES[6]).toMatchObject({ faValue: 'عملیات ≥ ۵۰٬۰۰۰ Đ نیازمند تایید انسانی است' });
    expect(CHARTER_ARTICLES[7]).toMatchObject({ faValue: 'خرید از مارکت فقط از مسیر اسکرو انجام می‌شود' });
    expect(CHARTER_ARTICLES[9]).toMatchObject({ faValue: 'گپ تاریخی −۶۳ Đ با یک ردیف تعدیل شفاف ثبت شده است' });
  });

  it('exposes localized accessors for both locales', () => {
    const cap = CHARTER_ARTICLES[2];
    const tax = CHARTER_ARTICLES[1];
    expect(cap).toBeDefined();
    expect(tax).toBeDefined();
    expect(charterTitle(cap!, 'fa')).toBe('سقف ضرب');
    expect(charterTitle(cap!, 'en')).toBe('Mint cap');
    expect(charterValue(tax!, 'en')).toContain('10% of every escrow settlement');
  });
});

describe('content — help mirrors the bot\u2019s commands', () => {
  it('lists the 14 user commands and the 5 admin commands', () => {
    expect(BOT_COMMANDS.length).toBe(14);
    expect(BOT_ADMIN_COMMANDS.length).toBe(5);
    const cmds = BOT_COMMANDS.map((c) => c.cmd);
    for (const c of ['/start', '/help', '/menu', '/balance', '/history', '/products', '/buy', '/status', '/fund', '/deliver', '/release', '/dispute', '/cancel', '/lang']) {
      expect(cmds).toContain(c);
    }
  });

  it('the FAQ answers real citizen questions in both locales', () => {
    expect(FAQ.length).toBeGreaterThanOrEqual(4);
    for (const item of FAQ) {
      expect(item.q.fa.length).toBeGreaterThan(10);
      expect(item.q.en.length).toBeGreaterThan(10);
      expect(item.a.fa.length).toBeGreaterThan(20);
      expect(item.a.en.length).toBeGreaterThan(20);
    }
    // The money-honesty answer mentions the 90/10 and the HITL threshold
    const faAll = FAQ.map((f) => `${f.q.fa} ${f.a.fa}`).join(' ');
    expect(faAll).toContain('۹۰٪');
    expect(faAll).toContain('۵۰٬۰۰۰');
  });
});

describe('format — the 8-state machine mirrors crates/domain/src/escrow.rs', () => {
  it('lists the 8 states in machine order', () => {
    expect(ESCROW_STATES).toEqual(['CREATED', 'FUNDED', 'DELIVERED', 'RELEASED', 'DISPUTED', 'REFUNDED', 'CANCELLED', 'EXPIRED']);
  });

  it('the transition table equals escrow_transitions', () => {
    expect(ESCROW_TRANSITIONS.CREATED).toEqual(['FUNDED', 'CANCELLED']);
    expect(ESCROW_TRANSITIONS.FUNDED).toEqual(['DELIVERED', 'DISPUTED', 'EXPIRED']);
    expect(ESCROW_TRANSITIONS.DELIVERED).toEqual(['RELEASED', 'DISPUTED']);
    expect(ESCROW_TRANSITIONS.DISPUTED).toEqual(['REFUNDED', 'RELEASED']);
    for (const terminal of ['RELEASED', 'REFUNDED', 'CANCELLED', 'EXPIRED'] as const) {
      expect(ESCROW_TRANSITIONS[terminal]).toEqual([]);
      expect(isTerminalState(terminal)).toBe(true);
    }
    expect(isTerminalState('CREATED')).toBe(false);
  });

  it('offers only legal money actions per state (§6)', () => {
    expect(STATE_ACTIONS.CREATED).toEqual(['fund', 'cancel']);
    expect(STATE_ACTIONS.FUNDED).toEqual(['deliver', 'dispute']); // expiry is the TTL sweep
    expect(STATE_ACTIONS.DELIVERED).toEqual(['release', 'dispute']);
    expect(STATE_ACTIONS.DISPUTED).toEqual([]); // arbitration decides
    for (const terminal of ['RELEASED', 'REFUNDED', 'CANCELLED', 'EXPIRED'] as const) {
      expect(STATE_ACTIONS[terminal]).toEqual([]);
    }
  });
});
