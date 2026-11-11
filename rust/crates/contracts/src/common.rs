//! اولیه‌های مشترک وایر (ADR-005) — پورت یک‌به‌یک `common.ts`.
//!
//! پول روی سیم عدد صحیحِ تعداد دیین است (ADR-002) — هرگز float، هرگز منفی.
//! کدهای خطا عمداً `string` شل هستند: دامنه فقط کد «اضافه می‌کند» و کلاینت‌های
//! قدیمی نمی‌شکنند (قاعدهٔ additive-only).
//!
//! نگاشت zod → Rust:
//! * `z.number().int().min(0).max(2^53−1)` → [`Din`] (custom serde + schema دستی)
//! * `idSchema` → [`Id`] (الگو + طول، اعتبارسنجی در Deserialize)
//! * `idempotencyKeySchema` → [`IdempotencyKey`]
//! * `z.string().datetime({offset:true})` → [`IsoDateTime`]
//! * `.nullable()` → `Option<T>` **بدون** `skip_serializing_if` (null صریح روی سیم)
//! * `.optional()` → `Option<T>` با `skip_serializing_if` (کلید غایب روی سیم)

use std::collections::BTreeMap;

use serde::de::{Deserializer, Error as DeError, Unexpected};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use utoipa::openapi::schema::{KnownFormat, Object, Schema, SchemaFormat, Type};
use utoipa::openapi::RefOr;
use utoipa::{PartialSchema, ToSchema};

/// بیشینهٔ صحیح امن جاوااسکریپت — `Number.MAX_SAFE_INTEGER` (ADR-002).
pub const DIN_MAX: i64 = 9_007_199_254_740_991;

/// کمینهٔ صحیح امن علامت‌دار — ردیف‌های دفتر می‌توانند منفی باشند.
pub const DIN_MIN_SIGNED: i64 = -9_007_199_254_740_991;

// ── DNX — پول روی سیم ────────────────────────────────────────────────────────

/// دیین روی وایر — newtype روی `i64` با همان قیود `DNX` در zod:
/// صحیح، ≥ 0، ≤ 2^53−1. Deserialize نقض قیود را **رد می‌کند** (fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Din(i64);

impl Din {
    /// سازندهٔ بررسی‌شده — مقدار خارج `[0, 2^53−1]` را نمی‌پذیرد.
    #[must_use]
    pub const fn new(raw: i64) -> Option<Self> {
        if raw >= 0 && raw <= DIN_MAX {
            Some(Self(raw))
        } else {
            None
        }
    }

    /// مقدار خام — برای محاسبات دامنه (تبدیل به `domain::Din` در فاز ۴).
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for Din {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} Đ", self.0)
    }
}

impl Serialize for Din {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(self.0)
    }
}

impl<'de> Deserialize<'de> for Din {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // JSON یک عدد صحیح است؛ float دریافتی (مثل 12.5) باید رد شود.
        let raw = i64::deserialize(deserializer)?;
        Self::new(raw).ok_or_else(|| {
            DeError::invalid_value(Unexpected::Signed(raw), &"DNX in [0, 9007199254740991]")
        })
    }
}

impl PartialSchema for Din {
    fn schema() -> RefOr<Schema> {
        Object::builder()
            .schema_type(Type::Integer)
            .description(Some("DNX — integer dinar; never a float, never negative"))
            .minimum(Some(0_i64))
            .maximum(Some(DIN_MAX))
            .build()
            .into()
    }
}

impl ToSchema for Din {}

/// صحیح علامت‌دار امن — ردیف‌های دفتر (ممکن است منفی باشند) و شمارنده‌های دامنهٔ کامل.
pub const SAFE_INT_SCHEMA_MIN: i64 = DIN_MIN_SIGNED;
/// بیشینهٔ متناظر.
pub const SAFE_INT_SCHEMA_MAX: i64 = DIN_MAX;

// ── شناسه‌ها ────────────────────────────────────────────────────────────────

/// الگوی `idSchema` — فقط حروف/رقم/`:`/`_`/`.`/`-`.
pub const ID_PATTERN: &str = "^[A-Za-z0-9:_.-]+$";

/// شناسهٔ حساب/امانت/HITL — ۱ تا ۱۲۸ کاراکتر با الگوی `^[A-Za-z0-9:_.-]+$`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(String);

