//! `dual-run` — پراکسی گذار فاز ۸: TS و Rust کنار هم روی **همان** SQLite.
//!
//! متغیرهای محیطی:
//! | متغیر | پیش‌فرض | نقش |
//! |---|---|---|
//! | `PORT` | `3001` | پورت پراکسی |
//! | `TS_URL` | الزامی | آپ‌استریم Node (مثل `http://127.0.0.1:3000`) |
//! | `RUST_URL` | الزامی | آپ‌استریم Rust |
//! | `RUST_TRAFFIC_PCT` | `0` | تک‌متغیرِ سوییچ (۰=سایه/برگشت، ۱۰۰=کامل) |
//! | `SHADOW` | `1` | `1` = اجرای سایه فعال (فقط با pct=۰ معتبر) |
//! | `STABILITY_DAYS` | `0` | روزهای پایداری — سپر HITL روی ۳ باز می‌شود |
//!
//! نقاط کنترل زنده: `GET /_dualrun/status` · `POST /_dualrun/pct {"pct":10}`
//! · `POST /_dualrun/rollback` — همهٔ تغییرها خط JSON روی stderr لاگ می‌شوند.
//!
//! بوت با کانفیگ متناقض رد می‌شود (fail-closed — مثل ServerConfig فاز ۴).

use dropagentx_tooling::dual_run::DualRunConfig;
use dropagentx_tooling::proxy::{proxy_router, ProxyState};

/// خواندن env اختیاری با پیش‌فرض.
fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| default.into())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = env_or("PORT", "3001").parse()?;
    let ts_url = std::env::var("TS_URL").ok().filter(|v| !v.trim().is_empty());
    let rust_url = std::env::var("RUST_URL").ok().filter(|v| !v.trim().is_empty());
    let (Some(ts_url), Some(rust_url)) = (ts_url, rust_url) else {
        eprintln!("dual-run: TS_URL و RUST_URL الزامی‌اند — بوت رد شد (fail-closed)");
        std::process::exit(2);
    };

    let cfg = DualRunConfig {
        rust_traffic_pct: env_or("RUST_TRAFFIC_PCT", "0").parse()?,
        shadow: env_or("SHADOW", "1") != "0",
        stability_days: env_or("STABILITY_DAYS", "0").parse()?,
        hitl_shield_din: dropagentx_tooling::dual_run::HITL_SHIELD_DIN,
    };
    if let Err(msg) = cfg.validate() {
        eprintln!("dual-run: کانفیگ متناقض — بوت رد شد: {msg}");
        std::process::exit(2);
    }

    let state = ProxyState::new(cfg, ts_url.clone(), rust_url.clone());
    let app = proxy_router(state);
    let addr = format!("0.0.0.0:{port}");
    println!("dual-run proxy → {addr}  (TS={ts_url} · RUST={rust_url})");
    println!("  کنترل: GET /_dualrun/status · POST /_dualrun/pct · POST /_dualrun/rollback");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
