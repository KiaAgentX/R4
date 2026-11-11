//! نقطهٔ ورود سرور — بوت fail-closed: کانفیگ ناقص LLM (ADR-006) یا دیتابیس
//! ناسالم پروسه را با کد خروج غیرصفر و پیام صادقانه می‌بندد.
//!
//! لاگ‌ها JSON (همان شکل فعلی) — trace layer آکسوم روی همهٔ مسیرها.

use dropagentx_server::{build_router, AppState, ServerConfig};

#[tokio::main]
async fn main() {
    // لاگ JSON — env-filter با RUST_LOG (پیش‌فرض info)
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = match ServerConfig::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("boot refused: {e}");
            std::process::exit(2); // fail-closed — هیچ سرویسی با کانفیگ ناقص بالا نمی‌آید
        }
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

    let admin_state = config.admin_key.is_some();
    let llm_state = config.llm.as_ref().map(|c| c.model.clone());

    // فاز ۶ — گیت‌وی AI از اسلات‌های ADR-008 (primary→secondary→fallback)
    let mut slots: Vec<dropagentx_ai::AnyProvider> = Vec::new();
    if let Some(cfg) = config.llm.as_ref() {
        slots.push(dropagentx_ai::AnyProvider::OpenAi(dropagentx_ai::OpenAiProvider::new(
            "primary",
            cfg.base_url.clone(),
            cfg.api_key.clone(),
            cfg.model.clone(),
        )));
    }
    if let Some(cfg) = config.llm_2.as_ref() {
        slots.push(dropagentx_ai::AnyProvider::OpenAi(dropagentx_ai::OpenAiProvider::new(
            "secondary",
            cfg.base_url.clone(),
            cfg.api_key.clone(),
            cfg.model.clone(),
        )));
    }
    if let Some(cfg) = config.llm_3.as_ref() {
        slots.push(dropagentx_ai::AnyProvider::OpenAi(dropagentx_ai::OpenAiProvider::new(
            "fallback",
            cfg.base_url.clone(),
            cfg.api_key.clone(),
            cfg.model.clone(),
        )));
    }
    let gateway = dropagentx_ai::Gateway::new(slots, config.ai_daily_token_cap);
    let static_dir = config.static_dir.clone();
    let addr = format!("{}:{}", config.host, config.port);

    let state = AppState::new(store, config).with_ai_gateway(gateway).shared();
    let router = build_router(state).into_make_service_with_connect_info::<std::net::SocketAddr>();

    tracing::info!(
        admin_enabled = admin_state,
        llm_model = llm_state,
        static_dir = ?static_dir,
        "dropagentx-server listening on http://{addr} — docs at /api/v1/docs"
    );

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("boot refused: cannot bind {addr} — {e}");
            std::process::exit(4);
        }
    };
    if let Err(e) = axum::serve(listener, router).await {
        eprintln!("server error: {e}");
        std::process::exit(5);
    }
}
