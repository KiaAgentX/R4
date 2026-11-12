//! بات polling توسعه — teloxide رانر + `StoreEnv` روی دیتابیس واقعی.
//!
//! در تولید webhook روی `/api/v1/bot/webhook` (سرور) استفاده می‌شود؛ این
//! باینری برای توسعهٔ محلی است (قاعدهٔ ADR-009 فاز ۵: «polling توسعه»).
//!
//! ```text
//! TELEGRAM_BOT_TOKEN=... DATABASE_URL=file:./db/v3.db BOT_ADMIN_IDS=42,43 \
//!   cargo run -p dropagentx-server --bin dropagentx-bot
//! ```
//!
//! بوت fail-closed با کد خروج صریح (۲ توکن غایب، ۳ دیتابیس، ۴ رانر) — همان
//! قاعدهٔ سرور؛ کانفیگ LLM اینجا لازم نیست (بات مستقل از AI فاز ۶ است).

use std::sync::Arc;

use dropagentx_server::{AppState, ServerConfig};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // کانفیگ بات — مستقل از from_env سرور (LLM لازم ندارد)
    let token = match std::env::var("TELEGRAM_BOT_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t,
        _ => {
            eprintln!("boot refused: TELEGRAM_BOT_TOKEN is missing (exit 2)");
            std::process::exit(2);
        }
    };
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "file:./db/v3.db".to_owned());
    let bot_admin_ids = std::env::var("BOT_ADMIN_IDS")
        .map(|v| v.split(',').filter_map(|s| s.trim().parse::<i64>().ok()).collect::<Vec<_>>())
        .unwrap_or_default();

    let config = ServerConfig {
        database_url: db_url.clone(),
        bot_token: Some(token.clone()),
        bot_admin_ids,
        bot_webhook_secret: None, // polling — webhook در این باینری خاموش است
        static_dir: None,
        ..ServerConfig::default()
    };

    let store = match dropagentx_db::V3Store::connect(&config.database_url).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("boot refused: cannot open database {} — {e}", config.database_url);
            std::process::exit(3);
        }
    };
    if let Err(e) = store.migrate().await {
        eprintln!("boot refused: migrations failed — {e}");
        std::process::exit(3);
    }
    if let Err(e) = store.ensure_system_accounts().await {
        eprintln!("boot refused: system accounts failed — {e}");
        std::process::exit(3);
    }

    let state = AppState::new(store, config).shared();
    let env =
        Arc::new(tokio::sync::Mutex::new(dropagentx_server::bot_env::StoreEnv::from_state(&state)));

    tracing::info!("dropagentx-bot polling started (dev mode)");
    if let Err(e) = dropagentx_bot::runner::run_polling(&token, env).await {
        eprintln!("polling stopped: {e}");
        std::process::exit(4);
    }
}
