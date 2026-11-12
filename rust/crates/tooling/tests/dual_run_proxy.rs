//! تست انتگراسیون پراکسی dual-run فاز ۸ — دو آپ‌استریم شبیه‌سازی‌شده روی
//! پورت موقت، کل چرخهٔ shadow → pct → rollback (ADR-009 §۱۲).

// یکپارچگی‌سنجی — unwrap/expect عمدی‌اند
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use dropagentx_tooling::dual_run::DualRunConfig;
use dropagentx_tooling::proxy::{proxy_router, ProxyState};
use http_body_util::BodyExt;
use tokio::net::TcpListener;
use tower::ServiceExt;

/// آپ‌استریم شبیه‌سازی‌شده — `GET /api/v1/health` و `GET /api/v1/echo/{what}`
/// با پیشوند tag (TS/RUST) تا تفاوت پاسخ‌ها قابل دیدن باشد.
async fn spawn_upstream(tag: &'static str) -> String {
    let app: Router = Router::new()
        .route("/api/v1/health", axum::routing::get(move || async move { format!("{tag}:health") }))
        .route(
            "/api/v1/echo/{what}",
            axum::routing::get(
                move |axum::extract::Path(what): axum::extract::Path<String>| async move {
                    format!("{tag}:{what}")
                },
            ),
        )
        // مسیر هم‌پاسخ — هر دو آپ‌استریم بدنهٔ یکسان می‌دهند (تست parity match)
        .route("/api/v1/same", axum::routing::get(|| async { "ok:1" }));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

/// یک درخواست GET به پراکسی — پاسخ (status، بدنه).
async fn send(router: &Router, uri: &str) -> (StatusCode, String) {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// POST با بدنهٔ JSON به پراکسی.
async fn post(router: &Router, uri: &str, body: &str) -> StatusCode {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    router.clone().oneshot(req).await.unwrap().status()
}

#[tokio::test]
async fn proxy_full_cycle_shadow_pct_rollback() {
    let ts = spawn_upstream("TS").await;
    let rust = spawn_upstream("RUST").await;
    // گام ۱ — shadow: پاسخ‌دهنده TS است + مقایسهٔ parity رخ می‌دهد
    let state = ProxyState::new(DualRunConfig::default(), ts.clone(), rust.clone());
    let router = proxy_router(state.clone());

    let (status, body) = send(&router, "/api/v1/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "TS:health");
    let snap = state.stats.snapshot();
    assert_eq!(snap["shadow"], 1);
    assert_eq!(snap["parityDiff"], 1, "تگ آپ‌استریم‌ها متفاوت است — نرمال‌ساز باید مغایرت را بشمارد");

    // مسیر هم‌پاسخ — parity match شمرده می‌شود
    let (status, body) = send(&router, "/api/v1/same").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok:1");
    let snap = state.stats.snapshot();
    assert_eq!(snap["parityMatch"], 1, "{snap}");

    // گام ۲ — ارتقای پله: pct=100 → همه به Rust
    assert_eq!(post(&router, "/_dualrun/pct", r#"{"pct":100}"#).await, StatusCode::OK);
    let (status, body) = send(&router, "/api/v1/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "RUST:health");
    let snap = state.stats.snapshot();
    assert_eq!(snap["rust"], 1);

    // کنترل نامعتبر — pct > 100 رد می‌شود
    assert_eq!(
        post(&router, "/_dualrun/pct", r#"{"pct":150}"#).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // گام ۳ — برگشت فوری: rollback → همه به TS
    assert_eq!(post(&router, "/_dualrun/rollback", "").await, StatusCode::OK);
    let (_, body) = send(&router, "/api/v1/health").await;
    assert_eq!(body, "TS:health");
    let snap = state.stats.snapshot();
    assert_eq!(snap["ts"], 1, "{snap} — برگشت یعنی TS خالص (بدون سایه)");
}

#[tokio::test]
async fn pct_split_is_deterministic_and_bounded() {
    let ts = spawn_upstream("TS").await;
    let rust = spawn_upstream("RUST").await;
    let state = ProxyState::new(
        DualRunConfig { rust_traffic_pct: 50, shadow: false, ..DualRunConfig::default() },
        ts,
        rust,
    );
    let router = proxy_router(state.clone());
    let mut rust_hits = 0u32;
    for i in 0..200 {
        let (_, body) = send(&router, &format!("/api/v1/echo/r{i}")).await;
        if body.starts_with("RUST:") {
            rust_hits += 1;
        }
    }
    // تخصیص دترمینیستیک است — همان مسیر همیشه همان سمت؛ با ۲۰۰ مسیر یکتا
    // توزیع نزدیک ۵۰٪ (باند سختی نمی‌بندد؛ فقط گارد آشوب)
    assert!(rust_hits > 40 && rust_hits < 160, "rust_hits = {rust_hits}");
    // status endpoint — شمارنده‌ها را نشان می‌دهد
    let (_, body) = send(&router, "/_dualrun/status").await;
    assert!(body.contains("\"rust\""), "{body}");
}

#[tokio::test]
async fn dead_upstream_is_fail_closed_502() {
    // آپ‌استریم TS زنده، Rust مرده (پورت بسته)
    let ts = spawn_upstream("TS").await;
    let rust = "http://127.0.0.1:1".to_string(); // بسته
    let state = ProxyState::new(
        DualRunConfig { rust_traffic_pct: 100, shadow: false, ..DualRunConfig::default() },
        ts,
        rust,
    );
    let router = proxy_router(state);
    let (status, body) = send(&router, "/api/v1/health").await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert!(body.contains("UPSTREAM_DOWN"), "{body}");
}

#[tokio::test]
async fn shadow_parity_diff_is_counted() {
    // آپ‌استریم‌ها عمداً پاسخ متفاوت می‌دهند — parity diff باید شمرده شود
    let ts = spawn_upstream("TS").await;
    let rust = spawn_upstream("RUST").await;
    let state = ProxyState::new(DualRunConfig::default(), ts, rust);
    let router = proxy_router(state.clone());
    // مسیر echo — TS و RUST بدنهٔ متفاوت می‌دهند (tag متفاوت)
    let (_, body) = send(&router, "/api/v1/echo/x").await;
    assert_eq!(body, "TS:x");
    let snap = state.stats.snapshot();
    assert_eq!(snap["parityDiff"], 1, "{snap}");
}
