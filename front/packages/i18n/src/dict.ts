/**
 * UI dictionaries fa/en — full merged view (tests + back-compat root export).
 * Phase 7 Step 7: the actual strings live in dict-core.ts (shared) and
 * area-keys-{shell,dashboard,webapp}.ts (area-owned); apps import their
 * AREA entry ('@agent-one/i18n/<area>') so no area's bundle carries another
 * area's strings. This module composes the full dict exactly as before the
 * split — the parity test asserts fa and en expose the same key set.
 */
import { escrowStateLabel, faCore, enCore, makeT } from './dict-core.js';
import { faShellKeys, enShellKeys } from './area-keys-shell.js';
import { faDashboardKeys, enDashboardKeys } from './area-keys-dashboard.js';
import { faWebappKeys, enWebappKeys } from './area-keys-webapp.js';

export const fa = {
  ...faCore,
  ...faShellKeys,
  ...faDashboardKeys,
  ...faWebappKeys,
};

/** `Dict` type = the fa shape; en must satisfy it exactly (parity test). */
export type Dict = typeof fa;

export const en: Dict = {
  ...enCore,
  ...enShellKeys,
  ...enDashboardKeys,
  ...enWebappKeys,
};

/** Full-dictionary translate — root/tests entry; areas use their typed `t`. */
export const t = makeT<keyof Dict>({ fa, en });

export { escrowStateLabel };
