/**
 * The citizen webapp frame — Phase 7 Step 5 (33-page map, STEP1-AUDIT §2.4).
 *
 *  • Aurora: RTL-first, dark/light, fa default (the bot rule), logical props
 *  • Hash router (D4): 33 pages + honest 404 — deep links never hit the
 *    server path router, the Phase 4 "no SPA fallback" decision stands
 *  • Route-level code splitting: every page file is a lazy chunk (budget §6:
 *    route chunk ≤ 120 KB gzip, gated by scripts/budget.mjs)
 *  • Identity (D14): the raw Telegram initData flows ONLY through the
 *    generated client's identity layer; the display-only user comes from
 *    initDataUnsafe public fields; a stored staff key upgrades to staff mode
 *  • Offline banner: real online/offline events — money never moves offline
 *  • The typed generated client is the ONLY HTTP surface (D5)
 */
import { Suspense, lazy, useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { createAgentOneClient, optionsFromEnv } from '@agent-one/api-client';
import { applyLocale } from '@agent-one/ui/dir';
import { applyTheme, getStoredTheme, toggleTheme } from '@agent-one/ui/theme';
import { oppositeLocale, t } from '@agent-one/i18n/webapp';
import { BOTTOM_NAV, PAGE_GROUPS, pageHref, routeFromHash, type PageId } from './router.js';
import { actorMode } from './actions.js';
import {
  appendActionLog,
  initialLocale,
  loadAdminKey,
  loadBotHandle,
  saveLocale,
  telegramInitData,
  telegramUser,
  type ActionLogEntry,
  type TelegramUser,
} from './session.js';
import { dirFor, PendingState, useOnline } from './ui.js';
import type { PageCtx } from './ctx.js';

/* Route-level code splitting — one lazy chunk per page file. */
const HomePage = lazy(() => import('./pages/home.js'));
const ProductsListPage = lazy(() => import('./pages/products.js'));
const ProductDetailPage = lazy(() => import('./pages/products.js').then((m) => ({ default: m.ProductDetailPage })));
const BuyFlowPage = lazy(() => import('./pages/products.js').then((m) => ({ default: m.BuyFlowPage })));
const EscrowsListPage = lazy(() => import('./pages/escrows.js'));
const EscrowNewPage = lazy(() => import('./pages/escrows.js').then((m) => ({ default: m.EscrowNewPage })));
const EscrowDetailPage = lazy(() => import('./pages/escrows.js').then((m) => ({ default: m.EscrowDetailPage })));
const EscrowConfirmPage = lazy(() => import('./pages/escrows.js').then((m) => ({ default: m.EscrowConfirmPage })));
const HitlStatusPage = lazy(() => import('./pages/hitl.js'));
const WalletPage = lazy(() => import('./pages/wallet.js'));
const TxHistoryPage = lazy(() => import('./pages/wallet.js').then((m) => ({ default: m.TxHistoryPage })));
const TxDetailPage = lazy(() => import('./pages/wallet.js').then((m) => ({ default: m.TxDetailPage })));
const OrdersPage = lazy(() => import('./pages/orders.js'));
const OrderDetailPage = lazy(() => import('./pages/orders.js').then((m) => ({ default: m.OrderDetailPage })));
const ProfilePage = lazy(() => import('./pages/profile.js'));
const SettingsPage = lazy(() => import('./pages/profile.js').then((m) => ({ default: m.SettingsPage })));
const LanguagePage = lazy(() => import('./pages/profile.js').then((m) => ({ default: m.LanguagePage })));
const AiChatPage = lazy(() => import('./pages/ai.js'));
const AiDirectoryPage = lazy(() => import('./pages/ai.js').then((m) => ({ default: m.AiDirectoryPage })));
const QuestsPage = lazy(() => import('./pages/growth.js').then((m) => ({ default: m.QuestsPage })));
const BookingsPage = lazy(() => import('./pages/growth.js').then((m) => ({ default: m.BookingsPage })));
const LeaderboardPage = lazy(() => import('./pages/growth.js').then((m) => ({ default: m.LeaderboardPage })));
const EconomyPage = lazy(() => import('./pages/economy.js').then((m) => ({ default: m.EconomyPage })));
const CharterPage = lazy(() => import('./pages/economy.js').then((m) => ({ default: m.CharterPage })));
const NotificationsPage = lazy(() => import('./pages/misc.js').then((m) => ({ default: m.NotificationsPage })));
const HelpPage = lazy(() => import('./pages/misc.js').then((m) => ({ default: m.HelpPage })));
const OfflinePage = lazy(() => import('./pages/misc.js').then((m) => ({ default: m.OfflinePage })));
const NotFoundPage = lazy(() => import('./pages/misc.js').then((m) => ({ default: m.NotFoundPage })));
const AboutPage = lazy(() => import('./pages/misc.js').then((m) => ({ default: m.AboutPage })));

/** The confirm specs for the five money pages — tiny literals stay in the
 *  frame so the escrows chunk keeps its own code-splitting boundary. */
const CONFIRM_SPECS: Record<
  'escrowFund' | 'escrowDeliver' | 'escrowRelease' | 'escrowDispute' | 'escrowCancel',
  { titleKey: Parameters<typeof t>[1]; bodyKey: Parameters<typeof t>[1]; action: 'fund' | 'deliver' | 'release' | 'dispute' | 'cancel'; withReason?: boolean }
> = {
  escrowFund: { titleKey: 'confirmFundTitle', bodyKey: 'confirmFundBody', action: 'fund' },
  escrowDeliver: { titleKey: 'confirmDeliverTitle', bodyKey: 'confirmDeliverBody', action: 'deliver' },
  escrowRelease: { titleKey: 'confirmReleaseTitle', bodyKey: 'confirmReleaseBody', action: 'release' },
  escrowDispute: { titleKey: 'confirmDisputeTitle', bodyKey: 'confirmDisputeBody', action: 'dispute', withReason: true },
  escrowCancel: { titleKey: 'confirmCancelTitle', bodyKey: 'confirmCancelBody', action: 'cancel' },
};

const PAGE_LABELS: Record<PageId, Parameters<typeof t>[1]> = {
  home: 'navHome',
  products: 'navStore',
  product: 'productDetailTitle',
  buy: 'buyTitle',
  escrowNew: 'escrowNewTitle',
  escrows: 'navEscrows',
  escrow: 'escrowDetailTitle',
  escrowFund: 'confirmFundTitle',
  escrowDeliver: 'confirmDeliverTitle',
  escrowRelease: 'confirmReleaseTitle',
  escrowDispute: 'confirmDisputeTitle',
  escrowCancel: 'confirmCancelTitle',
  hitl: 'hitlStatusTitle',
  wallet: 'walletTitle',
  tx: 'txTitle',
  txDetail: 'txDetailTitle',
  orders: 'navOrders',
  order: 'orderDetailTitle',
  profile: 'profileTitle',
  settings: 'settingsTitle',
  language: 'languageTitle',
  ai: 'aiChatTitle',
  aiDirectory: 'aiDirectoryTitle',
  quests: 'questsTitle',
  bookings: 'bookingsTitle',
  leaderboard: 'leaderboardTitle',
  economy: 'economyTitle',
  charter: 'charterTitle',
  notifications: 'notifTitle',
  help: 'helpTitle',
  offline: 'offlineTitle',
  about: 'aboutTitle',
};

const BOTTOM_NAV_KEYS: Record<(typeof BOTTOM_NAV)[number], Parameters<typeof t>[1]> = {
  home: 'navHome',
  products: 'navStore',
  escrows: 'navEscrows',
  wallet: 'navWallet',
  ai: 'aiChatTitle',
};

const GROUP_TITLES: Record<(typeof PAGE_GROUPS)[number]['titleKey'], Parameters<typeof t>[1]> = {
  navHome: 'navHome',
  navStore: 'navStore',
  navEscrows: 'navEscrows',
  navWallet: 'navWallet',
  aiTitle: 'aiTitle',
  leaderboardTitle: 'leaderboardTitle',
  economyTitle: 'economyTitle',
  settingsTitle: 'settingsTitle',
  helpTitle: 'helpTitle',
};

/* Chat prefill handoff — a one-shot module slot (per navigation, in-memory).
 * The ladder buttons stash a sentence here and navigate to #/ai. */
let CHAT_HANDOFF: string | undefined;
function takeChatHandoff(): string | undefined {
  const v = CHAT_HANDOFF;
  CHAT_HANDOFF = undefined;
  return v;
}

/** Dispatch one route to its page element (params differ per page). */
function renderPage(page: PageId | 'notFound', ctx: PageCtx, params: readonly string[], hash: string): ReactNode {
  switch (page) {
    case 'home':
      return <HomePage ctx={ctx} />;
    case 'products':
      return <ProductsListPage ctx={ctx} />;
    case 'product':
      return <ProductDetailPage ctx={ctx} id={params[0]} />;
    case 'buy':
      return <BuyFlowPage ctx={ctx} id={params[0]} />;
    case 'escrowNew':
      return <EscrowNewPage ctx={ctx} id={params[0]} />;
    case 'escrows':
      return <EscrowsListPage ctx={ctx} />;
    case 'escrow':
      return <EscrowDetailPage ctx={ctx} id={params[0]} />;
    case 'escrowFund':
    case 'escrowDeliver':
    case 'escrowRelease':
    case 'escrowDispute':
    case 'escrowCancel':
      return <EscrowConfirmPage ctx={ctx} id={params[0]} spec={CONFIRM_SPECS[page]} />;
    case 'hitl':
      return <HitlStatusPage ctx={ctx} />;
    case 'wallet':
      return <WalletPage ctx={ctx} />;
    case 'tx':
      return <TxHistoryPage ctx={ctx} />;
    case 'txDetail':
      return <TxDetailPage ctx={ctx} id={params[0]} />;
    case 'orders':
      return <OrdersPage ctx={ctx} />;
    case 'order':
      return <OrderDetailPage ctx={ctx} id={params[0]} />;
    case 'profile':
      return <ProfilePage ctx={ctx} />;
    case 'settings':
      return <SettingsPage ctx={ctx} />;
    case 'language':
      return <LanguagePage ctx={ctx} />;
    case 'ai':
      return <AiChatPage ctx={ctx} initialPrefill={takeChatHandoff()} />;
    case 'aiDirectory':
      return <AiDirectoryPage ctx={ctx} />;
    case 'quests':
      return <QuestsPage ctx={ctx} />;
    case 'bookings':
      return <BookingsPage ctx={ctx} />;
    case 'leaderboard':
      return <LeaderboardPage ctx={ctx} />;
    case 'economy':
      return <EconomyPage ctx={ctx} />;
    case 'charter':
      return <CharterPage ctx={ctx} />;
    case 'notifications':
      return <NotificationsPage ctx={ctx} />;
    case 'help':
      return <HelpPage ctx={ctx} />;
    case 'offline':
      return <OfflinePage ctx={ctx} />;
    case 'about':
      return <AboutPage ctx={ctx} />;
    case 'notFound':
      return <NotFoundPage ctx={ctx} hash={hash} />;
  }
}

function useHashRoute(): string {
  const [hash, setHash] = useState(() => window.location.hash);
  useEffect(() => {
    const onHash = (): void => setHash(window.location.hash);
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  }, []);
  return hash;
}

function Shell() {
  const hash = useHashRoute();
  const [locale, setLocaleState] = useState(initialLocale);
  const [adminKey, setAdminKeyState] = useState<string | null>(() => loadAdminKey());
  const [drawer, setDrawer] = useState(false);
  const online = useOnline();

  const client = useMemo(() => {
    const c = createAgentOneClient(optionsFromEnv(import.meta.env));
    const key = loadAdminKey();
    if (key !== null) c.setAdminKey(key);
    const initData = telegramInitData();
    if (initData !== null) c.setTelegramInitData(initData);
    return c;
  }, []);

  const initData = useMemo(() => telegramInitData(), []);
  const tgUser: TelegramUser | null = useMemo(() => telegramUser(), []);
  const botHandle = useMemo(() => loadBotHandle(), []);
  const mode = useMemo(() => actorMode(adminKey, initData), [adminKey, initData]);

  useEffect(() => {
    applyLocale(locale);
    applyTheme(getStoredTheme());
  }, [locale]);

  const navigate = useCallback((page: PageId, params: readonly string[] = []) => {
    window.location.hash = pageHref(page, params);
  }, []);

  const openChat = useCallback(
    (prefill: string) => {
      CHAT_HANDOFF = prefill;
      navigate('ai');
    },
    [navigate],
  );

  const logAction = useCallback((entry: Omit<ActionLogEntry, 'at'>) => {
    appendActionLog(entry); // the session layer stamps the receipt time
  }, []);

  const ctx: PageCtx = useMemo(
    () => ({ client, locale, mode, initData, tgUser, botHandle, navigate, openChat, logAction }),
    [client, locale, mode, initData, tgUser, botHandle, navigate, openChat, logAction],
  );

  const route = useMemo(() => routeFromHash(hash), [hash]);
  const dir = dirFor(locale);
  void setAdminKeyState; // staff upgrades happen in settings (saveAdminKey + client)

  return (
    <div className="min-h-dvh flex flex-col" dir={dir}>
      <header className="flex items-center justify-between gap-3 p-4 pb-2">
        <button type="button" className="aur-btn aur-btn--ghost aur-btn--sm" aria-label={t(locale, 'menuAll')} onClick={() => setDrawer(true)}>
          ☰
        </button>
        <div className="flex items-baseline gap-2">
          <span className="text-lg font-bold">{t(locale, 'appName')}</span>
          <span className="text-aur-faint text-xs">{t(locale, 'webappTitle')}</span>
          {mode.kind === 'staff' && <span className="aur-badge aur-badge--brand">{t(locale, 'staffPreviewBadge')}</span>}
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            className="aur-btn aur-btn--ghost aur-btn--sm"
            aria-label={t(locale, 'language')}
            onClick={() => {
              const next = oppositeLocale(locale);
              saveLocale(next);
              applyLocale(next);
              setLocaleState(next);
            }}
          >
            {locale === 'fa' ? 'EN' : 'FA'}
          </button>
          <button type="button" className="aur-btn aur-btn--ghost aur-btn--sm" aria-label={t(locale, 'theme')} onClick={() => toggleTheme()}>
            {getStoredTheme() === 'dark' ? '☀' : '☾'}
          </button>
        </div>
      </header>

      {!online && (
        <div role="alert" className="mx-4 mb-2 text-sm text-aur-warning">
          {t(locale, 'offlineBody')}
        </div>
      )}

      <main className="aur-app-main flex-1 px-4">
        <Suspense fallback={<PendingState locale={locale} />}>{renderPage(route.page, ctx, route.params, hash)}</Suspense>
      </main>

      {drawer && (
        <div className="aur-drawer" role="dialog" aria-label={t(locale, 'menuAll')}>
          <button type="button" className="aur-drawer__scrim" aria-label={t(locale, 'close')} onClick={() => setDrawer(false)} />
          <div className="aur-drawer__panel">
            <div className="flex items-center justify-between mb-3">
              <strong>{t(locale, 'menuAll')}</strong>
              <button type="button" className="aur-btn aur-btn--ghost aur-btn--sm" onClick={() => setDrawer(false)}>
                ✕
              </button>
            </div>
            {PAGE_GROUPS.map((group) => (
              <div key={group.titleKey} className="mb-3">
                <div className="text-aur-faint text-xs mb-1">{t(locale, GROUP_TITLES[group.titleKey])}</div>
                <div className="flex flex-col gap-1">
                  {group.pages.map((p) => (
                    <a key={p} className="aur-nav__link" href={pageHref(p)} onClick={() => setDrawer(false)}>
                      {t(locale, PAGE_LABELS[p])}
                    </a>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      <nav className="aur-bottomnav" aria-label={t(locale, 'webappTitle')}>
        {BOTTOM_NAV.map((p) => (
          <a key={p} href={pageHref(p)} aria-current={route.page === p ? 'page' : 'false'}>
            {t(locale, BOTTOM_NAV_KEYS[p])}
          </a>
        ))}
      </nav>
    </div>
  );
}

export default function App() {
  return <Shell />;
}
