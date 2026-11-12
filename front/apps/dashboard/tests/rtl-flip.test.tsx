/**
 * RTL flip — Phase 7 Step 7. The admin dashboard must flip <html> dir/lang
 * when the language toggles: fa (default) → rtl, en → ltr. The fetch stub is
 * installed with vi.hoisted BEFORE the App import: openapi-fetch captures
 * `globalThis.fetch` when the module-level `createAgentOneClient(...)` runs,
 * so a per-test stub would never be seen. The gate probe is served by the
 * stub so the flip test stays fully offline.
 */
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { StrictMode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const installOfflineFetch = vi.hoisted(() => {
  const jsonResponse = (payload: unknown): Response =>
    new Response(JSON.stringify(payload), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    });
  const stub = ((input: RequestInfo | URL): Promise<Response> => {
    const url = input instanceof Request ? input.url : String(input);
    if (url.includes('/overview')) {
      return Promise.resolve(
        jsonResponse({
          ok: true,
          data: {
            users: 1,
            products: 1,
            orders: 0,
            revenueStars: 0,
            revenueCredits: 0,
            pendingApprovals: 0,
            aiCalls: 0,
            series: [],
            topProducts: [],
            recentOrders: [],
          },
        }),
      );
    }
    if (url.includes('/brand')) {
      return Promise.resolve(jsonResponse({ ok: true, data: { name: 'Agent-One', tagline: 'Empire' } }));
    }
    return Promise.reject(new TypeError(`offline flip test: ${url}`));
  }) as typeof fetch;
  // openapi-fetch defaults its fetch at createClient() time — patch early.
  globalThis.fetch = stub;
  return stub;
});

import App from '../src/App.js';

const tick = (): Promise<void> => new Promise((r) => setTimeout(r, 0));

async function clickByAria(label: string): Promise<void> {
  const btn = document.querySelector(`button[aria-label="${label}"]`);
  if (!(btn instanceof HTMLButtonElement)) throw new Error(`no button with aria-label ${label}`);
  btn.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  await tick();
  await tick();
}

describe('dashboard — RTL flip', () => {
  let root: Root | undefined;

  beforeEach(() => {
    localStorage.clear();
    localStorage.setItem('ao-locale', 'fa');
    localStorage.setItem('ao-admin-key', 'test-admin-key-0123456789');
    globalThis.fetch = installOfflineFetch;
  });

  afterEach(() => {
    root?.unmount();
    root = undefined;
    document.documentElement.lang = '';
    document.documentElement.dir = '';
    localStorage.clear();
  });

  it('fa renders rtl; the language button flips <html> to ltr and back', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    root.render(
      <StrictMode>
        <QueryClientProvider client={client}>
          <App />
        </QueryClientProvider>
      </StrictMode>,
    );
    // the gate probe must settle (ok) before the header exists
    for (let i = 0; i < 12; i++) {
      await tick();
      if (document.querySelector('button[aria-label="زبان"], button[aria-label="Language"]') !== null) break;
    }

    expect(document.documentElement.dir).toBe('rtl');
    expect(document.documentElement.lang).toBe('fa');

    await clickByAria('زبان'); // fa → en
    expect(document.documentElement.dir).toBe('ltr');
    expect(document.documentElement.lang).toBe('en');
    expect(localStorage.getItem('ao-locale')).toBe('en');

    await clickByAria('Language'); // en → fa
    expect(document.documentElement.dir).toBe('rtl');
    expect(localStorage.getItem('ao-locale')).toBe('fa');
  });
});
