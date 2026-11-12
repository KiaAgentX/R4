/**
 * Static nation content for the webapp — charter articles, bot commands, FAQ.
 *
 * The charter mirrors `crates/domain/src/charter.rs → CHARTER_ARTICLES`
 * (10 articles, Persian VERBATIM from the domain source) with an honest
 * English rendering beside it. Same rule as the bot: one nation, one voice —
 * the webapp never paraphrases the charter.
 */

import type { Locale } from '@agent-one/i18n/webapp';

export interface CharterArticle {
  article: number;
  faTitle: string;
  faValue: string;
  enTitle: string;
  enValue: string;
}

/** 1:1 with CHARTER_ARTICLES — fa text copied verbatim from charter.rs. */
export const CHARTER_ARTICLES: readonly CharterArticle[] = [
  {
    article: 1,
    faTitle: 'واحد پول',
    faValue: 'دینار دیجیتال (Đ / DNX) — عدد صحیح، بدون اعشار',
    enTitle: 'Currency unit',
    enValue: 'Digital Din (Đ / DNX) — an integer, never fractional',
  },
  {
    article: 2,
    faTitle: 'مالیات بازار',
    faValue: '۱۰٪ هر تسویه اسکرو به خزانه می‌رود',
    enTitle: 'Market tax',
    enValue: '10% of every escrow settlement goes to the treasury',
  },
  {
    article: 3,
    faTitle: 'سقف ضرب',
    faValue: 'حداکثر ۱٬۰۰۰٬۰۰۰ Đ ضرب در سال',
    enTitle: 'Mint cap',
    enValue: 'At most 1,000,000 Đ may be minted per year',
  },
  {
    article: 4,
    faTitle: 'حفاظت عرضه',
    faValue: 'پول فقط از حساب mint صادر و فقط به حساب burn می‌رود',
    enTitle: 'Supply protection',
    enValue: 'Money only leaves the mint account and only enters the burn account',
  },
  {
    article: 5,
    faTitle: 'پایایی دفتر',
    faValue: 'هر تراکنش دوطرفه است و مجموع تراز همه حساب‌ها همیشه صفر است',
    enTitle: 'Ledger integrity',
    enValue: 'Every transaction is two-sided and the sum of all balances is always zero',
  },
  {
    article: 6,
    faTitle: 'شفافیت',
    faValue: 'هر تراکنش شناسه یکتا و memo دارد',
    enTitle: 'Transparency',
    enValue: 'Every transaction carries a unique id and a memo',
  },
  {
    article: 7,
    faTitle: 'آستانه HITL',
    faValue: 'عملیات ≥ ۵۰٬۰۰۰ Đ نیازمند تایید انسانی است',
    enTitle: 'HITL threshold',
    enValue: 'Operations ≥ 50,000 Đ require human approval',
  },
  {
    article: 8,
    faTitle: 'اسکرو اجباری',
    faValue: 'خرید از مارکت فقط از مسیر اسکرو انجام می‌شود',
    enTitle: 'Mandatory escrow',
    enValue: 'Market purchases only run through escrow',
  },
  {
    article: 9,
    faTitle: 'بازگشت وجه',
    faValue: 'اختلاف اسکرو به نفع خریدار بازگشت می‌شود مگر داوری خلاف آن را تعیین کند',
    enTitle: 'Refunds',
    enValue: 'An escrow dispute refunds the buyer unless arbitration decides otherwise',
  },
  {
    article: 10,
    faTitle: 'تعدادیل میراث',
    faValue: 'گپ تاریخی −۶۳ Đ با یک ردیف تعدیل شفاف ثبت شده است',
    enTitle: 'Legacy adjustment',
    enValue: 'The historical −63 Đ gap is recorded as one transparent adjustment row',
  },
];

export function charterTitle(article: CharterArticle, locale: Locale): string {
  return locale === 'fa' ? article.faTitle : article.enTitle;
}

export function charterValue(article: CharterArticle, locale: Locale): string {
  return locale === 'fa' ? article.faValue : article.enValue;
}

