import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

// Shell serves at the static root '/' (STEP1-AUDIT §5): dist/index.html is the
// strategic index; admin/app/site apps get their own bases in their steps.
// Dev proxy → a locally running Rust server so `reads.health()` works in dev.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: '/',
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
    // Shell budget (UX rule §6): ≤ 80 KB gzip initial — verified after build.
    chunkSizeWarningLimit: 200,
  },
});
