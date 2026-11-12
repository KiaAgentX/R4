//! قرارداد محیط orchestrator — پل «قلب خالص» به هر backend.
//!
//! طراحی ADR-009 §۸٫۵: `handle_update(env, msg)` بدون وابستگی به teloxide یا
//! SQL کار می‌کند؛ دو backend آن را اجرا می‌کنند:
//! * [`crate::memory::MemoryDomain`] — درون‌حافظه‌ای، آفلاین، برای تست‌ها
//! * `StoreEnv` (crate سرور) — روی `V3Store` واقعی با همان الگوی load →
//!   دامنهٔ خالص → persist داخل تراکنش فاز ۲
//!
//! کلید یکسان‌سازی همهٔ اقدام‌های بات از پیام تلگرام می‌آید:
//! `tg:{chat_id}:{message_id}` — retry وب‌هوک هرگز دوبار پول جابه‌جا نمی‌کند
//! (id واحد پیام فقط در چت یکتاست، پس chat_id هم قید می‌شود).
//!
//! همهٔ متدها `&self` هستند (جهت سهولت استفاده در هندلرهای async)؛ حالتِ
//! تغییرپذیر هر backend با درون‌موتوریشن خودش محافظت می‌شود.

use std::future::Future;

use dropagentx_domain::DomainErrorCode;

use crate::i18n::Locale;

/// شناسهٔ حساب شهروند از شناسهٔ تلگرام — قرارداد مشترک فاز ۲ (`user:tg:<id>`).
#[must_use]
pub fn user_account(tg_id: i64) -> String {
    format!("user:tg:{tg_id}")
}

/// نمای امانت روی سیم بات.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowItem {
    /// شناسهٔ امانت
    pub id: String,
    /// حساب خریدار
    pub buyer_id: String,
    /// حساب فروشنده
    pub seller_id: String,
    /// مبلغ به دیین
    pub amount: i64,
    /// وضعیت (`CREATED`، `FUNDED`، … — واژگان SCREAMING فاز ۳)
    pub status: String,
    /// ارجاع محصول اختیاری
    pub product_ref: Option<String>,
}

/// ردیف تاریخچه.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryItem {
    /// شناسهٔ تراکنش
    pub tx_id: String,
    /// مبلغ علامت‌دار
    pub amount: i64,
    /// یادداشت
    pub memo: Option<String>,
    /// زمان ISO
    pub at: String,
}

/// محصول فعال.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductItem {
    /// شناسهٔ محصول
    pub id: String,
    /// فروشنده
    pub seller_id: String,
    /// عنوان
    pub title: String,
    /// قیمت به دیین
    pub price_din: i64,
}

/// درخواست صف HITL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitlItem {
    /// شناسهٔ درخواست (دترمیینیستیک: `hitl:<escrow_id>`)
    pub id: String,
    /// امانت هدف
    pub escrow_id: String,
    /// مبلغ به دیین
    pub amount: i64,
    /// درخواست‌دهنده
    pub requester_id: String,
    /// وضعیت (`PENDING`، `APPROVED`، `REJECTED`، `EXPIRED`)
    pub status: String,
    /// زمان انقضا ISO (TTL ۲۴ ساعت)
    pub expires_at: String,
}

/// خروجی تصمیم HITL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitlOutcome {
    /// شناسهٔ درخواست
    pub id: String,
    /// وضعیت پس از تصمیم
    pub status: String,
    /// سهم فروشنده (فقط approve — دقیقاً ۹۰٪)
    pub seller_net: Option<i64>,
    /// مالیات خزانه (فقط approve — دقیقاً ۱۰٪)
    pub tax: Option<i64>,
    /// آیا تصمیم/تسویه یک replay بود؟
    pub replayed: bool,
}

/// خروجی گذار امانت.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionOutcome {
    /// امانت پس از گذار
    pub escrow: EscrowItem,
    /// آیا اجرا یک replay بود؟ (کلید یکسان‌سازی موجود)
    pub replayed: bool,
    /// اگر آزادسازی ≥ ۵۰٬۰۰۰ Đ باشد — درخواست HITL صف‌شده؛ امانت همان‌جا می‌ماند
    pub hitl_queued: Option<HitlItem>,
    /// شناسهٔ تراکنش پولی (fund/release)
    pub tx_id: Option<String>,
    /// سهم فروشنده (release)
    pub seller_net: Option<i64>,
    /// مالیات خزانه (release)
    pub tax: Option<i64>,
}

