//! تست‌های انتگراسیون گیت‌وی AI (فاز ۶) — provider درون‌پردازشی، بدون شبکه،
//! روی SQLite واقعی با `oneshot` (معادل `app.inject`).
//!
//! پوشش معیارهای پذیرش ADR-009 فاز ۶ در لایهٔ سیم:
//! ۱. بدون گیت‌وی نصب‌شده → 503 `AI_DISABLED`
//! ۲. پاسخ سالم reply → 200 + audit `ai.ok` در AuditLog
//! ۳. قطع primary → secondary جواب می‌دهد (نام برنده در پاسخ)
//! ۴. سقف تمام → 429 `AI_CAP_EXCEEDED` **قبل از provider**
//! ۵. ابزار wallet.balance اجرا می‌شود + ردیف AiUsage ثبت می‌شود
//! ۶. escrow_refund ۶۰٬۰۰۰ Đ → ردیف HITL با اسکرو هنوز FUNDED + audit `ai.pending`
//! ۷. redaction روی سیم با شمارش در پاسخ و audit
//! ۸. گیت staff بدون کلید ادمین → 401
//! ۹. هویت بدون کلید و بدون initData → 401 `AI_IDENTITY_REQUIRED`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use dropagentx_ai::{FakeProvider, Gateway};
use dropagentx_db::V3Store;
use dropagentx_server::{build_router, AppState, ServerConfig};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const KEY: &str = "test-admin-key-0123456789";
const CAP: i64 = 1_000_000;

fn ai_config(cap: i64) -> ServerConfig {
    ServerConfig {
        admin_key: Some(KEY.into()),
        database_url: "sqlite::memory:".into(),
        static_dir: None,
        ai_daily_token_cap: cap,
        bot_token: Some("7000000000:TEST-TOKEN-ai-gateway".into()),
        ..ServerConfig::default()
    }
}

async fn boot_with_gateway(
    cap: i64,
    primary: &FakeProvider,
    secondary: Option<&FakeProvider>,
) -> (Router, V3Store) {
    let cfg = ai_config(cap);
    let store = V3Store::connect(&cfg.database_url).await.unwrap();
    store.migrate().await.unwrap();
    store.ensure_system_accounts().await.unwrap();
    let mut slots = vec![dropagentx_ai::AnyProvider::Fake(primary.clone())];
    if let Some(s) = secondary {
        slots.push(dropagentx_ai::AnyProvider::Fake(s.clone()));
    }
    let gateway = Gateway::new(slots, cap);
    let state = AppState::new(store.clone(), cfg).with_ai_gateway(gateway).shared();
    (build_router(state), store)
}

async fn boot_without_gateway() -> (Router, V3Store) {
    let cfg = ai_config(CAP);
    let store = V3Store::connect(&cfg.database_url).await.unwrap();
    store.migrate().await.unwrap();
    store.ensure_system_accounts().await.unwrap();
    let state = AppState::new(store.clone(), cfg).shared();
    (build_router(state), store)
}

