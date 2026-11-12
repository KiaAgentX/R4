/**
 * PWA glue — Phase 7 Step 6 (STEP1-AUDIT §9: "installable PWA").
 *
 * Pure, dependency-injected guards so the decision logic is unit-testable
 * offline (happy-dom has no real service-worker container). The side effects
 * live only in `registerServiceWorker`, called once from `main.tsx`.
 */

export interface EnvLike {
  readonly PROD?: boolean;
  readonly MODE?: string;
}

export interface RegistrationOptionsLike {
  readonly scope?: string;
}

export interface ServiceWorkerContainerLike {
  register(scriptURL: string, options?: RegistrationOptionsLike): Promise<unknown>;
}

export interface NavigatorLike {
  readonly serviceWorker?: ServiceWorkerContainerLike;
  /** iOS Safari standalone flag (legacy but still the only one there). */
  readonly standalone?: boolean;
}

type MatchMedia = (query: string) => { matches: boolean };

/**
 * The shell registers its service worker only in production builds and only
 * when the browser actually supports the container — fail closed and honest:
 * without a SW the page keeps working as a plain document (audit §1: degrade,
 * never fake). Dev never registers — SW caching fights Vite hot reload.
 */
export function shouldRegisterServiceWorker(
  env: EnvLike = import.meta.env,
  nav: NavigatorLike = navigator,
): boolean {
  if (env.PROD !== true) return false;
  return typeof nav.serviceWorker !== 'undefined';
}

/** Registers `/sw.js` at the static root scope — the combined dist root (§5). */
export async function registerServiceWorker(
  env: EnvLike = import.meta.env,
  nav: NavigatorLike = navigator,
): Promise<void> {
  if (!shouldRegisterServiceWorker(env, nav)) return;
  await nav.serviceWorker?.register('/sw.js', { scope: '/' });
}

/**
 * True when running as an installed app (display-mode: standalone, or the
 * iOS-only navigator.standalone flag) — used by the shell to render the
 * "installed" state instead of the install hint.
 */
export function isStandaloneDisplay(
  nav: NavigatorLike = navigator,
  matchMedia: MatchMedia | undefined = typeof window !== 'undefined' && typeof window.matchMedia === 'function' ? window.matchMedia.bind(window) : undefined,
): boolean {
  if (nav.standalone === true) return true;
  return matchMedia?.('(display-mode: standalone)')?.matches === true;
}
