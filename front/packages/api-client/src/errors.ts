/**
 * Error envelope handling — the server always answers failures with
 * `{ "error": { "code", "message", "meta?" } }` (frozen phase-3 contract).
 * Client-side: parse into a typed ApiError; a malformed envelope becomes
 * MALFORMED_ERROR_BODY (same discipline as the phase-3 round-trip tests).
 * Localized user text comes from @agent-one/i18n — codes are audit-only.
 */

export interface ApiError {
  /** HTTP status, 0 when the request never reached the server. */
  status: number;
  /** Wire code — mirrors the backend registry exactly. */
  code: string;
  /** Server message (kept for logs; the UI localizes the CODE, not this). */
  message: string;
  /** Structured meta (e.g. RATE_LIMITED → { limit }). */
  meta?: Record<string, unknown>;
}

export class ApiRequestError extends Error {
  constructor(readonly api: ApiError) {
    super(api.message.length > 0 ? api.message : api.code);
    this.name = 'ApiRequestError';
  }
}

/** The client-side fallback code when the envelope itself is broken. */
export const MALFORMED_ERROR_BODY = 'MALFORMED_ERROR_BODY';
export const NETWORK_FAILED = 'NETWORK_FAILED';
export const REQUEST_ABORTED = 'REQUEST_ABORTED';

/** Parse any failure body into an ApiError — never throws. */
export function parseApiError(status: number, body: unknown): ApiError {
  const err = (body as { error?: unknown } | null | undefined)?.error;
  if (
    err !== null &&
    typeof err === 'object' &&
    typeof (err as { code?: unknown }).code === 'string' &&
    typeof (err as { message?: unknown }).message === 'string'
  ) {
    const e = err as { code: string; message: string; meta?: Record<string, unknown> };
    const api: ApiError = { status, code: e.code, message: e.message };
    if (e.meta !== undefined && e.meta !== null && typeof e.meta === 'object') api.meta = e.meta;
    return api;
  }
  return { status, code: MALFORMED_ERROR_BODY, message: '' };
}

/** openapi-fetch result shape (data / error / response). */
export interface FetchResult<T> {
  data?: T;
  error?: unknown;
  response: Response;
}

/**
 * Unwrap an openapi-fetch result into plain data.
 * Throws ApiRequestError with the parsed envelope on any failure.
 */
export async function unwrap<T>(res: FetchResult<T>): Promise<T> {
  if (res.error !== undefined && res.error !== null) {
    throw new ApiRequestError(parseApiError(res.response.status, res.error));
  }
  if (!res.response.ok) {
    throw new ApiRequestError({ status: res.response.status, code: MALFORMED_ERROR_BODY, message: '' });
  }
  return res.data as T;
}

/** Map a thrown fetch-layer exception (network / abort) to its client code. */
export function clientErrorCode(e: unknown): string {
  if (e instanceof ApiRequestError) return e.api.code;
  if (e instanceof DOMException && e.name === 'AbortError') return REQUEST_ABORTED;
  return NETWORK_FAILED;
}
