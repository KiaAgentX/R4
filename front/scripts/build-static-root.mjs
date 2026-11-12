#!/usr/bin/env node
// Static-root build wiring — Phase 7 Step 3+4 (STEP1-AUDIT §5, D3/D4).
// Assembles the ONE combined static root served by the existing ServeDir with
// only a STATIC_DIR value change — zero backend code:
//
//   dist/
//   ├── index.html            ← apps/shell build (strategic index, base '/')
//   ├── assets/…              ← shell assets
//   ├── build-manifest.json   ← honest availability of the three areas
//   ├── admin/                ← apps/dashboard build  (Step 4)
//   ├── app/                  ← apps/webapp build     (Step 5)
//   └── site/                 ← apps/site build       (later step)
//
// Areas whose app does not exist yet are simply absent — the shell renders
// them as "ships in a later Phase 7 step" (never a fake page).
import { cpSync, existsSync, mkdirSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const FRONT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const APPS = join(FRONT, 'apps');
const DIST = join(FRONT, 'dist');
// Area id → app package name (apps/<pkg>). webapp/site land in later steps.
const AREA_APPS = { admin: 'dashboard', app: 'webapp', site: 'site' };
const AREAS = Object.keys(AREA_APPS);

/** pnpm is not always on PATH (corepack environments) — resolve it once. */
function pnpm(args) {
  const bin = process.env.PNPM_BIN ?? 'pnpm';
  try {
    execSync(`${bin} ${args}`, { cwd: FRONT, stdio: 'inherit' });
  } catch (e) {
    if (bin !== 'pnpm' || e?.status !== 127) throw e;
    execSync(`corepack pnpm ${args}`, { cwd: FRONT, stdio: 'inherit' });
  }
}

console.log('▸ building shell…');
pnpm('--filter @agent-one/shell run build');

// Build each area app that exists (honest manifest even when unbuilt).
for (const area of AREAS) {
  const app = AREA_APPS[area];
  if (existsSync(join(APPS, app))) {
    console.log(`▸ building ${area} (@agent-one/${app})…`);
    pnpm(`--filter @agent-one/${app} run build`);
  } else {
    console.log(`• ${area}: @agent-one/${app} not built yet — ships in a later step`);
  }
}

rmSync(DIST, { recursive: true, force: true });
mkdirSync(DIST, { recursive: true });

// shell → root (base '/')
cpSync(join(APPS, 'shell', 'dist'), DIST, { recursive: true });

// areas → dist/<area>/ when their build exists
const manifest = { shell: true };
for (const area of AREAS) {
  const app = AREA_APPS[area];
  const built = existsSync(join(APPS, app, 'dist'));
  manifest[area] = built;
  if (built) cpSync(join(APPS, app, 'dist'), join(DIST, area), { recursive: true });
}
manifest.builtAt = new Date().toISOString();
writeFileSync(join(DIST, 'build-manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`);

const top = readdirSync(DIST).join('  ');
console.log(`✓ static root assembled → ${DIST}`);
console.log(`  entries: ${top}`);
console.log(`  manifest: ${JSON.stringify(manifest)}`);
