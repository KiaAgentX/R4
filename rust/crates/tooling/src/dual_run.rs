//! مغز مسیریابی dual-run فاز ۸ — پل بازگشت همیشه بالاست (ADR-009 §۱۲).
//!
//! سه گام گذار:
//! * **Shadow** — ۱۰۰٪ ترافیک به TS؛ Rust همان ورودی‌ها را سایه‌وار اجرا و
//!   فقط ثبت می‌کند (پاسخ Rust هرگز به مشتری نمی‌رود).
//! * **درصدی** — `RUST_TRAFFIC_PCT` = ۱٪ → ۱۰٪ → ۵۰٪ → ۱۰۰٪؛ هر پله ۴۸ ساعت
//!   پایداری + parity + p95. تخصیص **دترمینیستیک** است (هش پایدار ورودی) تا
//!   retry با همان کلید یکسان‌سازی همیشه به همان موتور برود.
//! * **تثبیت** — ۲ هفته ۱۰۰٪ سبز → تگ `rust-core-1.0.0` → خاموشی Node.
//!
//! سپر انسانی: مسیرهای پولی ≥ ۵۰٬۰۰۰ Đ (HITL — مادهٔ ۷) تا ۳ روزِ پایداریِ
//! کامل فقط به TS می‌روند — «آخرین لایه‌ای که سوییچ می‌شود، اولین چیزی که
//! آدیت می‌شود».
//!
//! برگشت: `RUST_TRAFFIC_PCT=0` → فوراً همه‌چیز به TS — چون هر دو موتور روی
//! **همان** SQLite با همان invariantها کار می‌کنند، برگشتِ داده‌ای لازم نیست.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// آستانهٔ سپر انسانی — مادهٔ ۷ منشور (HITL).
pub const HITL_SHIELD_DIN: i64 = 50_000;

/// روزهای پایداری لازم برای برداشتن سپر HITL.
pub const SHIELD_STABILITY_DAYS: u32 = 3;

/// تصمیم مسیریابی برای یک درخواست.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// به Rust برود (درصد یا shadow-سنجش)
    ToRust,
    /// به TS برود (پاسخ‌دهندهٔ واقعی)
    ToTs,
    /// حالت سایه: پاسخ از TS، اجرای سایه روی Rust + مقایسهٔ parity
    Shadow,
}

/// پیکربندی کامل dual-run — از محیط خوانده می‌شود (`DualRunConfig::from_env`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DualRunConfig {
    /// درصد ترافیک Rust (۰ تا ۱۰۰) — تک‌متغیرِ سوییچ
    pub rust_traffic_pct: u8,
    /// حالت سایه فعال؟ (مستقل از درصد — در shadow، درصد باید ۰ باشد)
    pub shadow: bool,
    /// روزهای پایداری ثبت‌شده — سپر HITL روی آن سوار است
    pub stability_days: u32,
    /// آستانهٔ سپر (پیش‌فرض ۵۰٬۰۰۰ Đ — مادهٔ ۷)
    pub hitl_shield_din: i64,
}

impl Default for DualRunConfig {
    fn default() -> Self {
        Self {
            rust_traffic_pct: 0,
            shadow: true,
            stability_days: 0,
            hitl_shield_din: HITL_SHIELD_DIN,
        }
    }
}

impl DualRunConfig {
    /// اعتبارسنجی — درصد > ۱۰۰ یا shadow همراه ترافیک رد می‌شود.
    ///
    /// # Errors
    /// `pct > 100` یا `shadow && pct > 0` — کانفیگ متناقض.
    pub fn validate(&self) -> Result<(), String> {
        if self.rust_traffic_pct > 100 {
            return Err(format!("rust_traffic_pct {} > 100", self.rust_traffic_pct));
        }
        if self.shadow && self.rust_traffic_pct > 0 {
            return Err(
                "shadow mode must have rust_traffic_pct = 0 (responses never come from Rust)"
                    .into(),
            );
        }
        Ok(())
    }
}

/// فرادادهٔ یک درخواست برای تصمیم مسیریابی — حداقلِ چیزی که پایدار هش می‌شود.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestMeta<'a> {
    /// متد HTTP
    pub method: &'a str,
    /// مسیر کامل
    pub path: &'a str,
    /// بدنهٔ خام (برای هش؛ خواندن مبلغ از آن جداگانه است)
    pub body: &'a [u8],
}

