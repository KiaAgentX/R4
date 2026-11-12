/**
 * Agent-One admin dashboard — Phase 7 Step 4 (STEP1-AUDIT §2.5, §7).
 *
 *  • Aurora frame: RTL-first, dark/light, fa default (bot rule)
 *  • Admin gate: X-KIA-Key verified against a REAL admin route (GET /overview)
 *    before anything renders; 401 → invalid, 503 ADMIN_DISABLED → disabled
 *  • Hash router (D4): #/overview … #/platform — deep links never hit the
 *    server path router, the Phase 4 "no SPA fallback" decision stands
 *  • Route-level code splitting: every section is a lazy chunk (budget §6)
 *  • The typed generated client is the ONLY HTTP surface (D5); the frozen
 *    backend contract is never bypassed
 */
import { Suspense, lazy, useEffect, useState, type ComponentType } from 'react';
import { useQuery } from '@tanstack/react-query';
import { createAgentOneClient, optionsFromEnv } from '@agent-one/api-client';
import { clientErrorCode, unwrap } from '@agent-one/api-client/errors';
import { applyLocale } from '@agent-one/ui/dir';
import { applyTheme, effectiveTheme, getStoredTheme, toggleTheme } from '@agent-one/ui/theme';
import { dirOf, oppositeLocale, t, type Locale } from '@agent-one/i18n/dashboard';
import { SECTION_IDS, sectionFromHash, sectionHref, type SectionId } from './nav.js';
import {
  clearAdminKey,
  initialLocale,
  loadAdminKey,
  LOCALE_STORE,
  saveAdminKey,
} from './session.js';
import { AdminGateView, PendingState, type GateState } from './ui.js';
import type { SectionProps } from './props.js';

const client = createAgentOneClient(optionsFromEnv(import.meta.env));

const SECTIONS: Record<SectionId, ComponentType<SectionProps>> = {
  overview: lazy(() => import('./sections/overview.js')),
  economy: lazy(() => import('./sections/economy.js')),
  products: lazy(() => import('./sections/products.js')),
  orders: lazy(() => import('./sections/orders.js')),
  approvals: lazy(() => import('./sections/approvals.js')),
  growth: lazy(() => import('./sections/growth.js')),
  ai: lazy(() => import('./sections/ai.js')),
  platform: lazy(() => import('./sections/platform.js')),
};

const NAV_KEYS: Record<SectionId, Parameters<typeof t>[1]> = {
  overview: 'navOverview',
  economy: 'navEconomy',
  products: 'navProducts',
  orders: 'navOrders',
  approvals: 'navApprovals',
  growth: 'navGrowth',
  ai: 'navAi',
  platform: 'navPlatform',
};

type Gate = GateState | { kind: 'ok' };

function useHashSection(): SectionId {
  const [section, setSection] = useState<SectionId>(() => sectionFromHash(window.location.hash));
  useEffect(() => {
    const onHash = (): void => setSection(sectionFromHash(window.location.hash));
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  }, []);
  return section;
}

function BrandTitle({ locale }: { locale: Locale }) {
  const brand = useQuery({
    queryKey: ['dash-brand'],
    queryFn: async () => unwrap(await client.reads.brand()),
    retry: 0,
    refetchOnWindowFocus: false,
    staleTime: 60_000,
  });
  const name = brand.data?.data.name;
  return <>{typeof name === 'string' && name.length > 0 ? name : t(locale, 'appName')}</>;
}

