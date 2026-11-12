import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// vitest runs from front/ — import.meta.url is a virtual module URL there.
const WEBAPP = resolve(process.cwd(), 'apps/webapp');

describe('webapp iOS/standalone hooks (Step 6)', () => {
  const html = readFileSync(join(WEBAPP, 'index.html'), 'utf8');

  it('apple-touch icon points at the static-root icon (one Aurora source)', () => {
    expect(html).toContain('rel="apple-touch-icon" href="/icons/apple-touch-icon.png"');
    expect(html).toContain('apple-mobile-web-app-capable');
    expect(html).toContain('apple-mobile-web-app-status-bar-style');
    expect(html).toContain('apple-mobile-web-app-title');
  });

  it('safe-area contract stays intact: viewport-fit=cover + env() consumers', () => {
    expect(html).toContain('viewport-fit=cover');
    const css = readFileSync(join(WEBAPP, 'src', 'webapp.css'), 'utf8');
    expect(css).toContain('env(safe-area-inset-bottom');
  });

  it('no service worker of its own — the shell at "/" owns the SW scope', () => {
    expect(html).not.toContain('manifest'); // area app stays plain; shell manifest covers scope
  });
});
