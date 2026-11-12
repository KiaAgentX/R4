#!/usr/bin/env node
// Phase 7 Step 7 — monetary E2E against the REAL Rust server binary.
//
// The acceptance chain (Phase 4/5 semantics, now driven end-to-end over HTTP):
//   boot → 401 gates → mint (+replay-safe) → escrow 40,000 → fund → deliver
//   → release = EXACT 90/10 → release replay (no double money) → escrow
//   60,000 → release without approvedByAdmin = 422 REQUIRES_HUMAN_APPROVAL
//   (escrow stays DELIVERED) → HITL queue row → admin approve settles EXACT
//   54,000/6,000 → second approve = typed 422 HITL_ALREADY_DECIDED with zero
//   money movement → trial balance Σ = 0 (minted == Σ wallets + treasury).
//
// Usage:
//   node scripts/e2e-money.mjs                # spawns the binary itself
//   SERVER_BIN=../rust/target/debug/dropagentx-server node scripts/e2e-money.mjs
//   BASE_URL=http://127.0.0.1:8080 KIA_ADMIN_KEY=… node scripts/e2e-money.mjs
//     (against an already-running server — no spawn, no teardown)
//
// Exit code 0 = every expectation held; 1 = first failed expectation.
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const FRONT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const DEFAULT_BIN = resolve(FRONT, '..', 'rust', 'target', 'debug', 'dropagentx-server');

const ADMIN_KEY = process.env.KIA_ADMIN_KEY ?? 'e2e-admin-key-0123456789abcdef';
const BASE = process.env.BASE_URL ?? null; // null → spawn our own server
const PORT = 18_097;
const API = BASE ?? `http://127.0.0.1:${PORT}/api/v1`;
const H = { 'content-type': 'application/json', 'x-kia-key': ADMIN_KEY };

let passed = 0;
function ok(label, cond, detail = '') {
  if (cond) {
    passed += 1;
    console.log(`  ✅ ${label}`);
  } else {
    console.error(`  ❌ ${label}${detail ? ` — ${detail}` : ''}`);
    throw new Error(`E2E failed at: ${label}`);
  }
}

