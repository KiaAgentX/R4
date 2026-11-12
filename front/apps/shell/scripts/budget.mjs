#!/usr/bin/env node
// Shell performance budget — UX rule §6 (STEP1-AUDIT): the strategic shell
// must stay ≤ 80 KB gzip (the same cap the Lighthouse budgets.json enforces
// for scripts). "Initial" = every asset referenced from dist/index.html.
// Enforced since Phase 7 Step 7 — the per-area dictionary split brought the
// shell back under the cap after the Step 4/5 i18n growth; this gate keeps
// it there. Fails the build when the budget is exceeded.
import { readFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const DIST = resolve(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const LIMIT_BYTES = 80 * 1024;

let html;
try {
  html = readFileSync(join(DIST, 'index.html'), 'utf8');
} catch {
  console.error('✗ budget: dist/index.html not found — run `vite build` first');
  process.exit(1);
}

const assets = [...html.matchAll(/(?:src|href)="(\/assets\/[^"]+)"/g)].map((m) => m[1]);
if (assets.length === 0) {
  console.error('✗ budget: no /assets referenced from index.html');
  process.exit(1);
}

let total = 0;
for (const a of assets) {
  const file = join(DIST, a.replace(/^\//, ''));
  const gz = gzipSync(readFileSync(file)).length;
  total += gz;
  console.log(`  ${a} → ${(gz / 1024).toFixed(1)} KB gzip`);
}

const kb = (total / 1024).toFixed(1);
if (total > LIMIT_BYTES) {
  console.error(`✗ shell initial budget exceeded: ${kb} KB gzip > 80 KB`);
  process.exit(1);
}
console.log(`✓ shell initial budget ok: ${kb} KB ≤ 80 KB gzip (initial)`);
