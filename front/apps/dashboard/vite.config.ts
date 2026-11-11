import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

// Dashboard serves at /admin/ (STEP1-AUDIT §5): hash-routed SPA — deep links
// never hit the server path router, so the Phase 4 "no SPA fallback" decision
// stands. Dev proxy → a locally running Rust server so admin routes work.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: '/admin/',
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
    // Dashboard budget (UX rule §6): initial ≤ 200 KB gzip — enforced by
    // scripts/budget.mjs after the build (sections are lazy chunks).
    chunkSizeWarningLimit: 200,
  },
});
