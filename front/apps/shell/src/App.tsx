import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { createAgentOneClient, optionsFromEnv } from '@agent-one/api-client';
import { unwrap } from '@agent-one/api-client/errors';
import { detectShell, LOCALE_STORE, primaryArea, type Detection } from './detect.js';
import { areasFromManifest, parseBuildManifest } from './areas.js';
import { applyLocale } from '@agent-one/ui/dir';
import { applyTheme, effectiveTheme, getStoredTheme, toggleTheme } from '@agent-one/ui/theme';
import { dirOf, t, type Locale } from '@agent-one/i18n/shell';

const client = createAgentOneClient(optionsFromEnv(import.meta.env));

function useBuildManifest() {
  return useQuery({
    queryKey: ['build-manifest'],
    queryFn: async (): Promise<ReturnType<typeof parseBuildManifest>> => {
      const res = await fetch('build-manifest.json', { headers: { accept: 'application/json' } });
      if (!res.ok) return null; // no manifest (dev / manual deploy) → assume present
      return parseBuildManifest(await res.json());
    },
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
}

function BackendChip({ locale }: { locale: Locale }) {
  const health = useQuery({
    queryKey: ['health'],
    queryFn: async () => unwrap(await client.reads.health()),
    retry: 1,
    refetchOnWindowFocus: false,
    staleTime: 15_000,
  });
  if (health.isPending) {
    return <span className="aur-spinner aur-spinner--inline" role="status" aria-label={t(locale, 'loading')} />;
  }
  if (health.isError) {
    return (
      <span className="aur-badge aur-badge--danger" title={t(locale, 'offlineBody')}>
        {t(locale, 'backendDown')}
      </span>
    );
  }
  return <span className="aur-badge aur-badge--brand">{t(locale, 'backendOk')}</span>;
}

function BrandChip({ locale }: { locale: Locale }) {
  const brand = useQuery({
    queryKey: ['brand'],
    queryFn: async () => unwrap(await client.reads.brand()),
    retry: 0,
    refetchOnWindowFocus: false,
    staleTime: 60_000,
  });
  const name = brand.data?.data.name;
  return <>{typeof name === 'string' && name.length > 0 ? name : t(locale, 'appName')}</>;
}

function roleLabel(locale: Locale, d: Detection): string {
  switch (d.role) {
    case 'admin':
      return t(locale, 'roleAdmin');
    case 'citizen':
      return t(locale, 'roleCitizen');
    case 'guest':
      return t(locale, 'roleGuest');
  }
}

export default function App() {
  const [detection] = useState<Detection>(() => detectShell(window));
  const [locale, setLocaleState] = useState<Locale>(detection.locale);
  const [themeTick, setThemeTick] = useState(0);
  const manifest = useBuildManifest();
  const areas = areasFromManifest(manifest.data ?? null);
  const rtl = dirOf(locale) === 'rtl';

  useEffect(() => {
    applyLocale(locale);
    applyTheme(getStoredTheme());
  }, [locale]);

  const switchLocale = (next: Locale): void => {
    try {
      localStorage.setItem(LOCALE_STORE, next);
    } catch {
      /* storage disabled → session-only switch */
    }
    setLocaleState(next);
  };

  const theme = effectiveTheme();
  void themeTick;

  return (
    <div className="min-h-dvh flex flex-col gap-6 p-5 sm:p-8" dir={rtl ? 'rtl' : 'ltr'}>
      <header className="flex items-center justify-between gap-4 flex-wrap">
        <div className="flex items-baseline gap-3">
          <h1 className="text-2xl font-bold">
            <BrandChip locale={locale} />
          </h1>
          <span className="text-aur-muted text-sm">{t(locale, 'tagline')}</span>
        </div>
        <div className="flex items-center gap-2">
          <BackendChip locale={locale} />
          <button
            type="button"
            className="aur-btn aur-btn--ghost aur-btn--sm"
            onClick={() => switchLocale(locale === 'fa' ? 'en' : 'fa')}
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
        </div>
      </header>

      <section className="aur-card" aria-labelledby="ctx">
        <div className="aur-card__header" id="ctx">
          {t(locale, 'contextTitle')}
        </div>
        <div className="aur-card__body flex flex-wrap items-center gap-2">
          <span className="aur-badge aur-badge--info">{roleLabel(locale, detection)}</span>
          <span className="aur-badge">
            {detection.device === 'mobile' ? t(locale, 'deviceMobile') : t(locale, 'deviceDesktop')}
          </span>
          <span className="aur-badge">{locale === 'fa' ? 'فارسی' : 'English'}</span>
          {detection.inTelegram && (
            <span className="aur-badge aur-badge--ai">
              {t(locale, 'telegramHint')}
              {detection.telegramPlatform !== null ? ` · ${detection.telegramPlatform}` : ''}
            </span>
          )}
        </div>
      </section>

      <section className="flex flex-col gap-3" aria-labelledby="areas">
        <h2 id="areas" className="text-lg font-bold">
          {t(locale, 'chooseArea')}
        </h2>
        <div className="grid gap-3 sm:grid-cols-3">
          {areas.map((a) => (
            <article key={a.id} className="aur-card flex flex-col justify-between gap-3 p-4">
              <div className="flex items-center justify-between gap-2">
                <h3 className="font-bold">
                  {a.id === 'admin' ? t(locale, 'areaDashboard') : a.id === 'app' ? t(locale, 'areaApp') : t(locale, 'areaSite')}
                </h3>
                {a.built ? (
                  <span className="aur-badge aur-badge--brand">✓</span>
                ) : (
                  <span className="aur-badge aur-badge--warning">…</span>
                )}
              </div>
              <code className="text-aur-faint text-xs" dir="ltr">
                {a.path}
              </code>
              {a.built ? (
                <a className="aur-btn aur-btn--primary justify-center" href={a.path}>
                  {t(locale, 'open')}
                </a>
              ) : (
                <span className="aur-badge aur-badge--warning justify-center py-1">{t(locale, 'areaPending')}</span>
              )}
            </article>
          ))}
        </div>
        <p className="text-aur-faint text-xs" dir="ltr">
          {t(locale, 'suggestedArea')}: {primaryArea(detection.role)}
        </p>
      </section>

      <footer className="mt-auto pt-4 text-aur-faint text-xs">{t(locale, 'idempotencyNote')}</footer>
    </div>
  );
}