/// خلاصهٔ اقتصاد برای /economy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomyItem {
    /// پول در گردش
    pub circulating_supply: i64,
    /// تراز خزانه
    pub treasury_balance: i64,
    /// قفل در امانت
    pub escrow_locked: i64,
    /// Σ کیف شهروندان
    pub users_sum: i64,
    /// اسکروهای باز
    pub open_escrows: i64,
    /// آیا تراز آزمایشی صفر است؟
    pub balanced: bool,
}

/// گذارهای امانتی که بات اجرا می‌کند — نگاشت ۱:۱ به توابع دامنهٔ فاز ۱.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Transition {
    /// تأمین مالی (CREATED → FUNDED) — پول: خریدار → اسکرو
    Fund,
    /// تحویل (FUNDED → DELIVERED) — بدون حرکت پول
    Deliver,
    /// آزادسازی (DELIVERED/DISPUTED → RELEASED) — تسویهٔ ۹۰/۱۰
    Release,
    /// اختلاف (FUNDED/DELIVERED → DISPUTED)
    Dispute,
    /// لغو (CREATED → CANCELLED) — بدون حرکت پول
    Cancel,
}

impl Transition {
    /// نام action برای audit — همان واژگان هندلرهای فاز ۴.
    #[must_use]
    pub const fn action(self) -> &'static str {
        match self {
            Transition::Fund => "escrow.fund",
            Transition::Deliver => "escrow.deliver",
            Transition::Release => "escrow.release",
            Transition::Dispute => "escrow.dispute",
            Transition::Cancel => "escrow.cancel",
        }
    }

    /// وضعیت‌های مجاز ورود به گذار — از ماشین ۸ حالتهٔ فاز ۱.
    #[must_use]
    pub fn source_statuses(self) -> &'static [dropagentx_domain::EscrowStatus] {
        use dropagentx_domain::EscrowStatus as S;
        match self {
            Transition::Fund => &[S::Created],
            Transition::Deliver => &[S::Funded],
            Transition::Release => &[S::Delivered, S::Disputed],
            Transition::Dispute => &[S::Funded, S::Delivered],
            Transition::Cancel => &[S::Created],
        }
    }

    /// آیا این گذار پول جابه‌جا می‌کند؟ (کلید یکسان‌سازی فقط اینجا معنا دارد)
    #[must_use]
    pub const fn moves_money(self) -> bool {
        matches!(self, Transition::Fund | Transition::Release)
    }

    /// نقش مجاز — ADR-009: «خریدار fund/release/dispute/cancel، فروشنده deliver».
    #[must_use]
    pub const fn buyer_only(self) -> bool {
        !matches!(self, Transition::Deliver)
    }
}

/// خطای بات — ترکیب کدهای دامنهٔ وایر + گاردهای authz خود بات.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotError {
    /// خطای دامنهٔ خالص — نگاشت کامل در i18n
    Domain(DomainErrorCode),
    /// فرمان ادمین توسط غیرادمین — fail-closed
    NotAdmin,
    /// عمل فقط برای خریدارِ امانت
    BuyerOnly,
    /// عمل فقط برای فروشندهٔ امانت
    SellerOnly,
    /// فراخوان هیچ نسبتی با امانت ندارد
    NotParty,
    /// امانت بازِ متناسب پیدا نشد (auto-resolve خالی)
    NoEligibleEscrow,
    /// امانت هدف وجود ندارد
    EscrowNotFound,
    /// آرگومان بد — متن استفادهٔ درست
    BadArgs(&'static str),
    /// خطای زیرساخت backend (SQL و…) — پیام عمومی؛ جزئیات فقط در لاگ
    Backend,
    /// متن ناشناخته (نباید به اینجا برسد — orchestrator منو می‌دهد)
    UnknownCommand(String),
}

impl BotError {
    /// کد وایر (فاز ۳) — برای audit/لاگ؛ کاربر پیام محلی می‌گیرد.
    #[must_use]
    pub fn wire_code(&self) -> String {
        match self {
            BotError::Domain(code) => code.as_str().to_owned(),
            BotError::NotAdmin => "UNAUTHORIZED".into(),
            BotError::BuyerOnly | BotError::SellerOnly | BotError::NotParty => {
                "UNAUTHORIZED".into()
            }
            BotError::NoEligibleEscrow | BotError::EscrowNotFound => "ESCROW_NOT_FOUND".into(),
            BotError::BadArgs(_) => "INVALID_REQUEST".into(),
            BotError::Backend => "INTERNAL".into(),
            BotError::UnknownCommand(_) => "UNKNOWN_COMMAND".into(),
        }
    }
}

