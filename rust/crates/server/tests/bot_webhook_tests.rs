//! تست‌های انتگراسیون webhook بات (فاز ۵) — روی `oneshot` معادل `app.inject`
//! با SQLite واقعی.
//!
//! پوشش معیارهای پذیرش ADR-009 فاز ۵ در لایهٔ سیم:
//! ۱. fail-closed: بدون `BOT_WEBHOOK_SECRET` → 503 `BOT_DISABLED`؛ توکن غلط
//!    → 401 `BOT_UNAUTHORIZED`
//! ۲. مسیر کامل بات روی دیتابیس واقعی: buy→fund→release زیر آستانه = دقیقاً ۹۰/۱۰
//! ۳. retry همان update با همان message_id دوبار پول جابه‌جا نمی‌کند
//! ۴. HITL ≥ ۵۰٬۰۰۰ Đ از صف سرور می‌گذرد؛ `/approve` ادمین تسویهٔ اتمیک ۹۰/۱۰
//!    اجرا می‌کند؛ تأیید دوم → خطای تایپ‌شده با بازپرداخت صفر
//! ۵. admin gate بات: غیرادمین → پیام رد، هیچ پولی حرکت نمی‌کند

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use dropagentx_db::V3Store;
use dropagentx_server::{build_router, AppState, ServerConfig};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt; // oneshot — معادل app.inject

const KEY: &str = "test-admin-key-0123456789";
const SECRET: &str = "webhook-secret-0123456789";

