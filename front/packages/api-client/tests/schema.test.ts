import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import openapiTS, { astToString } from 'openapi-typescript';

// vitest rewrites import.meta.url → resolve against the workspace root cwd (front/).
const GOLDEN = resolve(process.cwd(), '../rust/crates/contracts/tests/fixtures/golden-openapi.json');
const SCHEMA = resolve(process.cwd(), 'packages/api-client/src/schema.d.ts');

const HEADER_START = '/**\n * GENERATED FILE — DO NOT EDIT.';

describe('schema generation — D5 pinned contract', () => {
  it('golden fixture is the frozen 40-path / 46-operation registry', () => {
    const golden = JSON.parse(readFileSync(GOLDEN, 'utf8'));
    const paths = Object.keys(golden.paths);
    const ops = Object.values(golden.paths as Record<string, Record<string, unknown>>).reduce(
      (n, p) => n + Object.keys(p).filter((m) => ['get', 'post', 'patch', 'put', 'delete'].includes(m)).length,
      0,
    );
    expect(paths).toHaveLength(40);
    expect(ops).toBe(46);
    expect(golden.openapi).toBe('3.1.0');
  });

  it('committed schema.d.ts is exactly what the generator produces today (determinism)', async () => {
    // Same input path as scripts/generate.mjs (document TEXT) so the two
    // codegen paths must agree byte-for-byte.
    const ast = await openapiTS(readFileSync(GOLDEN, 'utf8'));
    let generated = astToString(ast);
    if (!generated.endsWith('\n')) generated += '\n';

    const committed = readFileSync(SCHEMA, 'utf8');
    expect(committed.startsWith(HEADER_START)).toBe(true);
    expect(committed.endsWith(generated), 'committed schema.d.ts body must equal regenerated output').toBe(true);
  });

  it('schema covers the money-mutation bodies with required idempotencyKey', async () => {
    type GoldenPost = {
      post?: {
        requestBody?: {
          content?: Record<string, { schema?: { properties?: Record<string, unknown> } }>;
        };
      };
    };
    const golden = JSON.parse(readFileSync(GOLDEN, 'utf8')) as { paths: Record<string, GoldenPost> };
    const moneyPaths = [
      '/wallet/mint',
      '/wallet/burn',
      '/escrows',
      '/escrows/{id}/fund',
      '/escrows/{id}/deliver',
      '/escrows/{id}/release',
      '/escrows/{id}/dispute',
      '/escrows/{id}/refund',
      '/escrows/{id}/cancel',
      '/escrows/{id}/expire',
    ];
    for (const p of moneyPaths) {
      const schema = golden.paths[p]!.post!.requestBody!.content!['application/json']!.schema!;
      // The wire carries the key in properties (D7: the CLIENT always sends one,
      // stricter than the schema — replay-safety is client-side discipline).
      expect(schema.properties, p).toHaveProperty('idempotencyKey');
    }
    // And nothing outside the money set claims it:
    for (const [p, ops] of Object.entries(golden.paths)) {
      const props = ops.post?.requestBody?.content?.['application/json']?.schema?.properties;
      if (props !== undefined && 'idempotencyKey' in props) expect(moneyPaths).toContain(p);
    }
  });
});
