/**
 * Render tests — the presentational webapp components prove the MONEY UX and
 * HITL UX acceptance criteria offline (react-dom/server, fixture data):
 *  • the 90/10 split ALWAYS shows on a release confirm (fees never hidden)
 *  • HITL amounts (≥ 50,000 Đ) carry the art. 7 badge, never a raw wire code
 *  • the 8-state timeline mirrors the domain machine with localized labels
 *  • the D14 ladder renders exactly ONE honest rung — no fake buttons
 */
import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createAgentOneClient } from '@agent-one/api-client';
import type { AgentOneClient } from '@agent-one/api-client';
import { EscrowConfirmView } from '../src/pages/escrows.js';
import { HelpPage, NotFoundPage, OfflinePage, NotificationsView } from '../src/pages/misc.js';
import { CharterPage } from '../src/pages/economy.js';
import { parseDinInput } from '../src/pages/products.js';
import { LadderView, Money, SplitPreview, StateTimeline } from '../src/ui.js';
import type { PageCtx } from '../src/ctx.js';
import type { ActionLogEntry } from '../src/session.js';
import type { EscrowRow } from '../src/pages/escrows.js';

const client: AgentOneClient = createAgentOneClient();

function ctxFixture(mode: 'staff' | 'citizen' | 'guest'): PageCtx {
  return {
    client,
    locale: 'fa',
    mode: { kind: mode },
    initData: mode === 'citizen' ? 'query_id=fixture' : null,
    tgUser: mode === 'citizen' ? { id: '42', name: 'Kia', username: 'kia', languageCode: 'fa' } : null,
    botHandle: 'DropAgentX_bot',
    navigate: () => undefined,
    openChat: () => undefined,
    logAction: () => undefined,
  };
}

const RELEASE_SPEC = { titleKey: 'confirmReleaseTitle', bodyKey: 'confirmReleaseBody', action: 'release' } as const;

function escrowFixture(overrides: Partial<EscrowRow> = {}): EscrowRow {
  return {
    id: 'esc-1',
    buyerId: 'user:tg:1',
    sellerId: 'user:tg:2',
    amount: 60_000,
    productRef: null,
    // DELIVERED is the state from which a release is legal (the money UX state)
    status: 'DELIVERED',
    createdAt: '2026-11-12T09:00:00Z',
    fundedTxId: 'tx-fund-1',
    settleTxId: null,
    disputeReason: null,
    deliveryRef: null,
    expiresAt: null,
    ...overrides,
  };
}

function render(node: React.ReactNode): string {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return renderToStaticMarkup(<QueryClientProvider client={qc}>{node}</QueryClientProvider>);
}

/* ── Money honesty (§6) ───────────────────────────────────────────────── */

describe('render — money mirrors the domain exactly', () => {
  it('Money renders `187,672 Đ` and marks HITL amounts automatically', () => {
    const html = render(<Money amount={187672} />);
    expect(html).toContain('187,672');
    expect(html).toContain('Đ');
    expect(html).toContain('data-hitl="true"'); // ≥ 50,000 Đ → art. 7
  });

  it('49,999 Đ is NOT marked; the explicit hitl=false override works', () => {
    expect(render(<Money amount={49999} />)).not.toContain('data-hitl');
    expect(render(<Money amount={60000} hitl={false} />)).not.toContain('data-hitl');
  });

  it('SplitPreview computes the floor split: 60,000 → 54,000 + 6,000', () => {
    const html = render(<SplitPreview locale="fa" amount={60000} />);
    expect(html).toContain('54,000');
    expect(html).toContain('6,000');
    expect(html).toContain('سهم فروشنده');
    expect(html).toContain('مالیات خزانه');
  });

  it('SplitPreview floors like mul_bps: 5 Đ → 5 seller + 0 tax', () => {
    const html = render(<SplitPreview locale="fa" amount={5} />);
    expect(html).toContain('5 Đ');
  });
});

/* ── The release confirm — the money UX acceptance core ───────────────── */

