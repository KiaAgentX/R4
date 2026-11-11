import { defineConfig } from 'vitest/config';

// Root workspace test config — Phase 7.
// happy-dom: ui theme/dir utilities, the localStorage idempotency store and
// shell detection touch document/window; node-safe code runs unchanged.
export default defineConfig({
  test: {
    environment: 'happy-dom',
    include: [
      'packages/*/tests/**/*.test.ts',
      'apps/*/tests/**/*.test.ts',
      // Step 4 — dashboard renders its presentational components offline via
      // react-dom/server, so tsx tests are part of the root run.
      'apps/*/tests/**/*.test.tsx',
    ],
    // Live round-trip against a running Rust server is a separate script
    // (scripts/round-trip-live.mjs) so `pnpm test` stays fully offline.
    deps: {
      optimizer: { web: { enabled: false } },
    },
  },
});
