/**
 * Minimal local mirror of `@capacitor/cli`'s `CapacitorConfig` — ONLY the
 * fields this skeleton uses. Step 6 deliberately ships without the Capacitor
 * SDK installed (no Android Studio / Xcode in this environment; audit §9
 * defers store steps). After installing the SDK on the owner machine:
 *
 *   import type { CapacitorConfig } from '@capacitor/cli';
 *
 * and delete this file — the field shapes below match the real interface, so
 * strict typecheck stays green both before and after the swap.
 */
export interface CapacitorConfig {
  appId?: string;
  appName?: string;
  webDir?: string;
  backgroundColor?: string;
  server?: {
    androidScheme?: 'http' | 'https';
    hostname?: string;
    cleartext?: boolean;
  };
  android?: {
    allowMixedContent?: boolean;
    captureInput?: boolean;
    backgroundColor?: string;
  };
  ios?: {
    contentInset?: 'always' | 'automatic' | 'never' | 'when-scrolling';
    scrollEnabled?: boolean;
    backgroundColor?: string;
  };
}
