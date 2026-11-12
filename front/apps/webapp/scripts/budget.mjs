#!/usr/bin/env node
// Webapp performance budget — UX rule §6 (STEP1-AUDIT): the webapp route
// chunk must stay ≤ 120 KB gzip. Enforced on BOTH the initial bundle (every
// asset referenced from dist/index.html) AND every lazy page chunk — the
// Mini App runs on phones inside Telegram, so no chunk may blow the budget.
// Fails the build when any budget is exceeded (wired into `pnpm build`).
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const DIST = resolve(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
const LIMIT_BYTES = 120 * 1024;

let html;
try {
  html = readFileSync(join(DIST, 'index.html'), 'utf8');
} catch {
  console.error('✗ budget: dist/index.html not found — run `vite build` first');
  process.exit(1);
}

const assets = [...html.matchAll(/(?:src|href)="(\/app\/assets\/[^"]+)"/g)].map((m) => m[1]);
if (assets.length === 0) {
  console.error('✗ budget: no /app/assets referenced from index.html');
  process.exit(1);
}

let initial = 0;
for (const a of assets) {
  const file = join(DIST, a.replace(/^\/app\//, ''));
  const gz = gzipSync(readFileSync(file)).length;
  initial += gz;
  console.log(`  initial ${a} → ${(gz / 1024).toFixed(1)} KB gzip`);
}

let kb = (initial / 1024).toFixed(1);
if (initial > LIMIT_BYTES) {
  console.error(`✗ webapp initial budget exceeded: ${kb} KB gzip > 120 KB`);
  process.exit(1);
}
console.log(`✓ webapp initial budget ok: ${kb} KB ≤ 120 KB gzip (initial)`);

// Every lazy chunk also ≤ 120 KB (route-level code splitting, §6)
const assetsDir = join(DIST, 'assets');
const chunks = readdirSync(assetsDir).filter((f) => f.endsWith('.js'));
let worst = 0;
for (const f of chunks) {
  const gz = gzipSync(readFileSync(join(assetsDir, f))).length;
  worst = Math.max(worst, gz);
  if (gz > LIMIT_BYTES) {
    console.error(`✗ webapp route chunk exceeded: ${f} = ${(gz / 1024).toFixed(1)} KB gzip > 120 KB`);
    process.exit(1);
  }
}
console.log(
  `✓ webapp chunk budget ok: ${chunks.length} JS chunks, worst ${(worst / 1024).toFixed(1)} KB ≤ 120 KB gzip`,
);
statSync(assetsDir); // sanity: assets dir exists (throws otherwise)