/// هش پایدار ۶۴بیتی ورودی — FNV-1a روی SHA-256 (دو لایه، پایدار بین پروسه‌ها).
#[must_use]
pub fn stable_bucket(meta: &RequestMeta<'_>) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(meta.method.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(meta.path.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(meta.body);
    let digest = hasher.finalize();
    let mut out: u64 = 0;
    for b in &digest[0..8] {
        out = (out << 8) | u64::from(*b);
    }
    out
}

/// آیا مسیر، تراکنشِ مشمول سپر انسانی (مسیر HITL) است؟
///
/// طبق ADR-009 §۱۲: «تراکنش‌های ≥ ۵۰٬۰۰۰ Đ (مسیر HITL) فقط بعد از ۳ روز
/// پایداری کامل به Rust می‌روند». مسیرِ HITL دقیقاً همین‌هاست:
/// * `POST …/hitl*` — صف و تصمیم‌های انسان-در-حلقه (مبلغش همان اسکرو ≥ آستانه است)
/// * `POST …/escrows/{id}/release` و `…/refund` — تسویه/بازپرداخت امانت
///   (تنها جاهایی که پول از اسکرو خارج می‌شود؛ مبلغ در دیتابیس مشترک است نه بدنه)
#[must_use]
pub fn hitl_shielded(meta: &RequestMeta<'_>) -> bool {
    if meta.method != "POST" {
        return false;
    }
    if meta.path.contains("/hitl") {
        return true;
    }
    meta.path.contains("/escrows")
        && (meta.path.ends_with("/release") || meta.path.ends_with("/refund"))
}

impl DualRunConfig {
    /// تصمیم مسیریابی برای یک درخواست — قلب پراکسی.
    ///
    /// قواعد به ترتیب:
    /// ۱. سپر HITL — تراکنش ≥ آستانه تا ۳ روز پایداری به TS می‌رود
    /// ۲. حالت سایه — همه به TS، Rust سایه‌وار
    /// ۳. درصد — bucket پایدار `< pct` → Rust
    #[must_use]
    pub fn decide(&self, meta: &RequestMeta<'_>) -> Decision {
        if self.stability_days < SHIELD_STABILITY_DAYS && hitl_shielded(meta) {
            // سپر — حتی در ۱۰۰٪ Rust؛ برداشتنش با stability_days است نه درصد
            return Decision::ToTs;
        }
        if self.shadow {
            return Decision::Shadow;
        }
        if stable_bucket(meta) % 100 < u64::from(self.rust_traffic_pct) {
            Decision::ToRust
        } else {
            Decision::ToTs
        }
    }
}

/// گیت ارتقای پله — بر اساس گزارش parity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateVerdict {
    /// سبز — مجاز به ارتقای پله بعدی
    Advance,
    /// قرمز — بمان و برگرد (pct → ۰ در parity-fail مالی)
    HoldOrRollback,
}

/// بررسی گیت parity — گزارش باید کامل سبز باشد (ADR: «هر parity-fail =
/// برگشت»).
#[must_use]
pub fn parity_gate(green: usize, total: usize, has_financial_diff: bool) -> GateVerdict {
    if has_financial_diff || green < total {
        GateVerdict::HoldOrRollback
    } else {
        GateVerdict::Advance
    }
}

