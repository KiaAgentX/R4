/**
 * Hash router for the citizen webapp (decision D4 — hash routing, no server
 * SPA fallback) — the 33-page map of STEP1-AUDIT §2.4 VERBATIM. Every page is
 * one entry with its own hash route; unknown hashes render the 404 page (#32).
 *
 * Deep links like `#/escrow/esc-1/release` never hit the server path router,
 * so the Phase 4 "no SPA fallback" decision stands.
 */

/** Every page id — exactly the 33 rows of the audit map, in order. */
export const PAGE_IDS = [
  'home', // 1  storefront home
  'products', // 2  products list
  'product', // 3  product detail          #/product/{id}
  'buy', // 4  buy flow (amount + 90/10) #/buy/{id}
  'escrowNew', // 5  escrow created (pending fund) #/escrow/{id}/new
  'escrows', // 6  my escrows list
  'escrow', // 7  escrow detail (8-state timeline) #/escrow/{id}
  'escrowFund', // 8  fund confirm   #/escrow/{id}/fund
  'escrowDeliver', // 9  deliver (seller) #/escrow/{id}/deliver
  'escrowRelease', // 10 release confirm (HITL badge) #/escrow/{id}/release
  'escrowDispute', // 11 dispute      #/escrow/{id}/dispute
  'escrowCancel', // 12 cancel confirm #/escrow/{id}/cancel
  'hitl', // 13 HITL status (queue position)
  'wallet', // 14 wallet (balance)
  'tx', // 15 transaction history
  'txDetail', // 16 transaction detail   #/tx/{escrowId}
  'orders', // 17 my orders (staff surface, honest ladder)
  'order', // 18 order detail          #/order/{id}
  'profile', // 19 profile (user:tg:*)
  'settings', // 20 settings
  'language', // 21 language
  'ai', // 22 AI assistant chat
  'aiDirectory', // 23 AI agents & skills directory
  'quests', // 24 quests (read-only)
  'bookings', // 25 bookings (read-only)
  'leaderboard', // 26 leaderboard (read-only)
  'economy', // 27 economy transparency
  'charter', // 28 charter (10 articles)
  'notifications', // 29 notifications (local receipts, honest)
  'help', // 30 help / FAQ
  'offline', // 31 offline shell state
  'about', // 33 about / brand
] as const;

export type PageId = (typeof PAGE_IDS)[number];

/** The 404 page is the unknown-hash fallback — with it the map has 33 pages. */
export type AnyPage = PageId | 'notFound';
export const ALL_PAGES: readonly AnyPage[] = [...PAGE_IDS, 'notFound'] as const;

/** Pages reachable from the mobile bottom nav (§6: bottom nav on mobile). */
export const BOTTOM_NAV = ['home', 'products', 'escrows', 'wallet', 'ai'] as const;

/** The nav groups of the drawer "all pages" index — content-based grouping. */
export const PAGE_GROUPS: readonly { titleKey: 'navHome' | 'navStore' | 'navEscrows' | 'navWallet' | 'aiTitle' | 'leaderboardTitle' | 'economyTitle' | 'settingsTitle' | 'helpTitle'; pages: readonly PageId[] }[] = [
  { titleKey: 'navHome', pages: ['home', 'profile', 'notifications', 'about'] },
  { titleKey: 'navStore', pages: ['products', 'product', 'buy', 'orders', 'order'] },
  {
    titleKey: 'navEscrows',
    pages: ['escrows', 'escrow', 'escrowNew', 'escrowFund', 'escrowDeliver', 'escrowRelease', 'escrowDispute', 'escrowCancel', 'hitl', 'tx', 'txDetail'],
  },
  { titleKey: 'aiTitle', pages: ['ai', 'aiDirectory'] },
  { titleKey: 'leaderboardTitle', pages: ['quests', 'bookings', 'leaderboard'] },
  { titleKey: 'economyTitle', pages: ['economy', 'charter'] },
  { titleKey: 'settingsTitle', pages: ['wallet', 'settings', 'language'] },
  { titleKey: 'helpTitle', pages: ['help', 'offline'] },
];

