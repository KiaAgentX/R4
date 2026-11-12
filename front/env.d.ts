/**
 * Ambient env surface for the root typecheck — apps are built by Vite (their
 * own tsconfigs carry `vite/client`), but the solution-wide `tsc -p
 * tsconfig.json` runs without it. This declares exactly the fields the apps
 * read; `VITE_API_BASE` is the single packaged-shell API-origin knob
 * (baseUrlFromEnv/optionsFromEnv — Step 6). Interface merging keeps this
 * compatible when `vite/client` is present.
 */
interface ImportMetaEnv {
  readonly PROD?: boolean;
  readonly MODE?: string;
  readonly VITE_API_BASE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
