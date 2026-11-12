import { describe, expect, it } from 'vitest';
import { areasFromManifest, parseBuildManifest } from '../src/areas.js';

describe('parseBuildManifest', () => {
  it('parses a full manifest', () => {
    const m = parseBuildManifest({ shell: true, admin: true, app: false, site: true, builtAt: '2026-11-11T17:00:00Z' });
    expect(m).toEqual({ shell: true, admin: true, app: false, site: true, builtAt: '2026-11-11T17:00:00Z' });
  });

  it('non-object → null; unknown extra keys tolerated', () => {
    expect(parseBuildManifest(null)).toBeNull();
    expect(parseBuildManifest('nope')).toBeNull();
    expect(parseBuildManifest({ admin: true, junk: 1 })).toEqual({ shell: undefined, admin: true, app: undefined, site: undefined, builtAt: undefined });
  });
});

describe('areasFromManifest — honest availability', () => {
  it('manifest present → only flagged areas are built (trailing-slash targets)', () => {
    const areas = areasFromManifest({ shell: true, admin: false, app: false, site: true });
    expect(areas).toEqual([
      { id: 'admin', path: '/admin/', built: false },
      { id: 'app', path: '/app/', built: false },
      { id: 'site', path: '/site/', built: true },
    ]);
  });

  it('no manifest (dev / manual deploy) → assume present', () => {
    for (const a of areasFromManifest(null)) expect(a.built).toBe(true);
  });
});
