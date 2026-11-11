//! xtask — اتوماسیون ورک‌اسپیس، بدون هیچ وابستگی خارجی.
//!
//! اجرا: `cargo xtask progress` یا `cargo xtask check`

use std::process::{Command, ExitCode};

/// جدول فازها بر اساس ADR-009 — نسخه‌بندی ۱۰/۱۰.
const PHASES: [(&str, &str, bool); 10] = [
    ("0", "زیرساخت و قواعد بازی", true),
    ("1", "هسته دامنه خالص — dropagentx-domain", true),
    ("2", "پیوند persistence — sqlx + Outbox + Genesis", true),
    ("3", "قراردادها — serde + utoipa (OpenAPI 3.1)", false),
    ("4", "سرور Axum + داشبورد static", false),
    ("5", "بات teloxide + صف HITL", false),
    ("6", "AI Gateway — ۶ ایجنت، ۱۶ مهارت، سقف و audit", false),
    ("7", "ادغام فرانت + WASM اختیاری", false),
    ("8", "Dual-run → مهاجرت داده → Cut-over", false),
    ("9", "امضای نهایی — نسخه ۱۰ از ۱۰", false),
];

fn done_count() -> usize {
    PHASES.iter().filter(|p| p.2).count()
}

fn cmd_status(program: &str, args: &[&str]) -> Option<bool> {
    match Command::new(program).args(args).status() {
        Ok(s) => Some(s.success()),
        Err(e) => {
            eprintln!("خطا در اجرای {program}: {e}");
            None
        }
    }
}

fn run_check() -> ExitCode {
    let steps: [(&str, &str, &[&str]); 3] = [
        ("fmt", "cargo", &["fmt", "--all", "--", "--check"]),
        ("clippy", "cargo", &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
        ("test", "cargo", &["test", "--workspace"]),
    ];
    let mut ok = true;
    for (name, program, args) in steps {
        print!("==> {name} ... ");
        match cmd_status(program, args) {
            Some(true) => println!("OK"),
            _ => {
                println!("FAILED");
                ok = false;
                break;
            }
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn run_progress() {
    let v = done_count();
    println!("پیشرفت مهاجرت: v{v}/10");
    println!("-----------------------------------------");
    for (num, title, done) in PHASES {
        let mark = if done { "[x]" } else { "[ ]" };
        println!("{mark} فاز {num}: {title}");
    }
}

fn main() -> ExitCode {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("check") => run_check(),
        Some("progress") => {
            run_progress();
            ExitCode::SUCCESS
        }
        _ => {
            println!("استفاده: cargo xtask <check|progress>");
            ExitCode::from(2)
        }
    }
}
