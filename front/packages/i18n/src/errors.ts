/**
 * Error dictionary — decision D8 (STEP1-AUDIT §12):
 * the 20 domain codes mirror `crates/bot/src/i18n.rs → domain_error_msg`
 * wording **verbatim** (same fa/en strings the bot tests pin), server wire
 * codes use the same honest voice, and raw codes never reach the user.
 */
import type { Locale } from './locale.js';

/** The 20 domain codes — mirrors DomainErrorCode::as_str (errors.rs). */
export const DOMAIN_ERROR_CODES = [
  'NON_INTEGER_AMOUNT',
  'NEGATIVE_AMOUNT',
  'AMOUNT_OUT_OF_RANGE',
  'BPS_OUT_OF_RANGE',
  'TOO_FEW_POSTINGS',
  'UNBALANCED_TRANSACTION',
  'UNKNOWN_ACCOUNT',
  'INVALID_ACCOUNT_KIND',
  'DUPLICATE_ACCOUNT',
  'ACCOUNT_OVERDRAFT',
  'INVALID_TRANSITION',
  'ESCROW_NOT_FOUND',
  'ESCROW_ALREADY_EXISTS',
  'REQUIRES_HUMAN_APPROVAL',
  'MINT_CAP_EXCEEDED',
  'HITL_NOT_FOUND',
  'HITL_NOT_REQUIRED',
  'HITL_ALREADY_DECIDED',
  'HITL_EXPIRED',
  'PRODUCT_NOT_FOUND',
] as const;
export type DomainErrorCode = (typeof DOMAIN_ERROR_CODES)[number];

/** Server wire codes beyond the domain set — HttpFailure constructors (error.rs). */
export const WIRE_ERROR_CODES = [
  'ADMIN_DISABLED',
  'ADMIN_KEY_REQUIRED',
  'INVALID_ADMIN_KEY',
  'UNAUTHORIZED',
  'NOT_FOUND',
  'INVALID_REQUEST',
  'RATE_LIMITED',
  'NOT_IMPLEMENTED',
  'RESERVED_ID',
  'DB_DOWN',
  'BACKUP_FAILED',
  'BACKUP_UNSUPPORTED',
  'BOT_DISABLED',
  'BOT_UNAUTHORIZED',
  'AI_IDENTITY_REQUIRED',
  'AI_IDENTITY_DISABLED',
  'AI_DISABLED',
  'AI_UPSTREAM',
  'AI_CAP_EXCEEDED',
  'AI_MALFORMED_PACKET',
] as const;
export type WireErrorCode = (typeof WIRE_ERROR_CODES)[number];

/** Client-side failures (never produced by the server). */
export const CLIENT_ERROR_CODES = ['NETWORK_FAILED', 'REQUEST_ABORTED', 'MALFORMED_ERROR_BODY'] as const;
export type ClientErrorCode = (typeof CLIENT_ERROR_CODES)[number];

export type ErrorCode = DomainErrorCode | WireErrorCode | ClientErrorCode | 'INTERNAL';

/** Fallback for codes outside the known map — mirrors the bot's `_` arm. */
const INTERNAL_FALLBACK: Record<Locale, string> = {
  fa: '🚨 خطای دامنهٔ داخلی — این خطا نباید به کاربر برسد؛ در audit ثبت شد.',
  en: '🚨 Internal error — this should never reach the user; logged to audit.',
};

