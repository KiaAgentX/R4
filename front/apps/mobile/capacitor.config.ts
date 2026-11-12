import type { CapacitorConfig } from './capacitor-config.types.js';

/**
 * Capacitor skeleton — Phase 7 Step 6 (STEP1-AUDIT §9: "Capacitor config +
 * app icon + splash + signing placeholder doc"; audit §7: `apps/mobile` is
 * config-only, NO React Native rewrite).
 *
 * The WebView loads the citizens webapp (apps/webapp/dist, rebuilt with
 * `--base=./` by scripts/sync-www.mjs so absolute /app/ asset paths become
 * webview-root relative). Money truth stays server-side: the frozen API is
 * reached through the SAME generated client; its absolute origin is baked
 * at build time via VITE_API_BASE (optionsFromEnv — see apps/mobile/README.md).
 * Aurora dark plate colors mirror packages/ui tokens — no drift.
 */
const config: CapacitorConfig = {
  appId: 'com.dropagentx.app',
  appName: 'DropAgentX',
  // Filled by `pnpm --filter @agent-one/mobile run sync:www` — never committed.
  webDir: 'www',
  backgroundColor: '#0c1614', // --aur-bg dark
  server: {
    // https scheme is required for service workers / modern web APIs inside
    // the WebView; cleartext stays off (money app — no downgrade).
    androidScheme: 'https',
    cleartext: false,
  },
  android: {
    allowMixedContent: false,
    captureInput: true,
    backgroundColor: '#0c1614',
  },
  ios: {
    // safe-area insets are consumed by webapp.css (env(safe-area-inset-*),
    // viewport-fit=cover) — the webview must not add its own insets on top.
    contentInset: 'always',
    backgroundColor: '#0c1614',
  },
};

export default config;