describe('render — release confirm (90/10 always, HITL badge, D14 ladder)', () => {
  function releaseHtml(mode: 'staff' | 'citizen' | 'guest'): string {
    return render(<EscrowConfirmView ctx={ctxFixture(mode)} e={escrowFixture()} spec={RELEASE_SPEC} />);
  }

  it('ALWAYS shows the 90/10 preview with localized labels (fees never hidden)', () => {
    const html = releaseHtml('guest');
    expect(html).toContain('تأیید آزادسازی');
    expect(html).toContain('54,000');
    expect(html).toContain('6,000');
    expect(html).toContain('مالیات هرگز پنهان نیست');
  });

  it('marks ≥ 50,000 Đ with the art. 7 HITL badge and never a raw code', () => {
    const html = releaseHtml('guest');
    expect(html).toContain('data-hitl="true"');
    expect(html).toContain('نیاز به تأیید انسانی');
    expect(html).not.toContain('REQUIRES_HUMAN_APPROVAL');
    expect(html).not.toContain('INVALID_ADMIN_KEY');
  });

  it('guest → the exact bot command + a real deep link, NO staff button', () => {
    const html = releaseHtml('guest');
    expect(html).toContain('/release esc-1');
    expect(html).toContain('https://t.me/DropAgentX_bot');
    expect(html).not.toContain('اجرا با کلید کارکنان');
  });

  it('citizen release is ALSO the bot rung (release is not a chat skill)', () => {
    const html = releaseHtml('citizen');
    expect(html).toContain('/release esc-1');
    expect(html).not.toContain('اجرا با کلید کارکنان');
  });

  it('staff → the armed direct button appears unarmed first (two-step money rule)', () => {
    const html = releaseHtml('staff');
    expect(html).toContain('اجرا با کلید کارکنان');
    // The honest warning appears ONLY after the first click (same as dashboard)
    expect(html).not.toContain('مطمئنی؟ این اقدام ثبت می‌شود.');
    expect(html).not.toContain('https://t.me/DropAgentX_bot');
  });

  it('release from CREATED is illegal — honest read-only note, no ladder', () => {
    const html = render(
      <EscrowConfirmView ctx={ctxFixture('staff')} e={escrowFixture({ status: 'CREATED' })} spec={RELEASE_SPEC} />,
    );
    expect(html).toContain('فقط خواندنی');
    expect(html).not.toContain('اجرا با کلید کارکنان');
    expect(html).not.toContain('/release esc-1');
  });

  it('dispute confirm carries the reason field (art. 9 wording)', () => {
    const html = render(
      <EscrowConfirmView
        ctx={ctxFixture('staff')}
        e={escrowFixture({ status: 'FUNDED' })}
        spec={{ titleKey: 'confirmDisputeTitle', bodyKey: 'confirmDisputeBody', action: 'dispute', withReason: true }}
      />,
    );
    expect(html).toContain('اعلام اختلاف');
    expect(html).toContain('داوری');
  });
});

/* ── The 8-state timeline ─────────────────────────────────────────────── */

describe('render — the 8-state timeline', () => {
  it('renders all 8 states with the current one highlighted', () => {
    const html = render(<StateTimeline locale="fa" state="FUNDED" />);
    for (const label of ['ساخته‌شده', 'تأمین‌شده', 'تحویل‌شده', 'تسویه‌شده', 'اختلافی', 'بازپرداخت‌شده', 'لغوشده', 'منقضی‌شده']) {
      expect(html).toContain(label);
    }
    expect(html.match(/aur-timeline__row/g)?.length).toBe(8);
    expect(html.match(/data-state="current"/g)?.length).toBe(1);
    expect(html.match(/data-state="done"/g)?.length).toBe(1); // CREATED before FUNDED
  });

  it('shows the legal next states for the current state (fa labels)', () => {
    const html = render(<StateTimeline locale="fa" state="CREATED" />);
    expect(html).toContain('→ تأمین‌شده · لغوشده');
  });

  it('terminal states close the machine — no next-hint, square dot', () => {
    const html = render(<StateTimeline locale="fa" state="RELEASED" />);
    expect(html).not.toContain('→');
    expect(html).toContain('data-terminal="true"');
  });
});

/* ── The D14 ladder variants ──────────────────────────────────────────── */