async function req(method, path, { key = ADMIN_KEY, body } = {}) {
  const res = await fetch(`${API}${path}`, {
    method,
    headers: key === null ? { 'content-type': 'application/json' } : { 'content-type': 'application/json', 'x-kia-key': key },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  let json = null;
  try { json = await res.json(); } catch { /* non-JSON */ }
  return { status: res.status, json };
}

const balance = async (id) => (await req('GET', `/accounts/${encodeURIComponent(id)}/balance`)).json.balance;

async function waitForServer(binary, child, dbDir) {
  for (let i = 0; i < 100; i++) {
    try {
      const res = await fetch(`${API}/health`);
      if (res.ok) return;
    } catch { /* not up yet */ }
    if (child.exitCode !== null) {
      console.error(`server exited early with code ${child.exitCode}`);
      rmSync(dbDir, { recursive: true, force: true });
      process.exit(1);
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  console.error('server did not become healthy in 10s');
  child.kill('SIGKILL');
  rmSync(dbDir, { recursive: true, force: true });
  process.exit(1);
}

async function main() {
  let child = null;
  let dbDir = null;
  if (BASE === null) {
    const bin = process.env.SERVER_BIN ?? DEFAULT_BIN;
    dbDir = mkdtempSync(join(tmpdir(), 'ao-e2e-'));
    child = spawn(bin, [], {
      env: {
        ...process.env,
        KIA_ADMIN_KEY: ADMIN_KEY,
        DATABASE_URL: `file:${join(dbDir, 'e2e.db')}`,
        HOST: '127.0.0.1',
        PORT: String(PORT),
        BACKUP_DIR: join(dbDir, 'backups'),
        // no LLM env → the AI cycle is disabled (ADR-006 all-or-nothing is satisfied)
      },
      stdio: ['ignore', 'ignore', 'inherit'],
    });
    await waitForServer(bin, child, dbDir);
    console.log(`▸ server up: ${API} (db ${dbDir})`);
  } else {
    console.log(`▸ against running server: ${API}`);
  }

  try {
    // ── 1 — auth gates ────────────────────────────────────────────────────
    console.log('▸ 401 gates');
    const noKey = await req('GET', '/overview', { key: null });
    ok('GET /overview without key → 401', noKey.status === 401, `got ${noKey.status}`);
    ok('code = ADMIN_KEY_REQUIRED', noKey.json?.error?.code === 'ADMIN_KEY_REQUIRED', JSON.stringify(noKey.json));
    const badKey = await req('GET', '/overview', { key: 'wrong-key-0123456789' });
    ok('GET /overview with wrong key → 401 INVALID_ADMIN_KEY', badKey.status === 401 && badKey.json?.error?.code === 'INVALID_ADMIN_KEY');
    const mintNoKey = await req('POST', '/wallet/mint', { key: null, body: { accountId: 'user:e2e:buyer', amount: 1 } });
    ok('POST /wallet/mint without key → 401', mintNoKey.status === 401);
    const health = await req('GET', '/health', { key: null });
    ok('GET /health is public → 200', health.status === 200);

    // ── 2 — accounts + mint with replay ──────────────────────────────────
    console.log('▸ mint + replay-safety');
    const mk = await req('POST', '/accounts', { body: { id: 'user:e2e:buyer', label: 'E2E buyer' } });
    ok('create buyer → 2xx', mk.status >= 200 && mk.status < 300, `status ${mk.status} ${JSON.stringify(mk.json)}`);
    const mkS = await req('POST', '/accounts', { body: { id: 'user:e2e:seller', label: 'E2E seller' } });
    ok('create seller → 2xx', mkS.status >= 200 && mkS.status < 300, `status ${mkS.status}`);
    const mintBody = { accountId: 'user:e2e:buyer', amount: 100_000, memo: 'e2e seed', idempotencyKey: 'e2e-mint-buyer-1' };
    const m1 = await req('POST', '/wallet/mint', { body: mintBody });
    ok('mint 100,000 → 2xx', m1.status >= 200 && m1.status < 300, JSON.stringify(m1.json));
    ok('balance after mint = 100,000', (await balance('user:e2e:buyer')) === 100_000);
    const m2 = await req('POST', '/wallet/mint', { body: mintBody });
    ok('mint replay → 2xx', m2.status >= 200 && m2.status < 300, JSON.stringify(m2.json));
    ok('mint replay flagged replayed:true', m2.json?.replayed === true, JSON.stringify(m2.json));
    ok('mint replay moved nothing (balance still 100,000)', (await balance('user:e2e:buyer')) === 100_000);

    // ── 3 — small escrow: exact 90/10 without HITL ───────────────────────
    console.log('▸ escrow 40,000 → release = exact 90/10 (36,000/4,000)');
    const e1 = await req('POST', '/escrows', {
      body: { buyerId: 'user:e2e:buyer', sellerId: 'user:e2e:seller', amount: 40_000, idempotencyKey: 'e2e-esc-1' },
    });
    ok('create escrow 40,000 → 2xx', e1.status >= 200 && e1.status < 300, JSON.stringify(e1.json));
    const esc1 = e1.json?.escrow?.id ?? e1.json?.data?.escrow?.id;
    ok('escrow id present', typeof esc1 === 'string' && esc1.length > 0);
    for (const [verb, path, body] of [
      ['fund', `/escrows/${esc1}/fund`, { idempotencyKey: 'e2e-fund-1' }],
      ['deliver', `/escrows/${esc1}/deliver`, { deliveryRef: 'e2e-delivery-1', idempotencyKey: 'e2e-deliver-1' }],
      ['release', `/escrows/${esc1}/release`, { idempotencyKey: 'e2e-release-1' }],
    ]) {
      const r = await req('POST', path, { body });
      ok(`${verb} → 200`, r.status === 200, JSON.stringify(r.json));
    }
    ok('buyer = 60,000 (100,000 − 40,000)', (await balance('user:e2e:buyer')) === 60_000);
    ok('seller = 36,000 (exact 90% of 40,000)', (await balance('user:e2e:seller')) === 36_000);
    ok('treasury = 4,000 (exact 10% of 40,000)', (await balance('treasury:nation')) === 4_000);

    const relReplay = await req('POST', `/escrows/${esc1}/release`, { body: { idempotencyKey: 'e2e-release-1' } });
    ok('release replay → replayed:true', relReplay.json?.replayed === true, JSON.stringify(relReplay.json));
    ok('release replay moved nothing (seller still 36,000)', (await balance('user:e2e:seller')) === 36_000);

    // ── 4 — HITL path: 60,000 needs art. 7 approval ──────────────────────
    console.log('▸ escrow 60,000 → HITL queue → approve = exact 54,000/6,000');
    await req('POST', '/wallet/mint', { body: { accountId: 'user:e2e:buyer', amount: 100_000, idempotencyKey: 'e2e-mint-buyer-2' } });
    const e2 = await req('POST', '/escrows', {
      body: { buyerId: 'user:e2e:buyer', sellerId: 'user:e2e:seller', amount: 60_000, idempotencyKey: 'e2e-esc-2' },
    });
    const esc2 = e2.json?.escrow?.id ?? e2.json?.data?.escrow?.id;
    ok('create escrow 60,000 → 2xx', e2.status >= 200 && e2.status < 300, JSON.stringify(e2.json));
    await req('POST', `/escrows/${esc2}/fund`, { body: { idempotencyKey: 'e2e-fund-2' } });
    await req('POST', `/escrows/${esc2}/deliver`, { body: { deliveryRef: 'e2e-delivery-2', idempotencyKey: 'e2e-deliver-2' } });

    const blocked = await req('POST', `/escrows/${esc2}/release`, { body: { idempotencyKey: 'e2e-release-2-blocked' } });
    ok('release 60,000 without admin flag → 422', blocked.status === 422, `got ${blocked.status}`);
    ok('code = REQUIRES_HUMAN_APPROVAL', blocked.json?.error?.code === 'REQUIRES_HUMAN_APPROVAL', JSON.stringify(blocked.json));
    const still = (await req('GET', `/escrows/${esc2}`)).json;
    ok('escrow stays DELIVERED while awaiting approval', still?.status === 'DELIVERED' || still?.escrow?.status === 'DELIVERED', JSON.stringify(still));

    const hq = await req('POST', '/hitl', {
      body: { kind: 'ESCROW_RELEASE', escrowId: esc2, requesterId: 'user:e2e:buyer', note: 'e2e release request' },
    });
    ok('POST /hitl → 2xx', hq.status >= 200 && hq.status < 300, JSON.stringify(hq.json));
    const hitlId = hq.json?.request?.id ?? hq.json?.id ?? hq.json?.data?.request?.id;
    ok('HITL row id present', typeof hitlId === 'string' && hitlId.length > 0);
    ok('HITL row is PENDING', (hq.json?.request?.status ?? hq.json?.status) === 'PENDING');

    const ap = await req('POST', `/hitl/${hitlId}/approve`, { body: { decidedBy: 'admin:e2e', note: 'e2e approve' } });
    ok('approve → 2xx', ap.status >= 200 && ap.status < 300, JSON.stringify(ap.json));
    const settled = ap.json?.escrow ?? ap.json?.data?.escrow;
    ok('approved settlement → escrow RELEASED', settled?.status === 'RELEASED', JSON.stringify(ap.json));
    ok('seller = 90,000 (36,000 + 54,000)', (await balance('user:e2e:seller')) === 90_000);
    ok('treasury = 10,000 (4,000 + 6,000)', (await balance('treasury:nation')) === 10_000);
    ok('buyer = 100,000 (200,000 − 100,000)', (await balance('user:e2e:buyer')) === 100_000);

    const ap2 = await req('POST', `/hitl/${hitlId}/approve`, { body: { decidedBy: 'admin:e2e-other', note: 'double decide' } });
    ok('second approve → 422 typed', ap2.status === 422 && ap2.json?.error?.code === 'HITL_ALREADY_DECIDED', JSON.stringify(ap2.json));
    ok('double approve moved nothing (treasury still 10,000)', (await balance('treasury:nation')) === 10_000);

    // ── 5 — trial balance: Σ everything minted is held exactly ───────────
    console.log('▸ trial balance');
    const sum = (await balance('user:e2e:buyer')) + (await balance('user:e2e:seller')) + (await balance('treasury:nation'));
    ok('Σ wallets + treasury = 200,000 minted, Σ postings = 0', sum === 200_000, `sum=${sum}`);

    console.log(`\n✅ E2E MONEY OK — ${passed} expectations held (mint→escrow→release 90/10, replay-safe, 401s, HITL art. 7)`);
  } finally {
    if (child !== null) {
      child.kill('SIGTERM');
      await new Promise((r) => setTimeout(r, 200));
      child.kill('SIGKILL');
      if (dbDir !== null) rmSync(dbDir, { recursive: true, force: true });
    }
  }
}

main().catch((e) => {
  console.error(`\n❌ E2E MONEY FAILED — ${e.message}`);
  process.exit(1);
});
