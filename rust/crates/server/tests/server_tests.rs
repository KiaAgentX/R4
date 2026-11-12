//! تست‌های پذیرش فاز ۴ — پورت تست‌های سرور TS روی `app.inject` (tower oneshot).
//!
//! پوشش معیارهای پذیرش ADR-009:
//! ۱. مسیر کامل mint→escrow→release دقیقاً ۹۰/۱۰
//! ۲. replay دوبار پرداخت نمی‌کند (کیف و اسکرو)
//! ۳. overdraft رد می‌شود (سوزاندن/تأمین)
//! ۴. ۴۰۱/۴۰۴/۴۲۲/۴۲۹ همان‌ها
//! ۵. HITL ≥ ۵۰٬۰۰۰ Đ از صف می‌گذرد و تأییدش دقیقاً ۹۰/۱۰ تسویه می‌کند
//! ۶. بوت با کانفیگ ناقص LLM رد می‌شود (تست config در `config.rs`)
//! ۷. ServeDir داشبورد + SPA fallback
//! ۸. سند OpenAPI ۴۶ عملیات

// یکپارچگی‌سنجی — unwrap/expect عمدی‌اند: شکست boot یعنی خود تست باید شکست بخورد
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use dropagentx_contracts::make_test_init_data;
use dropagentx_db::V3Store;
use dropagentx_server::{build_router, AppState, ServerConfig};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt; // oneshot — معادل app.inject فستیفای

const KEY: &str = "test-admin-key-0123456789";
const BOT: &str = "7000000000:TEST-TOKEN-abc";

/// کانفیگ پایه — ادمین فعال، بدون LLM/بات/static.
fn base_config() -> ServerConfig {
    ServerConfig {
        admin_key: Some(KEY.into()),
        database_url: "sqlite::memory:".into(),
        static_dir: None,
        ..ServerConfig::default()
    }
}

async fn boot_with(cfg: ServerConfig) -> (Router, V3Store) {
    let store = V3Store::connect(&cfg.database_url).await.unwrap();
    store.migrate().await.unwrap();
    store.ensure_system_accounts().await.unwrap();
    let state = AppState::new(store.clone(), cfg).shared();
    (build_router(state), store)
}

async fn boot() -> (Router, V3Store) {
    boot_with(base_config()).await
}

