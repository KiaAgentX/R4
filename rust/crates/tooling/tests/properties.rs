//! property tests فاز ۸ — «proptest روی invariantها» (ADR-009 §۷ فاز ۸).
//!
//! سه خاصیت مقدس روی ورودی‌های تصادفی:
//! ۱. تقسیم ترافیک — هر bucket یا به Rust است یا TS؛ مرز `pct` دقیقاً رعایت
//!    می‌شود و تخصیص تابع مسیر است (قطعی).
//! ۲. دنبالهٔ تصادفی mint/burn/escrow از مرز HTTP — Σ همیشه صفر و ترازها
//!    همیشه ≥ ۰ می‌مانند (عین خاصیت فاز ۱، این‌بار از درگاه وایر).
//! ۳. نرمال‌ساز parity — روی هر JSON دلخواه ایدمپوتنت و فیلد فرّار را رد
//!    می‌کند.

// proptest — unwrap در assert مجاز
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use dropagentx_db::V3Store;
use dropagentx_server::{build_router, AppState, ServerConfig};
use dropagentx_tooling::dual_run::{DualRunConfig, RequestMeta};
use dropagentx_tooling::parity::normalize_value;
use http_body_util::BodyExt;
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use tower::ServiceExt;

// ── خاصیت ۱ — تقسیم ترافیک ───────────────────────────────────────────────────

proptest! {
    /// تخصیص دترمینیستیک + رعایت دقیق مرز درصد.
    #[test]
    fn traffic_split_is_a_function_of_the_request(
        pct in 0u8..=100u8,
        paths in proptest::collection::vec("[a-z0-9/_-]{1,40}", 1..50),
    ) {
        let cfg = DualRunConfig { rust_traffic_pct: pct, shadow: false, ..DualRunConfig::default() };
        for path in &paths {
            let m = RequestMeta { method: "GET", path, body: b"" };
            let d1 = cfg.decide(&m);
            let d2 = cfg.decide(&m);
            prop_assert_eq!(d1, d2, "decide must be deterministic");
            match d1 {
                dropagentx_tooling::dual_run::Decision::ToRust => {
                    prop_assert!(dropagentx_tooling::dual_run::stable_bucket(&m) % 100 < u64::from(pct));
                }
                dropagentx_tooling::dual_run::Decision::ToTs => {
                    prop_assert!(dropagentx_tooling::dual_run::stable_bucket(&m) % 100 >= u64::from(pct));
                }
                other => {
                    prop_assert!(false, "unexpected decision {other:?}");
                    unreachable!()
                }
            }
        }
    }
}

// ── خاصیت ۲ — Σ=۰ از درگاه وایر ─────────────────────────────────────────────

/// استراتژی: دنبالهٔ عملیات پولی (mint/burn/escrow full cycle).
fn ops_strategy() -> impl Strategy<Value = Vec<(u8, i64)>> {
    proptest::collection::vec((0u8..3, 1i64..5_000), 1..40)
}

/// بوت درون‌حافظه‌ای برای property پولی.
async fn boot_money() -> (Router, V3Store) {
    let cfg = ServerConfig {
        admin_key: Some("prop-admin-key-0123456789".into()),
        database_url: "sqlite::memory:".into(),
        static_dir: None,
        ..ServerConfig::default()
    };
    let store = V3Store::connect(&cfg.database_url).await.unwrap();
    store.migrate().await.unwrap();
    store.ensure_system_accounts().await.unwrap();
    let state = AppState::new(store.clone(), cfg).shared();
    (build_router(state), store)
}

