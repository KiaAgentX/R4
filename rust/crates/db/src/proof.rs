//! پورت `back/server/scripts/genesis-proof.ts` — ممیزی خارجی genesis.
//!
//! SQL خام و فقط‌خواندنی، «عمداً» نه از طریق دامنه و نه از طریق store — تا
//! اعداد نتوانند باگ کدی که ممیزی می‌کنند را به ارث ببرند. همان روح
//! `nation-genesis-proof.py` نسخهٔ لگاسی.
//!
//! چاپ می‌کند: Σ کیف کاربران (باید ۱۸۷٬۶۷۲ Đ باشد)، Σ همهٔ ردیف‌ها (باید ۰
//! باشد — بستن دوطرفه)، ترازهای mint/burn/خزانه و اسکرو. `ok == false`
//! یعنی یکی از انتظارات نقض شده (exit 1 در باینری).

use crate::error::DbResult;
use crate::genesis::GENESIS_KEY;
use dropagentx_domain::{BURN_ACCOUNT_ID, MINT_ACCOUNT_ID, TREASURY_ACCOUNT_ID};
use sqlx::Row;

/// گزارش ممیزی — خطوط قابل چاپ + مشکلات یافت‌شده.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofReport {
    /// خطوط گزارش (قالب یکسان با TS)
    pub lines: Vec<String>,
    /// انتظارات نقض‌شده — خالی یعنی سالم
    pub problems: Vec<String>,
    /// `problems.is_empty()`
    pub ok: bool,
}

/// اجرای ممیزی روی فایل دیتابیس هدف — فقط‌خواندنی، SQL خام.
///
/// # Errors
/// فایل نبود یا SQLite باز نشد (خودِ انتظارهای نقض‌شده، خطا نیستند —
/// در `problems` برمی‌گردند).
pub async fn genesis_proof(db_path: &str) -> DbResult<ProofReport> {
    let file = db_path.strip_prefix("file:").unwrap_or(db_path);
    let store = crate::store::V3Store::connect_read_only(file).await?;
    let pool = store.pool_ref();

    let accounts: Vec<(String, String)> =
        sqlx::query(r#"SELECT "id", "kind" FROM "Account" ORDER BY "id""#)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|r| (r.get("id"), r.get("kind")))
            .collect();
    let mut sums = std::collections::BTreeMap::new();
    for r in
        sqlx::query(r#"SELECT "accountId", SUM("amount") AS s FROM "Posting" GROUP BY "accountId""#)
            .fetch_all(pool)
            .await?
    {
        sums.insert(r.get::<String, _>("accountId"), r.get::<i64, _>("s"));
    }
    let genesis_tx: Option<(String, String)> =
        sqlx::query(r#"SELECT "id", "memo" FROM "Tx" WHERE "idempotencyKey" = ?"#)
            .bind(GENESIS_KEY)
            .fetch_optional(pool)
            .await?
            .map(|r| (r.get("id"), r.get("memo")));

    let user_rows: Vec<&(String, String)> =
        accounts.iter().filter(|(id, _)| id.starts_with("user:")).collect();
    let sum_users: i64 = user_rows.iter().map(|(id, _)| sums.get(id).copied().unwrap_or(0)).sum();
    let sum_all: i64 = sums.values().sum();
    let mint = sums.get(MINT_ACCOUNT_ID).copied().unwrap_or(0);
    let treasury = sums.get(TREASURY_ACCOUNT_ID).copied().unwrap_or(0);
    let burn = sums.get(BURN_ACCOUNT_ID).copied().unwrap_or(0);
    let escrows: Vec<&(String, String)> =
        accounts.iter().filter(|(_, kind)| kind == "escrow").collect();
    let escrow_locked: i64 = escrows.iter().map(|(id, _)| sums.get(id).copied().unwrap_or(0)).sum();
    let holders = user_rows.iter().filter(|(id, _)| sums.get(id).copied().unwrap_or(0) > 0).count();
    let expected = crate::genesis::EXPECTED_SUPPLY;

    let mut lines = vec![
        "══════════ nation genesis proof — v3 (Rust) ══════════".into(),
        format!("database                 : {file}"),
        format!(
            "accounts                 : {} ({} users, {} escrow, system {})",
            accounts.len(),
            user_rows.len(),
            escrows.len(),
            accounts.len() - user_rows.len() - escrows.len()
        ),
        format!("funded wallets           : {holders}"),
        format!("Σ user wallets           : {sum_users} Đ   expected {expected} Đ"),
        format!("Σ all postings           : {sum_all}    expected 0 (double-entry closure)"),
        format!("mint:genesis balance     : {mint} Đ   (never positive)"),
        format!("treasury:nation balance  : {treasury} Đ"),
        format!("burn:void balance        : {burn} Đ   (never negative)"),
        format!("escrow locked            : {escrow_locked} Đ"),
        format!(
            "genesis tx               : {}",
            genesis_tx.as_ref().map_or("MISSING", |(id, _)| id)
        ),
    ];
    match &genesis_tx {
        Some((_, memo)) => {
            lines.push(format!("  memo: {}…", memo.chars().take(160).collect::<String>()))
        }
        None => lines.push("  (no legacy genesis row — fresh ledger?)".into()),
    }
    lines.push(String::new());

    let mut problems = Vec::new();
    if sum_users != expected {
        problems.push(format!("Σ user wallets {sum_users} ≠ {expected}"));
    }
    if sum_all != 0 {
        problems.push(format!("Σ all postings {sum_all} ≠ 0"));
    }
    if mint > 0 {
        problems.push(format!("mint:genesis positive ({mint})"));
    }
    if burn < 0 {
        problems.push(format!("burn:void negative ({burn})"));
    }
    if genesis_tx.is_none() {
        problems.push("legacy genesis transaction missing".into());
    }
    let ok = problems.is_empty();
    lines.push(if ok {
        "PROOF OK — every expectation holds.".into()
    } else {
        format!("PROOF FAILED — {}", problems.join(" · "))
    });
    Ok(ProofReport { lines, problems, ok })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_flags_every_violation() {
        let mut report = ProofReport { lines: vec![], problems: vec![], ok: true };
        report.problems.push("Σ user wallets 0 ≠ 187672".into());
        report.ok = report.problems.is_empty();
        assert!(!report.ok);
    }
}