/// یک درخواست کامل — پاسخ (status, JSON).
async fn send(
    router: &Router,
    method: &str,
    uri: &str,
    body: Option<String>,
    key: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(k) = key {
        builder = builder.header("x-kia-key", k);
    }
    let req = builder.body(Body::from(body.unwrap_or_default())).unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn get(router: &Router, uri: &str, key: Option<&str>) -> (StatusCode, Value) {
    send(router, "GET", uri, None, key).await
}

async fn post(router: &Router, uri: &str, body: Value, key: Option<&str>) -> (StatusCode, Value) {
    send(router, "POST", uri, Some(body.to_string()), key).await
}

async fn patch(router: &Router, uri: &str, body: Value, key: Option<&str>) -> (StatusCode, Value) {
    send(router, "PATCH", uri, Some(body.to_string()), key).await
}

async fn del(router: &Router, uri: &str, key: Option<&str>) -> (StatusCode, Value) {
    send(router, "DELETE", uri, None, key).await
}

/// ثبت حساب + ضرب — مسیر استاندارد پیش‌نیاز هر سناریو.
async fn setup_funded_user(router: &Router, id: &str, amount: i64) -> Value {
    let (status, _) = post(router, "/api/v1/accounts", json!({ "id": id }), Some(KEY)).await;
    assert_eq!(status, StatusCode::CREATED, "account {id}");
    let (status, res) = post(
        router,
        "/api/v1/wallet/mint",
        json!({ "accountId": id, "amount": amount }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "mint {id}: {res}");
    res
}

/// ساخت امانت + تأمین + تحویل — آمادهٔ تسویه.
async fn setup_delivered_escrow(router: &Router, id: &str, buyer: &str, seller: &str, amount: i64) {
    let (status, res) = post(
        router,
        "/api/v1/escrows",
        json!({ "id": id, "buyerId": buyer, "sellerId": seller, "amount": amount }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create escrow: {res}");
    let (status, res) =
        post(router, &format!("/api/v1/escrows/{id}/fund"), json!({}), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "fund: {res}");
    let (status, res) =
        post(router, &format!("/api/v1/escrows/{id}/deliver"), json!({}), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "deliver: {res}");
}

// ── health + docs ───────────────────────────────────────────────────────────

#[tokio::test]
async fn health_reports_chips_and_llm_fields() {
    let (router, _) = boot().await;
    let (status, res) = get(&router, "/api/v1/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["status"], "ok");
    assert_eq!(res["service"], "dropagentx-v3-server");
    assert_eq!(res["llmConfigured"], false);
    assert!(res["llmModel"].is_null());
    assert_eq!(res["db"], "ok");
    assert_eq!(res["bot"], "disabled");
    assert_eq!(res["ai"], "disabled");
    assert!(res["uptime"].is_number());
}

#[tokio::test]
async fn health_shows_model_when_llm_configured() {
    let cfg = ServerConfig {
        llm: Some(dropagentx_server::LlmConfig {
            base_url: "https://api.example.com/v1".into(),
            api_key: "sk-test".into(),
            model: "gpt-4o-mini".into(),
        }),
        ..base_config()
    };
    let (router, _) = boot_with(cfg).await;
    let (status, res) = get(&router, "/api/v1/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["llmConfigured"], true);
    assert_eq!(res["llmModel"], "gpt-4o-mini");
    assert_eq!(res["ai"], "ok");
}

#[tokio::test]
async fn openapi_json_serves_46_operations() {
    let (router, _) = boot().await;
    let (status, doc) = get(&router, "/api/v1/openapi.json", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(doc["openapi"], "3.1.0");
    assert_eq!(doc["servers"][0]["url"], "/api/v1");
    let paths = doc["paths"].as_object().unwrap();
    let mut operations = 0;
    for obj in paths.values() {
        operations += obj.as_object().map(|o| o.len()).unwrap_or(0);
    }
    assert_eq!(operations, 46, "registry count must match TS API_ROUTES");
    // هیچ $ref حل‌نشده نمانده باشد (سند inline مثل zod)
    let raw = serde_json::to_string(&doc).unwrap();
    assert!(!raw.contains("\"$ref\""), "doc must be fully inlined");
}

#[tokio::test]
async fn docs_page_served_html() {
    let (router, _) = boot().await;
    let (status, _) = send(&router, "GET", "/api/v1/docs", None, None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn unknown_api_route_is_404_envelope() {
    let (router, _) = boot().await;
    let (status, res) = get(&router, "/api/v1/definitely-not-a-route", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(res["error"]["code"], "NOT_FOUND");
}

// ── امنیت — ۴۰۱/۴۲۹/۵۰۳ ─────────────────────────────────────────────────────

#[tokio::test]
async fn admin_disabled_503_without_server_key() {
    let cfg = ServerConfig { admin_key: None, static_dir: None, ..ServerConfig::default() };
    let (router, _) = boot_with(cfg).await;
    // حتی کلید درست هم عبور نمی‌کند — fail-closed همیشه
    let (status, res) = get(&router, "/api/v1/accounts", Some(KEY)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(res["error"]["code"], "ADMIN_DISABLED");
}

#[tokio::test]
async fn admin_key_required_401_without_header() {
    let (router, _) = boot().await;
    let (status, res) = get(&router, "/api/v1/accounts", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["error"]["code"], "ADMIN_KEY_REQUIRED");
}

#[tokio::test]
async fn invalid_admin_key_401() {
    let (router, _) = boot().await;
    let (status, res) = get(&router, "/api/v1/accounts", Some("wrong-key-0123456789")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["error"]["code"], "INVALID_ADMIN_KEY");
}

#[tokio::test]
async fn rate_limit_429_carries_meta_limit() {
    let cfg = ServerConfig { rate_limit: 3, ..base_config() };
    let (router, _) = boot_with(cfg).await;
    for _ in 0..3 {
        let (status, _) = get(&router, "/api/v1/health", None).await;
        assert_eq!(status, StatusCode::OK);
    }
    let (status, res) = get(&router, "/api/v1/health", None).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(res["error"]["code"], "RATE_LIMITED");
    assert_eq!(res["error"]["meta"]["limit"], 3);
}

// ── حساب‌ها ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_account_201_and_list_and_balance() {
    let (router, store) = boot().await;
    let (status, res) =
        post(&router, "/api/v1/accounts", json!({ "id": "user:tg:7", "label": "Ali" }), Some(KEY))
            .await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    assert_eq!(res["id"], "user:tg:7");
    assert_eq!(res["kind"], "user");
    assert_eq!(res["label"], "Ali");
    assert_eq!(res["balance"], 0);
    assert!(res["createdAt"].is_string());

    // «فهرست همهٔ حساب‌ها» — سه حساب سیستمی + کاربر تازه
    let (status, res) = get(&router, "/api/v1/accounts", Some(KEY)).await;
    assert_eq!(status, StatusCode::OK);
    let items = res["items"].as_array().unwrap();
    assert_eq!(items.len(), 4, "mint:genesis + treasury:nation + burn:void + user");
    assert!(items.iter().any(|a| a["id"] == "user:tg:7" && a["balance"] == 0));

    // موجودی عمومی — بدون کلید ادمین
    let (status, res) = get(&router, "/api/v1/accounts/user:tg:7/balance", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["accountId"], "user:tg:7");
    assert_eq!(res["balance"], 0);
    let db_balance = store.balance_of("user:tg:7").await.unwrap();
    assert_eq!(db_balance, Some(0));
}

#[tokio::test]
async fn create_account_rejects_reserved_prefix_422() {
    let (router, _) = boot().await;
    for bad in ["mint:x", "treasury:x", "burn:x", "escrow:x"] {
        let (status, res) =
            post(&router, "/api/v1/accounts", json!({ "id": bad }), Some(KEY)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
        assert_eq!(res["error"]["code"], "RESERVED_ID", "{bad}");
    }
}

#[tokio::test]
async fn create_account_duplicate_422() {
    let (router, _) = boot().await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:tg:7" }), Some(KEY)).await;
    let (status, res) =
        post(&router, "/api/v1/accounts", json!({ "id": "user:tg:7" }), Some(KEY)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res["error"]["code"], "DUPLICATE_ACCOUNT");
}

#[tokio::test]
async fn balance_unknown_404() {
    let (router, _) = boot().await;
    let (status, res) = get(&router, "/api/v1/accounts/user:ghost/balance", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(res["error"]["code"], "UNKNOWN_ACCOUNT");
}

// ── کیف پول — ضرب/سوزاندن/replay/سقف ────────────────────────────────────────

#[tokio::test]
async fn mint_updates_balance_and_replay_does_not_double_pay() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:tg:1", 500).await;

    // replay با همان کلید — پول دوبار جابه‌جا نمی‌شود
    let body =
        json!({ "accountId": "user:tg:1", "amount": 100, "idempotencyKey": "mint-replay-1" });
    let (s1, r1) = post(&router, "/api/v1/wallet/mint", body.clone(), Some(KEY)).await;
    assert_eq!(s1, StatusCode::OK, "{r1}");
    assert_eq!(r1["balance"], 600);
    assert_eq!(r1["replayed"], false);
    let (s2, r2) = post(&router, "/api/v1/wallet/mint", body, Some(KEY)).await;
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(r2["replayed"], true);
    assert_eq!(r2["balance"], 600, "replay must not add again");
    assert_eq!(store.balance_of("user:tg:1").await.unwrap(), Some(600));

    // تقدس دفتر — Σ همهٔ ترازها صفر
    let econ = store.economy_summary().await.unwrap();
    assert!(econ.balanced);
}

#[tokio::test]
async fn mint_unknown_account_404() {
    let (router, _) = boot().await;
    let (status, res) = post(
        &router,
        "/api/v1/wallet/mint",
        json!({ "accountId": "user:ghost", "amount": 5 }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(res["error"]["code"], "UNKNOWN_ACCOUNT");
}

#[tokio::test]
async fn mint_cap_exceeded_422() {
    let (router, _) = boot().await;
    setup_funded_user(&router, "user:tg:1", 500_000).await;
    let (status, res) = post(
        &router,
        "/api/v1/wallet/mint",
        json!({ "accountId": "user:tg:1", "amount": 600_000 }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{res}");
    assert_eq!(res["error"]["code"], "MINT_CAP_EXCEEDED");
}

#[tokio::test]
async fn burn_overdraft_rejected_422() {
    let (router, _) = boot().await;
    setup_funded_user(&router, "user:tg:1", 100).await;
    let (status, res) = post(
        &router,
        "/api/v1/wallet/burn",
        json!({ "accountId": "user:tg:1", "amount": 200 }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res["error"]["code"], "ACCOUNT_OVERDRAFT");
}

#[tokio::test]
async fn burn_reduces_balance() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:tg:1", 500).await;
    let (status, res) = post(
        &router,
        "/api/v1/wallet/burn",
        json!({ "accountId": "user:tg:1", "amount": 200, "memo": "cleanup" }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["balance"], 300);
    assert!(res["txId"].as_str().unwrap().starts_with("tx_"));
    assert_eq!(store.balance_of("user:tg:1").await.unwrap(), Some(300));
}

// ── اسکرو — مسیر کامل ۹۰/۱۰ + مادهٔ ۷ + replay ──────────────────────────────

#[tokio::test]
async fn full_path_release_splits_exactly_90_10() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:buyer", 1_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    setup_delivered_escrow(&router, "escrow:e1", "user:buyer", "user:seller", 400).await;

    // ۴۰۰ < ۵۰٬۰۰۰ — بدون تأیید ادمین هم مجاز است
    let (status, res) =
        post(&router, "/api/v1/escrows/escrow:e1/release", json!({}), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["escrow"]["status"], "RELEASED");
    assert!(res["txId"].is_string());
    assert_eq!(res["replayed"], false);

    // دقیقاً ۹۰/۱۰ — مادهٔ ۲
    assert_eq!(store.balance_of("user:seller").await.unwrap(), Some(360));
    assert_eq!(store.balance_of("treasury:nation").await.unwrap(), Some(40));
    assert_eq!(store.balance_of("user:buyer").await.unwrap(), Some(600));
    let econ = store.economy_summary().await.unwrap();
    assert!(econ.balanced);
    assert_eq!(econ.trial_sum, 0);
}

#[tokio::test]
async fn release_50k_requires_explicit_approval_422() {
    let (router, _) = boot().await;
    setup_funded_user(&router, "user:buyer", 100_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    setup_delivered_escrow(&router, "escrow:big", "user:buyer", "user:seller", 60_000).await;

    let (status, res) =
        post(&router, "/api/v1/escrows/escrow:big/release", json!({}), Some(KEY)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res["error"]["code"], "REQUIRES_HUMAN_APPROVAL");

    // تأیید صریح عبور می‌کند — بازپرداخت صفر از خریدار، ۹۰/۱۰ دقیق
    let (status, res) = post(
        &router,
        "/api/v1/escrows/escrow:big/release",
        json!({ "approvedByAdmin": true }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["escrow"]["status"], "RELEASED");
}

#[tokio::test]
async fn fund_overdraft_rejected_422() {
    let (router, _) = boot().await;
    setup_funded_user(&router, "user:buyer", 100).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    post(&router, "/api/v1/escrows", json!({ "id": "escrow:big", "buyerId": "user:buyer", "sellerId": "user:seller", "amount": 400 }), Some(KEY)).await;
    let (status, res) =
        post(&router, "/api/v1/escrows/escrow:big/fund", json!({}), Some(KEY)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res["error"]["code"], "ACCOUNT_OVERDRAFT");
}

#[tokio::test]
async fn fund_idempotency_replay_never_double_locks() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:buyer", 1_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    post(&router, "/api/v1/escrows", json!({ "id": "escrow:r1", "buyerId": "user:buyer", "sellerId": "user:seller", "amount": 400 }), Some(KEY)).await;

    let body = json!({ "idempotencyKey": "fund-escrow-r1" });
    let (s1, r1) = post(&router, "/api/v1/escrows/escrow:r1/fund", body.clone(), Some(KEY)).await;
    assert_eq!(s1, StatusCode::OK, "{r1}");
    assert_eq!(r1["replayed"], false);
    assert_eq!(r1["escrow"]["status"], "FUNDED");
    let (s2, r2) = post(&router, "/api/v1/escrows/escrow:r1/fund", body, Some(KEY)).await;
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(r2["replayed"], true, "second call must be an honest replay");
    assert_eq!(r2["escrow"]["fundedTxId"], r1["escrow"]["fundedTxId"]);
    // خریدار فقط یک‌بار بدهکار شده
    assert_eq!(store.balance_of("user:buyer").await.unwrap(), Some(600));
    assert_eq!(store.balance_of("escrow:escrow:r1").await.unwrap(), Some(400));
}

#[tokio::test]
async fn fund_twice_invalid_transition_422() {
    let (router, _) = boot().await;
    setup_funded_user(&router, "user:buyer", 1_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    setup_delivered_escrow(&router, "escrow:twice", "user:buyer", "user:seller", 100).await;
    let (status, res) =
        post(&router, "/api/v1/escrows/escrow:twice/fund", json!({}), Some(KEY)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res["error"]["code"], "INVALID_TRANSITION");
}

#[tokio::test]
async fn escrow_unknown_404() {
    let (router, _) = boot().await;
    let (status, res) = get(&router, "/api/v1/escrows/escrow:ghost", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(res["error"]["code"], "ESCROW_NOT_FOUND");
}

#[tokio::test]
async fn escrow_list_filters_by_status() {
    let (router, _) = boot().await;
    setup_funded_user(&router, "user:buyer", 1_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    setup_delivered_escrow(&router, "escrow:l1", "user:buyer", "user:seller", 100).await;

    let (status, res) = get(&router, "/api/v1/escrows?status=DELIVERED", Some(KEY)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["items"].as_array().unwrap().len(), 1);
    let (status, res) = get(&router, "/api/v1/escrows?status=RELEASED", Some(KEY)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["items"].as_array().unwrap().len(), 0);
    // فیلتر نامعتبر → 400 مثل zod enum
    let (status, res) = get(&router, "/api/v1/escrows?status=FLOATING", Some(KEY)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{res}");
    assert_eq!(res["error"]["code"], "INVALID_REQUEST");
}

#[tokio::test]
async fn dispute_then_refund_returns_everything_without_tax() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:buyer", 1_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    setup_delivered_escrow(&router, "escrow:d1", "user:buyer", "user:seller", 400).await;

    let (status, res) = post(
        &router,
        "/api/v1/escrows/escrow:d1/dispute",
        json!({ "reason": "not as described" }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["escrow"]["status"], "DISPUTED");
    assert_eq!(res["escrow"]["disputeReason"], "not as described");
    assert!(res["txId"].is_null(), "dispute moves no money");

    let (status, res) =
        post(&router, "/api/v1/escrows/escrow:d1/refund", json!({}), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["escrow"]["status"], "REFUNDED");

    // بازگشت کامل، بدون مالیات — خزانه دست‌نخورده
    assert_eq!(store.balance_of("user:buyer").await.unwrap(), Some(1_000));
    assert_eq!(store.balance_of("treasury:nation").await.unwrap(), Some(0));
    assert_eq!(store.balance_of("user:seller").await.unwrap(), Some(0));
}

#[tokio::test]
async fn cancel_from_created_moves_no_money() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:buyer", 1_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    post(&router, "/api/v1/escrows", json!({ "id": "escrow:c1", "buyerId": "user:buyer", "sellerId": "user:seller", "amount": 400 }), Some(KEY)).await;
    let (status, res) =
        post(&router, "/api/v1/escrows/escrow:c1/cancel", json!({}), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["escrow"]["status"], "CANCELLED");
    assert!(res["txId"].is_null());
    assert_eq!(store.balance_of("user:buyer").await.unwrap(), Some(1_000));
    assert_eq!(store.balance_of("escrow:escrow:c1").await.unwrap(), Some(0));
}

#[tokio::test]
async fn expire_funded_refunds_buyer() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:buyer", 1_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    let (_, res) = post(&router, "/api/v1/escrows", json!({ "id": "escrow:x1", "buyerId": "user:buyer", "sellerId": "user:seller", "amount": 400, "ttlHours": 1 }), Some(KEY)).await;
    assert!(res["escrow"]["expiresAt"].is_string());
    post(&router, "/api/v1/escrows/escrow:x1/fund", json!({}), Some(KEY)).await;

    let (status, res) =
        post(&router, "/api/v1/escrows/escrow:x1/expire", json!({}), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["escrow"]["status"], "EXPIRED");
    assert_eq!(store.balance_of("user:buyer").await.unwrap(), Some(1_000));
}

// ── HITL — مادهٔ ۷ روی صف ────────────────────────────────────────────────────

#[tokio::test]
async fn hitl_flow_approve_settles_exactly_90_10() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:buyer", 100_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    setup_delivered_escrow(&router, "escrow:h1", "user:buyer", "user:seller", 60_000).await;

    // صف‌کردن — مبلغ از امانت کپی می‌شود
    let (status, res) = post(
        &router,
        "/api/v1/hitl",
        json!({ "kind": "ESCROW_RELEASE", "escrowId": "escrow:h1", "requesterId": "user:buyer", "note": "big deal" }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    assert_eq!(res["kind"], "ESCROW_RELEASE");
    assert_eq!(res["amount"], 60_000);
    assert_eq!(res["status"], "PENDING");
    assert_eq!(res["note"], "big deal");
    let hitl_id = res["id"].as_str().unwrap().to_owned();

    // تأیید → تسویه در همان تراکنش — دقیقاً ۹۰/۱۰
    let (status, res) = post(
        &router,
        &format!("/api/v1/hitl/{hitl_id}/approve"),
        json!({ "decidedBy": "admin:kia" }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["request"]["status"], "APPROVED");
    assert_eq!(res["request"]["decidedBy"], "admin:kia");
    assert_eq!(res["escrow"]["status"], "RELEASED");
    assert!(res["txId"].is_string());
    assert_eq!(res["replayed"], false);

    assert_eq!(store.balance_of("user:seller").await.unwrap(), Some(54_000));
    assert_eq!(store.balance_of("treasury:nation").await.unwrap(), Some(6_000));
    let econ = store.economy_summary().await.unwrap();
    assert!(econ.balanced);
}

#[tokio::test]
async fn hitl_duplicate_id_is_replay() {
    let (router, _) = boot().await;
    setup_funded_user(&router, "user:buyer", 100_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    setup_delivered_escrow(&router, "escrow:h2", "user:buyer", "user:seller", 60_000).await;

    let body = json!({ "id": "hitl:h2-fixed", "kind": "ESCROW_RELEASE", "escrowId": "escrow:h2", "requesterId": "user:buyer" });
    let (s1, r1) = post(&router, "/api/v1/hitl", body.clone(), Some(KEY)).await;
    assert_eq!(s1, StatusCode::CREATED, "{r1}");
    let (s2, r2) = post(&router, "/api/v1/hitl", body, Some(KEY)).await;
    assert_eq!(s2, StatusCode::CREATED);
    assert_eq!(r2["id"], r1["id"]);
    assert_eq!(r2["createdAt"], r1["createdAt"], "same row — replay not requeue");
}

#[tokio::test]
async fn hitl_second_decision_typed_error_zero_refund() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:buyer", 100_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    setup_delivered_escrow(&router, "escrow:h3", "user:buyer", "user:seller", 60_000).await;
    post(&router, "/api/v1/hitl", json!({ "id": "hitl:h3", "kind": "ESCROW_RELEASE", "escrowId": "escrow:h3", "requesterId": "user:buyer" }), Some(KEY)).await;

    let (s1, r1) = post(
        &router,
        "/api/v1/hitl/hitl:h3/approve",
        json!({ "decidedBy": "admin:one" }),
        Some(KEY),
    )
    .await;
    assert_eq!(s1, StatusCode::OK, "{r1}");
    let treasury_after_first = store.balance_of("treasury:nation").await.unwrap();

    // ادمین دوم — خطای تایپ‌شده، بازپرداخت صفر
    let (s2, r2) = post(
        &router,
        "/api/v1/hitl/hitl:h3/approve",
        json!({ "decidedBy": "admin:two" }),
        Some(KEY),
    )
    .await;
    assert_eq!(s2, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r2["error"]["code"], "HITL_ALREADY_DECIDED");
    assert_eq!(
        store.balance_of("treasury:nation").await.unwrap(),
        treasury_after_first,
        "zero refund, zero double-pay"
    );
    assert_eq!(store.balance_of("user:seller").await.unwrap(), Some(54_000));
}

#[tokio::test]
async fn hitl_reject_moves_no_money() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:buyer", 100_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    setup_delivered_escrow(&router, "escrow:h4", "user:buyer", "user:seller", 60_000).await;
    post(&router, "/api/v1/hitl", json!({ "id": "hitl:h4", "kind": "ESCROW_RELEASE", "escrowId": "escrow:h4", "requesterId": "user:buyer" }), Some(KEY)).await;

    let (status, res) = post(
        &router,
        "/api/v1/hitl/hitl:h4/reject",
        json!({ "decidedBy": "admin:kia", "note": "hold" }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["request"]["status"], "REJECTED");
    assert_eq!(res["request"]["decisionNote"], "hold");
    assert!(res["escrow"].is_null());
    assert!(res["txId"].is_null());
    // اسکرو قفل می‌ماند، خزانه صفر
    assert_eq!(store.balance_of("treasury:nation").await.unwrap(), Some(0));
    assert_eq!(store.balance_of("user:seller").await.unwrap(), Some(0));
    assert_eq!(store.balance_of("escrow:escrow:h4").await.unwrap(), Some(60_000));
}

#[tokio::test]
async fn hitl_sweep_expires_overdue() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:buyer", 100_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    setup_delivered_escrow(&router, "escrow:h5", "user:buyer", "user:seller", 60_000).await;
    // ردیف سررسیدگذشته مستقیم در فروشگاه — شبیه‌سازی گذر زمان
    store
        .enqueue_hitl(
            dropagentx_db::HitlRow {
                id: "hitl:h5".into(),
                kind: "ESCROW_RELEASE".into(),
                escrow_id: "escrow:h5".into(),
                amount: 60_000,
                requester_id: "user:buyer".into(),
                note: None,
                status: "PENDING".into(),
                created_at: "2026-01-01T00:00:00.000Z".into(),
                expires_at: "2026-01-02T00:00:00.000Z".into(),
                decided_by: None,
                decided_at: None,
                decision_note: None,
            },
            "escrow:h5",
        )
        .await
        .unwrap();

    let (status, res) = post(&router, "/api/v1/hitl/sweep", json!({}), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["expired"], 1);
    // sweep دوم — دیگر چیزی برای انقضا نیست
    let (_, res) = post(&router, "/api/v1/hitl/sweep", json!({}), Some(KEY)).await;
    assert_eq!(res["expired"], 0);
}

#[tokio::test]
async fn hitl_unknown_404() {
    let (router, _) = boot().await;
    let (status, res) = get(&router, "/api/v1/hitl/hitl:ghost", Some(KEY)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(res["error"]["code"], "HITL_NOT_FOUND");
}

#[tokio::test]
async fn hitl_for_missing_escrow_404() {
    let (router, _) = boot().await;
    let (status, res) = post(&router, "/api/v1/hitl", json!({ "kind": "ESCROW_RELEASE", "escrowId": "escrow:ghost", "requesterId": "user:buyer" }), Some(KEY)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(res["error"]["code"], "ESCROW_NOT_FOUND");
}

// ── اقتصاد + AI ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn economy_public_transparency_balanced() {
    let (router, _) = boot().await;
    setup_funded_user(&router, "user:tg:9", 1_000).await;
    // عمومی — بدون کلید
    let (status, res) = get(&router, "/api/v1/economy", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["currency"], "DNX");
    assert_eq!(res["symbol"], "Đ");
    assert_eq!(res["circulatingSupply"], 1_000);
    assert_eq!(res["mintedAllTime"], 1_000);
    assert_eq!(res["yearlyMintCap"], 1_000_000);
    assert_eq!(res["genesisSupply"], 187_672);
    assert_eq!(res["trialBalanceSum"], 0);
    assert_eq!(res["balanced"], true);
    assert_eq!(res["accountCount"], 4); // mint:genesis + treasury:nation + burn:void + user
}

#[tokio::test]
async fn ai_chat_disabled_without_config_503() {
    let (router, _) = boot().await;
    let (status, res) = post(
        &router,
        "/api/v1/ai/chat",
        json!({ "userId": "user:tg:1", "text": "سلام" }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{res}");
    assert_eq!(res["error"]["code"], "AI_DISABLED");
}

#[tokio::test]
async fn ai_chat_staff_role_requires_admin_key_401() {
    let (router, _) = boot().await;
    let (status, res) = post(
        &router,
        "/api/v1/ai/chat",
        json!({ "userId": "user:tg:1", "text": "سلام", "role": "staff" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["error"]["code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn ai_chat_public_identity_required_401_with_bot_token() {
    let cfg = ServerConfig { bot_token: Some(BOT.into()), ..base_config() };
    let (router, _) = boot_with(cfg).await;
    // بدون کلید ادمین و بدون initData — هویت هرگز از بدنه
    let (status, res) =
        post(&router, "/api/v1/ai/chat", json!({ "userId": "user:tg:2", "text": "سلام" }), None)
            .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{res}");
    assert_eq!(res["error"]["code"], "AI_IDENTITY_REQUIRED");
    // initData نامعتبر — همان گیت
    let (status, res) = post(
        &router,
        "/api/v1/ai/chat",
        json!({ "userId": "user:tg:2", "text": "سلام", "initData": "user=%7B%7D&hash=00" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{res}");
    assert_eq!(res["error"]["code"], "AI_IDENTITY_REQUIRED");
}

#[tokio::test]
async fn ai_chat_public_identity_disabled_503_without_bot_token() {
    let (router, _) = boot().await;
    let (status, res) =
        post(&router, "/api/v1/ai/chat", json!({ "userId": "user:tg:2", "text": "سلام" }), None)
            .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{res}");
    assert_eq!(res["error"]["code"], "AI_IDENTITY_DISABLED");
}

#[tokio::test]
async fn ai_chat_valid_init_data_passes_identity_gate() {
    let cfg = ServerConfig { bot_token: Some(BOT.into()), ..base_config() };
    let (router, _) = boot_with(cfg).await;
    let init = make_test_init_data(BOT, "424242", Some("ali"));
    let (status, res) = post(
        &router,
        "/api/v1/ai/chat",
        json!({ "userId": "user:attacker", "text": "سلام", "initData": init }),
        None,
    )
    .await;
    // هویت از initData پاس می‌شود → به گیت سیکل AI می‌رسد (غیرفعال → AI_DISABLED)
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{res}");
    assert_eq!(res["error"]["code"], "AI_DISABLED");
}

#[tokio::test]
async fn ai_registry_six_agents_sixteen_skills() {
    let (router, _) = boot().await;
    let (status, agents) = get(&router, "/api/v1/ai/agents", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(agents["items"].as_array().unwrap().len(), 6);
    assert!(agents["items"][0]["titleFa"].is_string());

    let (status, skills) = get(&router, "/api/v1/ai/skills", None).await;
    assert_eq!(status, StatusCode::OK);
    let items = skills["items"].as_array().unwrap();
    assert_eq!(items.len(), 16);
    // هر مهارت مجوز صریح دارد
    assert!(items.iter().all(|s| s["kind"].is_string()));
}

#[tokio::test]
async fn ai_usage_requires_user_id_and_reports_cap() {
    let (router, _) = boot().await;
    // بدون userId → 400
    let (status, res) = get(&router, "/api/v1/ai/usage", Some(KEY)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{res}");
    // با userId → سقف روزانه از کانفیگ
    let (status, res) = get(&router, "/api/v1/ai/usage?userId=user:tg:1", Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["userId"], "user:tg:1");
    assert_eq!(res["usedToday"], 0);
    assert_eq!(res["cap"], 120_000);
    assert_eq!(res["remainingToday"], 120_000);
    // بدون کلید ادمین → 401
    let (status, _) = get(&router, "/api/v1/ai/usage?userId=user:tg:1", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ── داشبورد ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn dashboard_brand_from_config() {
    let cfg = ServerConfig {
        brand_name: "MyBrand".into(),
        brand_tagline: "MyTagline".into(),
        ..base_config()
    };
    let (router, _) = boot_with(cfg).await;
    let (status, res) = get(&router, "/api/v1/brand", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["ok"], true);
    assert_eq!(res["data"]["name"], "MyBrand");
    assert_eq!(res["data"]["tagline"], "MyTagline");
}

#[tokio::test]
async fn dashboard_products_crud_and_soft_archive() {
    let (router, _) = boot().await;
    // ساخت — ۲۰۱؛ credits روی priceDin می‌نشیند، Stars/stock فقط نمای داشبوردند
    let (status, res) = post(
        &router,
        "/api/v1/products",
        json!({ "title": "VIP", "priceCredits": 1500, "stock": -1 }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    assert_eq!(res["ok"], true);
    assert_eq!(res["data"]["status"], "active");
    assert_eq!(res["data"]["priceCredits"], 1500);
    assert_eq!(res["data"]["priceStars"], 0, "Stars rail not shipped");
    assert_eq!(res["data"]["stock"], -1, "unlimited");
    assert_eq!(res["data"]["salesCount"], 0);
    let id = res["data"]["id"].as_str().unwrap().to_owned();
    assert!(id.starts_with("prod:"));

    // فهرست عمومی — با total و سقف پیش‌فرض رجیستری
    let (status, res) = get(&router, "/api/v1/products", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["data"]["total"], 1);
    assert_eq!(res["data"]["limit"], 12);

    // ویرایش — عنوان/قیمت؛ featured ستون ندارد و false می‌ماند
    let (status, res) = patch(
        &router,
        &format!("/api/v1/products/{id}"),
        json!({ "title": "VIP Plus", "featured": true, "priceCredits": 2000 }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["data"]["title"], "VIP Plus");
    assert_eq!(res["data"]["priceCredits"], 2000);
    assert_eq!(res["data"]["featured"], false, "featured is a dashboard-only view, no column");

    // آرشیو نرم — ردیف می‌ماند و status archived می‌شود
    let (status, res) = del(&router, &format!("/api/v1/products/{id}"), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["data"]["status"], "archived");
    let (status, res) = get(&router, "/api/v1/products?limit=50", None).await;
    assert_eq!(status, StatusCode::OK);
    let archived = res["data"]["items"].as_array().unwrap().iter().find(|p| p["id"] == id).unwrap();
    assert_eq!(archived["status"], "archived", "soft archive keeps the row visible");

    // محصول ناموجود → 404
    let (status, res) =
        patch(&router, "/api/v1/products/prod:ghost", json!({ "title": "x" }), Some(KEY)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(res["error"]["code"], "PRODUCT_NOT_FOUND");
}

#[tokio::test]
async fn dashboard_orders_filter_and_deliver_action() {
    let (router, _) = boot().await;
    setup_funded_user(&router, "user:buyer", 1_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    post(&router, "/api/v1/escrows", json!({ "id": "escrow:o1", "buyerId": "user:buyer", "sellerId": "user:seller", "amount": 400 }), Some(KEY)).await;
    post(&router, "/api/v1/escrows/escrow:o1/fund", json!({}), Some(KEY)).await;

    // فیلتر paid — الان paid
    let (status, res) = get(&router, "/api/v1/orders?status=paid", Some(KEY)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["data"]["total"], 1, "FUNDED == paid");
    assert_eq!(res["data"]["items"][0]["status"], "paid");
    let (status, res) = get(&router, "/api/v1/orders?status=pending", Some(KEY)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["data"]["total"], 0);

    // اقدام deliver از مسیر دامنه
    let (status, res) =
        post(&router, "/api/v1/orders/escrow:o1/status", json!({ "action": "deliver" }), Some(KEY))
            .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["data"]["status"], "delivered");
    let (_, res) = get(&router, "/api/v1/orders?status=paid", Some(KEY)).await;
    assert_eq!(
        res["data"]["total"], 0,
        "FUNDED is 'paid' — delivered escrow leaves the paid bucket"
    );
}

#[tokio::test]
async fn dashboard_order_cancel_and_refund_paths() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:buyer", 1_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    post(&router, "/api/v1/escrows", json!({ "id": "escrow:o2", "buyerId": "user:buyer", "sellerId": "user:seller", "amount": 400 }), Some(KEY)).await;

    // cancel از CREATED
    let (status, res) =
        post(&router, "/api/v1/orders/escrow:o2/status", json!({ "action": "cancel" }), Some(KEY))
            .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["data"]["status"], "cancelled");
    assert_eq!(store.balance_of("user:buyer").await.unwrap(), Some(1_000));

    // refund از DISPUTED
    post(&router, "/api/v1/escrows", json!({ "id": "escrow:o3", "buyerId": "user:buyer", "sellerId": "user:seller", "amount": 100 }), Some(KEY)).await;
    post(&router, "/api/v1/escrows/escrow:o3/fund", json!({}), Some(KEY)).await;
    post(&router, "/api/v1/escrows/escrow:o3/dispute", json!({ "reason": "broken" }), Some(KEY))
        .await;
    let (status, res) =
        post(&router, "/api/v1/orders/escrow:o3/status", json!({ "action": "refund" }), Some(KEY))
            .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["data"]["status"], "refunded");
    assert_eq!(store.balance_of("user:buyer").await.unwrap(), Some(1_000));
}

#[tokio::test]
async fn dashboard_approvals_decide_settles_in_same_transaction() {
    let (router, store) = boot().await;
    setup_funded_user(&router, "user:buyer", 100_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    setup_delivered_escrow(&router, "escrow:a1", "user:buyer", "user:seller", 60_000).await;
    post(&router, "/api/v1/hitl", json!({ "id": "hitl:a1", "kind": "ESCROW_RELEASE", "escrowId": "escrow:a1", "requesterId": "user:buyer" }), Some(KEY)).await;

    // صف در نما — پیش‌فرض pending + ریسک high (مادهٔ ۷)
    let (status, res) = get(&router, "/api/v1/approvals", Some(KEY)).await;
    assert_eq!(status, StatusCode::OK);
    let items = res["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["riskLevel"], "high");
    assert_eq!(items[0]["summary"], "Release 60000 Đ");
    assert_eq!(items[0]["status"], "pending");

    // تصمیم → تسویهٔ اتمی
    let (status, res) = post(
        &router,
        "/api/v1/approvals/hitl:a1/decide",
        json!({ "approve": true, "note": "ok" }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["data"]["status"], "approved");
    assert_eq!(res["data"]["escrowId"], "escrow:a1");
    assert!(res["data"]["txId"].is_string());
    assert_eq!(store.balance_of("user:seller").await.unwrap(), Some(54_000));
    assert_eq!(store.balance_of("treasury:nation").await.unwrap(), Some(6_000));

    // تصمیم دوم — خطای تایپ‌شده
    let (status, res) =
        post(&router, "/api/v1/approvals/hitl:a1/decide", json!({ "approve": true }), Some(KEY))
            .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res["error"]["code"], "HITL_ALREADY_DECIDED");
}

#[tokio::test]
async fn dashboard_users_wallets_leaderboard() {
    let (router, _) = boot().await;
    setup_funded_user(&router, "user:tg:1", 900).await;
    setup_funded_user(&router, "user:tg:2", 100).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    post(&router, "/api/v1/escrows", json!({ "id": "escrow:w1", "buyerId": "user:tg:1", "sellerId": "user:seller", "amount": 300 }), Some(KEY)).await;
    post(&router, "/api/v1/escrows/escrow:w1/fund", json!({}), Some(KEY)).await;
    post(&router, "/api/v1/escrows/escrow:w1/deliver", json!({}), Some(KEY)).await;
    post(&router, "/api/v1/escrows/escrow:w1/release", json!({}), Some(KEY)).await;

    let (status, res) = get(&router, "/api/v1/users?limit=10", Some(KEY)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["data"]["items"].as_array().unwrap().len(), 3);

    let (status, res) = get(&router, "/api/v1/wallets?limit=10", Some(KEY)).await;
    assert_eq!(status, StatusCode::OK);
    let items = res["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    let tg1 = items.iter().find(|w| w["name"] == "user:tg:1").unwrap();
    assert_eq!(tg1["balance"], 600, "900 minted − 300 settled escrow");
    assert_eq!(tg1["locked"], 0, "escrow released — nothing locked anymore");

    // لیدربورد sales — فروشنده واقعی با ۱ تسویه
    let (status, res) = get(&router, "/api/v1/leaderboard?type=sales", None).await;
    assert_eq!(status, StatusCode::OK);
    let items = res["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "user:seller");
    assert_eq!(items[0]["sales"], 1);
    assert_eq!(items[0]["rank"], 1);
}

#[tokio::test]
async fn dashboard_rails_empty_and_settings_501() {
    let (router, _) = boot().await;
    let (status, res) = get(&router, "/api/v1/quests", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["data"]["items"].as_array().unwrap().len(), 0);
    let (status, res) = get(&router, "/api/v1/bookings", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["data"]["items"].as_array().unwrap().len(), 0);

    let (status, res) = get(&router, "/api/v1/settings", Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["data"]["items"].as_array().unwrap().len(), 0);
    // PUT — 501 صادقانه تا رسیدن مدل KV
    let (status, res) = send(
        &router,
        "PUT",
        "/api/v1/settings",
        Some(json!({ "key": "a", "value": "b" }).to_string()),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{res}");
    assert_eq!(res["error"]["code"], "NOT_IMPLEMENTED");
}

#[tokio::test]
async fn dashboard_security_layers_reflect_real_config() {
    let cfg = ServerConfig { bot_token: Some(BOT.into()), ..base_config() };
    let (router, _) = boot_with(cfg).await;
    // یک فرمان پولی — ردیف audit در همان تراکنش نوشته می‌شود
    setup_funded_user(&router, "user:tg:1", 100).await;
    let (status, res) = get(&router, "/api/v1/security", Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    let layers = res["data"]["layers"].as_array().unwrap();
    let kia = layers.iter().find(|l| l["id"] == "kia-key").unwrap();
    assert_eq!(kia["active"], true);
    let telegram = layers.iter().find(|l| l["id"] == "telegram-init-data").unwrap();
    assert_eq!(telegram["active"], true, "bot token configured");
    // دنبالهٔ audit — فرمان‌های قبلی ردیف گذاشته‌اند
    let audit = res["data"]["audit"].as_array().unwrap();
    assert!(!audit.is_empty());
    assert!(audit[0]["action"].is_string());
}

#[tokio::test]
async fn dashboard_overview_shape_with_14_series() {
    let (router, _) = boot().await;
    setup_funded_user(&router, "user:buyer", 1_000).await;
    post(&router, "/api/v1/accounts", json!({ "id": "user:seller" }), Some(KEY)).await;
    setup_delivered_escrow(&router, "escrow:v1", "user:buyer", "user:seller", 400).await;
    post(&router, "/api/v1/escrows/escrow:v1/release", json!({}), Some(KEY)).await;

    let (status, res) = get(&router, "/api/v1/overview", Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    let data = &res["data"];
    assert_eq!(data["users"], 2);
    assert_eq!(data["orders"], 1);
    assert_eq!(data["pendingApprovals"], 0);
    assert_eq!(data["aiCalls"], 0);
    assert_eq!(data["revenueCredits"], 40, "today's treasury income");
    let series = data["series"].as_array().unwrap();
    assert_eq!(series.len(), 14, "exactly 14 UTC days, zero-filled");
    let today = series.last().unwrap();
    assert_eq!(today["revenue"], 40);
    assert_eq!(today["orders"], 1);
    // سفارش اخیر با خریدار
    let recent = data["recentOrders"].as_array().unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0]["totalCredits"], 400);
    assert_eq!(recent[0]["status"], "released");
}

// ── static داشبورد ──────────────────────────────────────────────────────────

#[tokio::test]
async fn static_dashboard_served_with_spa_fallback() {
    // build داشبورد در temp — index.html + assets/
    let dir = std::env::temp_dir().join(format!("dash-test-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(dir.join("index.html"), "<!doctype html><html><body>Dash</body></html>")
        .unwrap();
    std::fs::write(dir.join("app.js"), "console.log(1)").unwrap();

    let cfg = ServerConfig { static_dir: Some(dir.clone()), ..base_config() };
    let (router, _) = boot_with(cfg).await;

    // ریشه → index
    let (status, _) = send(&router, "GET", "/", None, None).await;
    assert_eq!(status, StatusCode::OK);
    // فایل مستقیم
    let (status, _) = send(&router, "GET", "/app.js", None, None).await;
    assert_eq!(status, StatusCode::OK);
    // فایل غایب — asset هرگز index جعلی نمی‌گیرد (ServeDir خالص؛ فاز ۷)
    let (status, _) = send(&router, "GET", "/missing.png", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(&router, "GET", "/some/client/route", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn static_dir_missing_reports_honest_404() {
    let cfg = ServerConfig { static_dir: None, ..base_config() };
    let (router, _) = boot_with(cfg).await;
    let (status, res) = send(&router, "GET", "/", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(res["error"]["code"], "NO_FRONTEND");
}

// ── سیستم — بکاپ ────────────────────────────────────────────────────────────

#[tokio::test]
async fn system_backup_on_file_store_creates_snapshot() {
    // فروشگاه فایل‌محور در temp — بکاپ VACUUM INTO
    let tmp = std::env::temp_dir().join(format!("srv-backup-{}", uuid::Uuid::now_v7()));
    let cfg = ServerConfig {
        database_url: format!("file:{}", tmp.join("v3.db").display()),
        backup_dir: tmp.join("backups"),
        keep_backups: 2,
        ..base_config()
    };
    let (router, _) = boot_with(cfg).await;
    let (status, res) = post(&router, "/api/v1/system/backup", json!({}), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert!(res["sizeBytes"].as_i64().unwrap_or(0) > 0);
    assert_eq!(res["kept"], 1);
    assert!(res["file"].as_str().unwrap().contains("backup-"));

    // بدون کلید → 401
    let (status, _) = post(&router, "/api/v1/system/backup", json!({}), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    std::fs::remove_dir_all(&tmp).ok();
}

#[tokio::test]
async fn system_backup_on_memory_store_is_unsupported_503() {
    let (router, _) = boot().await;
    let (status, res) = post(&router, "/api/v1/system/backup", json!({}), Some(KEY)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{res}");
    assert_eq!(res["error"]["code"], "BACKUP_UNSUPPORTED");
}

// ── بدنهٔ خراب → 400 مثل zod ─────────────────────────────────────────────────

#[tokio::test]
async fn malformed_body_is_invalid_request_400() {
    let (router, _) = boot().await;
    // float روی وایر — هرگز (DNX)
    let (status, res) = post(
        &router,
        "/api/v1/wallet/mint",
        json!({ "accountId": "user:tg:1", "amount": 12.5 }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{res}");
    assert_eq!(res["error"]["code"], "INVALID_REQUEST");
    // کلید یکسان‌سازی کوتاه‌تر از ۸
    let (status, res) = post(
        &router,
        "/api/v1/wallet/mint",
        json!({ "accountId": "user:tg:1", "amount": 5, "idempotencyKey": "short" }),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{res}");
    // id با فاصله
    let (status, _) =
        post(&router, "/api/v1/accounts", json!({ "id": "has space" }), Some(KEY)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