/// برنامهٔ پله‌های گذار — برای نمایش وضعیت فعلی در status پراکسی.
#[must_use]
pub fn rollout_plan() -> [(u8, &'static str); 5] {
    [
        (0, "shadow — پاسخ فقط از TS، Rust ثبت می‌کند"),
        (1, "۱٪ — آغاز ترافیک واقعی (۴۸ ساعت پایداری)"),
        (10, "۱۰٪ — پس از سبزی parity و p95 پلهٔ قبل"),
        (50, "۵۰٪ — همان گیت"),
        (100, "۱۰۰٪ — سپر HITL تا ۳ روز پایداری باقی است"),
    ]
}

/// خط وضعیت dual-run — برای `GET /_dualrun/status` و لاگ.
#[derive(Debug, Clone, Serialize)]
pub struct DualRunStatus {
    /// پیکربندی جاری
    pub config: DualRunConfig,
    /// برنامهٔ پله‌ها
    pub plan: Vec<(u8, &'static str)>,
    /// برگشت اضطراری فعال؟
    pub rollback: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    const GET: &str = "GET";

    fn meta<'a>(method: &'a str, path: &'a str, body: &'a [u8]) -> RequestMeta<'a> {
        RequestMeta { method, path, body }
    }

    #[test]
    fn bucket_is_deterministic_across_calls() {
        let m = meta("POST", "/api/v1/escrows", br#"{"amount":10}"#);
        let a = stable_bucket(&m);
        let b = stable_bucket(&m);
        assert_eq!(a, b);
        // بدنهٔ متفاوت → bucket متفاوت (احتمال برابری ۲⁻⁶۴ — عملاً ناممکن)
        assert_ne!(stable_bucket(&meta("POST", "/api/v1/escrows", br#"{"amount":11}"#)), a);
    }

    #[test]
    fn pct_zero_sends_everything_to_ts() {
        let cfg = DualRunConfig { rust_traffic_pct: 0, shadow: false, ..DualRunConfig::default() };
        for i in 0..64 {
            let path = format!("/api/v1/r{i}");
            let m = meta(GET, &path, b"");
            assert_eq!(cfg.decide(&m), Decision::ToTs, "path {path}");
        }
    }

    #[test]
    fn pct_hundred_sends_everything_to_rust() {
        let cfg =
            DualRunConfig { rust_traffic_pct: 100, shadow: false, ..DualRunConfig::default() };
        for i in 0..64 {
            let path = format!("/api/v1/r{i}");
            let m = meta(GET, &path, b"");
            assert_eq!(cfg.decide(&m), Decision::ToRust, "path {path}");
        }
    }

    #[test]
    fn shadow_mode_flags_every_request() {
        let cfg = DualRunConfig::default(); // pct=0 + shadow
        let m = meta(GET, "/api/v1/health", b"");
        assert_eq!(cfg.decide(&m), Decision::Shadow);
    }

    #[test]
    fn hitl_shield_keeps_big_money_on_ts_until_three_days() {
        let body = br#"{"escrowId":"e1"}"#;
        let m = meta("POST", "/api/v1/hitl", body);
        // روز صفر — سپر کامل
        let cfg = DualRunConfig {
            rust_traffic_pct: 100,
            shadow: false,
            stability_days: 0,
            ..DualRunConfig::default()
        };
        assert_eq!(cfg.decide(&m), Decision::ToTs);
        // روز ۲ — هنوز سپر
        let cfg2 = DualRunConfig { stability_days: 2, ..cfg };
        assert_eq!(cfg2.decide(&m), Decision::ToTs);
        // روز ۳ — سپر برداشته شد → ۱۰۰٪ Rust
        let cfg3 = DualRunConfig { stability_days: 3, ..cfg };
        assert_eq!(cfg3.decide(&m), Decision::ToRust);
    }

    #[test]
    fn release_route_is_shielded_until_three_days() {
        let cfg =
            DualRunConfig { rust_traffic_pct: 100, shadow: false, ..DualRunConfig::default() };
        let m = meta("POST", "/api/v1/escrows/e9/release", br#"{"approvedByAdmin":true}"#);
        // تسویهٔ اسکرو — مسیر HITL؛ تا روز ۳ به TS می‌رود حتی در ۱۰۰٪
        assert_eq!(cfg.decide(&m), Decision::ToTs);
        let cfg3 = DualRunConfig { stability_days: 3, ..cfg };
        assert_eq!(cfg3.decide(&m), Decision::ToRust);
    }

    #[test]
    fn mint_is_not_shielded_but_refund_is() {
        let cfg =
            DualRunConfig { rust_traffic_pct: 100, shadow: false, ..DualRunConfig::default() };
        let m = meta("POST", "/api/v1/wallet/mint", br#"{"amount":50000}"#);
        // mint مسیر سپر نیست (فقط hitl و release/refund)
        assert_eq!(cfg.decide(&m), Decision::ToRust);
        let m2 = meta("POST", "/api/v1/escrows/e1/refund", br#"{}"#);
        assert_eq!(cfg.decide(&m2), Decision::ToTs);
    }

    #[test]
    fn parity_gate_holds_on_any_failure() {
        assert_eq!(parity_gate(42, 42, false), GateVerdict::Advance);
        assert_eq!(parity_gate(41, 42, false), GateVerdict::HoldOrRollback);
        assert_eq!(parity_gate(42, 42, true), GateVerdict::HoldOrRollback);
    }

    #[test]
    fn config_validation_rejects_shadow_with_traffic() {
        let ok = DualRunConfig::default();
        assert!(ok.validate().is_ok());
        let bad = DualRunConfig { rust_traffic_pct: 101, shadow: false, ..ok };
        assert!(bad.validate().is_err());
        let mixed = DualRunConfig { rust_traffic_pct: 10, shadow: true, ..ok };
        assert!(mixed.validate().is_err());
    }

    #[test]
    fn rollout_plan_has_five_steps_to_hundred() {
        let plan = rollout_plan();
        assert_eq!(plan.len(), 5);
        assert_eq!(plan[0].0, 0);
        assert_eq!(plan[4].0, 100);
    }
}
