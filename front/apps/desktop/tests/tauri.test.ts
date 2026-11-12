import { existsSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// vitest runs from front/ — import.meta.url is a virtual module URL there.
const DESKTOP = resolve(process.cwd(), 'apps/desktop');
const SRC_TAURI = join(DESKTOP, 'src-tauri');

type TauriConf = {
  productName: string;
  identifier: string;
  build: { frontendDist: string; beforeBuildCommand: string };
  app: { windows: Array<{ title: string; minWidth: number; minHeight: number }> };
  bundle: { targets: string[]; icon: string[] };
};

describe('Tauri v2 skeleton (Step 6 — audit §9/D2: no Electron)', () => {
  const conf = JSON.parse(readFileSync(join(SRC_TAURI, 'tauri.conf.json'), 'utf8')) as TauriConf;

  it('identity + window follow the desktop decision', () => {
    expect(conf.productName).toBe('DropAgentX');
    expect(conf.identifier).toBe('com.dropagentx.desktop');
    expect(conf.app.windows[0]?.title).toContain('DropAgentX');
    expect(conf.app.windows[0]?.minWidth).toBe(480);
    expect(conf.app.windows[0]?.minHeight).toBe(640);
  });

  it('loads the ONE combined static root (D3) and builds it first', () => {
    expect(conf.build.frontendDist).toBe('../../../dist'); // relative to src-tauri → front/dist
    expect(conf.build.beforeBuildCommand).toContain('build:root');
  });

  it('bundle targets cover Windows/Linux/macOS per §9', () => {
    expect(conf.bundle.targets).toEqual(expect.arrayContaining(['msi', 'nsis', 'appimage', 'deb', 'dmg', 'app']));
    for (const icon of conf.bundle.icon) {
      expect(existsSync(join(SRC_TAURI, icon))).toBe(true);
    }
  });

  it('updater is a documented placeholder — config stays buildable without secrets', () => {
    expect(JSON.stringify(conf)).not.toContain('createUpdaterArtifacts');
    expect(JSON.stringify(conf)).not.toContain('updater');
    const doc = readFileSync(join(DESKTOP, 'docs', 'UPDATER.md'), 'utf8');
    expect(doc).toContain('Step 9');
    expect(doc).toContain('signer generate');
  });

  it('Rust crate is a standalone workspace — frozen rust/ and stale repo-root stay untouched', () => {
    const cargo = readFileSync(join(SRC_TAURI, 'Cargo.toml'), 'utf8');
    expect(cargo).toContain('[workspace]');
    expect(cargo).toContain('tauri-build = { version = "2"');
    expect(cargo).toContain('tauri = { version = "2"');
    expect(existsSync(join(SRC_TAURI, 'build.rs'))).toBe(true);
    const main = readFileSync(join(SRC_TAURI, 'src', 'main.rs'), 'utf8');
    expect(main).toContain('tauri::Builder::default()');
    expect(main).toContain('windows_subsystem');
  });

  it('capability is fail-closed: only core defaults on the main window', () => {
    const caps = JSON.parse(readFileSync(join(SRC_TAURI, 'capabilities', 'default.json'), 'utf8')) as {
      windows: string[];
      permissions: string[];
    };
    expect(caps.windows).toEqual(['main']);
    expect(caps.permissions).toEqual(['core:default']);
  });
});
