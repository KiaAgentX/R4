import { describe, expect, it } from 'vitest';
import { baseUrlFromEnv, optionsFromEnv } from '../src/client.js';

describe('baseUrlFromEnv — the single packaged-shell knob (Step 6)', () => {
  it('unset/empty/whitespace → undefined (golden /api/v1 parity in web builds)', () => {
    expect(baseUrlFromEnv(undefined)).toBeUndefined();
    expect(baseUrlFromEnv({})).toBeUndefined();
    expect(baseUrlFromEnv({ VITE_API_BASE: '' })).toBeUndefined();
    expect(baseUrlFromEnv({ VITE_API_BASE: '   ' })).toBeUndefined();
    expect(baseUrlFromEnv({ VITE_API_BASE: 42 })).toBeUndefined();
  });

  it('non-absolute or non-http(s) values are rejected, never guessed', () => {
    expect(baseUrlFromEnv({ VITE_API_BASE: 'api.example.com' })).toBeUndefined();
    expect(baseUrlFromEnv({ VITE_API_BASE: '/api/v9' })).toBeUndefined();
    expect(baseUrlFromEnv({ VITE_API_BASE: 'ftp://api.example.com' })).toBeUndefined();
    expect(baseUrlFromEnv({ VITE_API_BASE: 'javascript:alert(1)' })).toBeUndefined();
  });

  it('absolute http(s) origins pass through, trailing slashes stripped', () => {
    expect(baseUrlFromEnv({ VITE_API_BASE: 'https://api.dropagentx.example' })).toBe(
      'https://api.dropagentx.example',
    );
    expect(baseUrlFromEnv({ VITE_API_BASE: 'http://127.0.0.1:8080/' })).toBe('http://127.0.0.1:8080');
    expect(baseUrlFromEnv({ VITE_API_BASE: 'https://api.example.com//api///' })).toBe(
      'https://api.example.com//api',
    );
  });

  it('optionsFromEnv keeps exactOptionalPropertyTypes happy: {} by default', () => {
    expect(optionsFromEnv(undefined)).toEqual({});
    expect(optionsFromEnv({ VITE_API_BASE: 'nope' })).toEqual({});
    expect(optionsFromEnv({ VITE_API_BASE: 'https://api.example.com' })).toEqual({
      baseUrl: 'https://api.example.com',
    });
  });
});
