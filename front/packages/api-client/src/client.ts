/**
 * AgentOneClient — the typed HTTP surface of the frozen v3 contract.
 *
 * Layered per STEP1-AUDIT §3–§4, §8 (Phase 7 Step 2):
 *  • generated types from the pinned golden OpenAPI 3.1 (src/schema.d.ts, D5)
 *  • auth layer: admin `X-KIA-Key` header (dashboard) — never faked
 *  • Telegram identity layer: `initData` merged into POST /ai/chat (D14)
 *  • money layer: the 10 wire-idempotent mutations auto-inject a
 *    client-generated key (D7) — stable per logical action, reused on retry
 *  • no local money state, no optimistic moves — the server is the truth (D6)
 *
 * Results stay openapi-fetch style `{ data, error, response }`; `unwrap()`
 * converts them into typed `ApiRequestError`s — localized user text comes
 * from @agent-one/i18n, codes are audit-only and never user-facing.
 */
import createClient from 'openapi-fetch';
import type { paths } from './schema.js';
import {
  defaultIdempotencyStore,
  newIdempotencyKey,
  type IdempotencyStore,
} from './idempotency.js';

export type { paths } from './schema.js';

/** JSON request body of a POST path, extracted from the generated types. */
type JsonBody<Path extends keyof paths> =
  paths[Path] extends { post: { requestBody?: { content: { 'application/json': infer Body } } } }
    ? Body
    : never;

/**
 * The 10 money-mutation paths whose wire body carries `idempotencyKey`
 * (verified against the golden registry — nothing else is money-touching).
 */
export type MoneyPath =
  | '/wallet/mint'
  | '/wallet/burn'
  | '/escrows'
  | '/escrows/{id}/fund'
  | '/escrows/{id}/deliver'
  | '/escrows/{id}/release'
  | '/escrows/{id}/dispute'
  | '/escrows/{id}/refund'
  | '/escrows/{id}/cancel'
  | '/escrows/{id}/expire';

/** Caller may omit the key (client generates/persists it) or pin one. */
export type MoneyBody<Path extends MoneyPath> = Omit<JsonBody<Path>, 'idempotencyKey'> & {
  idempotencyKey?: string;
};

/** POST /ai/chat body — identity arrives via initData (Telegram or staff key). */
export type AiChatBody = JsonBody<'/ai/chat'>;

export interface AgentOneClientOptions {
  /** Default '/api/v1' — exactly the golden document's server url. */
  baseUrl?: string;
  /** Injectable fetch (tests, proxies, worker environments). */
  fetch?: typeof fetch;
  /** Key store — localStorage in the browser, memory elsewhere. */
  idempotencyStore?: IdempotencyStore;
}

const storeDefaults = defaultIdempotencyStore;

