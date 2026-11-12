/**
 * Admin dashboard — composed dictionary (core + own) with the strictly-typed area
 * translate function. Apps import `t` (and the shared helpers re-exported
 * here) from '@agent-one/i18n/dashboard' so the bundler never pulls other
 * areas' strings into this area's bundle (Phase 7 Step 7, §6 budget).
 */
import { faCore, enCore, escrowStateLabel, makeT, type DictCore } from './dict-core.js';
import { faDashboardKeys, enDashboardKeys, type DashboardKeys } from './area-keys-dashboard.js';

export type DictDashboard = DictCore & DashboardKeys;

export const faDashboard: DictDashboard = { ...faCore, ...faDashboardKeys };
export const enDashboard: DictDashboard = { ...enCore, ...enDashboardKeys };

/** Translate with this area's key union — a foreign key is a type error. */
export const t = makeT<keyof DictDashboard>({ fa: faDashboard, en: enDashboard });

export { escrowStateLabel };
export * from './locale.js';
export * from './errors.js';