impl Id {
    /// سازندهٔ بررسی‌شده — همان قواعد zod (طول و الگو).
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Option<Self> {
        let s = raw.into();
        let ok = (1..=128).contains(&s.chars().count())
            && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '.' | '-'));
        ok.then_some(Self(s))
    }

    /// مقدار خام.
    #[must_use]
    pub fn get(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Id {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::new(s.clone()).ok_or_else(|| DeError::invalid_value(Unexpected::Str(&s), &"a valid wire id"))
    }
}

impl PartialSchema for Id {
    fn schema() -> RefOr<Schema> {
        Object::builder()
            .schema_type(Type::String)
            .min_length(Some(1))
            .max_length(Some(128))
            .pattern(Some(ID_PATTERN))
            .build()
            .into()
    }
}

impl ToSchema for Id {}

/// کلید یکسان‌سازی — ۸ تا ۱۲۸ کاراکتر؛ تکرار = replay نه پرداخت دوباره.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// سازندهٔ بررسی‌شده — طول `[8, 128]`.
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Option<Self> {
        let s = raw.into();
        let len = s.chars().count();
        ((8..=128).contains(&len)).then_some(Self(s))
    }

    /// مقدار خام.
    #[must_use]
    pub fn get(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for IdempotencyKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for IdempotencyKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::new(s.clone())
            .ok_or_else(|| DeError::invalid_value(Unexpected::Str(&s), &"an idempotency key of 8..=128 chars"))
    }
}

impl PartialSchema for IdempotencyKey {
    fn schema() -> RefOr<Schema> {
        Object::builder()
            .schema_type(Type::String)
            .min_length(Some(8))
            .max_length(Some(128))
            .build()
            .into()
    }
}

impl ToSchema for IdempotencyKey {}

// ── زمان ISO-8601 با آفست اجباری ────────────────────────────────────────────

/// `z.string().datetime({offset:true})` — RFC 3339 با آفست (Z یا ±HH:MM) الزامی.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IsoDateTime(String);

impl IsoDateTime {
    /// سازندهٔ بررسی‌شده — فقط رشتهٔ RFC 3339 دارای آفست.
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Option<Self> {
        let s = raw.into();
        chrono::DateTime::parse_from_rfc3339(&s).ok().map(|_| Self(s))
    }

    /// مقدار خام.
    #[must_use]
    pub fn get(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for IsoDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for IsoDateTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for IsoDateTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::new(s.clone()).ok_or_else(|| {
            DeError::invalid_value(Unexpected::Str(&s), &"an RFC 3339 timestamp with an explicit offset")
        })
    }
}

impl PartialSchema for IsoDateTime {
    fn schema() -> RefOr<Schema> {
        Object::builder()
            .schema_type(Type::String)
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::DateTime)))
            .build()
            .into()
    }
}

impl ToSchema for IsoDateTime {}

// ── متادیتا و پاکت خطا ──────────────────────────────────────────────────────

/// مقدار متادیتای خطا روی وایر — `string | number` (یونتگ، مثل TS).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum MetaValue {
    /// مقدار متنی
    Text(String),
    /// مقدار صحیح
    Int(i64),
}

/// متادیتای خطا — `Record<string, string | number>`؛ مرتب تا خروجی دترمینیستیک بماند.
pub type Meta = BTreeMap<String, MetaValue>;

/// بدنهٔ خطای وایر — `{ error: { code, message, meta? } }` (قابل انتقال بین همهٔ کلاینت‌ها).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEnvelope {
    /// آبجکت خطا
    pub error: ApiError,
}

/// خطای API — کد + پیام + متادیتای اختیاری (`.optional()` → غایب روی سیم).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    /// کد خطای وایر (additive-only)
    pub code: String,
    /// پیام انسانی
    pub message: String,
    /// زمینهٔ ساختاریافتهٔ اختیاری
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

impl ApiError {
    /// خطای ساده بدون متادیتا.
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: message.into(), meta: None }
    }

    /// خطا با متادیتا.
    #[must_use]
    pub fn with_meta(code: impl Into<String>, message: impl Into<String>, meta: Meta) -> Self {
        Self { code: code.into(), message: message.into(), meta: Some(meta) }
    }
}

// ── وضعیت امانت ─────────────────────────────────────────────────────────────

