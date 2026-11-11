import { describe, expect, it } from 'vitest';
import {
  ORDER_ACTIONS,
  ORDER_STATUS_FILTERS,
  approvalStatusLabel,
  formatBytes,
  formatDate,
  orderActionLabel,
  orderStatusLabel,
  riskLabel,
  skillKindLabel,
} from '../src/format.js';

describe('format — dashboard display layer', () => {
  it('maps the preserved gen2 order vocabulary (fa + en) and falls back for unknown words', () => {
    expect(orderStatusLabel('fa', 'paid')).toBe('پرداخت‌شده');
    expect(orderStatusLabel('fa', 'PENDING')).toBe('در انتظار');
    expect(orderStatusLabel('en', 'delivered')).toBe('Delivered');
    expect(orderStatusLabel('fa', 'weird')).toBe('weird'); // data word, shown as-is
    expect(orderStatusLabel('en', 'weird')).toBe('weird');
  });

  it('exposes all six order filters and the three wire actions', () => {
    expect(ORDER_STATUS_FILTERS).toEqual(['all', 'paid', 'pending', 'delivered', 'refunded', 'cancelled']);
    expect(ORDER_ACTIONS).toEqual(['deliver', 'cancel', 'refund']);
    expect(orderActionLabel('fa', 'deliver')).toBe('تحویل');
    expect(orderActionLabel('fa', 'refund')).toBe('بازپرداخت');
    expect(orderActionLabel('en', 'cancel')).toBe('Cancel');
  });

  it('localizes approval/HITL statuses and risk levels with honest fallbacks', () => {
    expect(approvalStatusLabel('fa', 'PENDING')).toBe('در انتظار');
    expect(approvalStatusLabel('fa', 'REJECTED')).toBe('ردشده');
    expect(approvalStatusLabel('en', 'EXPIRED')).toBe('Expired');
    expect(approvalStatusLabel('fa', 'whatever')).toBe('whatever');
    expect(riskLabel('fa', 'low')).toBe('ریسک کم');
    expect(riskLabel('en', 'high')).toBe('High risk');
    expect(riskLabel('fa', 'unknown')).toBe('unknown');
    expect(skillKindLabel('fa', 'money')).toBe('پولی');
    expect(skillKindLabel('en', 'staff')).toBe('Staff');
  });

  it('formats bytes like a human (decimal units)', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(1024 * 1024)).toBe('1 MB');
    expect(formatBytes(-5)).toBe('—');
  });

  it('formats dates per locale and degrades on invalid input', () => {
    const fa = formatDate('2026-11-12T10:00:00Z', 'fa');
    const en = formatDate('2026-11-12T10:00:00Z', 'en');
    expect(fa.length).toBeGreaterThan(0);
    expect(en.length).toBeGreaterThan(0);
    expect(en).not.toBe(fa); // Persian vs Gregorian rendering
    expect(formatDate('not-a-date', 'fa')).toBe('not-a-date');
  });
});