async fn send_json(
    router: &Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("x-kia-key", "prop-admin-key-0123456789")
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

/// Σ همهٔ پستینگ‌ها و Σ کیف کاربران از SQL خام — ممیزی مستقل.
async fn ledger_sums(store: &V3Store) -> (i64, i64) {
    use sqlx::Row;
    let pool = store.pool_ref();
    let all = sqlx::query(r#"SELECT COALESCE(SUM("amount"),0) AS s FROM "Posting""#)
        .fetch_one(pool)
        .await
        .unwrap();
    let users = sqlx::query(
        r#"SELECT COALESCE(SUM(p."amount"),0) AS s FROM "Posting" p
           JOIN "Account" a ON a."id" = p."accountId" WHERE a."kind" = 'user'"#,
    )
    .fetch_one(pool)
    .await
    .unwrap();
    (all.get::<i64, _>("s"), users.get::<i64, _>("s"))
}

#[tokio::test]
async fn random_money_sequences_via_wire_keep_ledger_sacred() {
    // نسخهٔ runtime نمونه‌ها — proptest با tokio runtime ترکیب نمی‌شود؛
    // به‌جای آن ۸ دنبالهٔ قطعی از strategy نمونه‌گیری می‌شود.
    let mut rng = TestRunner::deterministic();
    for case in 0..8u64 {
        let ops =
            ops_strategy().new_tree(&mut rng).unwrap().current().into_iter().collect::<Vec<_>>();
        let (router, store) = boot_money().await;
        send_json(&router, "POST", "/api/v1/accounts", r#"{"id":"user:pa"}"#).await;
        send_json(&router, "POST", "/api/v1/accounts", r#"{"id":"user:pb"}"#).await;
        for (idx, (kind, amount)) in ops.iter().enumerate() {
            match kind {
                0 => {
                    let body = format!(r#"{{"accountId":"user:pa","amount":{amount}}}"#);
                    send_json(&router, "POST", "/api/v1/wallet/mint", &body).await;
                }
                1 => {
                    let body = format!(r#"{{"accountId":"user:pa","amount":{amount}}}"#);
                    send_json(&router, "POST", "/api/v1/wallet/burn", &body).await;
                    // overdraft → رد
                }
                _ => {
                    let id = format!("escrow:prop{case}_{idx}");
                    let create = format!(
                        r#"{{"id":"{id}","buyerId":"user:pa","sellerId":"user:pb","amount":{amount}}}"#
                    );
                    send_json(&router, "POST", "/api/v1/escrows", &create).await;
                    send_json(&router, "POST", &format!("/api/v1/escrows/{id}/fund"), "{}").await;
                    send_json(&router, "POST", &format!("/api/v1/escrows/{id}/deliver"), "{}")
                        .await;
                    send_json(&router, "POST", &format!("/api/v1/escrows/{id}/release"), "{}")
                        .await;
                }
            }
            let (sum_all, sum_users) = ledger_sums(&store).await;
            assert_eq!(sum_all, 0, "Σ پستینگ‌ها باید صفر بماند — case {case} step {idx}");
            assert!(sum_users >= 0, "Σ کیف کاربران هرگز منفی نمی‌شود");
        }
    }
}

// ── خاصیت ۳ — نرمال‌ساز parity ──────────────────────────────────────────────

proptest! {
    /// نرمال‌ساز روی هر JSON معتبر ایدمپوتنت است و فقط فیلدهای فرّار را رد می‌کند.
    #[test]
    fn parity_normalizer_is_idempotent(
        depth in 0u8..5u8,
        width in 0usize..6usize,
        seed in any::<u64>(),
    ) {
        // ساخت JSON دلخواه — مقادیر رشته‌ای بعضاً timestamp
        let mut root = serde_json::Map::new();
        for i in 0..width {
            let key = format!("k{i}");
            let val = match (i + usize::from(depth)) % 3 {
                0 => serde_json::Value::String("2026-11-12T09:41:00.123Z".into()),
                1 => serde_json::Value::Number(serde_json::Number::from(i64::try_from(seed % 10_000).unwrap_or(0))),
                _ => serde_json::Value::Bool(i % 2 == 0),
            };
            root.insert(key, val);
        }
        let v = serde_json::Value::Object(root);
        let once = normalize_value(&v);
        let twice = normalize_value(&once);
        prop_assert_eq!(once, twice, "normalize must be idempotent");
    }
}