describe('render — LadderView rungs', () => {
  it('chat rung shows the prefilled sentence and opens the assistant', () => {
    const html = render(
      <LadderView
        locale="fa"
        route={{ via: 'chat', prefill: 'سلام — تحویل امانت esc-9 را ثبت کن.' }}
        botHandle="bot"
        busy={false}
        onStaffConfirm={() => undefined}
        onChat={() => undefined}
      />,
    );
    expect(html).toContain('انجام با دستیار (تلگرام)');
    expect(html).toContain('esc-9');
  });

  it('readonly rung is an honest note — no button at all', () => {
    const html = render(
      <LadderView
        locale="fa"
        route={{ via: 'readonly' }}
        botHandle={null}
        busy={false}
        onStaffConfirm={() => undefined}
        onChat={() => undefined}
      />,
    );
    expect(html).toContain('فقط خواندنی');
    expect(html).not.toContain('aur-btn aur-btn--primary');
  });

  it('bot rung without a handle shows the honest settings hint (no dead link)', () => {
    const html = render(
      <LadderView
        locale="fa"
        route={{ via: 'bot', command: '/fund esc-1' }}
        botHandle={null}
        busy={false}
        onStaffConfirm={() => undefined}
        onChat={() => undefined}
      />,
    );
    expect(html).toContain('نام کاربری ربات را در تنظیمات وارد کن');
    expect(html).not.toContain('https://t.me/');
    expect(html).toContain('/fund esc-1');
  });
});

/* ── Notifications receipts (local-only honesty) ──────────────────────── */

describe('render — notifications are local receipts', () => {
  it('renders receipts with tx ids and the replay badge', () => {
    const entries: ActionLogEntry[] = [
      { at: '2026-11-12T10:00:00Z', kind: 'staff', action: 'escrow.release', target: 'esc-1', txId: 'tx-9', replayed: true },
    ];
    const html = render(<NotificationsView entries={entries} locale="fa" />);
    expect(html).toContain('escrow.release');
    expect(html).toContain('tx-9');
    expect(html).toContain('پخش مجدد');
  });

  it('empty log says so honestly', () => {
    const html = render(<NotificationsView entries={[]} locale="fa" />);
    expect(html).toContain('فعالیتی ثبت نشده است.');
  });
});

/* ── Pages that render without any query (SSR-safe) ───────────────────── */

describe('render — no-query pages', () => {
  it('charter prints all 10 articles verbatim with numbers', () => {
    const html = render(<CharterPage ctx={ctxFixture('guest')} />);
    expect(html).toContain('منشور کشور (۱۰ ماده)');
    expect(html.match(/aur-receipt"/g)?.length).toBe(10);
    expect(html).toContain('واحد پول');
    expect(html).toContain('آستانه HITL');
  });

  it('help lists the bot commands; admin commands need a staff key', () => {
    const guest = render(<HelpPage ctx={ctxFixture('guest')} />);
    expect(guest).toContain('/buy');
    expect(guest).not.toContain('/audit');
    const staff = render(<HelpPage ctx={ctxFixture('staff')} />);
    expect(staff).toContain('/audit');
  });

  it('offline page reads the real navigator state and promises no offline money moves', () => {
    const html = render(<OfflinePage ctx={ctxFixture('guest')} />);
    expect(html).toContain('هیچ اقدام پولی آفلاین اجرا نمی‌شود');
  });

  it('404 shows the bad hash and the way home', () => {
    const html = render(<NotFoundPage ctx={ctxFixture('guest')} hash="#/nope" />);
    expect(html).toContain('این صفحه پیدا نشد');
    expect(html).toContain('#/nope');
    expect(html).toContain('بازگشت به خانه');
  });
});

/* ── The Din input parser (buy flow) ──────────────────────────────────── */

describe('parseDinInput — fail-closed at the edge', () => {
  it('accepts plain, grouped, Persian and Arabic digits', () => {
    expect(parseDinInput('54000')).toBe(54000);
    expect(parseDinInput('187,672')).toBe(187672);
    expect(parseDinInput('۵۴٬۰۰۰')).toBe(54000); // fa digits + U+066C grouping
    expect(parseDinInput('٥٤٠٠٠')).toBe(54000); // Arabic-Indic digits
    expect(parseDinInput('0')).toBe(0);
  });

  it('rejects floats, negatives and out-of-range values (never silently rounds)', () => {
    expect(parseDinInput('1.5')).toBeNull();
    expect(parseDinInput('-5')).toBeNull();
    expect(parseDinInput('abc')).toBeNull();
    expect(parseDinInput('')).toBeNull();
    expect(parseDinInput('9007199254740992')).toBeNull(); // > 2^53−1
    expect(parseDinInput('9007199254740991')).toBe(9007199254740991);
  });
});
