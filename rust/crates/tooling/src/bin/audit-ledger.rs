//! `audit-ledger` — ممیزی نهایی cut-over فاز ۸ (ADR-009 §۷ فاز ۸).
//!
//! معیار پذیرش: «ممیزی نهایی: Σ کیف‌ها = 187,672 Đ و trialBalance.balanced
//! == true — همان عدد نسل‌ها، حالا با هسته Rust».
//!
//! این باینری روی فایل دیتابیس هدف اجرا می‌شود (فقط‌خواندنی، SQL خام مثل
//! `genesis-proof`) و در پایان حکم go/no-go چاپ می‌کند:
//! * `AUDIT PASS` → exit ۰ — مجاز به تگ `rust-core-1.0.0` و خاموشی Node
//! * `AUDIT FAILED` → exit ۱ — هرگز با parity-fail مالی cut-over نکنید
//!
//! استفاده: `audit-ledger <db-path>`

use dropagentx_db::genesis_proof;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(db_path) = args.first() else {
        eprintln!("usage: audit-ledger <db-path>");
        std::process::exit(2);
    };

    println!("══════════ cut-over final audit — ADR-009 phase 8 ══════════");
    println!("database : {db_path}");
    println!();

    let report = match genesis_proof(db_path).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("AUDIT FAILED — cannot open/audit database: {e}");
            std::process::exit(1);
        }
    };
    for line in &report.lines {
        println!("{line}");
    }

    // حکم نهایی — همان دو شرط پذیرش ADR-009
    println!();
    if report.ok {
        println!("AUDIT PASS — Σ wallets = 187,672 Đ · trialBalance.balanced = true");
        println!("→ مجاز به تگ rust-core-1.0.0 و خاموشی Node (پس از ۲ هفته ۱۰۰٪ سبز).");
        Ok(())
    } else {
        println!("AUDIT FAILED — {}", report.problems.join(" · "));
        println!("→ برگشت: RUST_TRAFFIC_PCT=0 — هرگز با مغایرت مالی cut-over نکنید.");
        std::process::exit(1);
    }
}
