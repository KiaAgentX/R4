import { describe, expect, it } from 'vitest';
import { SECTION_IDS, sectionFromHash, sectionHref } from '../src/nav.js';

describe('nav — dashboard hash router (D4)', () => {
  it('parses every §2.5 section', () => {
    for (const id of SECTION_IDS) {
      expect(sectionFromHash(`#/${id}`)).toBe(id);
    }
  });

  it('defaults to overview for empty / unknown / malformed hashes', () => {
    expect(sectionFromHash('')).toBe('overview');
    expect(sectionFromHash('#')).toBe('overview');
    expect(sectionFromHash('#/does-not-exist')).toBe('overview');
    expect(sectionFromHash('#/Products')).toBe('overview'); // ids are lowercase-exact
    expect(sectionFromHash('garbage')).toBe('overview');
  });

  it('href round-trips through the parser', () => {
    for (const id of SECTION_IDS) {
      expect(sectionFromHash(sectionHref(id))).toBe(id);
    }
  });

  it('exposes exactly the eight sections of the audit map, unique', () => {
    expect([...SECTION_IDS]).toEqual([
      'overview',
      'economy',
      'products',
      'orders',
      'approvals',
      'growth',
      'ai',
      'platform',
    ]);
    expect(new Set(SECTION_IDS).size).toBe(8);
  });
});
