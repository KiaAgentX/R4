/**
 * RTL mirror audit — Phase 7 Step 7. Aurora's RTL strategy (UX rule §6) is
 * "logical properties only": flipping <html dir> must be the entire mirror
 * job. This guard scans every shipped stylesheet for physical direction
 * properties (left/right margins, paddings, borders, offsets, floats,
 * text-align) — a physical property sneaking in would silently break one
 * side of the mirror.
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const FRONT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');

/** Stylesheets shipped to users (sources, not generated dist). */
function cssFiles(): string[] {
  const roots = ['packages/ui/src', 'apps/shell/src', 'apps/dashboard/src', 'apps/webapp/src'];
  const out: string[] = [];
  const walk = (dir: string): void => {
    for (const e of readdirSync(dir)) {
      const p = join(dir, e);
      if (statSync(p).isDirectory()) walk(p);
      else if (e.endsWith('.css')) out.push(p);
    }
  };
  for (const r of roots) walk(join(FRONT, r));
  return out.sort();
}

/**
 * Physical-direction declarations that must NOT appear:
 *  • `left:` / `right:` offsets (use inset-inline-* instead)
 *  • margin/padding/border-(left|right) (use *-inline-* instead)
 *  • text-align: left|right (use start|end)
 *  • float: left|right (use logical flow or flex/grid)
 * Strip comments first so prose mentioning "left" cannot false-positive.
 */
const PHYSICAL = [
  /(^|[^\w-])left\s*:/,
  /(^|[^\w-])right\s*:/,
  /margin-(left|right)\s*:/,
  /padding-(left|right)\s*:/,
  /border-(left|right)(-[\w-]+)?\s*:/,
  /text-align\s*:\s*(left|right)\b/,
  /float\s*:\s*(left|right)\b/,
];

describe('RTL mirror — logical properties only', () => {
  it('every shipped stylesheet is free of physical direction declarations', () => {
    const files = cssFiles();
    expect(files.length).toBeGreaterThanOrEqual(4); // ui base/components/tokens + 3 app sheets
    const offenders: string[] = [];
    for (const f of files) {
      const css = readFileSync(f, 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
      for (const line of css.split('\n')) {
        if (PHYSICAL.some((re) => re.test(line))) offenders.push(`${f}: ${line.trim()}`);
      }
    }
    expect(offenders).toEqual([]);
  });
});
