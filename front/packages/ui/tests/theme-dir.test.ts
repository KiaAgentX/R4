import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { applyTheme, effectiveTheme, getStoredTheme, toggleTheme } from '../src/theme.js';
import { applyLocale, dirOf } from '../src/dir.js';

describe('theme', () => {
  beforeEach(() => {
    document.documentElement.removeAttribute('data-theme');
    localStorage.clear();
    window.matchMedia = window.matchMedia ?? ((): MediaQueryList => ({ matches: false }) as MediaQueryList);
  });

  afterEach(() => {
    document.documentElement.removeAttribute('data-theme');
    localStorage.clear();
  });

  it('defaults to system (no data-theme attribute)', () => {
    expect(getStoredTheme()).toBe('system');
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false);
  });

  it('explicit dark wins and persists', () => {
    applyTheme('dark');
    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(getStoredTheme()).toBe('dark');
    expect(localStorage.getItem('aurora-theme')).toBe('dark');
  });

  it('system clears the attribute and the stored choice', () => {
    applyTheme('light');
    applyTheme('system');
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false);
    expect(localStorage.getItem('aurora-theme')).toBeNull();
  });

  it('toggle flips the effective theme', () => {
    applyTheme('light');
    expect(toggleTheme()).toBe('dark');
    expect(effectiveTheme()).toBe('dark');
    expect(toggleTheme()).toBe('light');
  });
});

describe('dirOf — RTL first-class', () => {
  it('fa → rtl (default locale of the nation)', () => {
    expect(dirOf('fa')).toBe('rtl');
    expect(dirOf('fa-IR')).toBe('rtl');
  });

  it('en → ltr', () => {
    expect(dirOf('en')).toBe('ltr');
    expect(dirOf('en-US')).toBe('ltr');
  });

  it('unknown codes fall back to ltr', () => {
    expect(dirOf('de')).toBe('ltr');
  });

  it('applyLocale sets both lang and dir on <html>', () => {
    applyLocale('fa');
    expect(document.documentElement.lang).toBe('fa');
    expect(document.documentElement.dir).toBe('rtl');
    applyLocale('en');
    expect(document.documentElement.lang).toBe('en');
    expect(document.documentElement.dir).toBe('ltr');
  });
});
