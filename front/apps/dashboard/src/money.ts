/**
 * Treasury money discipline (decision D7 — replay-safe admin mint/burn).
 *
 * An idempotency key is created ONCE per logical action and REUSED across
 * retries of that action; it is released only when the action settles.
 *  • released on success and on TERMINAL errors (4xx envelopes) — a fresh,
 *    deliberate repetition of the same parameters is a new logical action
 *  • KEPT on NETWORK_FAILED / REQUEST_ABORTED — the server may have committed
 *    even though the response was lost, so a retry must replay the SAME key
 *    and hit the backend `Tx.n` UNIQUE semantics (original tx returns)
 * Double-click is additionally blocked by the pending state of the mutation.
 */
import type { AgentOneClient } from '@agent-one/api-client';
import { clientErrorCode } from '@agent-one/api-client/errors';

/** Stable action id for a treasury operation (per exact parameters). */
export function treasuryActionId(kind: 'mint' | 'burn', accountId: string, amount: number): string {
  return `dash-${kind}:${accountId}:${amount}`;
}

/**
 * Release the action's key once it settled. Terminal = anything that is NOT
 * a lost-response (network/abort) failure.
 */
export function settleAction(client: AgentOneClient, actionId: string, error: unknown): void {
  const code = clientErrorCode(error);
  if (code !== 'NETWORK_FAILED' && code !== 'REQUEST_ABORTED') {
    client.idempotency.release(actionId);
  }
}
