//! تست بازپخش parity فاز ۸ — گیت درون‌CI.
//!
//! همهٔ سناریوهای طلایی روی روتر Rust (دیتابیس درون‌حافظه‌ای تازه به ازای هر
//! سناریو) اجرا و پاسخ + وضعیت دیتابیس **بایت‌به‌بایت** با گلدن‌های
//! `tests/golden/*.json` مقایسه می‌شود. هر drift بایتی یعنی قرارداد تغییر
//! کرده — یا باگ است یا تغییر عمدی قرارداد که در آن صورت `parity --record`
//! باید آگاهانه اجرا شود و diff بازبینی شود.

// یکپارچگی‌سنجی — unwrap/expect عمدی‌اند
#![allow(clippy::unwrap_used, clippy::expect_used)]

use dropagentx_tooling::parity;

/// گیت اصلی — همهٔ سناریوها سبز یا شکست تست.
#[tokio::test]
async fn every_golden_scenario_replays_byte_for_byte() {
    let report = parity::verify_all(|_| "sqlite::memory:".into()).await.unwrap();
    let (green, total) = report.summary();
    assert_eq!(green, total, "parity drift detected:\n{}", report.render());
    // ADR-009 فاز ۸ — پوشش «~۳۰ سناریوی طلایی»: رجیستری باید ≥۳۰ باشد
    assert!(total >= 30, "golden scenario registry shrank: {total}");
}

/// نرمال‌ساز — timestamp و uptime فرّار باید به توکن ثابت برسند.
#[test]
fn normalizer_flattens_volatile_fields() {
    let body = serde_json::json!({
        "createdAt": "2026-11-12T09:41:00.123Z",
        "uptime": 41.5,
        "nested": { "decidedAt": "2026-11-12T09:41:03+03:30", "amount": 10 }
    });
    let out = parity::normalize_value(&body);
    assert_eq!(out["createdAt"], "<TS>");
    assert_eq!(out["uptime"], "<UPTIME>");
    assert_eq!(out["nested"]["decidedAt"], "<TS>");
    assert_eq!(out["nested"]["amount"], 10);
    // نرمال‌سازی دوباره — ایدمپوتنت
    assert_eq!(parity::normalize_value(&out), out);
}

/// گیت مالی parity — تخطی از ۹۰/۱۰ حتی در نرمال‌سازی هم مغایرت است.
#[tokio::test]
async fn release_scenario_db_state_is_locked_in_golden() {
    let scenarios = parity::scenarios();
    let release = scenarios
        .iter()
        .find(|s| s.name == "escrow_release_settles_90_10")
        .unwrap_or_else(|| panic!("release scenario missing"));
    let result = parity::run_scenario(release, "sqlite::memory:").await.unwrap();
    assert_eq!(result.db.user_wallet_sum, Some(199_000));
    assert_eq!(result.db.treasury, Some(1_000));
    assert_eq!(result.db.escrow_status.get("escrow:escrow:e3"), Some(&"RELEASED".to_string()));
}
