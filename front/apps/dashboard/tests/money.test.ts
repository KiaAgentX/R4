import { describe, expect, it } from 'vitest';
import { createAgentOneClient } from '@agent-one/api-client';
import { ApiRequestError } from '@agent-one/api-client/errors';
import { MemoryIdempotencyStore } from '@agent-one/api-client/idempotency';
import { settleAction, treasuryActionId } from '../src/money.js';

describe('money — D7 treasury idempotency discipline', () => {
  it('action ids are stable per parameters and distinct per kind/account/amount', () => {
    expect(treasuryActionId('mint', 'user:1', 5000)).toBe('dash-mint:user:1:5000');
    expect(treasuryActionId('burn', 'user:1', 5000)).not.toBe(treasuryActionId('mint', 'user:1', 5000));
    expect(treasuryActionId('mint', 'user:2', 5000)).not.toBe(treasuryActionId('mint', 'user:1', 5000));
    expect(treasuryActionId('mint', 'user:1', 6000)).not.toBe(treasuryActionId('mint', 'user:1', 5000));
  });

  it('releases the key on terminal (envelope) errors — a deliberate repeat is a new action', () => {
    const store = new MemoryIdempotencyStore();
    const client = createAgentOneClient({ idempotencyStore: store });
    const id = treasuryActionId('mint', 'user:1', 100);
    store.for(id);
    expect(store.peek(id)).not.toBeNull();
    settleAction(client, id, new ApiRequestError({ status: 400, code: 'INVALID_REQUEST', message: '' }));
    expect(store.peek(id)).toBeNull();
  });

  it('keeps the key on lost responses — a retry must replay the SAME key (Tx.n UNIQUE)', () => {
    const store = new MemoryIdempotencyStore();
    const client = createAgentOneClient({ idempotencyStore: store });
    const id = treasuryActionId('mint', 'user:1', 100);
    const first = store.for(id);
    settleAction(client, id, new Error('network down')); // mapped to NETWORK_FAILED
    expect(store.peek(id)).toBe(first);
  });

  it('keeps the key on aborts too (the server may still have committed)', () => {
    const store = new MemoryIdempotencyStore();
    const client = createAgentOneClient({ idempotencyStore: store });
    const id = treasuryActionId('burn', 'user:9', 250);
    const first = store.for(id);
    settleAction(client, id, new DOMException('aborted', 'AbortError'));
    expect(store.peek(id)).toBe(first);
  });
});
