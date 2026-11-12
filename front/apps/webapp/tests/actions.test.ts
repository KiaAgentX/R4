import { describe, expect, it } from 'vitest';
import {
  actorMode,
  botCommand,
  CHAT_ACTION,
  chatPrefill,
  fillTemplate,
  routeAction,
  telegramDeepLink,
  type ActorMode,
} from '../src/actions.js';

const GUEST: ActorMode = { kind: 'guest' };
const CITIZEN: ActorMode = { kind: 'citizen' };
const STAFF: ActorMode = { kind: 'staff' };

describe('actions — the D14 ladder (staff > citizen > guest)', () => {
  it('staff mode always routes to the direct idempotency-keyed call', () => {
    for (const action of ['create', 'fund', 'deliver', 'release', 'dispute', 'cancel'] as const) {
      expect(routeAction({ mode: STAFF, action, locale: 'fa', escrowId: 'esc-1' }).via).toBe('staff');
    }
  });

  it('citizen chat covers exactly the registry\u2019s citizen skills (create/deliver/dispute)', () => {
    expect(CHAT_ACTION.create).toBe(true);
    expect(CHAT_ACTION.deliver).toBe(true);
    expect(CHAT_ACTION.dispute).toBe(true);
    expect(CHAT_ACTION.fund).toBe(false); // bot verb only
    expect(CHAT_ACTION.release).toBe(false); // staff settlement on the frozen contract
    expect(CHAT_ACTION.cancel).toBe(false); // bot verb only
  });

  it('citizen + chat-capable action → the assistant with a prefilled sentence', () => {
    const route = routeAction({ mode: CITIZEN, action: 'deliver', locale: 'fa', escrowId: 'esc-9' });
    expect(route).toEqual({ via: 'chat', prefill: expect.stringContaining('esc-9') });
    expect(route.via === 'chat' && route.prefill.length > 5).toBe(true);
  });

  it('citizen + bot-verb action (fund/release/cancel) → the Telegram bot with the exact command', () => {
    const route = routeAction({ mode: CITIZEN, action: 'fund', locale: 'fa', escrowId: 'esc-9' });
    expect(route).toEqual({ via: 'bot', command: '/fund esc-9' });
    expect(routeAction({ mode: CITIZEN, action: 'release', locale: 'en', escrowId: 'esc-9' })).toEqual({
      via: 'bot',
      command: '/release esc-9',
    });
  });

  it('guest always gets the honest bot handoff — never a fake in-app confirm', () => {
    const route = routeAction({ mode: GUEST, action: 'dispute', locale: 'fa', escrowId: 'esc-1', reason: 'delay' });
    expect(route).toEqual({ via: 'bot', command: '/dispute esc-1 <reason>' });
  });

  it('guest on a product page (id only) gets the bot\u2019s own /buy grammar', () => {
    const route = routeAction({ mode: GUEST, action: 'create', locale: 'fa', productId: 'prod-3' });
    expect(route).toEqual({ via: 'bot', command: '/buy prod-3' });
  });

  it('guest with an explicit seller+amount gets /buy <seller> <amount>', () => {
    const route = routeAction({ mode: GUEST, action: 'create', locale: 'fa', seller: 'user:tg:7', amount: 20000, productId: 'prod-3' });
    expect(route.via === 'bot' && route.command.startsWith('/buy user:tg:7 20000')).toBe(true);
  });
});

describe('actions — bot command grammar mirrors crates/bot/src/cmd.rs', () => {
  it('formats every escrow verb', () => {
    expect(botCommand('fund', { escrowId: 'e1' })).toBe('/fund e1');
    expect(botCommand('deliver', { escrowId: 'e1' })).toBe('/deliver e1');
    expect(botCommand('release', { escrowId: 'e1' })).toBe('/release e1');
    expect(botCommand('cancel', { escrowId: 'e1' })).toBe('/cancel e1');
    expect(botCommand('dispute', { escrowId: 'e1' })).toBe('/dispute e1 <reason>');
    expect(botCommand('create', { productId: 'p1' })).toBe('/buy p1');
    expect(botCommand('create', { seller: 'user:tg:2', amount: 1000 })).toBe('/buy user:tg:2 1000');
  });

  it('never fabricates ids — placeholders stay visible', () => {
    expect(botCommand('fund', {})).toBe('/fund <id>');
  });
});

describe('actions — prefills are localized templates', () => {
  it('fills {id} / {seller} / {amount} / {reason}', () => {
    expect(chatPrefill('fund', 'fa', { escrowId: 'esc-2' })).toBe('سلام — می‌خواهم امانت esc-2 را تأمین مالی کنم.');
    expect(chatPrefill('create', 'en', { seller: 'user:tg:7', amount: 20000 })).toContain('user:tg:7');
    expect(chatPrefill('create', 'en', { seller: 'user:tg:7', amount: 20000 })).toContain('20000');
    expect(chatPrefill('dispute', 'fa', { escrowId: 'e', reason: 'دیر شد' })).toContain('دیر شد');
  });

  it('fillTemplate leaves unknown placeholders literal (honest templates)', () => {
    expect(fillTemplate('a {x} b {id}', { id: '9' })).toBe('a {x} b 9');
  });
});

describe('actions — actor mode and deep links', () => {
  it('admin key wins over Telegram (the shell role rule)', () => {
    expect(actorMode('kia', 'initData').kind).toBe('staff');
    expect(actorMode(null, 'initData').kind).toBe('citizen');
    expect(actorMode(null, null).kind).toBe('guest');
    expect(actorMode('  ', 'initData').kind).toBe('citizen'); // blank ≠ key
  });

  it('deep links need a handle — missing handle is null, not a fake link', () => {
    expect(telegramDeepLink('DropAgentX_bot')).toBe('https://t.me/DropAgentX_bot');
    expect(telegramDeepLink('@bot')).toBe('https://t.me/bot');
    expect(telegramDeepLink(null)).toBeNull();
    expect(telegramDeepLink('  ')).toBeNull();
  });
});
