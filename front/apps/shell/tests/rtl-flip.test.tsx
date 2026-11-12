/**
 * RTL flip — Phase 7 Step 7. The shell must flip <html> dir/lang when the
 * language toggles: fa (the nation's default) → rtl, en → ltr. The CSS layer
 * is logical-property-only (guarded by the ui-package audit test), so
 * flipping `dir` is the entire mirror job — this test proves the app really
 * flips it, online or offline. The fetch stub is installed with vi.hoisted
 * BEFORE the App import: openapi-fetch captures `globalThis.fetch` when the
 * module-level `createAgentOneClient(...)` runs, so a per-test stub would
 * never be seen.
 */
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { StrictMode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const installOfflineFetch = vi.hoisted(() => {
  const stub = ((input: RequestInfo | URL): Promise<Response> =>
    Promise.reject(new TypeError(`offline flip test: ${String(input)}`))) as typeof fetch;
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

describe('shell — RTL flip', () => {
  let root: Root | undefined;

  beforeEach(() => {
    localStorage.clear();
    localStorage.setItem('ao-locale', 'fa');
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
    await tick();
    await tick();

    expect(document.documentElement.dir).toBe('rtl');
    expect(document.documentElement.lang).toBe('fa');

    await clickByAria('زبان'); // fa → en
    expect(document.documentElement.dir).toBe('ltr');
    expect(document.documentElement.lang).toBe('en');
    expect(localStorage.getItem('ao-locale')).toBe('en');

    await clickByAria('Language'); // en → fa
    expect(document.documentElement.dir).toBe('rtl');
    expect(document.documentElement.lang).toBe('fa');
    expect(localStorage.getItem('ao-locale')).toBe('fa');
  });
});
