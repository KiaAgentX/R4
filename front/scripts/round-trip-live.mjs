#!/usr/bin/env node
// Phase 7 — decision D5: the generated client is pinned to the golden
// OpenAPI document AND to the live server. This script fetches
// GET /api/v1/openapi.json from a running server and applies the SAME
// zero-breaking normalization as the backend golden test
// (rust/crates/contracts/tests/golden_openapi.rs → canon()) before diffing.
//
// Root level: structural checks only (info.version is formatting — the TS
// package version vs the Rust workspace version legitimately differ).
//
// Usage: BASE_URL=http://127.0.0.1:8080 node scripts/round-trip-live.mjs
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

const GOLDEN = fileURLToPath(new URL('../../rust/crates/contracts/tests/fixtures/golden-openapi.json', import.meta.url));
const BASE = process.env.BASE_URL ?? 'http://127.0.0.1:8080';

// ── canon — faithful port of golden_openapi.rs::canon ────────────────────────
const DROP_KEYS = ['$schema', 'description', 'title', 'additionalProperties', 'propertyNames'];

function canon(v, required) {
  if (Array.isArray(v)) return v.map((m) => canon(m, required));
  if (v === null || typeof v !== 'object') return v;
  const map = v;

  if ('$ref' in map) throw new Error('unresolved $ref leaked into the live doc');

  for (const combinator of ['anyOf', 'oneOf']) {
    const members = map[combinator];
    if (Array.isArray(members)) {
      const nonNull = members.filter((m) => (m ?? {}).type !== 'null');
      if (nonNull.length === 1 && members.length === 2) {
        // [T, null] → T + nullable (required context only)
        const inner = canon(nonNull[0], required);
        if (inner !== null && typeof inner === 'object' && !Array.isArray(inner) && required !== null) {
          inner.nullable = true;
        }
        return inner;
      }
      if (members.length === 1) {
        // single-member oneOf → the member + combinator siblings (e.g. default)
        const inner = canon(members[0], required);
        if (inner !== null && typeof inner === 'object' && !Array.isArray(inner)) {
          for (const [k, val] of Object.entries(map)) {
            if (k !== combinator && k !== '$ref' && !(k in inner)) inner[k] = val;
          }
        }
        return inner;
      }
    }
  }

  const out = {};
  if ('const' in map) {
    const c = map.const;
    if (typeof c === 'string') {
      out.type = 'string';
      out.enum = [c];
    } else if (typeof c === 'boolean') {
      out.type = 'boolean';
    } else {
      out.const_other = c;
    }
  }

  for (const [k, val] of Object.entries(map)) {
    if (DROP_KEYS.includes(k) || k === 'const') continue;
    if (k === 'pattern' && map.format === 'date-time') continue;
    if (k === 'format') {
      if (val === 'int64' || val === 'double' || val === 'float' || val === 'int32') continue;
      out[k] = val;
      continue;
    }
    if (k === 'type') {
      if (Array.isArray(val)) {
        const list = val.filter((t) => t !== 'null');
        const hadNull = list.length !== val.length;
        const scalar = list.length === 1 ? list[0] : list;
        if (hadNull && required !== null) out.nullable = true;
        out.type = scalar;
      } else {
        out.type = val;
      }
      continue;
    }
    if (Array.isArray(val)) {
      if (k === 'required') out[k] = [...val].sort();
      else if (k === 'enum') out[k] = val;
      else if (k === 'anyOf' || k === 'oneOf') out[k] = val.map((m) => canon(m, required));
      else out[k] = canon(val, null);
    } else if (val !== null && typeof val === 'object') {
      if (k === 'properties') {
        const reqList = Array.isArray(map.required) ? map.required : null;
        const props = {};
        for (const [pk, pv] of Object.entries(val)) {
          props[pk] = reqList !== null && reqList.includes(pk) ? canon(pv, [pk]) : canon(pv, null);
        }
        out[k] = props;
      } else {
        out[k] = canon(val, null);
      }
    } else {
      out[k] = val;
    }
  }
  return out;
}