/** Domain codes — VERBATIM mirror of `domain_error_msg` (bot i18n.rs). */
const DOMAIN_MSG: Record<Locale, Record<DomainErrorCode, string>> = {
  fa: {
    UNKNOWN_ACCOUNT: '🤷 حساب شما در کشور موجود نیست — اول ادمین برای شما کیف بسازد.',
    ESCROW_NOT_FOUND: '🤷 امانت پیدا نشد — شناسه را چک کن.',
    ESCROW_ALREADY_EXISTS: '🛡 امانت با این شناسه از قبل وجود دارد.',
    INVALID_TRANSITION: '🚦 این گذار در وضعیت فعلی امانت مجاز نیست (ماشین ۸ حالته).',
    REQUIRES_HUMAN_APPROVAL: '🛡 این مبلغ بالای آستانهٔ ۵۰٬۰۰۰ Đ است — باید از صف HITL تأیید ادمین بگذرد.',
    HITL_NOT_FOUND: '🤷 درخواست HITL پیدا نشد — با /queue فهرست را ببین.',
    HITL_ALREADY_DECIDED: '🚫 این درخواست قبلاً تصمیم‌گیری شده — تصمیم دوم بی‌اثر است (بازپرداخت صفر).',
    HITL_EXPIRED: '⏰ درخواست HITL منقضی شده (TTL ۲۴ ساعت) — باید درخواست تازه ساخته شود.',
    HITL_NOT_REQUIRED: 'ℹ️ این مبلغ زیر آستانهٔ HITL است — مستقیم تسویه می‌شود.',
    ACCOUNT_OVERDRAFT: '💸 موجودی کافی نیست — کیف هرگز منفی نمی‌شود.',
    MINT_CAP_EXCEEDED: '🏦 سقف ضرب سالانهٔ مادهٔ ۳ پر است.',
    PRODUCT_NOT_FOUND: '🤷 محصول پیدا نشد — با /products فهرست را ببین.',
    INVALID_ACCOUNT_KIND: '⛔ نوع حساب مجاز نیست — خریدار و فروشنده باید هر دو کاربر باشند.',
    DUPLICATE_ACCOUNT: '🛡 این حساب قبلاً ساخته شده.',
    NON_INTEGER_AMOUNT: '⚠️ مبلغ باید عدد صحیح مثبت در مرز امن (≤ ۲^۵۳−۱) باشد.',
    NEGATIVE_AMOUNT: '⚠️ مبلغ باید عدد صحیح مثبت در مرز امن (≤ ۲^۵۳−۱) باشد.',
    AMOUNT_OUT_OF_RANGE: '⚠️ مبلغ باید عدد صحیح مثبت در مرز امن (≤ ۲^۵۳−۱) باشد.',
    BPS_OUT_OF_RANGE: '🚨 خطای دامنهٔ داخلی — این خطا نباید به کاربر برسد؛ در audit ثبت شد.',
    TOO_FEW_POSTINGS: '🚨 خطای دامنهٔ داخلی — این خطا نباید به کاربر برسد؛ در audit ثبت شد.',
    UNBALANCED_TRANSACTION: '🚨 خطای دامنهٔ داخلی — این خطا نباید به کاربر برسد؛ در audit ثبت شد.',
  },
  en: {
    UNKNOWN_ACCOUNT: '🤷 Your account does not exist in the nation yet — ask an admin.',
    ESCROW_NOT_FOUND: '🤷 Escrow not found — check the id.',
    ESCROW_ALREADY_EXISTS: '🛡 An escrow with this id already exists.',
    INVALID_TRANSITION: '🚦 This transition is not allowed in the escrow\u2019s current state (8-state machine).',
    REQUIRES_HUMAN_APPROVAL: '🛡 This amount is above the 50,000 Đ threshold — it must pass the HITL queue.',
    HITL_NOT_FOUND: '🤷 HITL request not found — see /queue for the list.',
    HITL_ALREADY_DECIDED: '🚫 This request was already decided — a second decision is void (zero refund).',
    HITL_EXPIRED: '⏰ The HITL request expired (24h TTL) — a fresh request is needed.',
    HITL_NOT_REQUIRED: 'ℹ️ This amount is below the HITL threshold — it settles directly.',
    ACCOUNT_OVERDRAFT: '💸 Insufficient balance — wallets never go negative.',
    MINT_CAP_EXCEEDED: '🏦 The yearly mint cap (art. 3) is exhausted.',
    PRODUCT_NOT_FOUND: '🤷 Product not found — see /products for the list.',
    INVALID_ACCOUNT_KIND: '⛔ Invalid account kind — buyer and seller must both be users.',
    DUPLICATE_ACCOUNT: '🛡 This account already exists.',
    NON_INTEGER_AMOUNT: '⚠️ Amount must be a positive integer within the safe bound (≤ 2^53−1).',
    NEGATIVE_AMOUNT: '⚠️ Amount must be a positive integer within the safe bound (≤ 2^53−1).',
    AMOUNT_OUT_OF_RANGE: '⚠️ Amount must be a positive integer within the safe bound (≤ 2^53−1).',
    BPS_OUT_OF_RANGE: '🚨 Internal error — this should never reach the user; logged to audit.',
    TOO_FEW_POSTINGS: '🚨 Internal error — this should never reach the user; logged to audit.',
    UNBALANCED_TRANSACTION: '🚨 Internal error — this should never reach the user; logged to audit.',
  },
};

