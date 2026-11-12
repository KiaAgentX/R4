/**
 * The D14 action ladder — how a citizen webapp performs money actions against
 * a FROZEN contract whose mutation routes are staff-keyed:
 *
 *   staff key present  → direct money call via the generated client (the same
 *                        idempotency-keyed path the admin dashboard uses)
 *   Telegram identity  → the identity-gated POST /ai/chat is the citizen's
 *                        action surface — but ONLY for skills the registry
 *                        actually grants citizens (escrow.create / deliver /
 *                        dispute); funding / release / cancel are bot verbs
 *   guest              → honest handoff to the Telegram bot with the exact
 *                        command to run — never a fake in-app confirmation
 *
 * Nothing here invents capability: the ladder mirrors
 * `crates/ai/src/registry.rs` (16 skills, staff_only flags) and the bot's
 * 14 user commands. The bot's /start takes no payload, so a deep-link OPENS
 * the bot and the exact command is shown next to it (copyable) — honest.
 */

import { t, type Locale } from '@agent-one/i18n/webapp';

export type ActorMode =
  | { kind: 'staff' }
  | { kind: 'citizen' }
  | { kind: 'guest' };

/** Precedence = the shell's role rule: stored staff key > Telegram > guest.
 *  A blank/whitespace key is NOT a key (same rule as loadAdminKey). */
export function actorMode(adminKey: string | null, initData: string | null): ActorMode {
  if (adminKey !== null && adminKey.trim().length > 0) return { kind: 'staff' };
  if (initData !== null) return { kind: 'citizen' };
  return { kind: 'guest' };
}

/** The six escrow money actions of the webapp map (pages 4, 8–12). */
export type EscrowAction = 'create' | 'fund' | 'deliver' | 'release' | 'dispute' | 'cancel';

/**
 * Can the citizen's AI chat execute this action? Mirrors the registry:
 * `escrow.create` / `escrow.deliver` / `escrow.dispute` are citizen skills;
 * fund / release / cancel exist only as bot verbs (or staff skills).
 */
export const CHAT_ACTION: Record<EscrowAction, boolean> = {
  create: true,
  deliver: true,
  dispute: true,
  fund: false,
  release: false,
  cancel: false,
};

/** The exact bot command for the action — mirrors `crates/bot/src/cmd.rs`.
 *  For `create` the seller+amount grammar wins when both are known; the
 *  product id form is the bot's first alternative. */
export function botCommand(action: EscrowAction, args: { escrowId?: string; productId?: string; seller?: string; amount?: number }): string {
  switch (action) {
    case 'create':
      // /buy <productId> | /buy <sellerTgId> <amount> — the bot's own grammar
      if (args.seller !== undefined && args.amount !== undefined) return `/buy ${args.seller} ${args.amount}`;
      if (args.productId !== undefined) return `/buy ${args.productId}`;
      return `/buy ${args.seller ?? '<seller>'} ${args.amount ?? '<amount>'}`;
    case 'fund':
      return `/fund ${args.escrowId ?? '<id>'}`;
    case 'deliver':
      return `/deliver ${args.escrowId ?? '<id>'}`;
    case 'release':
      return `/release ${args.escrowId ?? '<id>'}`;
    case 'dispute':
      return `/dispute ${args.escrowId ?? '<id>'} <reason>`;
    case 'cancel':
      return `/cancel ${args.escrowId ?? '<id>'}`;
  }
}

/** AI-chat prefill sentence — localized, with {id}/{reason}/{seller}/{amount}. */
export function chatPrefill(action: EscrowAction, locale: Locale, args: { escrowId?: string; reason?: string; seller?: string; amount?: number }): string {
  const key = {
    fund: 'chatPrefillFund',
    deliver: 'chatPrefillDeliver',
    release: 'chatPrefillRelease',
    dispute: 'chatPrefillDispute',
    cancel: 'chatPrefillCancel',
    create: 'chatPrefillBuy',
  } as const satisfies Record<EscrowAction, Parameters<typeof t>[1]>;
  const text = t(locale, key[action]);
  return fillTemplate(text, {
    id: args.escrowId ?? '',
    reason: args.reason ?? '—',
    seller: args.seller ?? '',
    amount: args.amount !== undefined ? String(args.amount) : '',
  });
}

/** `{x}` substitution — unknown placeholders stay literal (honest template). */
export function fillTemplate(text: string, values: Record<string, string>): string {
  return text.replace(/\{(\w+)\}/g, (whole, key: string) => values[key] ?? whole);
}

export type ActionRoute =
  | { via: 'staff' }
  | { via: 'chat'; prefill: string }
  | { via: 'bot'; command: string }
  | { via: 'readonly' };

/**
 * The ladder decision — pure, fully testable. Staff wins (shell precedence);
 * citizens get chat for chat-capable skills and the bot for everything else;
 * guests always get the honest bot handoff.
 */
export function routeAction(input: {
  mode: ActorMode;
  action: EscrowAction;
  locale: Locale;
  escrowId?: string;
  productId?: string;
  reason?: string;
  seller?: string;
  amount?: number;
}): ActionRoute {
  const { mode, action, locale } = input;
  if (mode.kind === 'staff') return { via: 'staff' };
  if (mode.kind === 'citizen' && CHAT_ACTION[action]) {
    return { via: 'chat', prefill: chatPrefill(action, locale, input) };
  }
  if (mode.kind === 'guest' && action === 'create' && input.productId !== undefined && input.seller === undefined) {
    // A guest on a product page gets the bot's own /buy grammar.
    return { via: 'bot', command: botCommand('create', { productId: input.productId }) };
  }
  return { via: 'bot', command: botCommand(action, input) };
}

/** Telegram deep-link — OPENS the bot chat (its /start takes no payload). */
export function telegramDeepLink(handle: string | null): string | null {
  const h = handle?.trim().replace(/^@/, '') ?? '';
  return h.length > 0 ? `https://t.me/${h}` : null;
}

/** Guards a money mutation against double-execution while in flight. */
export class ActionBusyError extends Error {
  constructor() {
    super('an action is already in flight');
    this.name = 'ActionBusyError';
  }
}
