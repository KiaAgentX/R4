/**
 * Hash router for the dashboard SPA (decision D4 — hash routing, no server
 * SPA fallback). The section map is STEP1-AUDIT §2.5 verbatim: the eight
 * admin surfaces over the frozen dashboard routes.
 */

export const SECTION_IDS = [
  'overview',
  'economy',
  'products',
  'orders',
  'approvals',
  'growth',
  'ai',
  'platform',
] as const;

export type SectionId = (typeof SECTION_IDS)[number];

/** `#/products` → products; empty/unknown hash → the default section. */
export function sectionFromHash(hash: string): SectionId {
  const m = /^#\/([a-z]+)/.exec(hash);
  const id = m?.[1];
  return (SECTION_IDS as readonly string[]).includes(id ?? '') ? (id as SectionId) : 'overview';
}

/** Canonical href of a section — round-trips through sectionFromHash. */
export function sectionHref(id: SectionId): string {
  return `#/${id}`;
}
