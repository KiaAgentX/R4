/**
 * Areas of the combined static root (STEP1-AUDIT §5):
 *   /admin/  dashboard build   |   /app/  webapp build   |   /site/  public MPA
 * Availability comes from dist/build-manifest.json emitted by the static-root
 * wiring script — unbuilt areas render an honest "ships in a later step" state,
 * never a fake page.
 */

export type AreaId = 'admin' | 'app' | 'site';

export interface Area {
  id: AreaId;
  /** Trailing-slash target — ServeDir resolves directory indexes (D3/D4). */
  path: `/admin/` | `/app/` | `/site/`;
  built: boolean;
}

/** Shape written by front/scripts/build-static-root.mjs. */
export interface BuildManifest {
  shell?: boolean;
  admin?: boolean;
  app?: boolean;
  site?: boolean;
  builtAt?: string;
}

export function parseBuildManifest(raw: unknown): BuildManifest | null {
  if (raw === null || typeof raw !== 'object') return null;
  const m = raw as Record<string, unknown>;
  const out: BuildManifest = {};
  const flag = (k: 'shell' | 'admin' | 'app' | 'site'): void => {
    if (typeof m[k] === 'boolean') out[k] = m[k] as boolean;
  };
  flag('shell');
  flag('admin');
  flag('app');
  flag('site');
  if (typeof m.builtAt === 'string') out.builtAt = m.builtAt;
  return out;
}

/**
 * The three areas with honest availability.
 * No manifest at all (manual deploys / vite dev) → assume present.
 */
export function areasFromManifest(manifest: BuildManifest | null): Area[] {
  const built = (id: AreaId): boolean => manifest === null ? true : (manifest[id] === true);
  return [
    { id: 'admin', path: '/admin/', built: built('admin') },
    { id: 'app', path: '/app/', built: built('app') },
    { id: 'site', path: '/site/', built: built('site') },
  ];
}
