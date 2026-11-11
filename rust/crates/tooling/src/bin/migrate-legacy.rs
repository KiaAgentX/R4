//! پورت `back/server/scripts/migrate-legacy.ts` — پل روز-اول از دیتابیس
//! بازیابی‌شدهٔ Railway به v3.
//!
//! رفتار TS حفظ شده: replay-safe (اجرای دوم فقط گزارش می‌دهد)، `--dry-run`
//! هیچ نوشتنی ندارد، و گزارش آشتی Σ کیف‌ها را برابر ۱۸۷٬۶۷۲ Đ نشان می‌دهد.
//!
//! ```text
//! cargo run -p dropagentx-tooling --bin migrate-legacy -- \
//!   --legacy /path/to/marketplace-20260924-0400.db \
//!   --target file:./db/v3.db [--dry-run]
//! ```

use std::process::ExitCode;

fn arg_of(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

async fn migrate(legacy_path: &str, target: &str, dry_run: bool) -> anyhow::Result<()> {
    use anyhow::Context;
    let store = dropagentx_db::V3Store::connect(target).await.context("open target store")?;
    store.migrate().await.context("run schema migrations")?;
    let snapshot = dropagentx_db::read_legacy(legacy_path).await.context("read legacy db")?;
    let source = std::path::Path::new(legacy_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| legacy_path.to_owned());
    let report = dropagentx_db::migrate_legacy(
        &store,
        &snapshot.users,
        &snapshot.products,
        snapshot.tx_count,
        &source,
        dry_run,
    )
    .await?;
    for line in &report.lines {
        println!("{line}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let Some(legacy_path) = arg_of("--legacy") else {
        eprintln!("Usage: migrate-legacy --legacy <legacy.db> [--target <v3.db url>] [--dry-run]");
        return ExitCode::from(2);
    };
    let dry_run = std::env::args().any(|a| a == "--dry-run");
    let target = match arg_of("--target") {
        Some(t) => t,
        None => match std::env::var("DATABASE_URL") {
            Ok(u) => u,
            Err(_) => {
                eprintln!("No target: pass --target file:./db/v3.db or set DATABASE_URL");
                return ExitCode::from(2);
            }
        },
    };
    println!("[migrate] legacy = {legacy_path}");
    println!("[migrate] target = {target}{}", if dry_run { " (dry-run)" } else { "" });
    match migrate(&legacy_path, &target, dry_run).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("[migrate] FAILED: {e:#}");
            ExitCode::FAILURE
        }
    }
}
