//! پیکربندی سرور از متغیرهای محیطی — پورت `env` خواندن‌های `server/src/*.ts`.
//!
//! قاعدهٔ ADR-006: کانفیگ LLM «همه‌یا-هیچ» است — اگر یکی از سه متغیر
//! `OPENAI_BASE_URL` / `OPENAI_API_KEY` / `MODEL` تنظیم شده باشد و دیگری نباشد،
//! **بوت رد می‌شود** (fail-closed). سه‌تایی کامل → سیکل AI فعال؛ هیچ‌کدام →
//! AI غیرفعال (`llmConfigured: false` در health).
//!
//! متغیرها (نام‌ها عین کدهای صریح فاز ۳ — `HttpFailure::ai_disabled` و
//! رجیستری امنیت):
//!
//! | متغیر | پیش‌فرض | نقش |
//! |---|---|---|
//! | `KIA_ADMIN_KEY` | هیچ (ادمین غیرفعال) | کلید ادمین — fail-closed |
//! | `DATABASE_URL` | `file:./db/v3.db` | SQLite واقعی v3 |
//! | `OPENAI_BASE_URL` / `OPENAI_API_KEY` / `MODEL` | هیچ | سه‌اسلات ADR-006 |
//! | `TELEGRAM_BOT_TOKEN` | هیچ | HMAC initData هویت عمومی AI |
//! | `BRAND_NAME` / `BRAND_TAGLINE` | `DropAgentX` / `Empire` | شل وایت‌لیبل |
//! | `HOST` / `PORT` | `0.0.0.0` / `3000` | بایند HTTP |
//! | `STATIC_DIR` | `front/dashboard/dist` | ServeDir داشبورد/Mini App |
//! | `RATE_LIMIT` | `120` | درخواست در پنجرهٔ ۶۰ثانیه‌ای |
//! | `KEEP_BACKUPS` | `7` | هرس اسنپ‌شات‌های `VACUUM INTO` |
//! | `BACKUP_DIR` | `db/backups` | مقصد بکاپ |
//! | `AI_DAILY_TOKEN_CAP` | `120000` | سقف روزانهٔ توکن per-user (فاز ۶) |

use std::path::PathBuf;

/// خطای پیکربندی — بوت را متوقف می‌کند (هرگز open نمی‌شود).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// کانفیگ LLM ناقص — «همه‌یا-هیچ» (ADR-006).
    IncompleteLlm {
        /// نام متغیرهای محیطی غایب از سه‌اسلات ADR-006
        missing: Vec<&'static str>,
    },
    /// `PORT` عددی نیست.
    InvalidPort(String),
    /// `RATE_LIMIT` عددی نیست.
    InvalidRateLimit(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IncompleteLlm { missing } => {
                write!(
                    f,
                    "Incomplete LLM config (ADR-006 all-or-nothing) — missing: {}",
                    missing.join(", ")
                )
            }
            Self::InvalidPort(v) => write!(f, "PORT is not a number: {v}"),
            Self::InvalidRateLimit(v) => write!(f, "RATE_LIMIT is not a number: {v}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// اسلات LLM — سه‌تایی کامل ADR-006. کلید هرگز در لاگ نمی‌رود (فاز ۶ redaction).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmConfig {
    /// base URL آپ‌استریم
    pub base_url: String,
    /// کلید API — فقط برای client (فاز ۶)
    pub api_key: String,
    /// نام مدل — عین مقدار در health (`llmModel`)
    pub model: String,
}

/// پیکربندی کامل سرور — یک‌بار در بوت ساخته می‌شود و تغییرناپذیر می‌ماند.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// کلید ادمین — `None` یعنی مسیرهای ادمین 503 `ADMIN_DISABLED`.
    pub admin_key: Option<String>,
    /// URL دیتابیس (SQLite واقعی v3).
    pub database_url: String,
    /// سیکل AI — `None` یعنی غیرفعال (`AI_DISABLED` 503).
    pub llm: Option<LlmConfig>,
    /// توکن بات — هویت عمومی AI از initData مشتق می‌شود.
    pub bot_token: Option<String>,
    /// نام برند شل داشبورد.
    pub brand_name: String,
    /// شعار برند.
    pub brand_tagline: String,
    /// هاست بایند.
    pub host: String,
    /// پورت بایند.
    pub port: u16,
    /// ریشهٔ static داشبورد/Mini App — مسیر ناموجود = فقط API.
    pub static_dir: Option<PathBuf>,
    /// سقف درخواست در پنجرهٔ لغزان ۶۰ثانیه‌ای.
    pub rate_limit: usize,
    /// تعداد اسنپ‌شات نگه‌داشته‌شده.
    pub keep_backups: usize,
    /// پوشهٔ بکاپ‌ها.
    pub backup_dir: PathBuf,
    /// سقف روزانهٔ توکن AI (فاز ۶ مصرف می‌کند).
    pub ai_daily_token_cap: i64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            admin_key: None,
            database_url: "file:./db/v3.db".into(),
            llm: None,
            bot_token: None,
            brand_name: "DropAgentX".into(),
            brand_tagline: "Empire".into(),
            host: "0.0.0.0".into(),
            port: 3000,
            static_dir: Some(PathBuf::from("front/dashboard/dist")),
            rate_limit: 120,
            keep_backups: 7,
            backup_dir: PathBuf::from("db/backups"),
            ai_daily_token_cap: 120_000,
        }
    }
}

fn env_opt(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}

