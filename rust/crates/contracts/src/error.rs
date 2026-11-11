//! طبقه‌بندی خطای HTTP — پورت `server/src/errors.ts` + کدهای صریح `api.ts`/`guards.ts`.
//!
//! قاعده: کدهای دامنه به وضعیت‌های «صادقانه» نگاشت می‌شوند — ۴۰۴ (یافت‌نشده)،
//! ۴۲۲ (قاعدهٔ دامنه/اعتبارسنجی بدنه)، ۴۰۰ (اعتبارسنجی درخواست)، ۴۰۱ (احراز)،
//! ۴۲۹ (نرخ)، ۵۰۱/۵۰۲/۵۰۳ (زیرساخت). بدنهٔ همیشگی: `{ error: { code, message, meta? } }`.
//! کدها additive-only هستند — کلاینت‌های قدیمی نمی‌شکنند.

use dropagentx_domain::{DomainError, DomainErrorCode};
use crate::common::{ApiError, ErrorEnvelope, Meta, MetaValue};

// ── مجموعه‌های نگاشت — یک‌به‌یک با errors.ts ────────────────────────────────

/// کدهای ۴۰۴ — `NOT_FOUND_CODES`.
const NOT_FOUND_CODES: &[DomainErrorCode] = &[
    DomainErrorCode::EscrowNotFound,
    DomainErrorCode::UnknownAccount,
    DomainErrorCode::HitlNotFound,
    DomainErrorCode::ProductNotFound,
];

/// کدهای ۴۲۲ — `UNPROCESSABLE_CODES` (نقض قاعدهٔ دامنه، نه خطای سیستم).
const UNPROCESSABLE_CODES: &[DomainErrorCode] = &[
    DomainErrorCode::InvalidTransition,
    DomainErrorCode::RequiresHumanApproval,
    DomainErrorCode::AccountOverdraft,
    DomainErrorCode::UnbalancedTransaction,
    DomainErrorCode::NonIntegerAmount,
    DomainErrorCode::NegativeAmount,
    DomainErrorCode::AmountOutOfRange,
    DomainErrorCode::BpsOutOfRange,
    DomainErrorCode::MintCapExceeded,
    DomainErrorCode::EscrowAlreadyExists,
    DomainErrorCode::DuplicateAccount,
    DomainErrorCode::InvalidAccountKind,
    DomainErrorCode::TooFewPostings,
    DomainErrorCode::HitlNotRequired,
    DomainErrorCode::HitlAlreadyDecided,
    DomainErrorCode::HitlExpired,
];

/// وضعیت HTTP برای کد دامنه — همان منطق `toHttpError`.
#[must_use]
pub fn status_for_domain_code(code: DomainErrorCode) -> u16 {
    if NOT_FOUND_CODES.contains(&code) {
        404
    } else if UNPROCESSABLE_CODES.contains(&code) {
        422
    } else {
        500
    }
}

/// خطای برنامه‌نویسی/زیرساخت با وضعیت صریح — معادل `HttpFailure` TS.
/// سازنده‌های named برای «همهٔ» کدهای صریح سرور فعلی تعریف شده‌اند تا کد وایر
/// دقیق بماند و جایی رشتهٔ آزاد نوشته نشود.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpFailure {
    /// وضعیت HTTP
    pub status: u16,
    /// کد وایر
    pub code: String,
    /// پیام
    pub message: String,
    /// متادیتای اختیاری
    pub meta: Option<Meta>,
}

impl HttpFailure {
    /// سازندهٔ عمومی (داخل crate).
    pub(crate) fn new(status: u16, code: &str, message: impl Into<String>) -> Self {
        Self { status, code: code.into(), message: message.into(), meta: None }
    }

    /// خطای دامنه ناقص — `Command completed without escrow state.`
    #[must_use]
    pub fn internal_no_escrow() -> Self {
        Self::new(500, "INTERNAL", "Command completed without escrow state.")
    }

    /// خطای دامنه ناقص — `Command completed without a transaction.`
    #[must_use]
    pub fn internal_no_tx() -> Self {
        Self::new(500, "INTERNAL", "Command completed without a transaction.")
    }

    /// تصمیم REJECTED نباید تسویه اجرا کند.
    #[must_use]
    pub fn internal_rejected_settlement() -> Self {
        Self::new(500, "INTERNAL", "REJECTED decision must not execute a settlement.")
    }

    /// بکاپ VACUUM INTO شکست خورد.
    #[must_use]
    pub fn backup_failed(detail: impl std::fmt::Display) -> Self {
        Self::new(500, "BACKUP_FAILED", format!("VACUUM INTO failed — {detail}"))
    }

    /// پیشوند رزرو‌شدهٔ سیستم.
    #[must_use]
    pub fn reserved_id() -> Self {
        Self::new(422, "RESERVED_ID", "The mint:/treasury:/burn:/escrow: prefixes are system-reserved.")
    }

