import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

// Webapp serves at /app/ (STEP1-AUDIT §5): hash-routed SPA — deep links never
// hit the server path router, so the Phase 4 "no SPA fallback" decision stands.
// Dev proxy → a locally running Rust server so public routes work in dev.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: '/app/',
  server: {
    proxy: {
      '/api': {
        target: process.env.DEV_API_TARGET ?? 'http://127.0.0.1:8080',
        changeOrigin: true,
      },
    },
  },
  build: {
    sourcemap: false,
    // Webapp budget (UX rule §6): every route chunk ≤ 120 KB gzip — enforced
    // by scripts/budget.mjs after the build (initial + every lazy chunk).
    chunkSizeWarningLimit: 120,
  },
});
