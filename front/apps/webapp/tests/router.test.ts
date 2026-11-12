import { describe, expect, it } from 'vitest';
import {
  allPageHrefs,
  ALL_PAGES,
  BOTTOM_NAV,
  PAGE_GROUPS,
  PAGE_IDS,
  pageHref,
  routeFromHash,
  type PageId,
} from '../src/router.js';

describe('router — the 33-page map (STEP1-AUDIT §2.4)', () => {
  it('exposes 32 routable pages + the 404 fallback = the 33-page map, unique', () => {
    expect(PAGE_IDS.length).toBe(32);
    expect(ALL_PAGES.length).toBe(33); // the map's #32 (Error/404) is the fallback
    expect(new Set(ALL_PAGES).size).toBe(33);
    expect(PAGE_IDS[0]).toBe('home');
    expect(PAGE_IDS[31]).toBe('about');
  });

  it('round-trips every bare page href through the parser (param pages use demo ids)', () => {
    for (const page of PAGE_IDS) {
      const href = pageHref(page);
      if (href.endsWith('/') || href.includes('//')) continue; // '{id}' page without a param — covered below
      const route = routeFromHash(href);
      expect(route.page, `${href} → ${route.page}`).toBe(page);
      expect(route.params).toEqual([]);
    }
  });

  it('round-trips the parametrized pages (product/buy/escrow×6/tx/order)', () => {
    const cases: [PageId, string][] = [
      ['product', 'prod-1'],
      ['buy', 'prod-1'],
      ['escrowNew', 'esc-1'],
      ['escrow', 'esc-1'],
      ['escrowFund', 'esc-1'],
      ['escrowDeliver', 'esc-1'],
      ['escrowRelease', 'esc-1'],
      ['escrowDispute', 'esc-1'],
      ['escrowCancel', 'esc-1'],
      ['txDetail', 'esc-1'],
      ['order', 'ord-9'],
    ];
    for (const [page, id] of cases) {
      const href = pageHref(page, [id]);
      expect(href).toContain(id);
      const route = routeFromHash(href);
      expect(route.page).toBe(page);
      expect(route.params).toEqual([id]);
    }
  });

  it('parses deep compound routes like the release confirm', () => {
    expect(routeFromHash('#/escrow/esc:ai:9/release')).toEqual({ page: 'escrowRelease', params: ['esc:ai:9'] });
    expect(routeFromHash('#/escrow/esc:ai:9/fund')).toEqual({ page: 'escrowFund', params: ['esc:ai:9'] });
  });

  it('maps the bare app entry to home and every unknown hash to 404', () => {
    expect(routeFromHash('')).toEqual({ page: 'home', params: [] });
    expect(routeFromHash('#')).toEqual({ page: 'home', params: [] });
    expect(routeFromHash('#/nope')).toEqual({ page: 'notFound', params: [] });
    expect(routeFromHash('#/ESCROW/esc-1')).toEqual({ page: 'notFound', params: [] }); // ids are case-sensitive
    expect(routeFromHash('#/escrow')).toEqual({ page: 'notFound', params: [] }); // missing param is not a page
  });

  it('keeps ids exact — trailing segments never leak into params', () => {
    expect(routeFromHash('#/escrow/esc-1/extra')).toEqual({ page: 'notFound', params: [] });
  });

  it('allPageHrefs yields one renderable href per page (acceptance driver)', () => {
    const all = allPageHrefs();
    expect(all.length).toBe(33);
    for (const entry of all) {
      expect(routeFromHash(entry.href).page).toBe(entry.page);
    }
  });

  it('bottom nav is a subset of pages (§6 mobile rule)', () => {
    expect(BOTTOM_NAV.length).toBe(5);
    for (const p of BOTTOM_NAV) expect(PAGE_IDS).toContain(p);
  });

  it('the drawer groups cover all 32 routable pages exactly once (404 needs no nav)', () => {
    const covered = PAGE_GROUPS.flatMap((g) => g.pages);
    expect(covered.length).toBe(32);
    expect(new Set(covered).size).toBe(32);
    for (const p of PAGE_IDS) expect(covered).toContain(p);
  });
});
