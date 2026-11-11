//! وضعیت مشترک سرور — یک `Arc<AppState>` بین همهٔ هندلرها.
//!
//! دیتابیس ([`V3Store`]) نقطهٔ تنها اتصال دامنه به SQLite است؛ limiter نرخ
//! درون‌حافظه‌ای همان پنجرهٔ لغزان فاز ۳ را با Mutex پخش می‌کند و رجیستری
//! ایجنت/مهارت دادهٔ ثابت `/ai/agents` و `/ai/skills` است (فاز ۶ اجرای واقعی
//! مهارت‌ها را روی همین شناسه‌ها سیم می‌کند).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use dropagentx_contracts::{AiAgentView, AiSkillView, RateLimiter};
use dropagentx_db::V3Store;

use crate::config::ServerConfig;

/// نمای وایر ایجنت‌های Zenovix — از رجیستری واحد crate AI (فاز ۶).
#[must_use]
pub fn zenovix_agents() -> Vec<AiAgentView> {
    dropagentx_ai::agent_views()
}

/// رجیستری ۱۶ مهارت KIA-OS — متادیتای مجوز (فاز ۶ گیت fail-closed قبل از اجرا
/// را روی همین `kind`ها می‌سازد؛ `staff_only` با کلید ادمین گیت می‌خورد).
/// نمای وایر ۱۶ مهارت KIA-OS — از رجیستری واحد crate AI (فاز ۶).
#[must_use]
pub fn kia_os_skills() -> Vec<AiSkillView> {
    dropagentx_ai::skill_views()
}

/// وضعیت مشترک همهٔ هندلرها.
#[derive(Debug)]
pub struct AppState {
    /// فروشگاه v3 — نقطهٔ تنها اتصال دامنه به SQLite.
    pub store: V3Store,
    /// پیکربندی بوت (تغییرناپذیر).
    pub config: ServerConfig,
    /// limiter نرخ پنجرهٔ لغزان — یکی به ازای سرور.
    pub limiter: Mutex<RateLimiter>,
    /// انتخاب زبان کاربران بات (`/lang`) — session درون‌حافظه‌ای فاز ۵
    /// (دول‌ران parity روی زبان session نیست؛ تصمیم ثبت‌شدهٔ فاز ۵).
    pub bot_sessions: Arc<Mutex<BTreeMap<i64, dropagentx_bot::Locale>>>,
    /// گیت‌وی AI فاز ۶ — از اسلات‌های کانفیگ ساخته می‌شود؛ `None` یعنی
    /// AI_DISABLED (همان گیت کانفیگ).
    pub ai_gateway: Option<Arc<dropagentx_ai::Gateway>>,
    /// لحظهٔ شروع — uptime در health.
    pub started_at: Instant,
}

impl AppState {
    /// ساخت وضعیت — `started_at` همین‌جا قفل می‌شود.
    #[must_use]
    pub fn new(store: V3Store, config: ServerConfig) -> Self {
        Self {
            limiter: Mutex::new(RateLimiter::new(config.rate_limit)),
            bot_sessions: Arc::new(Mutex::new(BTreeMap::new())),
            ai_gateway: None,
            store,
            config,
            started_at: Instant::now(),
        }
    }

    /// نصب گیت‌وی AI — از اسلات‌های کانفیگ در بوت (فاز ۶).
    #[must_use]
    pub fn with_ai_gateway(mut self, gateway: dropagentx_ai::Gateway) -> Self {
        self.ai_gateway = Some(Arc::new(gateway));
        self
    }

    /// وضعیت اشتراکی — `Arc` برای کلون‌شدن داخل هر هندلر.
    #[must_use]
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}
