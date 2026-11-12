/**
 * Shell dictionary — shell-area dictionary OWN keys (Phase 7 Step 7 split).
 * Strategic index (Phase 7 Step 3): area cards, context detection, role labels.
 * Strings are carried over VERBATIM from the pre-split dict.ts;
 * the shared vocabulary lives in dict-core.ts and is composed in
 * at the area entry (area-shell.ts).
 */
import type { Locale } from './locale.js';

export const faShellKeys = {
  tagline: 'کشور Agent-One — منشور ۱۰ ماده‌ای، پول Din و امانت ۸ مرحله‌ای',
  chooseArea: 'محیط خود را انتخاب کن',
  contextTitle: 'زمینهٔ شناسایی‌شده',
  areaDashboard: 'داشبورد ادمین',
  areaApp: 'اپ شهروندان (Mini App)',
  areaSite: 'سایت عمومی',
  areaPending: 'در گام‌های بعدی فاز ۷ ساخته می‌شود',
  open: 'ورود',
  backendDown: 'سرور در دسترس نیست',
  roleAdmin: 'ادمین',
  roleCitizen: 'شهروند در تلگرام',
  deviceMobile: 'موبایل',
  deviceDesktop: 'دسکتاپ',
  telegramHint: 'داخل تلگرام اجرا می‌شود',
  suggestedArea: 'مسیر پیشنهادی شما',
};

export type ShellKeys = typeof faShellKeys;

export const enShellKeys: ShellKeys = {
  tagline: 'The Agent-One nation — a 10-article charter, Din money, 8-state escrow',
  chooseArea: 'Choose your area',
  contextTitle: 'Detected context',
  areaDashboard: 'Admin dashboard',
  areaApp: 'Citizen app (Mini App)',
  areaSite: 'Public site',
  areaPending: 'Ships in a later Phase 7 step',
  open: 'Enter',
  backendDown: 'Server unreachable',
  roleAdmin: 'Admin',
  roleCitizen: 'Citizen in Telegram',
  deviceMobile: 'Mobile',
  deviceDesktop: 'Desktop',
  telegramHint: 'Runs inside Telegram',
  suggestedArea: 'Your suggested area',
};