    /// حساب تکراری (نقض UNIQUE).
    #[must_use]
    pub fn duplicate_account(id: &str) -> Self {
        Self::new(422, "DUPLICATE_ACCOUNT", format!("Account {id} already exists."))
    }

    /// DB در دسترس نیست.
    #[must_use]
    pub fn db_down() -> Self {
        Self::new(503, "DB_DOWN", "The server cannot reach its database.")
    }

    /// بکاپ فقط برای SQLite فایل‌محور.
    #[must_use]
    pub fn backup_unsupported() -> Self {
        Self::new(503, "BACKUP_UNSUPPORTED", "Backups apply to file: (SQLite) databases only.")
    }

    /// تنظیمات هنوز پیاده نشده — ردِ صادقانه بهتر از no-op خاموش.
    #[must_use]
    pub fn not_implemented() -> Self {
        Self::new(
            501,
            "NOT_IMPLEMENTED",
            "Settings persistence is not wired yet — branding comes from BRAND_NAME / BRAND_TAGLINE env.",
        )
    }

    /// نقش staff بدون کلید معتبر.
    #[must_use]
    pub fn unauthorized_staff() -> Self {
        Self::new(401, "UNAUTHORIZED", "Staff role requires a valid X-KIA-Key.")
    }

    /// تماس‌گیرندهٔ عمومی AI بدون initData.
    #[must_use]
    pub fn ai_identity_required() -> Self {
        Self::new(
            401,
            "AI_IDENTITY_REQUIRED",
            "Public AI callers must send valid Telegram initData — the acting account is derived from it, never from the body.",
        )
    }

    /// سرور نمی‌تواند هویت عمومی AI را تأیید کند (توکن بات تنظیم نشده).
    #[must_use]
    pub fn ai_identity_disabled() -> Self {
        Self::new(
            503,
            "AI_IDENTITY_DISABLED",
            "This server cannot verify public AI callers (TELEGRAM_BOT_TOKEN unset) — send a valid X-KIA-Key.",
        )
    }

    /// سیکل AI کانفیگ نشده.
    #[must_use]
    pub fn ai_disabled() -> Self {
        Self::new(
            503,
            "AI_DISABLED",
            "The AI cycle is not configured — set OPENAI_BASE_URL / OPENAI_API_KEY / MODEL (ADR-006).",
        )
    }

    /// آپ‌استریم LLM شکست خورد — کد آپ‌استریم عیناً پاس می‌شود.
    #[must_use]
    pub fn ai_upstream(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self { status: 502, code: code.into(), message: message.into(), meta: None }
    }

    /// محدودیت نرخ — meta حتماً `limit` دارد.
    #[must_use]
    pub fn rate_limited(limit: i64) -> Self {
        let mut meta = Meta::new();
        meta.insert("limit".into(), MetaValue::Int(limit));
        Self {
            status: 429,
            code: "RATE_LIMITED".into(),
            message: format!("More than {limit} requests per minute."),
            meta: Some(meta),
        }
    }

    /// کلید ادمین سرور تنظیم نشده — fail-closed، هرگز open.
    #[must_use]
    pub fn admin_disabled() -> Self {
        Self::new(
            503,
            "ADMIN_DISABLED",
            "KIA_ADMIN_KEY is not configured on this server — admin endpoints are disabled.",
        )
    }

    /// هدر X-KIA-Key غایب.
    #[must_use]
    pub fn admin_key_required() -> Self {
        Self::new(401, "ADMIN_KEY_REQUIRED", "Send the admin key in the X-KIA-Key header.")
    }

    /// هدر X-KIA-Key نادرست.
    #[must_use]
    pub fn invalid_admin_key() -> Self {
        Self::new(401, "INVALID_ADMIN_KEY", "Invalid admin key.")
    }

    /// اعتبارسنجی درخواست — `ValidationFailure` TS → 400 INVALID_REQUEST.
    #[must_use]
    pub fn invalid_request(issues: usize) -> Self {
        let mut meta = Meta::new();
        meta.insert("issues".into(), MetaValue::Int(i64::try_from(issues).unwrap_or(i64::MAX)));
        Self {
            status: 400,
            code: "INVALID_REQUEST".into(),
            message: format!("Invalid request — {issues} issue(s)."),
            meta: Some(meta),
        }
    }

    /// تبدیل به بدنهٔ وایر.
    #[must_use]
    pub fn to_envelope(&self) -> ErrorEnvelope {
        ErrorEnvelope { error: self.to_api_error() }
    }

    /// تبدیل به `ApiError`.
    #[must_use]
    pub fn to_api_error(&self) -> ApiError {
        match &self.meta {
            Some(meta) => ApiError::with_meta(self.code.clone(), self.message.clone(), meta.clone()),
            None => ApiError::new(self.code.clone(), self.message.clone()),
        }
    }
}

/// خطای HTTP برآمده از دامنه — معادل `DomainFailure` TS.
#[derive(Debug, Clone, PartialEq)]
pub struct DomainFailure {
    /// خطای دامنهٔ اصلی
    pub error: DomainError,
}

