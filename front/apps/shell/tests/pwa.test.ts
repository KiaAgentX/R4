import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import { isStandaloneDisplay, registerServiceWorker, shouldRegisterServiceWorker } from '../src/pwa.js';

// vitest runs from front/ — import.meta.url is a virtual module URL there.
const SHELL = resolve(process.cwd(), 'apps/shell');
const PUBLIC = join(SHELL, 'public');

/** PNG IHDR width/height live at bytes 16..24 (big-endian). */
function pngSize(path: string): { width: number; height: number } {
  const buf = readFileSync(path);
  expect(buf.subarray(12, 16).toString('latin1')).toBe('IHDR');
  return { width: buf.readUInt32BE(16), height: buf.readUInt32BE(20) };
}

describe('PWA manifest (Step 6 — installable shell)', () => {
  const manifest = JSON.parse(readFileSync(join(PUBLIC, 'manifest.webmanifest'), 'utf8')) as {
    name: string; short_name: string; lang: string; dir: string; start_url: string;
    scope: string; display: string; theme_color: string; background_color: string;
    icons: Array<{ src: string; sizes: string; type: string; purpose?: string }>;
    shortcuts: Array<{ name: string; url: string }>;
  };

  it('identity + scope follow the strategic shell at the static root', () => {
    expect(manifest.name).toContain('Agent-One');
    expect(manifest.short_name).toBe('Agent-One');
    expect(manifest.lang).toBe('fa');
    expect(manifest.dir).toBe('rtl');
    expect(manifest.start_url).toBe('/?source=pwa');
    expect(manifest.scope).toBe('/');
    expect(manifest.display).toBe('standalone');
  });

  it('colors mirror the Aurora tokens (no drift)', () => {
    // tokens.css: --aur-brand-500 / --aur-bg dark
    expect(manifest.theme_color).toBe('#0fbf92');
    expect(manifest.background_color).toBe('#0c1614');
  });

  it('declares 192/512 icons in both purposes, files exist with exact dimensions', () => {
    const any = manifest.icons.filter((i) => i.purpose === 'any');
    const maskable = manifest.icons.filter((i) => i.purpose === 'maskable');
    expect(any.map((i) => i.sizes)).toEqual(expect.arrayContaining(['192x192', '512x512']));
    expect(maskable.map((i) => i.sizes)).toEqual(expect.arrayContaining(['192x192', '512x512']));
    expect(maskable.every((i) => i.purpose === 'maskable')).toBe(true);
    for (const icon of manifest.icons) {
      const file = join(PUBLIC, icon.src.replace(/^\//, ''));
      expect(pngSize(file)).toEqual({
        width: Number(icon.sizes.split('x')[0]),
        height: Number(icon.sizes.split('x')[1]),
      });
    }
  });

  it('shortcuts point at the honest trailing-slash areas', () => {
    expect(manifest.shortcuts.map((s) => s.url)).toEqual(['/app/', '/admin/']);
  });

  it('apple-touch-icon exists at 180x180 (opaque)', () => {
    expect(pngSize(join(PUBLIC, 'icons', 'apple-touch-icon.png'))).toEqual({ width: 180, height: 180 });
    expect(readFileSync(join(PUBLIC, 'favicon.svg'), 'utf8')).toContain('Đ');
    // ICO magic bytes
    const ico = readFileSync(join(PUBLIC, 'favicon.ico'));
    expect([ico[0], ico[1]]).toEqual([0, 0]);
    expect([ico[2], ico[3]]).toEqual([1, 0]);
  });
});

describe('service worker — offline SHELL, money-honest', () => {
  const sw = readFileSync(join(PUBLIC, 'sw.js'), 'utf8');

  it('versioned caches; activation deletes every older cache', () => {
    expect(sw).toMatch(/const VERSION = 'ao-shell-v(\d+)';/);
    expect(sw).toContain('caches.delete');
    expect(sw).toContain('!key.startsWith(VERSION)');
  });

  it('precache covers the shell fallback set', () => {
    for (const needle of ["'/'", "'/index.html'", "'/manifest.webmanifest'", "'/icons/icon-512.png'"]) {
      expect(sw).toContain(needle);
    }
  });

  it('the frozen API is NEVER intercepted (money truth is server-side)', () => {
    const apiGuard = sw.indexOf("url.pathname.startsWith('/api')");
    const respondWith = sw.indexOf('respondWith');
    expect(apiGuard).toBeGreaterThan(-1);
    expect(respondWith).toBeGreaterThan(-1);
    expect(apiGuard).toBeLessThan(respondWith); // guard returns before any respondWith
    expect(sw).toContain('request.method !== \'GET\''); // no non-GET interception either
  });

  it('documents are network-first with cached-shell fallback; assets cache-first', () => {
    expect(sw).toContain("request.mode === 'navigate'");
    expect(sw).toContain('async function navigate');
    expect(sw).toContain("precache.match('/')"); // last-resort shell fallback
    expect(sw).toContain('runtime.put');
  });

  it('takes over promptly (skipWaiting + clientsClaim)', () => {
    expect(sw).toContain('skipWaiting');
    expect(sw).toContain('clients.claim');
  });
});

describe('pwa.ts — injectable guards', () => {
  const swContainer = { register: vi.fn(async () => ({})) };

  function navWith(sw?: unknown) {
    return (sw === undefined ? {} : { serviceWorker: sw }) as Navigator;
  }

  it('registers only in production builds with a service-worker container', () => {
    expect(shouldRegisterServiceWorker({ PROD: true }, navWith(swContainer))).toBe(true);
    expect(shouldRegisterServiceWorker({ PROD: false }, navWith(swContainer))).toBe(false);
    expect(shouldRegisterServiceWorker({}, navWith(swContainer))).toBe(false);
    expect(shouldRegisterServiceWorker({ PROD: true }, navWith(undefined))).toBe(false);
  });

  it('registerServiceWorker hits /sw.js at root scope — or degrades silently', async () => {
    const register = vi.fn(async () => ({}));
    await registerServiceWorker({ PROD: true }, { serviceWorker: { register } });
    expect(register).toHaveBeenCalledWith('/sw.js', { scope: '/' });

    const unused = vi.fn(async () => ({}));
    await registerServiceWorker({ PROD: false }, { serviceWorker: { register: unused } });
    await registerServiceWorker({ PROD: true }, {} as Navigator);
    expect(unused).not.toHaveBeenCalled();
  });

  it('standalone detection: iOS flag wins, display-mode query follows', () => {
    const mq = (matches: boolean) => () => ({ matches }) as MediaQueryList;
    expect(isStandaloneDisplay({ standalone: true }, undefined)).toBe(true);
    expect(isStandaloneDisplay({}, mq(true))).toBe(true);
    expect(isStandaloneDisplay({}, mq(false))).toBe(false);
    expect(isStandaloneDisplay({}, undefined)).toBe(false);
  });
});

describe('shell index.html — PWA head hooks', () => {
  const html = readFileSync(join(SHELL, 'index.html'), 'utf8');

  it('links manifest, favicons and apple-touch icon at the static root', () => {
    expect(html).toContain('rel="manifest" href="/manifest.webmanifest"');
    expect(html).toContain('href="/favicon.svg"');
    expect(html).toContain('href="/favicon.ico"');
    expect(html).toContain('href="/icons/apple-touch-icon.png"');
  });

  it('Safari tab tint follows Aurora light/dark; standalone metas present', () => {
    expect(html).toContain('media="(prefers-color-scheme: light)" content="#f6f8f9"');
    expect(html).toContain('media="(prefers-color-scheme: dark)" content="#0c1614"');
    expect(html).toContain('apple-mobile-web-app-capable');
    expect(html).toContain('apple-mobile-web-app-title');
    expect(html).toContain('viewport-fit=cover');
  });

  it('main.tsx registers the service worker once (prod-guarded call site)', () => {
    const main = readFileSync(join(SHELL, 'src', 'main.tsx'), 'utf8');
    expect(main).toContain('registerServiceWorker');
    expect(main).toContain("catch");
  });
});

describe('Lighthouse budget config (Step 6, R2 mitigation)', () => {
  const FRONT = process.cwd();
  const budgets = JSON.parse(readFileSync(join(FRONT, 'lighthouse', 'budgets.json'), 'utf8')) as {
    budgets: Array<{ path: string; resourceSizes: Array<{ resourceType: string; budget: number }>; timings: Array<{ metric: string; budget: number }> }>;
  };
  const lhci = readFileSync(join(FRONT, 'lighthouserc.cjs'), 'utf8');

  it('budgets mirror the §6 caps (script 80 KiB, fonts 240 KiB)', () => {
    const b = budgets.budgets[0];
    expect(b?.path).toBe('/*');
    expect(b?.resourceSizes.find((r) => r.resourceType === 'script')?.budget).toBe(80);
    expect(b?.resourceSizes.find((r) => r.resourceType === 'font')?.budget).toBe(240);
    expect(b?.timings.find((t) => t.metric === 'first-contentful-paint')?.budget).toBe(2000);
  });

  it('lhci runs against the ONE combined static root with the budgets file', () => {
    expect(lhci).toContain("staticDistDir: './dist'");
    expect(lhci).toContain("budgetsFile: './lighthouse/budgets.json'");
    expect(lhci).toContain("'categories:pwa': 'off'"); // deferred per §9, honestly
  });
});