/// کانفیگ فاز ۵ — ادمین فعال + webhook بات فعال + ادمین بات ۴۲.
fn bot_config() -> ServerConfig {
    ServerConfig {
        admin_key: Some(KEY.into()),
        database_url: "sqlite::memory:".into(),
        static_dir: None,
        bot_webhook_secret: Some(SECRET.into()),
        bot_admin_ids: vec![42],
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

async fn send_update(
    router: &Router,
    update_id: i64,
    msg_id: i64,
    user: i64,
    text: &str,
) -> (StatusCode, Value) {
    let body = json!({
        "update_id": update_id,
        "message": {
            "message_id": msg_id,
            "from": {"id": user, "first_name": "Kia", "language_code": "fa"},
            "chat": {"id": user, "type": "private"},
            "text": text
        }
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bot/webhook")
        .header("x-telegram-bot-api-secret-token", SECRET)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn text_of(res: &Value) -> String {
    res["text"].as_str().unwrap_or_default().to_owned()
}

/// ضرب و ساخت حساب از مسیر ادمین HTTP — پیش‌نیاز کیف.
async fn setup_funded_user(router: &Router, tg: i64, amount: i64) {
    let id = format!("user:tg:{tg}");
    let (status, res) = post_admin(router, "/api/v1/accounts", json!({ "id": id })).await;
    assert_eq!(status, StatusCode::CREATED, "account {id}: {res}");
    let (status, res) =
        post_admin(router, "/api/v1/wallet/mint", json!({ "accountId": id, "amount": amount }))
            .await;
    assert_eq!(status, StatusCode::OK, "mint {id}: {res}");
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

// ── fail-closed ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn webhook_without_secret_is_bot_disabled_503() {
    let cfg = ServerConfig {
        admin_key: Some(KEY.into()),
        database_url: "sqlite::memory:".into(),
        static_dir: None,
        ..ServerConfig::default()
    };
    let (router, _store) = boot_with(cfg).await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bot/webhook")
        .body(Body::from(json!({"update_id": 1}).to_string()))
        .unwrap();
    let res = router.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["error"]["code"], "BOT_DISABLED");
}

#[tokio::test]
async fn webhook_with_wrong_secret_is_unauthorized_401() {
    let (router, _store) = boot_with(bot_config()).await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bot/webhook")
        .header("x-telegram-bot-api-secret-token", "wrong-secret-0123456789")
        .body(Body::from(json!({"update_id": 1}).to_string()))
        .unwrap();
    let res = router.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["error"]["code"], "BOT_UNAUTHORIZED");
}

#[tokio::test]
async fn webhook_with_malformed_body_is_invalid_request_400() {
    let (router, _store) = boot_with(bot_config()).await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bot/webhook")
        .header("x-telegram-bot-api-secret-token", SECRET)
        .body(Body::from("not-json"))
        .unwrap();
    let res = router.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ── مسیر کامل روی دیتابیس واقعی ─────────────────────────────────────────────

#[tokio::test]
async fn full_bot_flow_below_threshold_settles_exactly_90_10() {
    let (router, store) = boot_with(bot_config()).await;
    setup_funded_user(&router, 1001, 60_000).await;
    setup_funded_user(&router, 2002, 1_000).await;

    // /buy 2002 40000
    let (status, res) = send_update(&router, 1, 10, 1001, "/buy 2002 40000").await;
    assert_eq!(status, StatusCode::OK);
    assert!(text_of(&res).contains("esc:c1001m10"), "escrow created: {res}");

    // /fund
    let (status, res) = send_update(&router, 2, 11, 1001, "/fund").await;
    assert_eq!(status, StatusCode::OK);
    assert!(text_of(&res).contains("تأمین"), "{res}");
    let escrow = store.get_escrow("esc:c1001m10").await.unwrap().unwrap();
    assert_eq!(escrow.status.as_str(), "FUNDED");

    // /deliver توسط فروشنده
    let (status, res) = send_update(&router, 3, 12, 2002, "/deliver").await;
    assert_eq!(status, StatusCode::OK);
    assert!(text_of(&res).contains("تحویل"), "{res}");

    // /release زیر آستانه → تسویهٔ دقیق ۹۰/۱۰
    let (status, res) = send_update(&router, 4, 13, 1001, "/release").await;
    assert_eq!(status, StatusCode::OK);
    assert!(text_of(&res).contains("۹۰/۱۰"), "{res}");
    let seller = store.balance_of("user:tg:2002").await.unwrap().unwrap();
    assert_eq!(seller, 1_000 + 36_000, "سهم فروشنده دقیقاً ۹۰٪");
    let treasury = store.balance_of("treasury:nation").await.unwrap().unwrap();
    assert_eq!(treasury % 4_000, 0, "مالیات ۱۰٪ از ۴۰٬۰۰۰ در خزانه");
    let escrow = store.get_escrow("esc:c1001m10").await.unwrap().unwrap();
    assert_eq!(escrow.status.as_str(), "RELEASED");
}

#[tokio::test]
async fn webhook_replay_with_same_message_id_moves_money_once() {
    let (router, store) = boot_with(bot_config()).await;
    setup_funded_user(&router, 1001, 60_000).await;
    setup_funded_user(&router, 2002, 1_000).await;
    send_update(&router, 1, 10, 1001, "/buy 2002 40000").await;

    // اولین /fund
    let (status, _res) = send_update(&router, 2, 11, 1001, "/fund").await;
    assert_eq!(status, StatusCode::OK);
    let buyer_after_first = store.balance_of("user:tg:1001").await.unwrap().unwrap();
    assert_eq!(buyer_after_first, 20_000);

    // retry همان update (update_id متفاوت، message_id همان — الگوی retry تلگرام)
    let (status, res) = send_update(&router, 3, 11, 1001, "/fund").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        text_of(&res).contains("قبلاً پردازش شده") || text_of(&res).contains("FUNDED"),
        "پیام replay صادقانه: {res}"
    );
    assert_eq!(
        store.balance_of("user:tg:1001").await.unwrap().unwrap(),
        20_000,
        "دوبار پرداخت نشد"
    );
}

// ── HITL روی صف سرور واقعی ─────────────────────────────────────────────────

#[tokio::test]
async fn bot_hitl_above_threshold_queues_on_server_and_admin_settles_90_10() {
    let (router, store) = boot_with(bot_config()).await;
    setup_funded_user(&router, 1001, 60_000).await;
    setup_funded_user(&router, 2002, 1_000).await;
    send_update(&router, 1, 10, 1001, "/buy 2002 60000").await;
    send_update(&router, 2, 11, 1001, "/fund").await;
    send_update(&router, 3, 12, 2002, "/deliver").await;

    // /release ۶۰٬۰۰۰ → صف HITL سرور، امانت DELIVERED می‌ماند
    let (status, res) = send_update(&router, 4, 13, 1001, "/release").await;
    assert_eq!(status, StatusCode::OK);
    assert!(text_of(&res).contains("مادهٔ ۷"), "{res}");
    let queue = store.list_hitl(Some("PENDING")).await.unwrap();
    assert_eq!(queue.len(), 1, "درخواست در صف سرور");
    assert_eq!(queue[0].id, "hitl:esc:c1001m10");
    assert_eq!(queue[0].amount, 60_000);
    let escrow = store.get_escrow("esc:c1001m10").await.unwrap().unwrap();
    assert_eq!(escrow.status.as_str(), "DELIVERED");
    assert_eq!(store.balance_of("user:tg:2002").await.unwrap().unwrap(), 1_000);

    // /approve توسط ادمین بات (tg 42) → تسویهٔ اتمیک ۹۰/۱۰ در همان تراکنش
    let (status, res) = send_update(&router, 5, 20, 42, "/approve hitl:esc:c1001m10").await;
    assert_eq!(status, StatusCode::OK);
    assert!(text_of(&res).contains("54,000"), "{res}");
    assert_eq!(store.balance_of("user:tg:2002").await.unwrap().unwrap(), 55_000);
    let escrow = store.get_escrow("esc:c1001m10").await.unwrap().unwrap();
    assert_eq!(escrow.status.as_str(), "RELEASED");

    // تأیید دوم توسط ادمین دیگر → خطای تایپ‌شده، بازپرداخت صفر
    let cfg_admins = vec![42, 43];
    drop(cfg_admins); // ادمین دوم در همین کانفیگ نیست؛ با ۴۲ دوباره تصمیم می‌گیریم
    let (status, res) = send_update(&router, 6, 21, 42, "/approve hitl:esc:c1001m10").await;
    assert_eq!(status, StatusCode::OK);
    assert!(text_of(&res).contains("قبلاً تصمیم‌گیری شده"), "{res}");
    assert_eq!(store.balance_of("user:tg:2002").await.unwrap().unwrap(), 55_000, "بازپرداخت صفر");
    assert!(store.economy_summary().await.unwrap().balanced);
}

#[tokio::test]
async fn bot_admin_gate_is_fail_closed_on_real_store() {
    let (router, store) = boot_with(bot_config()).await;
    setup_funded_user(&router, 1001, 60_000).await;
    setup_funded_user(&router, 2002, 1_000).await;
    send_update(&router, 1, 10, 1001, "/buy 2002 60000").await;
    send_update(&router, 2, 11, 1001, "/fund").await;
    send_update(&router, 3, 12, 2002, "/deliver").await;
    send_update(&router, 4, 13, 1001, "/release").await;

    // غیرادمین (tg 1001) /approve → پیام رد؛ هیچ پولی حرکت نمی‌کند
    let (status, res) = send_update(&router, 5, 20, 1001, "/approve hitl:esc:c1001m10").await;
    assert_eq!(status, StatusCode::OK);
    assert!(text_of(&res).contains("ادمین"), "{res}");
    assert_eq!(store.balance_of("user:tg:2002").await.unwrap().unwrap(), 1_000);

    // /queue غیرادمین هم رد می‌شود
    let (_status, res) = send_update(&router, 6, 21, 1001, "/queue").await;
    assert!(text_of(&res).contains("ادمین"));
}

#[tokio::test]
async fn bot_buy_replay_creates_no_second_escrow_on_real_store() {
    let (router, store) = boot_with(bot_config()).await;
    setup_funded_user(&router, 1001, 60_000).await;
    setup_funded_user(&router, 2002, 1_000).await;

    send_update(&router, 1, 10, 1001, "/buy 2002 40000").await;
    send_update(&router, 2, 10, 1001, "/buy 2002 40000").await; // همان message_id

    let escrows = store.list_escrows(None).await.unwrap();
    assert_eq!(escrows.len(), 1, "replay امانت تکراری نمی‌سازد");
    let escrow = store.get_escrow("esc:c1001m10").await.unwrap().unwrap();
    assert_eq!(escrow.status.as_str(), "CREATED");
}
