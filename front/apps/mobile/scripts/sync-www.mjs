#!/usr/bin/env node
// Capacitor webDir filler — Phase 7 Step 6 (audit §9).
//
// Rebuilds the citizens webapp with ROOT-RELATIVE asset paths (`--base=./`)
// and copies it into apps/mobile/www — the webDir the Capacitor WebView
// loads. The dashboard build uses base '/app/' (static-root serving, D3/D4);
// inside a WebView served from https://localhost that absolute prefix would
// 404, so this script rebuilds instead of blind-copying — honest by design.
//
// API origin: the webapp's generated client reads VITE_API_BASE (baked at
// build time via optionsFromEnv). Export it before building, e.g.:
//   VITE_API_BASE=https://api.dropagentx.example pnpm --filter @agent-one/mobile run sync:www
import { cpSync, existsSync, readdirSync, rmSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const MOBILE = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const FRONT = resolve(MOBILE, '..', '..');
const WWW = join(MOBILE, 'www');

function pnpm(args) {
  const bin = process.env.PNPM_BIN ?? 'pnpm';
  try {
    execSync(`${bin} ${args}`, { cwd: FRONT, stdio: 'inherit' });
  } catch (e) {
    if (bin !== 'pnpm' || e?.status !== 127) throw e;
    execSync(`corepack pnpm ${args}`, { cwd: FRONT, stdio: 'inherit' });
  }
}

console.log('▸ rebuilding @agent-one/webapp with --base=./ for the WebView…');
pnpm('--filter @agent-one/webapp exec vite build --base=./');

const built = join(FRONT, 'apps', 'webapp', 'dist');
if (!existsSync(join(built, 'index.html'))) {
  console.error('✗ apps/webapp/dist/index.html missing after build — aborting honestly');
  process.exit(1);
}

rmSync(WWW, { recursive: true, force: true });
cpSync(built, WWW, { recursive: true });
console.log(`✓ www/ filled (${readdirSync(WWW).length} top-level entries)`);
console.log('  next — needs the Capacitor SDK (see apps/mobile/README.md):');
console.log('    npx cap sync android && npx cap sync ios');
