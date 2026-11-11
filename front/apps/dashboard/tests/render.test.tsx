/**
 * Offline render tests — the presentational dashboard components render the
 * honest states (pending/error/empty/data) with fixture data, via
 * react-dom/server. Money is the domain mirror; error CODES never appear.
 */
import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { ApiRequestError } from '@agent-one/api-client/errors';
import {
  ArmButton,
  BarChart,
  Card,
  EmptyState,
  ErrorState,
  Meter,
  Money,
  PendingState,
  RawItemsTable,
  Tile,
} from '../src/ui.js';
import { OverviewView, type OverviewData } from '../src/sections/overview.js';
import { ApprovalsView } from '../src/sections/approvals.js';

const overview: OverviewData = {
  users: 7,
  products: 12,
  orders: 34,
  revenueStars: 1500,
  revenueCredits: 320,
  pendingApprovals: 2,
  aiCalls: 41,
  series: [
    { date: '2026-11-06', revenue: 120, orders: 3 },
    { date: '2026-11-07', revenue: 240, orders: 5 },
    { date: '2026-11-08', revenue: 60, orders: 1 },
  ],
  topProducts: [{ title: 'پکیج نقره‌ای', salesCount: 9 }],
  recentOrders: [
    {
      id: 'order-1',
      title: 'پکیج نقره‌ای',
      buyer: { name: 'Kia', username: 'kia' },
      totalStars: 300,
      totalCredits: 50,
      status: 'paid',
      createdAt: '2026-11-12T10:00:00Z',
    },
  ],
};

describe('render — Aurora primitives', () => {
  it('Money renders the exact domain format `187,672 Đ`', () => {
    const html = renderToStaticMarkup(<Money amount={187672} />);
    expect(html).toContain('187,672');
    expect(html).toContain('Đ');
  });

  it('Money marks HITL amounts (≥ 50,000 Đ) with the art. 7 attribute', () => {
    expect(renderToStaticMarkup(<Money amount={60000} hitl />)).toContain('data-hitl="true"');
    expect(renderToStaticMarkup(<Money amount={49999} />)).not.toContain('data-hitl');
  });

  it('Meter exposes progressbar semantics', () => {
    const html = renderToStaticMarkup(<Meter value={75} max={100} label="cap" />);
    expect(html).toContain('aria-valuenow="75"');
    expect(html).toContain('aria-valuemax="100"');
  });

  it('BarChart renders one column per point', () => {
    const html = renderToStaticMarkup(
      <BarChart
        points={[
          { label: '11-06', value: 1 },
          { label: '11-07', value: 2 },
          { label: '11-08', value: 3 },
        ]}
        ariaLabel="series"
      />,
    );
    expect(html.match(/aur-bars__col/g)?.length).toBe(3);
  });

  it('state surfaces are localized', () => {
    expect(renderToStaticMarkup(<PendingState locale="fa" />)).toContain('در حال بارگذاری…');
    expect(renderToStaticMarkup(<EmptyState locale="fa" />)).toContain('چیزی اینجا نیست');
  });

  it('ErrorState shows the LOCALIZED message and never the raw wire code (D8)', () => {
    const err = new ApiRequestError({ status: 401, code: 'INVALID_ADMIN_KEY', message: 'server msg' });
    const html = renderToStaticMarkup(<ErrorState locale="fa" error={err} />);
    expect(html).toContain('کلید ادمین نامعتبر است.');
    expect(html).not.toContain('INVALID_ADMIN_KEY');
    expect(html).not.toContain('server msg');
  });

  it('ArmButton renders unarmed first — armed warning only after interaction', () => {
    const html = renderToStaticMarkup(
      <ArmButton label="آرشیو" armedLabel="مطمئنی؟ این اقدام ثبت می‌شود." onConfirm={() => undefined} />,
    );
    expect(html).toContain('آرشیو');
    expect(html).not.toContain('مطمئنی؟');
  });

  it('RawItemsTable renders gen2 opaque shapes verbatim', () => {
    const html = renderToStaticMarkup(
      <RawItemsTable items={[{ id: 'q1', title: 'ماموریت', score: 5 }]} />,
    );
    expect(html).toContain('<th');
    expect(html).toContain('ماموریت');
    expect(html).toContain('q1');
  });

  it('Card shows title and optional footer note', () => {
    const html = renderToStaticMarkup(
      <Card title="اقتصاد" note="توجه">
        <span>بدنه</span>
      </Card>,
    );
    expect(html).toContain('اقتصاد');
    expect(html).toContain('بدنه');
    expect(html).toContain('توجه');
  });
});

describe('render — Overview section (command center)', () => {
  it('renders tiles, the series bars, top products and localized recent orders', () => {
    const html = renderToStaticMarkup(<OverviewView locale="fa" data={overview} brandName="Agent-One" />);
    expect(html).toContain('کاربران');
    expect(html).toContain('فراخوانی‌های AI');
    expect(html.match(/aur-bars__col/g)?.length).toBe(overview.series.length);
    expect(html).toContain('پکیج نقره‌ای');
    expect(html).toContain('پرداخت‌شده'); // gen2 vocabulary preserved + localized
    expect(html).toContain('Kia');
  });
});

describe('render — Approvals section (money honesty, §6)', () => {
  const approvals = [
    {
      id: 'apr-1',
      type: 'ESCROW_RELEASE',
      riskLevel: 'high' as const,
      summary: 'آزادسازی امانت ابر',
      note: null,
      amount: 60000,
      status: 'pending',
      createdAt: '2026-11-12T09:00:00Z',
      expiresAt: null,
    },
  ];
  const hitl = [
    {
      id: 'hitl-1',
      kind: 'ESCROW_RELEASE' as const,
      escrowId: 'esc-9',
      amount: 60000,
      requesterId: 'user:tg:123',
      note: null,
      status: 'PENDING' as const,
      createdAt: '2026-11-12T09:00:00Z',
      expiresAt: '2026-11-13T09:00:00Z',
      decidedBy: null,
      decidedAt: null,
      decisionNote: null,
    },
  ];

  function view() {
    return renderToStaticMarkup(
      <ApprovalsView
        locale="fa"
        approvals={approvals}
        hitl={hitl}
        busy={false}
        note=""
        onNote={() => undefined}
        onDecide={() => undefined}
        onHitlDecide={() => undefined}
        onSweep={() => undefined}
        sweepResult={null}
        settleResult={null}
      />,
    );
  }

  it('always shows the 90/10 split preview on a pending release (fees never hidden)', () => {
    const html = view();
    // 60,000 Đ → seller 54,000 + treasury 6,000 (floor semantics of mul_bps)
    expect(html).toContain('54,000');
    expect(html).toContain('6,000');
    expect(html).toContain('سهم فروشنده');
    expect(html).toContain('مالیات خزانه');
  });

  it('marks ≥ 50,000 Đ amounts as HITL and shows both queues', () => {
    const html = view();
    expect(html.match(/data-hitl="true"/g)?.length).toBeGreaterThanOrEqual(2);
    expect(html).toContain('تأییدهای ادمین (نسل ۲)');
    expect(html).toContain('صف HITL (مادهٔ ۷)');
    expect(html).toContain('esc-9');
  });
});