/** The bot's 14 user commands — mirrors `crates/bot/src/cmd.rs`. */
export const BOT_COMMANDS: readonly { cmd: string; fa: string; en: string }[] = [
  { cmd: '/start', fa: 'خوش‌آمد', en: 'Welcome' },
  { cmd: '/help', fa: 'راهنما', en: 'Help' },
  { cmd: '/menu', fa: 'منو', en: 'Menu' },
  { cmd: '/balance', fa: 'موجودی کیف', en: 'Wallet balance' },
  { cmd: '/history', fa: 'تاریخچهٔ تراکنش‌ها', en: 'Transaction history' },
  { cmd: '/products', fa: 'فهرست محصولات', en: 'Product list' },
  { cmd: '/buy', fa: 'خرید با امانت', en: 'Buy with escrow' },
  { cmd: '/status', fa: 'وضعیت آخرین امانت', en: 'Latest escrow status' },
  { cmd: '/fund', fa: 'تأمین مالی امانت', en: 'Fund an escrow' },
  { cmd: '/deliver', fa: 'ثبت تحویل (فروشنده)', en: 'Record delivery (seller)' },
  { cmd: '/release', fa: 'آزادسازی امانت', en: 'Release the escrow' },
  { cmd: '/dispute', fa: 'اعلام اختلاف', en: 'File a dispute' },
  { cmd: '/cancel', fa: 'لغو امانت', en: 'Cancel the escrow' },
  { cmd: '/lang', fa: 'تغییر زبان', en: 'Switch language' },
];

/** The 5 admin commands — shown only in the help page with a staff key. */
export const BOT_ADMIN_COMMANDS: readonly { cmd: string; fa: string; en: string }[] = [
  { cmd: '/queue', fa: 'صف تأییدهای HITL', en: 'The HITL approval queue' },
  { cmd: '/approve', fa: 'تأیید درخواست HITL', en: 'Approve a HITL request' },
  { cmd: '/reject', fa: 'رد درخواست HITL', en: 'Reject a HITL request' },
  { cmd: '/economy', fa: 'اقتصاد کشور', en: 'Nation economy' },
  { cmd: '/audit', fa: 'ممیزی اخیر', en: 'Recent audit' },
];

export function botCommandHelp(c: { fa: string; en: string }, locale: Locale): string {
  return locale === 'fa' ? c.fa : c.en;
}

export interface FaqItem {
  q: { fa: string; en: string };
  a: { fa: string; en: string };
}

export const FAQ: readonly FaqItem[] = [
  {
    q: { fa: 'پول من کی قفل می‌شود و کی آزاد می‌شود؟', en: 'When is my money locked and released?' },
    a: {
      fa: 'با تأمین مالی، مبلغ در حساب امانت قفل می‌شود؛ با آزادسازی، ۹۰٪ به فروشنده و ۱۰٪ مالیات به خزانه می‌رود (مادهٔ ۲ و ۸).',
      en: 'Funding locks the amount in the escrow account; release pays 90% to the seller and 10% tax to the treasury (art. 2 and 8).',
    },
  },
  {
    q: { fa: 'چرا برای مبالغ بزرگ تأیید ادمین لازم است؟', en: 'Why do large amounts need admin approval?' },
    a: {
      fa: 'هر عملیات ≥ ۵۰٬۰۰۰ Đ از صف HITL (مادهٔ ۷) می‌گذرد تا انسان تصمیم بگیرد — این قانون منشور است، نه محدودیت اپ.',
      en: 'Every operation ≥ 50,000 Đ passes the HITL queue (art. 7) so a human decides — that is charter law, not an app limitation.',
    },
  },
  {
    q: { fa: 'اگر فروشنده تحویل ندهد چه می‌شود؟', en: 'What if the seller never delivers?' },
    a: {
      fa: 'می‌توانی اختلاف ثبت کنی (مادهٔ ۹)؛ بازگشت پیش‌فرض به نفع خریدار است مگر داوری خلاف آن را تعیین کند.',
      en: 'File a dispute (art. 9); refunds default in the buyer\u2019s favor unless arbitration decides otherwise.',
    },
  },
  {
    q: { fa: 'چرا برخی دکمه‌ها من را به ربات یا دستیار می‌فرستند؟', en: 'Why do some buttons send me to the bot or the assistant?' },
    a: {
      fa: 'روی قرارداد منجمد، گذارهای پولی شهروندان از ربات (فرمان‌های /fund و ...) و دستیار هویت‌دار اجرا می‌شود؛ این اپ خواندنی و راهنماست — صداقت، نه جعل دکمه.',
      en: 'On the frozen contract, citizen money transitions run from the bot (/fund …) and the identity-gated assistant; this app reads and guides — honesty over fake buttons.',
    },
  },
  {
    q: { fa: 'تراکنش‌های من کجا ثبت می‌شوند؟', en: 'Where are my transactions recorded?' },
    a: {
      fa: 'روی دفتر دوطرفهٔ کشور؛ هر تراکنش شناسهٔ یکتا و memo دارد (مادهٔ ۵ و ۶) و موجودی‌ها فقط از دفتر خوانده می‌شود.',
      en: 'On the nation\u2019s two-sided ledger; every transaction has a unique id and memo (art. 5 and 6) and balances are read only from the ledger.',
    },
  },
];
