#!/usr/bin/env node
// Codegen — decision D5 (STEP1-AUDIT §12): @agent-one/api-client types are
// GENERATED from the pinned golden OpenAPI 3.1 document, never hand-written.
// The golden file is itself guarded by the Rust contract tests (zero-diff vs
// the TS zod source), so this is a transitive pin to the frozen contract.
//
// Deterministic: same input → byte-identical output (tested in
// tests/schema.test.ts — regeneration must not dirty the repo).
import { readFileSync, writeFileSync } from 'node:fs';
import openapiTS, { astToString } from 'openapi-typescript';
import { fileURLToPath } from 'node:url';

const GOLDEN = fileURLToPath(
  new URL('../../../../rust/crates/contracts/tests/fixtures/golden-openapi.json', import.meta.url),
);
const OUT = fileURLToPath(new URL('../src/schema.d.ts', import.meta.url));

const HEADER = `/**
 * GENERATED FILE — DO NOT EDIT.
 * Source of truth: rust/crates/contracts/tests/fixtures/golden-openapi.json
 * (pinned golden fixture, guarded by the contract's own golden tests).
 * Regenerate: pnpm --filter @agent-one/api-client run generate
 * Hand-written API URLs are banned elsewhere (D5 — pnpm lint:urls).
 */

`;

const golden = JSON.parse(readFileSync(GOLDEN, 'utf8'));
const pathCount = Object.keys(golden.paths ?? {}).length;
const opCount = Object.values(golden.paths ?? {}).reduce(
  (n, p) => n + Object.keys(p).filter((m) => ['get', 'post', 'patch', 'put', 'delete'].includes(m)).length,
  0,
);

// Pass the document TEXT (identical input path as tests/schema.test.ts so the
// two codegen paths produce byte-identical output; no URL/fetch handling).
const ast = await openapiTS(readFileSync(GOLDEN, 'utf8'));
let contents = astToString(ast);
if (!contents.endsWith('\n')) contents += '\n';
writeFileSync(OUT, HEADER + contents);

console.log(`✓ generated src/schema.d.ts — ${pathCount} paths / ${opCount} operations from golden-openapi.json`);
if (pathCount !== 40 || opCount !== 46) {
  console.error(`✗ contract shape changed — expected 40 paths / 46 operations (frozen registry, ADR-005).`);
  process.exit(1);
}
