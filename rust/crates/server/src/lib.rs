//! `dropagentx-server` — دروازهٔ HTTP Agent-One (فاز ۴ از ADR-009).
//!
//! جایگزین کامل فستیفای v3 — همان قرارداد وایر، همان امنیت چندلایه:
//!
//! * [`routes::build_router`] — ۴۶ مسیر رجیستری واحد روی آکسوم + سند
//!   OpenAPI 3.1 (`/api/v1/openapi.json`) + صفحهٔ ردوک + ServeDir داشبورد
//! * [`middleware`] — نرخ پنجرهٔ لغزان (باکت ادمین جدا) و گارد `X-KIA-Key`
//!   fail-closed با مقایسهٔ زمان‌ثابت
//! * [`handlers`] — همهٔ فرمان‌های پولی از دامنهٔ خالص + `V3Store` می‌گذرند؛
//!   replay دوبار پرداخت نمی‌کند، overdraft ناممکن، مادهٔ ۷ HITL-gated
//! * [`config`] — بوت با کانفیگ ناقص LLM رد می‌شود (ADR-006 همه‌یا-هیچ)
//!
//! سلامت: `GET /api/v1/health` خوانش عمیق دفتر است — سروری که به دفترش
//! نمی‌رسد سالم نیست.
#![forbid(unsafe_code)]

pub mod bot_env;
pub mod config;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;
pub mod views;

/// شناسه کرِیت — برای smoke test و لاگ‌ها.
pub const NAME: &str = "dropagentx-server";

/// نسخه کرِیت — از `Cargo.toml` خوانده می‌شود؛ در `health.version` می‌رود.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use config::{ConfigError, LlmConfig, ServerConfig};
pub use error::{ApiFailure, ApiResult};
pub use routes::build_router;
pub use state::AppState;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(NAME, "dropagentx-server");
        assert!(!VERSION.is_empty());
    }

    /// روتر روی فروشگاه درون‌حافظه‌ای کامل می‌بندد — سیم‌کشی پایه.
    #[tokio::test]
    async fn router_builds_on_in_memory_store() {
        let store = dropagentx_db::V3Store::connect_in_memory().await.unwrap();
        store.migrate().await.unwrap();
        store.ensure_system_accounts().await.unwrap();
        let state = AppState::new(store, ServerConfig::default()).shared();
        let _router = build_router(state);
    }
}