// ── diff — recursive, reports JSON-pointer-ish paths ────────────────────────
function diff(a, b, path, out) {
  if (a === null || b === null || typeof a !== 'object' || typeof b !== 'object' || Array.isArray(a) !== Array.isArray(b)) {
    if (a !== b) out.push(`${path} — ${JSON.stringify(a)} vs ${JSON.stringify(b)}`);
    return;
  }
  if (Array.isArray(a)) {
    if (a.length !== b.length) {
      out.push(`${path} — length ${a.length}/${b.length}`);
      return;
    }
    a.forEach((x, i) => diff(x, b[i], `${path}[${i}]`, out));
    return;
  }
  const ka = Object.keys(a).sort();
  const kb = Object.keys(b).sort();
  if (ka.join(',') !== kb.join(',')) {
    const only = [...new Set([...ka, ...kb])].filter((k) => !(k in a) || !(k in b));
    out.push(`${path}: key sets differ → ${only.map((k) => `${k}(${k in a ? 'golden' : 'live'})`).join(', ')}`);
    return;
  }
  for (const k of ka) diff(a[k], b[k], `${path}.${k}`, out);
}

// ── main ─────────────────────────────────────────────────────────────────────
/** Canonical JSON stringify — object keys sorted (serde_json BTreeMap parity). */
const stable = (v) =>
  JSON.stringify(v, (_k, val) =>
    val !== null && typeof val === 'object' && !Array.isArray(val)
      ? Object.fromEntries(Object.entries(val).sort(([a], [b]) => (a < b ? -1 : 1)))
      : val,
  );

const golden = JSON.parse(readFileSync(GOLDEN, 'utf8'));
const res = await fetch(`${BASE}/api/v1/openapi.json`, { headers: { accept: 'application/json' } });
if (!res.ok) {
  console.error(`✗ live server answered ${res.status} — is the Rust server running at ${BASE}?`);
  process.exit(1);
}
const live = await res.json();

const problems = [];
// 1) root — structural only (same as the backend golden test)
if (live.openapi !== golden.openapi) problems.push(`$.openapi: ${golden.openapi} !== ${live.openapi}`);
if (live.info?.title !== golden.info?.title) problems.push('$.info.title differs');
if (live.info?.description !== golden.info?.description) problems.push('$.info.description differs');
if (typeof live.info?.version !== 'string' || live.info.version.length === 0) problems.push('$.info.version empty');
if (stable(live.servers) !== stable(golden.servers)) problems.push('$.servers differs');
if (stable(live.components ?? null) !== stable(golden.components ?? null)) problems.push('$.components differs');
if (stable(live.tags) !== stable(golden.tags)) problems.push('$.tags differs');

// 2) path sets — 1:1
const gPaths = Object.keys(golden.paths ?? {}).sort();
const lPaths = Object.keys(live.paths ?? {}).sort();
if (gPaths.join(',') !== lPaths.join(',')) {
  problems.push(`path sets differ: ${gPaths.filter((p) => !lPaths.includes(p)).join(', ')} (golden-only) / ${lPaths.filter((p) => !gPaths.includes(p)).join(', ')} (live-only)`);
}

// 3) every operation — canon'd zero-diff (the frozen contract)
const mismatches = [];
for (const [path, gitem] of Object.entries(golden.paths ?? {})) {
  const litem = live.paths?.[path];
  if (litem === undefined) continue;
  for (const [method, gop] of Object.entries(gitem)) {
    const lop = litem[method];
    if (lop === undefined) {
      mismatches.push(`${method} ${path} — missing on live`);
      continue;
    }
    const local = [];
    diff(canon(gop, null), canon(lop, null), `${method} ${path}`, local);
    mismatches.push(...local);
  }
}

problems.push(...mismatches);
if (problems.length > 0) {
  console.error(`✗ ROUND-TRIP FAILED — live /api/v1/openapi.json differs from pinned golden (${problems.length}):`);
  for (const p of problems.slice(0, 20)) console.error(`  ${p}`);
  process.exit(1);
}
const ops = Object.values(live.paths).reduce((n, p) => n + Object.keys(p).filter((m) => m !== 'parameters').length, 0);
console.log(`✓ ROUND-TRIP OK — live == golden (zero breaking diff): ${lPaths.length} paths / ${ops} operations`);
