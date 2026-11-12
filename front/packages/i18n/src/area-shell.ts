/**
 * Shell — composed dictionary (core + own) with the strictly-typed area
 * translate function. Apps import `t` (and the shared helpers re-exported
 * here) from '@agent-one/i18n/shell' so the bundler never pulls other
 * areas' strings into this area's bundle (Phase 7 Step 7, §6 budget).
 */
import { faCore, enCore, escrowStateLabel, makeT, type DictCore } from './dict-core.js';
import { faShellKeys, enShellKeys, type ShellKeys } from './area-keys-shell.js';

export type DictShell = DictCore & ShellKeys;

export const faShell: DictShell = { ...faCore, ...faShellKeys };
export const enShell: DictShell = { ...enCore, ...enShellKeys };

/** Translate with this area's key union — a foreign key is a type error. */
export const t = makeT<keyof DictShell>({ fa: faShell, en: enShell });

export { escrowStateLabel };
export * from './locale.js';
export * from './errors.js';
