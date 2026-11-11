//! Typed errors + Result pattern — مرز دامنه fail-closed.
//!
//! قاعده: هیچ panic از مرز دامنه عبور نمی‌کند؛ هر عملیات
//! `Result<T, DomainError>` برمی‌گرداند. panic فقط خطای برنامه‌نویس است
//! (داخل تست‌ها مجاز است). یک‌به‌یک با `errors.ts` نسخه TypeScript.

use std::collections::BTreeMap;
use std::fmt;

/// ۲۰ کد خطای دامنه — یک‌به‌یک با `DomainErrorCode` نسخه TS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DomainErrorCode {
    /// مقدار پول صحیح نیست (در TS: عدد غیر صحیح یا خارج محدوده امن)
    NonIntegerAmount,
    /// مقدار پول منفی است
    NegativeAmount,
    /// مقدار از محدوده امن صحیح (2^53−1) بیرون می‌زند
    AmountOutOfRange,
    /// نقاط پایه (bps) خارج از [0, 10000]
    BpsOutOfRange,
    /// تراکنش کمتر از دو ردیف دارد
    TooFewPostings,
    /// مجموع ردیف‌های تراکنش صفر نیست
    UnbalancedTransaction,
    /// حساب ناشناخته
    UnknownAccount,
    /// نوع حساب برای این عملیات مجاز نیست
    InvalidAccountKind,
    /// حساب تکراری
    DuplicateAccount,
    /// نقض قاعده جهت حساب (کسر موجودی/ضرب mint و…)
    AccountOverdraft,
    /// گذار غیرمجاز ماشین وضعیت
    InvalidTransition,
    /// امانت یافت نشد
    EscrowNotFound,
    /// امانت تکراری
    EscrowAlreadyExists,
    /// عملیات ≥ ۵۰,۰۰۰ Đ بدون تأیید انسانی
    RequiresHumanApproval,
    /// سقف ضرب سالانه نقض شد
    MintCapExceeded,
    /// رکورد HITL یافت نشد
    HitlNotFound,
    /// این عملیات نیازی به HITL ندارد
    HitlNotRequired,
    /// این رکورد HITL قبلاً تصمیم گرفته شده
    HitlAlreadyDecided,
    /// رکورد HITL منقضی شده
    HitlExpired,
    /// محصول یافت نشد
    ProductNotFound,
}

impl DomainErrorCode {
    /// رشته کانانیکی — یکسان با نسخه TS (برای لاگ، API و dual-run).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NonIntegerAmount => "NON_INTEGER_AMOUNT",
            Self::NegativeAmount => "NEGATIVE_AMOUNT",
            Self::AmountOutOfRange => "AMOUNT_OUT_OF_RANGE",
            Self::BpsOutOfRange => "BPS_OUT_OF_RANGE",
            Self::TooFewPostings => "TOO_FEW_POSTINGS",
            Self::UnbalancedTransaction => "UNBALANCED_TRANSACTION",
            Self::UnknownAccount => "UNKNOWN_ACCOUNT",
            Self::InvalidAccountKind => "INVALID_ACCOUNT_KIND",
            Self::DuplicateAccount => "DUPLICATE_ACCOUNT",
            Self::AccountOverdraft => "ACCOUNT_OVERDRAFT",
            Self::InvalidTransition => "INVALID_TRANSITION",
            Self::EscrowNotFound => "ESCROW_NOT_FOUND",
            Self::EscrowAlreadyExists => "ESCROW_ALREADY_EXISTS",
            Self::RequiresHumanApproval => "REQUIRES_HUMAN_APPROVAL",
            Self::MintCapExceeded => "MINT_CAP_EXCEEDED",
            Self::HitlNotFound => "HITL_NOT_FOUND",
            Self::HitlNotRequired => "HITL_NOT_REQUIRED",
            Self::HitlAlreadyDecided => "HITL_ALREADY_DECIDED",
            Self::HitlExpired => "HITL_EXPIRED",
            Self::ProductNotFound => "PRODUCT_NOT_FOUND",
        }
    }
}

impl fmt::Display for DomainErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// مقدار متادیتا — معادل `string | number` در TS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaValue {
    /// مقدار متنی
    Text(String),
    /// مقدار صحیح
    Int(i64),
}

impl fmt::Display for MetaValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(s) => f.write_str(s),
            Self::Int(i) => write!(f, "{i}"),
        }
    }
}

/// متادیتای خطا — کلیدها مرتب (BTreeMap) تا خروجی dual-run دترمینیستیک باشد.
pub type Meta = BTreeMap<String, MetaValue>;

/// خطای دامنه: کد تایپ‌شده + پیام + متادیتای اختیاری.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainError {
    /// کد خطا
    pub code: DomainErrorCode,
    /// پیام توضیحی (انگلیسی/فارسی — برای لاگ و UI)
    pub message: String,
    /// متادیتای ساختاریافته (مثل `sum`، `accountId`، `newBalance`)
    pub meta: Meta,
}

impl DomainError {
    /// خطای ساده بدون متادیتا.
    #[must_use]
    pub fn new(code: DomainErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), meta: Meta::new() }
    }

    /// افزودن یک جفت متادیتا (builder).
    #[must_use]
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<MetaValue>) -> Self {
        self.meta.insert(key.into(), value.into());
        self
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DomainError {}

impl From<&str> for MetaValue {
    fn from(s: &str) -> Self {
        Self::Text(s.to_owned())
    }
}

impl From<String> for MetaValue {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<i64> for MetaValue {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

/// نتیجه عملیات دامنه — معادل `Result<T>` سفارشی نسخه TS.
pub type DomainResult<T> = Result<T, DomainError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twenty_codes_match_ts_strings() {
        assert_eq!(DomainErrorCode::NonIntegerAmount.as_str(), "NON_INTEGER_AMOUNT");
        assert_eq!(DomainErrorCode::ProductNotFound.as_str(), "PRODUCT_NOT_FOUND");
        assert_eq!(DomainErrorCode::RequiresHumanApproval.to_string(), "REQUIRES_HUMAN_APPROVAL");
    }

    #[test]
    fn error_display_and_meta_builder() {
        let e = DomainError::new(DomainErrorCode::UnbalancedTransaction, "bad sum")
            .with_meta("sum", 15_i64)
            .with_meta("note", "فارسی");
        assert_eq!(e.to_string(), "UNBALANCED_TRANSACTION: bad sum");
        assert_eq!(e.meta.get("sum"), Some(&MetaValue::Int(15)));
        assert_eq!(e.meta.get("note"), Some(&MetaValue::Text("فارسی".into())));
    }
}
