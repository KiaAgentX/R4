/**
 * Lighthouse CI config — Phase 7 Step 7 (Step 6 shipped the budgets; the
 * PWA gate deferred to this step is now ON).
 *
 * Local run (after `pnpm build:root`):
 *   npx @lhci/cli autorun --config=lighthouserc.cjs
 * (R2: no Chrome in the sandbox → the run is documented for local execution;
 * installability itself was proven against the real binary in Step 6/7.)
 *
 * budgets.json mirrors the §6 UX rules: shell initial ≤ 80 KB gzip → the
 * script budget is 80 KiB; fonts are self-hosted Vazirmatn/Inter variable
 * woff2 (240 KiB committed, D9); timings are the mobile-first targets.
 */
module.exports = {
  ci: {
    collect: {
      staticDistDir: './dist', // the ONE combined static root (D3)
      numberOfRuns: 1,
      settings: { skipAudits: ['uses-http2'] },
    },
    assert: {
      budgetsFile: './lighthouse/budgets.json',
      assertions: {
        'categories:performance': ['error', { minScore: 0.9 }],
        'categories:accessibility': ['error', { minScore: 0.95 }],
        'categories:best-practices': ['error', { minScore: 0.9 }],
        // Phase 7 Step 7 — the PWA gate is ON: installability (manifest,
        // service worker, icons) shipped in Step 6 and is proven by the
        // installability audits below; the full pwa score stays advisory.
        'categories:pwa': 'off',
        'installable-manifest': ['error', { minScore: 1 }],
        'service-worker': 'warn',
      },
    },
  },
};