/// محیط orchestrator — هر backend وضعیت کشور را پشت همین قرارداد می‌دهد.
pub trait Env {
    /// لوکِیل کاربر — hint زبان کلاینت وقتی انتخاب صریحی نیست.
    fn locale(&self, user: i64, hint: Option<&str>) -> Locale;

    /// نشاندن انتخاب صریح زبان (`/lang`).
    fn set_locale(&self, user: i64, locale: Locale);

    /// گیت ادمین — fail-closed: فهرست خالی یعنی هیچ‌کس ادمین نیست.
    fn is_admin(&self, user: i64) -> bool;

    /// موجودی کیف شهروند.
    ///
    /// # Errors
    /// [`BotError::Domain`] حساب ناشناخته.
    fn balance(&self, user: i64) -> impl Future<Output = Result<i64, BotError>> + Send;

    /// تاریخچهٔ کیف (تازه‌ترین‌ها اول).
    ///
    /// # Errors
    /// [`BotError::Domain`] حساب ناشناخته.
    fn history(
        &self,
        user: i64,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<HistoryItem>, BotError>> + Send;

    /// محصولات فعال.
    ///
    /// # Errors
    /// خطای زیرساخت به‌شکل [`BotError::Domain`].
    fn products(
        &self,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<ProductItem>, BotError>> + Send;

    /// خواندن یک امانت — `None` یعنی وجود ندارد.
    ///
    /// # Errors
    /// خطای زیرساخت.
    fn escrow(
        &self,
        escrow_id: &str,
    ) -> impl Future<Output = Result<Option<EscrowItem>, BotError>> + Send;

    /// امانت‌های بازِ یک شهروند (خریدار یا فروشنده).
    ///
    /// # Errors
    /// خطای زیرساخت.
    fn open_escrows_of(
        &self,
        user: i64,
    ) -> impl Future<Output = Result<Vec<EscrowItem>, BotError>> + Send;

    /// خرید → ساخت امانت با شناسهٔ دترمیینیستیک `esc:c{chat}m{msg}`؛
    /// retry همان پیام همان امانت را برمی‌گرداند (replay=true).
    ///
    /// # Errors
    /// [`BotError::Domain`] حساب ناشناخته/مبلغ نامعتبر.
    fn buy(
        &mut self,
        user: i64,
        chat_id: i64,
        message_id: i64,
        seller: &str,
        amount: i64,
        product_ref: Option<&str>,
    ) -> impl Future<Output = Result<(EscrowItem, bool), BotError>> + Send;

    /// گذار امانت با کلید یکسان‌سازی از پیام + گارد نقش fail-closed.
    ///
    /// # Errors
    /// [`BotError::BuyerOnly`]/[`BotError::SellerOnly`]، [`BotError::Domain`].
    fn transition(
        &mut self,
        user: i64,
        chat_id: i64,
        message_id: i64,
        escrow_id: &str,
        verb: Transition,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<TransitionOutcome, BotError>> + Send;

    /// صف HITL (PENDING).
    ///
    /// # Errors
    /// خطای زیرساخت.
    fn hitl_queue(&self) -> impl Future<Output = Result<Vec<HitlItem>, BotError>> + Send;

    /// تأیید HITL → تسویهٔ اتمیک ۹۰/۱۰ در همان تراکنش؛ کلید قطعی
    /// `hitl:<id>:approve`؛ تصمیم دوم → [`BotError::Domain`] تایپ‌شده.
    ///
    /// # Errors
    /// [`BotError::Domain`] `HITL_ALREADY_DECIDED`/`HITL_EXPIRED`/تسویه.
    fn hitl_approve(
        &mut self,
        admin: i64,
        hitl_id: &str,
        note: Option<&str>,
    ) -> impl Future<Output = Result<HitlOutcome, BotError>> + Send;

    /// رد HITL — هیچ پولی جابه‌جا نمی‌شود.
    ///
    /// # Errors
    /// [`BotError::Domain`] `HITL_ALREADY_DECIDED`.
    fn hitl_reject(
        &mut self,
        admin: i64,
        hitl_id: &str,
        note: Option<&str>,
    ) -> impl Future<Output = Result<HitlOutcome, BotError>> + Send;

    /// خلاصهٔ اقتصاد.
    ///
    /// # Errors
    /// خطای زیرساخت.
    fn economy(&self) -> impl Future<Output = Result<EconomyItem, BotError>> + Send;

    /// انتهای audit log.
    ///
    /// # Errors
    /// خطای زیرساخت.
    fn audit_tail(
        &self,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<String>, BotError>> + Send;
}
