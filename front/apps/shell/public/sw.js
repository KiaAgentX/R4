/*
 * Agent-One service worker — Phase 7 Step 6 (STEP1-AUDIT §5 + §9).
 *
 * Offline SHELL only — an honest skeleton, not a data layer:
 *   1. MONEY TRUTH IS SERVER-SIDE. Every path under /api is passed through
 *      untouched — never cached, never answered offline. A fabricated balance
 *      is worse than an honest failure (audit §8: "never optimistic money").
 *   2. Documents are network-first; each area's index.html (/admin/, /app/)
 *      is runtime-cached on its first ONLINE visit, so an offline revisit of
 *      a previously-seen area works, and a first-time-offline navigation
 *      falls back to the cached strategic shell — which renders its honest
 *      availability state from build-manifest.json, never a fake page.
 *   3. Hashed Vite assets, self-hosted fonts and icons are immutable →
 *      cache-first, filled lazily.
 *   4. Versioned cache names; activation deletes every cache from an older
 *      VERSION (stale shells never survive a deploy).
 */
const VERSION = 'ao-shell-v1';
const PRECACHE = `${VERSION}-precache`;
const RUNTIME = `${VERSION}-runtime`;

const PRECACHE_URLS = [
  '/', // strategic shell — also the last-resort offline fallback
  '/index.html',
  '/manifest.webmanifest',
  '/favicon.ico',
  '/favicon.svg',
  '/icons/icon-192.png',
  '/icons/icon-512.png',
  '/icons/icon-maskable-192.png',
  '/icons/icon-maskable-512.png',
  '/icons/apple-touch-icon.png',
];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(PRECACHE)
      .then((cache) => cache.addAll(PRECACHE_URLS))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(keys.filter((key) => !key.startsWith(VERSION)).map((key) => caches.delete(key))),
      )
      .then(() => self.clients.claim()),
  );
});

/** Network-first for documents, with the honest two-step fallback of §2. */
async function navigate(request) {
  const runtime = await caches.open(RUNTIME);
  try {
    const fresh = await fetch(request);
    runtime.put(request, fresh.clone());
    return fresh;
  } catch (offline) {
    const precache = await caches.open(PRECACHE);
    const hit = (await runtime.match(request)) ?? (await precache.match('/'));
    if (hit) return hit;
    throw offline; // nothing cached at all → let the browser show its offline state
  }
}

self.addEventListener('fetch', (event) => {
  const request = event.request;
  if (request.method !== 'GET') return;
  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;
  // (1) money truth + bot webhook: never intercepted
  if (url.pathname.startsWith('/api')) return;
  if (request.mode === 'navigate') {
    event.respondWith(navigate(request));
    return;
  }
  // (3) immutable same-origin assets → cache-first, fill lazily
  event.respondWith(
    (async () => {
      const precache = await caches.open(PRECACHE);
      const runtime = await caches.open(RUNTIME);
      const hit = (await runtime.match(request)) ?? (await precache.match(request));
      if (hit) return hit;
      const fresh = await fetch(request);
      if (fresh.ok) runtime.put(request, fresh.clone());
      return fresh;
    })(),
  );
});