function buildClient(options: AgentOneClientOptions) {
  const store = options.idempotencyStore ?? storeDefaults();
  const api = createClient<paths>({
    baseUrl: options.baseUrl ?? '/api/v1',
    ...(options.fetch !== undefined ? { fetch: options.fetch } : {}),
  });

  let adminKey: string | undefined;
  let initData: string | undefined;

  api.use({
    onRequest({ request }) {
      if (adminKey !== undefined && !request.headers.has('X-KIA-Key')) {
        request.headers.set('X-KIA-Key', adminKey);
      }
      return request;
    },
  });

  /** The key for this logical action — explicit pin wins, else store, else fresh. */
  function resolveKey(actionId: string | undefined, pinned: string | undefined): string {
    if (pinned !== undefined) return pinned;
    return actionId !== undefined ? store.for(actionId) : newIdempotencyKey();
  }

  const money = {
    /** Money mutations — idempotency key injected from the store (D7). */
    mint: (body: MoneyBody<'/wallet/mint'>, actionId?: string) =>
      api.POST('/wallet/mint', {
        body: { ...body, idempotencyKey: resolveKey(actionId, body.idempotencyKey) },
      }),
    burn: (body: MoneyBody<'/wallet/burn'>, actionId?: string) =>
      api.POST('/wallet/burn', {
        body: { ...body, idempotencyKey: resolveKey(actionId, body.idempotencyKey) },
      }),
    createEscrow: (body: MoneyBody<'/escrows'>, actionId?: string) =>
      api.POST('/escrows', {
        body: { ...body, idempotencyKey: resolveKey(actionId, body.idempotencyKey) },
      }),
    fund: (escrowId: string, body: MoneyBody<'/escrows/{id}/fund'>, actionId?: string) =>
      api.POST('/escrows/{id}/fund', {
        params: { path: { id: escrowId } },
        body: { ...body, idempotencyKey: resolveKey(actionId, body.idempotencyKey) },
      }),
    deliver: (escrowId: string, body: MoneyBody<'/escrows/{id}/deliver'>, actionId?: string) =>
      api.POST('/escrows/{id}/deliver', {
        params: { path: { id: escrowId } },
        body: { ...body, idempotencyKey: resolveKey(actionId, body.idempotencyKey) },
      }),
    release: (escrowId: string, body: MoneyBody<'/escrows/{id}/release'>, actionId?: string) =>
      api.POST('/escrows/{id}/release', {
        params: { path: { id: escrowId } },
        body: { ...body, idempotencyKey: resolveKey(actionId, body.idempotencyKey) },
      }),
    dispute: (escrowId: string, body: MoneyBody<'/escrows/{id}/dispute'>, actionId?: string) =>
      api.POST('/escrows/{id}/dispute', {
        params: { path: { id: escrowId } },
        body: { ...body, idempotencyKey: resolveKey(actionId, body.idempotencyKey) },
      }),
    refund: (escrowId: string, body: MoneyBody<'/escrows/{id}/refund'>, actionId?: string) =>
      api.POST('/escrows/{id}/refund', {
        params: { path: { id: escrowId } },
        body: { ...body, idempotencyKey: resolveKey(actionId, body.idempotencyKey) },
      }),
    cancel: (escrowId: string, body: MoneyBody<'/escrows/{id}/cancel'>, actionId?: string) =>
      api.POST('/escrows/{id}/cancel', {
        params: { path: { id: escrowId } },
        body: { ...body, idempotencyKey: resolveKey(actionId, body.idempotencyKey) },
      }),
    expire: (escrowId: string, body: MoneyBody<'/escrows/{id}/expire'>, actionId?: string) =>
      api.POST('/escrows/{id}/expire', {
        params: { path: { id: escrowId } },
        body: { ...body, idempotencyKey: resolveKey(actionId, body.idempotencyKey) },
      }),
  };

  /** AI console — initData merged from the identity layer when unset. */
  const ai = {
    chat: (body: AiChatBody & { initData?: string }) =>
      api.POST('/ai/chat', {
        body: body.initData === undefined && initData !== undefined ? { ...body, initData } : body,
      }),
  };

  /** Citizen read helpers — the D14 read-only webapp surface. */
  const reads = {
    health: () => api.GET('/health'),
    economy: () => api.GET('/economy'),
    balance: (accountId: string) =>
      api.GET('/accounts/{id}/balance', { params: { path: { id: accountId } } }),
    escrow: (id: string) => api.GET('/escrows/{id}', { params: { path: { id } } }),
    escrows: () => api.GET('/escrows'),
    products: () => api.GET('/products'),
    brand: () => api.GET('/brand'),
    agents: () => api.GET('/ai/agents'),
    skills: () => api.GET('/ai/skills'),
    leaderboard: () => api.GET('/leaderboard'),
    quests: () => api.GET('/quests'),
    bookings: () => api.GET('/bookings'),
  };

  return { api, store, money, ai, reads, setAdminKey, setTelegramInitData };

  function setAdminKey(key: string | undefined): void {
    adminKey = key;
  }

  function setTelegramInitData(data: string | undefined): void {
    initData = data;
  }
}

type Built = ReturnType<typeof buildClient>;

/** The assembled client — precise types flow from the implementation. */
export interface AgentOneClient extends Pick<Built, 'api' | 'setAdminKey' | 'setTelegramInitData'> {
  /** Full generated surface — every one of the 46 operations, typed. */
  readonly api: Built['api'];
  /** Stable per-runtime idempotency store (inspect the offline queue). */
  readonly idempotency: IdempotencyStore;
  readonly money: Built['money'];
  readonly ai: Built['ai'];
  readonly reads: Built['reads'];
}

export function createAgentOneClient(options: AgentOneClientOptions = {}): AgentOneClient {
  const built = buildClient(options);
  return {
    api: built.api,
    idempotency: built.store,
    money: built.money,
    ai: built.ai,
    reads: built.reads,
    setAdminKey: built.setAdminKey,
    setTelegramInitData: built.setTelegramInitData,
  };
}
