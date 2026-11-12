#!/usr/bin/env node
// Dashboard performance budget — UX rule §6 (STEP1-AUDIT): dashboard initial
// ≤ 200 KB gzip. "Initial" = every asset referenced from dist/index.html;
// the lazy section chunks are NOT referenced there and don't count.
// Fails the build when the budget is exceeded (wired into `pnpm build`).
import { readFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const DIST = resolve(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const LIMIT_BYTES = 200 * 1024;

let html;
try {
  html = readFileSync(join(DIST, 'index.html'), 'utf8');
} catch {
  console.error('✗ budget: dist/index.html not found — run `vite build` first');
  process.exit(1);
}

const assets = [...html.matchAll(/(?:src|href)="(\/admin\/assets\/[^"]+)"/g)].map((m) => m[1]);
if (assets.length === 0) {
  console.error('✗ budget: no /admin/assets referenced from index.html');
  process.exit(1);
}

let total = 0;
for (const a of assets) {
  const file = join(DIST, a.replace(/^\/admin\//, ''));
  const gz = gzipSync(readFileSync(file)).length;
  total += gz;
  console.log(`  ${a} → ${(gz / 1024).toFixed(1)} KB gzip`);
}

const kb = (total / 1024).toFixed(1);
if (total > LIMIT_BYTES) {
  console.error(`✗ dashboard initial budget exceeded: ${kb} KB gzip > 200 KB`);
  process.exit(1);
}
console.log(`✓ dashboard initial budget ok: ${kb} KB ≤ 200 KB gzip (initial)`);
