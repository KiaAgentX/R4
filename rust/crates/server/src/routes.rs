//! سیم‌کشی مسیرها — «یک تعریف به ازای هر endpoint» (ADR-005).
//!
//! رجیستری [`api_routes`] (فاز ۳) منبع متادیتاست؛ این ماژول همان ۴۶ مسیر را
//! روی آکسوم می‌بندد و دو نقطهٔ ادغام دیگر را اضافه می‌کند:
//! * `/api/v1/openapi.json` + `/api/v1/docs` — سند OpenAPI 3.1 و صفحهٔ ردوک
//! * fallback — ServeDir داشبورد/Mini App با fallback SPA (جایگزین static
//!   serving فستیفای)
//!
//! نکتهٔ لایه‌بندی: نرخ‌سنجی قبل از همهٔ مسیرهای API اعمال می‌شود (ادمین
//! معتبر باکت مشترک دارد)؛ گارد ادمین درون هر هندلر `Security::Admin` است.

use axum::http::{header, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use dropagentx_contracts::{api_routes, build_openapi_doc_inlined, docs_page_html, openapi_info};

use crate::handlers::{accounts, ai, dashboard, escrows, hitl, system, wallet};
use crate::middleware::rate_limit_middleware;
use crate::state::AppState;

/// پاکت 404 مسیرهای ناشناخته — additive-only؛ بات و SPA هرگز بدنهٔ بی‌شکل نمی‌گیرند.
async fn api_not_found() -> Response {
    let envelope = serde_json::json!({
        "error": { "code": "NOT_FOUND", "message": "No such route under /api/v1 — see /api/v1/openapi.json" }
    });
    (StatusCode::NOT_FOUND, [(header::CONTENT_TYPE, "application/json")], envelope.to_string())
        .into_response()
}

/// سند OpenAPI — کاملاً inline مثل خروجی zod (قابل diff با گلدن TS).
async fn openapi_json() -> Json<serde_json::Value> {
    Json(build_openapi_doc_inlined(&api_routes(), &openapi_info()))
}

/// صفحهٔ ردوک.
async fn docs() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], docs_page_html())
}

/// چرا فال‌بک static برای این بوت فعال نیست — پیام صادقانه برای ریشهٔ خالی.
async fn no_static() -> Response {
    let envelope = serde_json::json!({
        "error": { "code": "NO_FRONTEND", "message": "No static dir configured/found — set STATIC_DIR to the dashboard build (front/dashboard/dist)." }
    });
    (StatusCode::NOT_FOUND, [(header::CONTENT_TYPE, "application/json")], envelope.to_string())
        .into_response()
}

/// ساخت روتر کامل — همان شکل ورودی `serve`.
pub fn build_router(state: Arc<AppState>) -> Router {
    // ── API — /api/v1 با نرخ‌سنجی سراسری (Router<Arc<AppState>>) ────────────
    let api: Router<Arc<AppState>> = Router::new()
        // system / meta
        .route("/health", get(system::health))
        .route("/economy", get(system::get_economy))
        .route("/system/backup", post(system::create_backup))
        // accounts
        .route("/accounts", post(accounts::create_account).get(accounts::list_accounts))
        .route("/accounts/{id}/balance", get(accounts::get_balance))
        // wallet
        .route("/wallet/mint", post(wallet::mint))
        .route("/wallet/burn", post(wallet::burn))
        // escrows
        .route("/escrows", post(escrows::create).get(escrows::list))
        .route("/escrows/{id}", get(escrows::get_one))
        .route("/escrows/{id}/fund", post(escrows::fund))
        .route("/escrows/{id}/deliver", post(escrows::deliver))
        .route("/escrows/{id}/release", post(escrows::release))
        .route("/escrows/{id}/dispute", post(escrows::dispute))
        .route("/escrows/{id}/refund", post(escrows::refund))
        .route("/escrows/{id}/cancel", post(escrows::cancel))
        .route("/escrows/{id}/expire", post(escrows::expire))
        // hitl
        .route("/hitl", post(hitl::create).get(hitl::list))
        .route("/hitl/sweep", post(hitl::sweep))
        .route("/hitl/{id}", get(hitl::get_one))
        .route("/hitl/{id}/approve", post(hitl::approve))
        .route("/hitl/{id}/reject", post(hitl::reject))
        // ai
        .route("/ai/chat", post(ai::chat))
        .route("/ai/agents", get(ai::agents))
        .route("/ai/skills", get(ai::skills))
        .route("/ai/usage", get(ai::usage))
        // dashboard
        .route("/brand", get(dashboard::brand))
        .route("/overview", get(dashboard::overview))
        .route("/products", post(dashboard::create_product).get(dashboard::list_products))
        .route(
            "/products/{id}",
            patch(dashboard::update_product).delete(dashboard::archive_product),
        )
        .route("/orders", get(dashboard::list_orders))
        .route("/orders/{id}/status", post(dashboard::order_action))
        .route("/users", get(dashboard::list_users))
        .route("/wallets", get(dashboard::list_wallets))
        .route("/quests", get(dashboard::quests))
        .route("/bookings", get(dashboard::bookings))
        .route("/leaderboard", get(dashboard::leaderboard))
        .route("/approvals", get(dashboard::list_approvals))
        .route("/approvals/{id}/decide", post(dashboard::approval_decision))
        .route("/settings", get(dashboard::list_settings).put(dashboard::put_setting))
        .route("/security", get(dashboard::security))
        // سند قرارداد — خودِ مرجع
        .route("/openapi.json", get(openapi_json))
        .route("/docs", get(docs))
        .fallback(api_not_found)
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware));

    // ── static — جایگزین static serving فستیفای ─────────────────────────────
    // تصمیم فاز ۴: ServeDir خالص — ریشه/پوشه‌ها index می‌گیرند، فایل غایب
    // ۴۰۴ واقعی می‌دهد (فایل‌های asset هرگز index جعلی نمی‌گیرند).
    // SPA deep-link fallback با ادغام فرانت در فاز ۷ شکل قطعی می‌گیرد.
    let root: Router<Arc<AppState>> = match state.config.static_dir.as_ref().filter(|p| p.is_dir())
    {
        Some(dir) => Router::new()
            .fallback_service(ServeDir::new(dir).append_index_html_on_directories(true)),
        None => Router::new().fallback(no_static),
    };

    // trace layer → همان شکل لاگ فعلی؛ فرمت JSON در main انتخاب می‌شود
    let api = api.layer(TraceLayer::new_for_http());

    root.nest("/api/v1", api).with_state(state)
}