impl ServerConfig {
    /// خواندن از متغیرهای محیطی — قواعد fail-closed ADR-006.
    ///
    /// # Errors
    /// [`ConfigError::IncompleteLlm`] اگر سه‌اسلات LLM ناقص باشد؛ خطاهای عددی
    /// برای `PORT`/`RATE_LIMIT`.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut cfg = Self { admin_key: env_opt("KIA_ADMIN_KEY"), ..Self::default() };
        if let Some(v) = env_opt("DATABASE_URL") {
            cfg.database_url = v;
        }
        if let Some(v) = env_opt("TELEGRAM_BOT_TOKEN") {
            cfg.bot_token = Some(v);
        }
        if let Some(v) = env_opt("BRAND_NAME") {
            cfg.brand_name = v;
        }
        if let Some(v) = env_opt("BRAND_TAGLINE") {
            cfg.brand_tagline = v;
        }

        // ADR-006 — همه‌یا-هیچ روی سه‌اسلات LLM
        let base = env_opt("OPENAI_BASE_URL");
        let key = env_opt("OPENAI_API_KEY");
        let model = env_opt("MODEL");
        match (base, key, model) {
            (Some(base_url), Some(api_key), Some(model)) => {
                cfg.llm = Some(LlmConfig { base_url, api_key, model });
            }
            (None, None, None) => cfg.llm = None,
            (base, key, model) => {
                let mut missing = Vec::new();
                if base.is_none() {
                    missing.push("OPENAI_BASE_URL");
                }
                if key.is_none() {
                    missing.push("OPENAI_API_KEY");
                }
                if model.is_none() {
                    missing.push("MODEL");
                }
                return Err(ConfigError::IncompleteLlm { missing });
            }
        }

        if let Some(v) = env_opt("HOST") {
            cfg.host = v;
        }
        if let Some(v) = env_opt("PORT") {
            cfg.port = v.parse::<u16>().map_err(|_| ConfigError::InvalidPort(v))?;
        }
        if let Some(v) = env_opt("STATIC_DIR") {
            cfg.static_dir = Some(PathBuf::from(v));
        }
        if let Some(v) = env_opt("RATE_LIMIT") {
            cfg.rate_limit = v.parse::<usize>().map_err(|_| ConfigError::InvalidRateLimit(v))?;
        }
        if let Some(v) = env_opt("KEEP_BACKUPS") {
            cfg.keep_backups = v.parse::<usize>().unwrap_or(cfg.keep_backups);
        }
        if let Some(v) = env_opt("BACKUP_DIR") {
            cfg.backup_dir = PathBuf::from(v);
        }
        if let Some(v) = env_opt("AI_DAILY_TOKEN_CAP") {
            if let Ok(n) = v.parse::<i64>() {
                cfg.ai_daily_token_cap = n;
            }
        }
        Ok(cfg)
    }

    /// آیا پوشهٔ static واقعاً موجود است؟ — ServeDir فقط برای مسیر موجود سوار می‌شود.
    #[must_use]
    pub fn static_dir_exists(&self) -> bool {
        self.static_dir.as_ref().is_some_and(|p| p.is_dir())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // تست‌های env وضعیت فرآیند را دستکاری می‌کنند — سریال با قفل مشترک
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn clear_llm_env() {
        // سه متغیر ADR-006 را از محیط تست پاک می‌کند
        for name in ["OPENAI_BASE_URL", "OPENAI_API_KEY", "MODEL"] {
            std::env::remove_var(name);
        }
    }

    #[test]
    fn no_llm_env_means_disabled_cycle() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_llm_env();
        let cfg = ServerConfig::from_env().unwrap();
        assert!(cfg.llm.is_none());
    }

    #[test]
    fn complete_llm_triple_enables_the_cycle() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_llm_env();
        std::env::set_var("OPENAI_BASE_URL", "https://api.example.com/v1");
        std::env::set_var("OPENAI_API_KEY", "sk-test");
        std::env::set_var("MODEL", "gpt-4o-mini");
        let cfg = ServerConfig::from_env().unwrap();
        let llm = cfg.llm.expect("complete triple must enable the cycle");
        assert_eq!(llm.model, "gpt-4o-mini");
        clear_llm_env();
    }

    #[test]
    fn partial_llm_config_is_rejected_at_boot_all_or_nothing() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_llm_env();
        // فقط دو اسلات — بوت باید رد شود (ADR-006)
        std::env::set_var("OPENAI_BASE_URL", "https://api.example.com/v1");
        std::env::set_var("OPENAI_API_KEY", "sk-test");
        let err = ServerConfig::from_env().unwrap_err();
        assert_eq!(err, ConfigError::IncompleteLlm { missing: vec!["MODEL"] });
        // فقط یک اسلات
        std::env::remove_var("OPENAI_API_KEY");
        let err = ServerConfig::from_env().unwrap_err();
        assert_eq!(err, ConfigError::IncompleteLlm { missing: vec!["OPENAI_API_KEY", "MODEL"] });
        clear_llm_env();
    }

    #[test]
    fn defaults_are_production_sane() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let cfg = ServerConfig::default();
        assert_eq!(cfg.brand_name, "DropAgentX");
        assert_eq!(cfg.port, 3000);
        assert_eq!(cfg.rate_limit, 120);
        assert_eq!(cfg.ai_daily_token_cap, 120_000);
        assert_eq!(cfg.database_url, "file:./db/v3.db");
    }
}
