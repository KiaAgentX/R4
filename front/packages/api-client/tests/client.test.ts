import { afterEach, describe, expect, it } from 'vitest';
import { createAgentOneClient, type AgentOneClient } from '../src/client.js';
import { ApiRequestError, clientErrorCode, unwrap } from '../src/errors.js';
import { MemoryIdempotencyStore } from '../src/idempotency.js';

type CapturedCall = { url: string; request: Request };

function mockFetch(onCall?: (c: CapturedCall, i: number) => Response | Promise<Response>) {
  const calls: CapturedCall[] = [];
  const impl = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const request = input instanceof Request ? input : new Request(input, init);
    const i = calls.length;
    calls.push({ url: request.url, request });
    const custom = await onCall?.(calls[i]!, i);
    return (
      custom ??
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      })
    );
  };
  return { calls, impl: impl as typeof fetch };
}

function clientWith(fetchImpl: typeof fetch, store = new MemoryIdempotencyStore()): AgentOneClient {
  return createAgentOneClient({ fetch: fetchImpl, idempotencyStore: store });
}

afterEach(() => {
  localStorage.clear();
});

describe('money layer — idempotency key injection (D7)', () => {
  it('same actionId → same body key across retries (replay-safe)', async () => {
    const { calls, impl } = mockFetch();
    const c = clientWith(impl);
    const body = { accountId: 'user:tg:1', amount: 40_000 };

    await c.money.mint(body, 'mint:user:tg:1:2026-11');
    await c.money.mint(body, 'mint:user:tg:1:2026-11');

    expect(calls).toHaveLength(2);
    const first = (await calls[0]!.request.json()) as { idempotencyKey: string };
    const second = (await calls[1]!.request.json()) as { idempotencyKey: string };
    expect(first.idempotencyKey).toBe(second.idempotencyKey);
    expect(first.idempotencyKey.length).toBeGreaterThanOrEqual(8);
  });

  it('no actionId → a fresh key every call; explicit pin wins', async () => {
    const { calls, impl } = mockFetch();
    const c = clientWith(impl);
    const body = { accountId: 'user:tg:1', amount: 1 };

    await c.money.mint(body);
    await c.money.mint(body);
    await c.money.mint({ ...body, idempotencyKey: 'pinned-key-1' });

    const k0 = ((await calls[0]!.request.json()) as { idempotencyKey: string }).idempotencyKey;
    const k1 = ((await calls[1]!.request.json()) as { idempotencyKey: string }).idempotencyKey;
    const k2 = ((await calls[2]!.request.json()) as { idempotencyKey: string }).idempotencyKey;
    expect(k0).not.toBe(k1);
    expect(k2).toBe('pinned-key-1');
  });

  it('escrow transition carries the path id + stable key', async () => {
    const { calls, impl } = mockFetch();
    const c = clientWith(impl);
    await c.money.release('esc:c1m2', { approvedByAdmin: false }, 'release:esc:c1m2');
    await c.money.release('esc:c1m2', { approvedByAdmin: false }, 'release:esc:c1m2');

    const url = new URL(calls[0]!.url);
    const segments = decodeURIComponent(url.pathname).split('/');
    expect(segments).toContain('esc:c1m2');
    expect(segments.at(-1)).toBe('release');
    const b0 = (await calls[0]!.request.json()) as { idempotencyKey: string };
    const b1 = (await calls[1]!.request.json()) as { idempotencyKey: string };
    expect(b0.idempotencyKey).toBe(b1.idempotencyKey);
  });

  it('keys persist in the shared store across client instances (reload semantics)', async () => {
    const { impl } = mockFetch();
    const store = new MemoryIdempotencyStore();
    const a = clientWith(impl, store);
    const b = clientWith(impl, store);
    await a.money.mint({ accountId: 'u', amount: 5 }, 'shared:action');
    await b.money.mint({ accountId: 'u', amount: 5 }, 'shared:action');
    expect(store.peek('shared:action')).not.toBeNull();
  });
});

describe('auth + identity layers', () => {
  it('admin key → X-KIA-Key header; clearing removes it', async () => {
    const { calls, impl } = mockFetch();
    const c = clientWith(impl);

    await c.reads.health();
    expect(calls[0]!.request.headers.get('X-KIA-Key')).toBeNull();

    c.setAdminKey('kia-admin-secret-0123456789');
    await c.reads.health();
    expect(calls[1]!.request.headers.get('X-KIA-Key')).toBe('kia-admin-secret-0123456789');

    c.setAdminKey(undefined);
    await c.reads.health();
    expect(calls[2]!.request.headers.get('X-KIA-Key')).toBeNull();
  });

  it('ai.chat merges stored initData; explicit body initData wins; cleared → absent', async () => {
    const { calls, impl } = mockFetch();
    const c = clientWith(impl);
    const base = { userId: 'user:tg:9', text: 'سلام' } as { userId: string; text: string; initData?: string; role?: 'user' | 'staff' };

    await c.ai.chat(base);
    expect((await calls[0]!.request.json() as { initData?: string }).initData).toBeUndefined();

    c.setTelegramInitData('query_id=AA&hash=abc');
    await c.ai.chat(base);
    expect((await calls[1]!.request.json() as { initData?: string }).initData).toBe('query_id=AA&hash=abc');

    await c.ai.chat({ ...base, initData: 'explicit-hash' });
    expect((await calls[2]!.request.json() as { initData?: string }).initData).toBe('explicit-hash');

    c.setTelegramInitData(undefined);
    await c.ai.chat(base);
    expect((await calls[3]!.request.json() as { initData?: string }).initData).toBeUndefined();
  });

  it('default baseUrl is exactly the golden server url /api/v1', async () => {
    const { calls, impl } = mockFetch();
    const c = clientWith(impl);
    await c.reads.products();
    expect(new URL(calls[0]!.url).pathname.replace(/\/$/, '')).toBe('/api/v1/products');
  });
});

describe('failure mapping — localized text comes from codes, never raw bodies', () => {
  it('401 envelope → ApiRequestError with code INVALID_ADMIN_KEY', async () => {
    const { impl } = mockFetch(
      () =>
        new Response(JSON.stringify({ error: { code: 'INVALID_ADMIN_KEY', message: 'nope' } }), {
          status: 401,
          headers: { 'content-type': 'application/json' },
        }),
    );
    const c = clientWith(impl);
    const res = await c.money.mint({ accountId: 'u', amount: 5 });
    const err = await unwrap(res).catch((e) => e);
    expect(err).toBeInstanceOf(ApiRequestError);
    expect((err as ApiRequestError).api).toMatchObject({ status: 401, code: 'INVALID_ADMIN_KEY' });
  });

  it('network failure → NETWORK_FAILED; abort → REQUEST_ABORTED', async () => {
    const failing = (async () => {
      throw new TypeError('Failed to fetch');
    }) as unknown as typeof fetch;
    const c = clientWith(failing);
    try {
      await c.reads.health();
      expect.unreachable('should have thrown');
    } catch (e) {
      expect(clientErrorCode(e)).toBe('NETWORK_FAILED');
    }

    const aborting = (async () => {
      throw new DOMException('aborted', 'AbortError');
    }) as unknown as typeof fetch;
    const c2 = clientWith(aborting);
    try {
      await c2.reads.health();
      expect.unreachable('should have thrown');
    } catch (e) {
      expect(clientErrorCode(e)).toBe('REQUEST_ABORTED');
    }
  });
});