export default function App() {
  const [locale, setLocaleState] = useState<Locale>(initialLocale);
  const [themeTick, setThemeTick] = useState(0);
  const [adminKey, setAdminKeyState] = useState<string | null>(() => loadAdminKey());
  const [probeNonce, setProbeNonce] = useState(0);
  const [gate, setGate] = useState<Gate>({ kind: 'checking' });
  const section = useHashSection();

  useEffect(() => {
    applyLocale(locale);
    applyTheme(getStoredTheme());
  }, [locale]);

  // Verify the stored/entered key against a real admin route — one probe,
  // honest states for invalid / disabled / unreachable.
  useEffect(() => {
    let alive = true;
    async function probe(): Promise<void> {
      if (adminKey === null) {
        setGate({ kind: 'form' });
        return;
      }
      client.setAdminKey(adminKey);
      setGate({ kind: 'checking' });
      try {
        await unwrap(await client.api.GET('/overview'));
        if (alive) setGate({ kind: 'ok' });
      } catch (e) {
        if (!alive) return;
        const code = clientErrorCode(e);
        if (code === 'INVALID_ADMIN_KEY' || code === 'ADMIN_KEY_REQUIRED') {
          setGate({ kind: 'invalid' });
        } else if (code === 'ADMIN_DISABLED') {
          setGate({ kind: 'disabled' });
        } else {
          setGate({ kind: 'error', error: e });
        }
      }
    }
    void probe();
    return () => {
      alive = false;
    };
  }, [adminKey, probeNonce]);

  const switchLocale = (next: Locale): void => {
    try {
      localStorage.setItem(LOCALE_STORE, next);
    } catch {
      /* storage disabled → session-only switch */
    }
    setLocaleState(next);
  };

  const submitKey = (key: string): void => {
    const trimmed = key.trim();
    saveAdminKey(trimmed); // false → storage disabled, key stays session-only
    client.setAdminKey(trimmed);
    setAdminKeyState(trimmed);
  };

  const signOut = (): void => {
    clearAdminKey();
    client.setAdminKey(undefined);
    setAdminKeyState(null);
    setGate({ kind: 'form' });
  };

  if (gate.kind !== 'ok') {
    return (
      <AdminGateView
        locale={locale}
        gate={gate}
        onSubmit={submitKey}
        onRetry={() => setProbeNonce((n) => n + 1)}
      />
    );
  }

  const Section = SECTIONS[section];
  const rtl = dirOf(locale) === 'rtl';
  const theme = effectiveTheme();
  void themeTick;

  return (
    <div className="min-h-dvh flex flex-col gap-5 p-4 sm:p-6" dir={rtl ? 'rtl' : 'ltr'}>
      <header className="flex items-center justify-between gap-4 flex-wrap">
        <div className="flex items-baseline gap-3">
          <h1 className="text-2xl font-bold">
            <BrandTitle locale={locale} />
          </h1>
          <span className="text-aur-muted text-sm">{t(locale, 'dashboardTitle')}</span>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            className="aur-btn aur-btn--ghost aur-btn--sm"
            onClick={() => switchLocale(oppositeLocale(locale))}
            aria-label={t(locale, 'language')}
          >
            {locale === 'fa' ? 'EN' : 'FA'}
          </button>
          <button
            type="button"
            className="aur-btn aur-btn--ghost aur-btn--sm"
            onClick={() => {
              toggleTheme();
              setThemeTick((n) => n + 1);
            }}
            aria-label={t(locale, 'theme')}
          >
            {theme === 'dark' ? '☀' : '☾'}
          </button>
          <button
            type="button"
            className="aur-btn aur-btn--danger aur-btn--sm"
            onClick={signOut}
            aria-label={t(locale, 'signOut')}
          >
            {t(locale, 'signOut')}
          </button>
        </div>
      </header>

      <nav className="aur-nav" aria-label={t(locale, 'dashboardTitle')}>
        {SECTION_IDS.map((id) => (
          <a
            key={id}
            className="aur-nav__link"
            href={sectionHref(id)}
            aria-current={section === id ? 'page' : 'false'}
          >
            {t(locale, NAV_KEYS[id])}
          </a>
        ))}
      </nav>

      <main className="flex-1">
        <Suspense fallback={<PendingState locale={locale} />}>
          <Section client={client} locale={locale} />
        </Suspense>
      </main>

      <footer className="text-aur-faint text-xs">{t(locale, 'idempotencyNote')}</footer>
    </div>
  );
}