impl DomainFailure {
    /// بسته‌بندی خطای دامنه.
    #[must_use]
    pub fn new(error: DomainError) -> Self {
        Self { error }
    }

    /// وضعیت صادقانه برای این کد.
    #[must_use]
    pub fn status(&self) -> u16 {
        status_for_domain_code(self.error.code)
    }

    /// بدنهٔ وایر — meta دامنه عیناً پاس می‌شود.
    #[must_use]
    pub fn to_envelope(&self) -> ErrorEnvelope {
        let mut meta: Meta = Meta::new();
        for (k, v) in &self.error.meta {
            // MetaValue دامنه → MetaValue وایر (هر دو مقدار string | number؛
            // Int از دامنه از مسیر Display به string می‌رود — همان خروجی JSON.stringify)
            let wire_value = match v {
                dropagentx_domain::MetaValue::Int(i) => MetaValue::Int(*i),
                dropagentx_domain::MetaValue::Text(t) => MetaValue::Text(t.clone()),
            };
            meta.insert(k.clone(), wire_value);
        }
        let api = if meta.is_empty() {
            ApiError::new(self.error.code.to_string(), self.error.message.clone())
        } else {
            ApiError::with_meta(self.error.code.to_string(), self.error.message.clone(), meta)
        };
        ErrorEnvelope { error: api }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dropagentx_domain::DomainErrorCode as Code;
    use serde_json::to_string;

    #[test]
    fn domain_codes_map_to_honest_statuses() {
        assert_eq!(status_for_domain_code(Code::EscrowNotFound), 404);
        assert_eq!(status_for_domain_code(Code::UnknownAccount), 404);
        assert_eq!(status_for_domain_code(Code::HitlNotFound), 404);
        assert_eq!(status_for_domain_code(Code::ProductNotFound), 404);
        assert_eq!(status_for_domain_code(Code::AccountOverdraft), 422);
        assert_eq!(status_for_domain_code(Code::RequiresHumanApproval), 422);
        assert_eq!(status_for_domain_code(Code::InvalidTransition), 422);
        assert_eq!(status_for_domain_code(Code::MintCapExceeded), 422);
        assert_eq!(status_for_domain_code(Code::HitlExpired), 422);
        // بقیه = 500 (خطای برنامه‌نویس)
        assert_eq!(status_for_domain_code(Code::BpsOutOfRange), 422);
        assert_eq!(status_for_domain_code(Code::TooFewPostings), 422);
    }

    #[test]
    fn full_taxonomy_covers_every_domain_code() {
        // هر ۲۰ کد دامنه باید یکی از سه وضعیت 404/422/500 بگیرد — هیچ کدی نگاشت‌نیافته نمی‌ماند
        for code in [
            Code::NonIntegerAmount, Code::NegativeAmount, Code::AmountOutOfRange,
            Code::BpsOutOfRange, Code::TooFewPostings, Code::UnbalancedTransaction,
            Code::UnknownAccount, Code::InvalidAccountKind, Code::DuplicateAccount,
            Code::AccountOverdraft, Code::InvalidTransition, Code::EscrowNotFound,
            Code::EscrowAlreadyExists, Code::RequiresHumanApproval, Code::MintCapExceeded,
            Code::HitlNotFound, Code::HitlNotRequired, Code::HitlAlreadyDecided,
            Code::HitlExpired, Code::ProductNotFound,
        ] {
            let s = status_for_domain_code(code);
            assert!([404, 422, 500].contains(&s), "{code:?} → {s}");
        }
    }

    #[test]
    fn domain_failure_envelope_carries_code_message_meta() {
        let err = DomainError::new(Code::AccountOverdraft, "no funds")
            .with_meta("needed", 90_i64);
        let env = DomainFailure::new(err).to_envelope();
        assert_eq!(to_string(&env).unwrap(), r#"{"error":{"code":"ACCOUNT_OVERDRAFT","message":"no funds","meta":{"needed":90}}}"#);
    }

    #[test]
    fn rate_limited_carries_limit_meta() {
        let f = HttpFailure::rate_limited(120);
        assert_eq!(f.status, 429);
        assert_eq!(f.code, "RATE_LIMITED");
        assert_eq!(to_string(&f.to_envelope()).unwrap(), r#"{"error":{"code":"RATE_LIMITED","message":"More than 120 requests per minute.","meta":{"limit":120}}}"#);
    }

    #[test]
    fn admin_disabled_is_503_fail_closed() {
        let f = HttpFailure::admin_disabled();
        assert_eq!(f.status, 503);
        assert_eq!(f.code, "ADMIN_DISABLED");
    }

    #[test]
    fn invalid_request_counts_issues() {
        let f = HttpFailure::invalid_request(3);
        assert_eq!(f.status, 400);
        assert!(f.message.contains('3'));
    }
}
