/**
 * The context every webapp page receives from the App frame — the assembled
 * typed client, the active locale, the D14 actor mode (staff / citizen /
 * guest), the display-only Telegram user, the bot handle and navigation.
 */
import type { AgentOneClient } from '@agent-one/api-client';
import type { Locale } from '@agent-one/i18n/webapp';
import type { ActorMode } from './actions.js';
import type { PageId } from './router.js';
import type { ActionLogEntry, TelegramUser } from './session.js';

export interface PageCtx {
  client: AgentOneClient;
  locale: Locale;
  mode: ActorMode;
  /** Raw Telegram initData — identity-critical calls carry it verbatim. */
  initData: string | null;
  /** Display-only Telegram user (initDataUnsafe public fields). */
  tgUser: TelegramUser | null;
  botHandle: string | null;
  navigate: (page: PageId, params?: readonly string[]) => void;
  /** Jump to the assistant chat with a prefilled sentence (D14 chat rung). */
  openChat: (prefill: string) => void;
  /** Append a receipt to the local action log (notifications page). */
  logAction: (entry: Omit<ActionLogEntry, 'at'>) => void;
}
