import { existsSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// vitest runs from front/ — import.meta.url is a virtual module URL there.
const MOBILE = resolve(process.cwd(), 'apps/mobile');
import config from '../capacitor.config.js';

describe('Capacitor skeleton (Step 6 — audit §9: config only)', () => {
  it('identity + webDir follow the mobile decision', () => {
    expect(config.appId).toBe('com.dropagentx.app');
    expect(config.appName).toBe('DropAgentX');
    expect(config.webDir).toBe('www');
    expect(config.backgroundColor).toBe('#0c1614'); // --aur-bg dark
  });

  it('https scheme + no mixed content (money app — no downgrade)', () => {
    expect(config.server?.androidScheme).toBe('https');
    expect(config.server?.cleartext).toBe(false);
    expect(config.android?.allowMixedContent).toBe(false);
    expect(config.android?.captureInput).toBe(true);
  });

  it('iOS: webview adds no insets on top of the webapp safe-area CSS', () => {
    expect(config.ios?.contentInset).toBe('always');
  });

  it('sync-www rebuilds the webapp with root-relative base (WebView reality)', () => {
    const script = readFileSync(join(MOBILE, 'scripts', 'sync-www.mjs'), 'utf8');
    expect(script).toContain('--base=./');
    expect(script).toContain('@agent-one/webapp');
    expect(script).toContain('process.exit(1)'); // honest failure when build missing
    expect(existsSync(join(MOBILE, 'scripts', 'sync-www.mjs'))).toBe(true);
  });

  it('deferred-step docs exist (signing / TestFlight placeholders)', () => {
    expect(existsSync(join(MOBILE, 'android', 'README-signing.md'))).toBe(true);
    expect(existsSync(join(MOBILE, 'ios', 'README-testflight.md'))).toBe(true);
    const signing = readFileSync(join(MOBILE, 'android', 'README-signing.md'), 'utf8');
    expect(signing).toContain('keytool');
    const testflight = readFileSync(join(MOBILE, 'ios', 'README-testflight.md'), 'utf8');
    expect(testflight).toContain('safe-area');
  });

  it('splash sources ship from the one Aurora generator (audit §9: app icon + splash)', () => {
    for (const name of ['splash-2732.png', 'splash-2048.png', 'splash-1284.png']) {
      expect(existsSync(join(MOBILE, 'splash', name))).toBe(true);
    }
    const gen = readFileSync(resolve(process.cwd(), 'scripts/gen-icons.py'), 'utf8');
    expect(gen).toContain('make_splash');
  });

  it('www/ and native projects are never committed', () => {
    const gitignore = readFileSync(join(MOBILE, '.gitignore'), 'utf8');
    expect(gitignore).toContain('www/');
    expect(gitignore).toContain('android/app/');
    expect(gitignore).toContain('ios/App/');
  });
});
