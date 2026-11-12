#!/usr/bin/env node
// Phase 7 — decision D5 (STEP1-AUDIT §12): hand-written API URLs are banned.
// Every HTTP path must come from @agent-one/api-client (generated from the
// pinned golden-openapi.json). This linter fails the build if any source file
// outside packages/api-client hardcodes an API path.
//
// Run: pnpm lint:urls
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import process from 'node:process';

const ROOT = new URL('..', import.meta.url).pathname;
const ALLOWED = ['packages/api-client', 'node_modules', 'rust', 'docs', 'scripts'];
// The server's own additive webhook route is enforced on the backend side;
// the frontend must never write it by hand either — no exception here.
const TEST_FILE = /\.(test|spec)\.[cm]?[jt]sx?$/;

function* walk(dir) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const rel = relative(ROOT, full);
    if (ALLOWED.some((a) => rel === a || rel.startsWith(a + sep))) continue;
    const st = statSync(full);
    if (st.isDirectory()) {
      if (entry === 'node_modules' || entry === 'dist' || entry.startsWith('.')) continue;
      yield* walk(full);
    } else if (/\.[cm]?[jt]sx?$/.test(entry) && !TEST_FILE.test(entry)) {
      yield full;
    }
  }
}

const offenders = [];
for (const file of walk(ROOT)) {
  const text = readFileSync(file, 'utf8');
  const lines = text.split('\n');
  lines.forEach((line, i) => {
    if (/(^|[^A-Za-z0-9-])api\/v1\//.test(line)) {
      offenders.push(`${relative(ROOT, file)}:${i + 1}: ${line.trim().slice(0, 120)}`);
    }
  });
}

if (offenders.length > 0) {
  console.error(`✗ D5 violation — hand-written API URLs outside packages/api-client:`);
  for (const o of offenders) console.error(`  ${o}`);
  console.error(`\nFix: use the generated client (createAgentOneClient) instead of raw URLs.`);
  process.exit(1);
}
console.log(`✓ D5 ok — no hand-written API URLs outside packages/api-client`);
