/**
 * Citizen webapp — composed dictionary (core + own) with the strictly-typed area
 * translate function. Apps import `t` (and the shared helpers re-exported
 * here) from '@agent-one/i18n/webapp' so the bundler never pulls other
 * areas' strings into this area's bundle (Phase 7 Step 7, §6 budget).
 */
import { faCore, enCore, escrowStateLabel, makeT, type DictCore } from './dict-core.js';
import { faWebappKeys, enWebappKeys, type WebappKeys } from './area-keys-webapp.js';

export type DictWebapp = DictCore & WebappKeys;

export const faWebapp: DictWebapp = { ...faCore, ...faWebappKeys };
export const enWebapp: DictWebapp = { ...enCore, ...enWebappKeys };

/** Translate with this area's key union — a foreign key is a type error. */
export const t = makeT<keyof DictWebapp>({ fa: faWebapp, en: enWebapp });

export { escrowStateLabel };
export * from './locale.js';
export * from './errors.js';