async fn post_ai(router: &Router, body: Value, key: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("POST").uri("/api/v1/ai/chat");
    if let Some(k) = key {
        builder = builder.header("x-kia-key", k);
    }
    let req = builder
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

async fn setup_user(router: &Router, tg: i64, amount: i64) {
    let id = format!("user:tg:{tg}");
    let (status, res) = post_admin(router, "/api/v1/accounts", json!({"id": id})).await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    let (status, res) =
        post_admin(router, "/api/v1/wallet/mint", json!({"accountId": id, "amount": amount})).await;
    assert_eq!(status, StatusCode::OK, "{res}");
}

async fn setup_funded_escrow(router: &Router, id: &str, buyer: &str, seller: &str, amount: i64) {
    let (status, res) = post_admin(
        router,
        "/api/v1/escrows",
        json!({"id": id, "buyerId": buyer, "sellerId": seller, "amount": amount}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    let (status, res) = post_admin(router, &format!("/api/v1/escrows/{id}/fund"), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{res}");
}

async fn post_admin(router: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("x-kia-key", KEY)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn chat_body(user: &str, text: &str, role: &str) -> Value {
    json!({"userId": user, "text": text, "role": role, "language": "fa"})
}

async fn audit_rows(store: &V3Store, actor: &str) -> Vec<(String, String)> {
    let rows = store.list_audit(50).await.unwrap();
    rows.iter()
        .filter(|r| r.actor == actor)
        .map(|r| (r.action.clone(), r.meta.clone().unwrap_or_default()))
        .collect()
}

// ── گیت‌ها ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn ai_chat_without_gateway_is_ai_disabled_503() {
    let (router, _store) = boot_without_gateway().await;
    let (status, res) = post_ai(&router, chat_body("user:tg:1001", "سلام", "user"), Some(KEY)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(res["error"]["code"], "AI_DISABLED");
}

#[tokio::test]
async fn identity_without_key_or_init_data_is_401() {
    let fake = FakeProvider::new("primary", "fake", "m");
    let (router, _store) = boot_with_gateway(CAP, &fake, None).await;
    let (status, res) = post_ai(&router, chat_body("user:tg:1001", "سلام", "user"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["error"]["code"], "AI_IDENTITY_REQUIRED", "{res}");
}

#[tokio::test]
async fn staff_role_without_admin_key_is_401() {
    let fake = FakeProvider::new("primary", "fake", "m");
    let (router, _store) = boot_with_gateway(CAP, &fake, None).await;
    let (status, res) = post_ai(&router, chat_body("user:tg:1001", "سلام", "staff"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(res["error"]["code"], "UNAUTHORIZED");
}

// ── پاسخ سالم + ممیزی ───────────────────────────────────────────────────────

#[tokio::test]
async fn happy_reply_path_returns_200_and_audits_ai_ok() {
    let fake = FakeProvider::new("primary", "fake-openai", "model-x");
    fake.push_ok(r#"{"reply":"سلام! من پذیرش کشور هستم."}"#);
    let (router, store) = boot_with_gateway(CAP, &fake, None).await;
    setup_user(&router, 1001, 5_000).await;

    let (status, res) = post_ai(&router, chat_body("user:tg:1001", "سلام", "user"), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["reply"], "سلام! من پذیرش کشور هستم.");
    assert_eq!(res["agent"], "receptionist", "سلام → پذیرش");
    assert_eq!(res["via"], "llm");
    assert_eq!(res["provider"], "fake-openai");
    assert_eq!(res["redactions"], 0);
    assert_eq!(res["pendingApproval"], false);
    assert!(res["usage"]["totalTokens"].as_i64().unwrap() > 0);

    // ممیزی ai.ok با زمینهٔ کامل
    let rows = audit_rows(&store, "user:tg:1001").await;
    let (action, meta) = rows.iter().find(|(a, _)| a == "ai.ok").expect("ai.ok row");
    assert_eq!(*action, "ai.ok".to_string());
    assert!(meta.contains("fake-openai"), "{meta}");
    assert!(meta.contains("redactions"), "{meta}");
}

// ── fallback قطعی ───────────────────────────────────────────────────────────

#[tokio::test]
async fn primary_down_secondary_answers_on_wire() {
    let primary = FakeProvider::new("primary", "fake", "p");
    let secondary = FakeProvider::new("secondary", "fake-2", "s");
    primary.push_err(dropagentx_ai::LlmError::Down);
    secondary.push_ok(r#"{"reply":"از اسلات دوم جواب می‌دهم"}"#);
    let (router, _store) = boot_with_gateway(CAP, &primary, Some(&secondary)).await;
    setup_user(&router, 1001, 5_000).await;

    let (status, res) = post_ai(&router, chat_body("user:tg:1001", "سلام", "user"), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["provider"], "fake-2", "نام برنده در پاسخ");
}

#[tokio::test]
async fn all_slots_down_is_502_upstream() {
    let primary = FakeProvider::new("primary", "fake", "p");
    let secondary = FakeProvider::new("secondary", "fake", "s");
    primary.push_err(dropagentx_ai::LlmError::Down);
    secondary.push_err(dropagentx_ai::LlmError::Timeout);
    let (router, _store) = boot_with_gateway(CAP, &primary, Some(&secondary)).await;
    let (status, res) = post_ai(&router, chat_body("user:tg:1001", "سلام", "user"), Some(KEY)).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(res["error"]["code"], "AI_UPSTREAM");
}

// ── سقف روزانه — قبل از provider ────────────────────────────────────────────

#[tokio::test]
async fn cap_exceeded_is_429_before_provider() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"reply":"اول"}"#);
    let (router, _store) = boot_with_gateway(50, &fake, None).await;
    setup_user(&router, 1001, 5_000).await;

    let (status, res) = post_ai(&router, chat_body("user:tg:1001", "سلام", "user"), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(fake.call_count(), 1);
    let used = res["usage"]["totalTokens"].as_i64().unwrap();

    // فراخوانی دوم — بالای سقف ۵۰ (اگر used >= 50؛ فراخوانی اول کمی مصرف دارد)
    let (status, res) = post_ai(&router, chat_body("user:tg:1001", "سلام", "user"), Some(KEY)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{res}");
    assert_eq!(res["error"]["code"], "AI_CAP_EXCEEDED");
    assert_eq!(res["error"]["meta"]["limit"], json!(50));
    assert!(res["error"]["meta"]["used"].as_i64().unwrap() >= used);
    assert_eq!(fake.call_count(), 1, "سقف قبل از provider — فراخوانی دوم نشد");
}

// ── ابزارها ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn tool_wallet_balance_executes_and_records_usage() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"wallet.balance","args":{}}"#);
    let (router, store) = boot_with_gateway(CAP, &fake, None).await;
    setup_user(&router, 1001, 42_000).await;

    let (status, res) =
        post_ai(&router, chat_body("user:tg:1001", "موجودی", "user"), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    let trace = &res["toolTraces"][0];
    assert_eq!(trace["skill"], "wallet.balance");
    assert_eq!(trace["ok"], true);
    assert_eq!(trace["pendingApproval"], false);
    assert!(trace["code"].is_null());
    assert!(res["reply"].as_str().unwrap().contains("42,000"), "{res}");

    // ردیف AiUsage ثبت شد (مصرف واقعی برای سقف)
    let (used, calls) = store.ai_tokens_today("user:tg:1001").await.unwrap();
    assert!(used > 0 && calls == 1, "used={used} calls={calls}");
}

#[tokio::test]
async fn escrow_refund_60k_queues_hitl_and_escrow_stays_funded() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"escrow_refund","args":{"escrowId":"esc:e1"}}"#);
    let (router, store) = boot_with_gateway(CAP, &fake, None).await;
    setup_user(&router, 1001, 60_000).await;
    setup_user(&router, 2002, 1_000).await;
    setup_funded_escrow(&router, "esc:e1", "user:tg:1001", "user:tg:2002", 60_000).await;

    let (status, res) =
        post_ai(&router, chat_body("user:tg:1001", "بازپرداخت", "staff"), Some(KEY)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["pendingApproval"], true, "{res}");
    let trace = &res["toolTraces"][0];
    assert_eq!(trace["skill"], "escrow_refund");
    assert_eq!(trace["pendingApproval"], true);
    assert_eq!(trace["code"], "REQUIRES_HUMAN_APPROVAL");

    // ردیف HITL در صف سرور — اسکرو هنوز FUNDED
    let queue = store.list_hitl(Some("PENDING")).await.unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].kind, "ESCROW_REFUND");
    assert_eq!(queue[0].escrow_id, "esc:e1");
    assert_eq!(queue[0].amount, 60_000);
    let escrow = store.get_escrow("esc:e1").await.unwrap().unwrap();
    assert_eq!(escrow.status.as_str(), "FUNDED", "اسکرو هنوز FUNDED");

    // ممیزی ai.pending
    let rows = audit_rows(&store, "user:tg:1001").await;
    assert!(rows.iter().any(|(a, _)| a == "ai.pending"));
}

// ── redaction روی سیم ───────────────────────────────────────────────────────

#[tokio::test]
async fn redaction_counts_on_wire_and_never_leaks_to_provider() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"reply":"received your secret sk-abcdefghijklmnop1234 — oh wait"}"#);
    let (router, store) = boot_with_gateway(CAP, &fake, None).await;
    setup_user(&router, 1001, 5_000).await;

    let (status, res) = post_ai(
        &router,
        chat_body("user:tg:1001", "my key sk-abcdefghijklmnop1234 and Bearer qq-token-99", "user"),
        Some(KEY),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    // ورودی: ۲ قاعده + خروجی: ۱ قاعده = ۳
    assert_eq!(res["redactions"], 3, "{res}");
    assert!(!res["reply"].as_str().unwrap().contains("sk-abcdefghijklmnop"));

    // پرامپتِ واقعاً فرستاده‌شده به provider پاک است
    let (_, user_prompt) = fake.last_call().expect("recorded");
    assert!(user_prompt.contains("[REDACTED:API_KEY]"));
    assert!(!user_prompt.contains("sk-abcdefghijklmnop"));

    // شمارش در audit
    let rows = audit_rows(&store, "user:tg:1001").await;
    let (_, meta) = rows.iter().find(|(a, _)| a == "ai.ok").expect("ai.ok row");
    assert!(meta.contains("\"redactions\":3"), "{meta}");
}