/** Server wire codes — same voice, new surface (dashboard-only failures). */
const WIRE_MSG: Record<Locale, Record<WireErrorCode, string>> = {
  fa: {
    ADMIN_DISABLED: 'پنل ادمین روی این سرور خاموش است.',
    ADMIN_KEY_REQUIRED: 'کلید ادمین (X-KIA-Key) همراه درخواست نبود.',
    INVALID_ADMIN_KEY: 'کلید ادمین نامعتبر است.',
    UNAUTHORIZED: 'هویت شما تأیید نشد — این مسیر برای شما باز نیست.',
    NOT_FOUND: 'چیزی با این شناسه پیدا نشد.',
    INVALID_REQUEST: 'بدنهٔ درخواست خراب است — فیلدها را چک کن.',
    RATE_LIMITED: 'خیلی سریع و خیلی زیاد — کمی صبر کن و دوباره تلاش کن.',
    NOT_IMPLEMENTED: 'این قابلیت هنوز روی سرور پیاده نشده — صادقانه: بعداً می‌آید.',
    RESERVED_ID: 'این شناسه رزرو شده است و قابل استفاده نیست.',
    DB_DOWN: 'دیتابیس در دسترس نیست — چند لحظه بعد دوباره تلاش کن.',
    BACKUP_FAILED: 'بکاپ کامل نشد — جزئیات در audit ثبت شد.',
    BACKUP_UNSUPPORTED: 'بکاپ در این محیط پشتیبانی نمی‌شود.',
    BOT_DISABLED: 'وبهوک بات روی این سرور خاموش است.',
    BOT_UNAUTHORIZED: 'امضای وبهوک تلگرام معتبر نیست.',
    AI_IDENTITY_REQUIRED: 'برای گفتگو با دستیار، هویت لازم است — از تلگرام وارد شو یا کلید staff بده.',
    AI_IDENTITY_DISABLED: 'ورود هویتی به دستیار روی این سرور خاموش است.',
    AI_DISABLED: 'دستیار AI روی این سرور فعال نیست.',
    AI_UPSTREAM: 'سرویس هوش مصنوعی بالادستی پاسخ نداد — کمی بعد تلاش کن.',
    AI_CAP_EXCEEDED: 'سقف روزانهٔ استفاده از دستیار پر شد — فردا دوباره بیا.',
    AI_MALFORMED_PACKET: 'پاسخ دستیار خراب بود — ثبت شد تا بررسی شود.',
  },
  en: {
    ADMIN_DISABLED: 'The admin panel is disabled on this server.',
    ADMIN_KEY_REQUIRED: 'The admin key (X-KIA-Key) was not sent with the request.',
    INVALID_ADMIN_KEY: 'The admin key is invalid.',
    UNAUTHORIZED: 'Your identity was not verified — this path is not open to you.',
    NOT_FOUND: 'Nothing was found with this id.',
    INVALID_REQUEST: 'The request body is malformed — check the fields.',
    RATE_LIMITED: 'Too many requests — wait a moment and try again.',
    NOT_IMPLEMENTED: 'This capability is not implemented on the server yet — it ships later.',
    RESERVED_ID: 'This id is reserved and cannot be used.',
    DB_DOWN: 'The database is unreachable — try again shortly.',
    BACKUP_FAILED: 'The backup did not complete — details were logged to audit.',
    BACKUP_UNSUPPORTED: 'Backup is not supported in this environment.',
    BOT_DISABLED: 'The bot webhook is disabled on this server.',
    BOT_UNAUTHORIZED: 'The Telegram webhook signature is invalid.',
    AI_IDENTITY_REQUIRED: 'Talking to the assistant needs identity — sign in from Telegram or provide a staff key.',
    AI_IDENTITY_DISABLED: 'Identity entry to the assistant is disabled on this server.',
    AI_DISABLED: 'The AI assistant is not enabled on this server.',
    AI_UPSTREAM: 'The upstream AI service did not answer — try again shortly.',
    AI_CAP_EXCEEDED: 'The daily AI cap is reached — come back tomorrow.',
    AI_MALFORMED_PACKET: 'The assistant reply was malformed — it was logged for review.',
  },
};

const CLIENT_MSG: Record<Locale, Record<ClientErrorCode, string>> = {
  fa: {
    NETWORK_FAILED: 'شبکه در دسترس نیست — درخواست ارسال نشد؛ بعداً تلاش کن.',
    REQUEST_ABORTED: 'درخواست لغو شد.',
    MALFORMED_ERROR_BODY: 'پاکت خطای سرور خراب بود.',
  },
  en: {
    NETWORK_FAILED: 'Network is unreachable — the request was not sent; try again later.',
    REQUEST_ABORTED: 'The request was aborted.',
    MALFORMED_ERROR_BODY: 'The server error envelope was malformed.',
  },
};

/**
 * The localized message for any error code — the raw code is an audit/log
 * concern and is NEVER returned to the user; unknown codes fall back to the
 * same internal-error text the bot uses.
 */
export function errorMessage(locale: Locale, code: string): string {
  const known =
    (DOMAIN_MSG[locale] as Record<string, string>)[code] ??
    (WIRE_MSG[locale] as Record<string, string>)[code] ??
    (CLIENT_MSG[locale] as Record<string, string>)[code];
  return known ?? INTERNAL_FALLBACK[locale];
}
