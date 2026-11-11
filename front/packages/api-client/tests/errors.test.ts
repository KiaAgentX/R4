import { describe, expect, it } from 'vitest';
import { ApiRequestError, clientErrorCode, parseApiError, unwrap } from '../src/errors.js';

describe('parseApiError — the frozen {error:{code,message,meta}} envelope', () => {
  it('parses a proper envelope', () => {
    const e = parseApiError(429, { error: { code: 'RATE_LIMITED', message: 'slow down', meta: { limit: 60 } } });
    expect(e).toEqual({ status: 429, code: 'RATE_LIMITED', message: 'slow down', meta: { limit: 60 } });
  });

  it('meta is optional', () => {
    const e = parseApiError(404, { error: { code: 'ESCROW_NOT_FOUND', message: 'nope' } });
    expect(e.code).toBe('ESCROW_NOT_FOUND');
    expect(e.meta).toBeUndefined();
  });

  it('malformed body → MALFORMED_ERROR_BODY (phase-3 discipline)', () => {
    expect(parseApiError(500, 'boom').code).toBe('MALFORMED_ERROR_BODY');
    expect(parseApiError(500, { oops: true }).code).toBe('MALFORMED_ERROR_BODY');
    expect(parseApiError(500, { error: { code: 1, message: null } }).code).toBe('MALFORMED_ERROR_BODY');
    expect(parseApiError(500, null).code).toBe('MALFORMED_ERROR_BODY');
  });
});

describe('unwrap + clientErrorCode', () => {
  const jsonRes = (status: number, body: unknown) =>
    new Response(JSON.stringify(body), {
      status,
      headers: { 'content-type': 'application/json' },
    });

  it('unwrap returns data on success', async () => {
    const data = await unwrap({ data: { ok: true }, response: jsonRes(200, { ok: true }) });
    expect(data).toEqual({ ok: true });
  });

  it('unwrap throws ApiRequestError with the parsed envelope', async () => {
    const err = await unwrap({
      error: { error: { code: 'INVALID_ADMIN_KEY', message: 'nope' } },
      response: jsonRes(401, { error: { code: 'INVALID_ADMIN_KEY', message: 'nope' } }),
    }).catch((e) => e);
    expect(err).toBeInstanceOf(ApiRequestError);
    expect((err as ApiRequestError).api).toMatchObject({ status: 401, code: 'INVALID_ADMIN_KEY' });
  });

  it('clientErrorCode: ApiRequestError → its code, abort → REQUEST_ABORTED, else NETWORK_FAILED', () => {
    const apiErr = new ApiRequestError({ status: 429, code: 'RATE_LIMITED', message: '' });
    expect(clientErrorCode(apiErr)).toBe('RATE_LIMITED');
    expect(clientErrorCode(new DOMException('user aborted', 'AbortError'))).toBe('REQUEST_ABORTED');
    expect(clientErrorCode(new TypeError('Failed to fetch'))).toBe('NETWORK_FAILED');
  });
});