/** A parsed route — page + the path segments after it (`undefined` when bare). */
export interface Route {
  page: PageId | 'notFound';
  /** Param segments, e.g. ['esc-1'] or ['esc-1', 'release']. */
  params: string[];
}

interface PageSpec {
  /** Path segments after `#/`; '{id}' consumes one segment. */
  path: readonly string[];
}

/** `#/` = the 33 ordered pages; unknown → notFound (#32). */
const ROUTES: Record<PageId | 'notFound', PageSpec> = {
  home: { path: [] },
  products: { path: ['products'] },
  product: { path: ['product', '{id}'] },
  buy: { path: ['buy', '{id}'] },
  escrowNew: { path: ['escrow', '{id}', 'new'] },
  escrows: { path: ['escrows'] },
  escrow: { path: ['escrow', '{id}'] },
  escrowFund: { path: ['escrow', '{id}', 'fund'] },
  escrowDeliver: { path: ['escrow', '{id}', 'deliver'] },
  escrowRelease: { path: ['escrow', '{id}', 'release'] },
  escrowDispute: { path: ['escrow', '{id}', 'dispute'] },
  escrowCancel: { path: ['escrow', '{id}', 'cancel'] },
  hitl: { path: ['hitl'] },
  wallet: { path: ['wallet'] },
  tx: { path: ['tx'] },
  txDetail: { path: ['tx', '{id}'] },
  orders: { path: ['orders'] },
  order: { path: ['order', '{id}'] },
  profile: { path: ['profile'] },
  settings: { path: ['settings'] },
  language: { path: ['language'] },
  ai: { path: ['ai'] },
  aiDirectory: { path: ['ai', 'directory'] },
  quests: { path: ['quests'] },
  bookings: { path: ['bookings'] },
  leaderboard: { path: ['leaderboard'] },
  economy: { path: ['economy'] },
  charter: { path: ['charter'] },
  notifications: { path: ['notifications'] },
  help: { path: ['help'] },
  offline: { path: ['offline'] },
  about: { path: ['about'] },
  notFound: { path: [] }, // never matched by a hash — the unknown-hash fallback
};

/**
 * `#/escrow/esc-1/release` → escrowRelease + ['esc-1'].
 * Empty/unknown → home / notFound. Case-sensitive ids (wire ids are exact).
 */
export function routeFromHash(hash: string): Route {
  const segments = hash.replace(/^#\/?/, '').split('/').filter((s) => s.length > 0);
  for (const id of PAGE_IDS) {
    const spec = ROUTES[id].path;
    if (segments.length !== spec.length) continue;
    const params: string[] = [];
    let ok = true;
    for (let i = 0; i < spec.length; i++) {
      const want = spec[i] as string;
      const got = segments[i] as string;
      if (want === '{id}') {
        if (got.length === 0) {
          ok = false;
          break;
        }
        params.push(got);
      } else if (want !== got) {
        ok = false;
        break;
      }
    }
    if (ok) return { page: id, params };
  }
  // Bare app entry (no hash) is the storefront home; anything else is 404.
  return segments.length === 0 ? { page: 'home', params: [] } : { page: 'notFound', params: [] };
}

/** Canonical href of a page — round-trips through routeFromHash. */
export function pageHref(page: PageId, params: readonly string[] = []): string {
  const spec = ROUTES[page].path;
  let pi = 0;
  const parts = spec.map((seg) => (seg === '{id}' ? (params[pi++] ?? '') : seg));
  return `#/${parts.join('/')}`;
}

/** All 33 hash routes a build must prove renderable (acceptance driver).
 *  The 404 page proves itself through `#/no-such-page`. */
export function allPageHrefs(): { page: AnyPage; href: string; params: string[] }[] {
  const out: { page: AnyPage; href: string; params: string[] }[] = [];
  for (const page of PAGE_IDS) {
    const params = ROUTES[page].path.filter((s) => s === '{id}').map(() => 'demo-1');
    out.push({ page, href: pageHref(page, params), params });
  }
  out.push({ page: 'notFound', href: '#/no-such-page', params: [] });
  return out;
}