/// ۸ وضعیت ماشین امانت (ADR-004) — دقیقاً همان رشته‌های TS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EscrowStatus {
    /// ساخته‌شده — پول جابه‌جا نشده
    Created,
    /// تأمین‌شده — پول قفل شده
    Funded,
    /// تحویل‌شده — پول هنوز قفل است
    Delivered,
    /// تسویه‌شده — ۹۰٪ فروشنده + ۱۰٪ خزانه
    Released,
    /// در اختلاف — قفل تا داوری
    Disputed,
    /// بازپرداخت‌شده — بدون مالیات
    Refunded,
    /// لغو‌شده — هیچ پولی حرکت نکرده
    Cancelled,
    /// منقضی‌شده — sweep خودکار TTL
    Expired,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn din_accepts_safe_range_and_rejects_floats_and_negatives() {
        assert_eq!(from_str::<Din>("0").unwrap(), Din::new(0).unwrap());
        assert_eq!(from_str::<Din>("9007199254740991").unwrap(), Din::new(DIN_MAX).unwrap());
        assert_eq!(to_string(&Din::new(187_672).unwrap()).unwrap(), "187672");
        assert!(from_str::<Din>("-1").is_err());
        assert!(from_str::<Din>("9007199254740992").is_err());
        // float روی وایر — هرگز
        assert!(from_str::<Din>("12.5").is_err());
    }

    #[test]
    fn id_validates_pattern_and_length() {
        assert!(Id::new("user:tg:123").is_some());
        assert!(Id::new("a.b_c-d").is_some());
        assert!(Id::new("").is_none());
        assert!(Id::new("has space").is_none());
        assert!(Id::new("a".repeat(129)).is_none());
        assert!(Id::new("a".repeat(128)).is_some());
    }

    #[test]
    fn idempotency_key_enforces_min_8() {
        assert!(IdempotencyKey::new("mint-ali-1").is_some());
        assert!(IdempotencyKey::new("esc-1").is_none());
    }

    #[test]
    fn iso_datetime_requires_offset() {
        assert!(IsoDateTime::new("2026-02-01T00:00:00Z").is_some());
        assert!(IsoDateTime::new("2026-02-01T00:00:00+03:30").is_some());
        assert!(IsoDateTime::new("2026-02-01T00:00:00").is_none()); // بدون آفست — مثل zod
        assert!(IsoDateTime::new("not-a-date").is_none());
    }

    #[test]
    fn escrow_status_wire_names_match_ts() {
        assert_eq!(to_string(&EscrowStatus::Created).unwrap(), "\"CREATED\"");
        assert_eq!(to_string(&EscrowStatus::Funded).unwrap(), "\"FUNDED\"");
        assert_eq!(to_string(&EscrowStatus::Delivered).unwrap(), "\"DELIVERED\"");
        assert_eq!(to_string(&EscrowStatus::Released).unwrap(), "\"RELEASED\"");
        assert_eq!(to_string(&EscrowStatus::Disputed).unwrap(), "\"DISPUTED\"");
        assert_eq!(to_string(&EscrowStatus::Refunded).unwrap(), "\"REFUNDED\"");
        assert_eq!(to_string(&EscrowStatus::Cancelled).unwrap(), "\"CANCELLED\"");
        assert_eq!(to_string(&EscrowStatus::Expired).unwrap(), "\"EXPIRED\"");
        assert!(from_str::<EscrowStatus>("\"FLOATING\"").is_err());
    }

    #[test]
    fn error_envelope_roundtrip() {
        let json = r#"{"error":{"code":"ACCOUNT_OVERDRAFT","message":"no funds","meta":{"needed":90}}}"#;
        let env: ErrorEnvelope = from_str(json).unwrap();
        assert_eq!(env.error.code, "ACCOUNT_OVERDRAFT");
        assert_eq!(env.error.meta, Some(Meta::from([("needed".into(), MetaValue::Int(90))])));
        let bare: ErrorEnvelope =
            from_str(r#"{"error":{"code":"ESCROW_NOT_FOUND","message":"missing"}}"#).unwrap();
        assert_eq!(bare.error.meta, None);
        assert_eq!(to_string(&bare).unwrap(), r#"{"error":{"code":"ESCROW_NOT_FOUND","message":"missing"}}"#);
    }
}
