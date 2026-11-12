//! پورت `back/server/scripts/migrate-legacy.ts` — پل روز-اول از دیتابیس
//! بازیابی‌شدهٔ Railway به v3.
//!
//! رفتار یک‌به‌یک با نسخهٔ TS:
//!  ۱. برنامهٔ genesis از ردیف‌های خام لگاسی (floor هر شهروند — ADR-002)
//!  ۲. اگر genesis قبلاً هست (`legacy:genesis:v1`) → خروج امنِ replay با
//!     گزارش آشتی — اجرای دوم هرگز دوبار ضرب نمی‌کند.
//!  ۳. dry-run → فقط برنامه، هیچ نوشتنی.
//!  ۴. درج: ۴۶۶ حساب `user:tg:<id>` + تراکنش genesis + ردیف audit
//!     `legacy.adjustment` با ریز گپ −63.0 + ۳۰ محصول `legacy-<id>`.
//!
//! یک تفاوت عمدی و سخت‌گیرانه‌تر از TS: ردیف‌های مالی genesis مستقیم درج
//! نمی‌شوند؛ از تابع خالص `post()` دامنهٔ فاز ۱ می‌گذرند (الگوی
//! load → apply → persist). یعنی اگر شهروندی floor منفی داشته باشد یا
//! ردیف‌ها ناتوازن شوند، «دامنه» مهاجرت را رد می‌کند — fail-closed.

use crate::error::{DbError, DbResult};
use crate::genesis::{plan_genesis, GENESIS_KEY, GENESIS_TX_ID};
use crate::store::{
    insert_audit, insert_tx_with_postings, load_ledger_state, now_iso, tx_by_key, V3Store,
};
use dropagentx_domain::{AccountKind, PostCommand, Posting, MINT_ACCOUNT_ID};
use sqlx::Row;

/// گزارش مهاجرت — خطوط آمادهٔ چاپ + پرچم‌های وضعیت.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrateReport {
    /// خطوط گزارش (فارسی/انگلیسی مخلوط مثل کنسول TS)
    pub lines: Vec<String>,
    /// آیا genesis از قبل بود و replay شد؟
    pub replayed: bool,
    /// آیا dry-run بود؟
    pub dry_run: bool,
}

/// گزارش آشتی — همان `reconcile` در TS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileReport {
    /// Σ کیف کاربران — عدد پذیرش: ۱۸۷٬۶۷۲ Đ
    pub users_sum: i64,
    /// تعداد کاربران
    pub user_count: usize,
    /// تعداد کل حساب‌ها (کاربر + سیستمی)
    pub account_count: usize,
    /// آیا تراز آزمایشی برقرار است؟
    pub balanced: bool,
    /// Σ همهٔ ردیف‌ها — باید ۰ باشد
    pub trial_sum: i64,
    /// تعداد محصولات موجود
    pub product_count: i64,
    /// تعداد محصولات لگاسی که باید وارد می‌شدند
    pub expected_products: i64,
    /// تعداد ردیف‌های تعدیل لگاسی
    pub adjustment_rows: i64,
}

impl ReconcileReport {
    /// خطوط چاپی گزارش — همان قالب TS.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        vec![
            format!(
                "  Σ user wallets        = {} Đ  (acceptance: {})",
                self.users_sum,
                crate::genesis::EXPECTED_SUPPLY
            ),
            format!("  trialBalance.balanced = {} (Σ postings {})", self.balanced, self.trial_sum),
            format!(
                "  accounts              = {} ({} users + system)",
                self.account_count, self.user_count
            ),
            format!("  products              = {}/{}", self.product_count, self.expected_products),
            format!("  adjustment rows       = {}", self.adjustment_rows),
        ]
    }
}

/// آشتی فروشگاه هدف — برای خروج replay و پایان مهاجرت (مثل `reconcile` در TS).
///
/// # Errors
/// خطای SQL.
pub async fn reconcile(store: &V3Store, expected_products: i64) -> DbResult<ReconcileReport> {
    let accounts = store.list_accounts_with_balances().await?;
    let users: Vec<_> = accounts.iter().filter(|a| a.id.starts_with("user:")).collect();
    let users_sum: i64 = users.iter().map(|a| a.balance).sum();
    let pool = store.pool_ref();
    let products: i64 =
        sqlx::query(r#"SELECT COUNT(*) AS "c" FROM "Product""#).fetch_one(pool).await?.get("c");
    let adjustments: i64 = sqlx::query(
        r#"SELECT COUNT(*) AS "c" FROM "AuditLog" WHERE "action" = 'legacy.adjustment'"#,
    )
    .fetch_one(pool)
    .await?
    .get("c");
    let summary = store.economy_summary().await?;
    Ok(ReconcileReport {
        users_sum,
        user_count: users.len(),
        account_count: accounts.len(),
        balanced: summary.balanced,
        trial_sum: summary.trial_sum,
        product_count: products,
        expected_products,
        adjustment_rows: adjustments,
    })
}

/// اجرای مهاجرت لگاسی روی فروشگاه هدف.
///
/// # Errors
/// خطای SQL، نقض ثبات در دامنه (fail-closed)، یا لگاسی خوانده‌نشدنی.
pub async fn migrate_legacy(
    store: &V3Store,
    users: &[crate::genesis::LegacyUserRow],
    products: &[crate::genesis::LegacyProductRow],
    tx_count: i64,
    source: &str,
    dry_run: bool,
) -> DbResult<MigrateReport> {
    let plan = plan_genesis(users);
    let mut lines = vec![
        format!(
            "[migrate] legacy users: {} (banned {}) · Σ credits {:.1} Đ · tx rows {tx_count}",
            users.len(),
            plan.banned,
            plan.sum_credits
        ),
        format!(
            "[migrate] per-citizen floor: Σ {} Đ across {} holder(s)",
            plan.sum_din, plan.holders
        ),
        format!("[migrate] legacy products: {}", products.len()),
    ];

    store.ensure_system_accounts().await?;

    // اجرای دوم → خروج امنِ replay با گزارش آشتی (بدون دو ضرب).
    let mut conn = store.acquire_conn().await?;
    if tx_by_key(&mut conn, GENESIS_KEY).await?.is_some() {
        lines.push("[migrate] genesis already present — replay-safe exit.".into());
        let rec = reconcile(store, i64::try_from(products.len()).unwrap_or(i64::MAX)).await?;
        lines.extend(rec.lines());
        return Ok(MigrateReport { lines, replayed: true, dry_run });
    }

    if dry_run {
        lines.push("[migrate] dry-run — no writes. Plan:".into());
        lines.push(format!("  accounts: {} user account(s) + 4 system accounts", users.len()));
        lines.push(format!(
            "  genesis tx: mint:genesis −{} → {} user posting(s)",
            plan.sum_din, plan.holders
        ));
        lines.push(format!("  products: {} row(s)", products.len()));
        return Ok(MigrateReport { lines, replayed: false, dry_run });
    }

    // تراکنش اصلی — همه یا هیچ.
    let mut tx = store.begin_tx().await?;
    for c in &plan.citizens {
        crate::store::insert_account(&mut tx, &c.account_id, AccountKind::User, Some(&c.label))
            .await?;
    }

    // genesis از مسیر دامنهٔ خالص — fail-closed (سخت‌گیرانه‌تر از TS).
    let ledger = load_ledger_state(&mut tx).await?;
    let mut postings = vec![Posting::new(MINT_ACCOUNT_ID, -plan.sum_din)];
    postings.extend(
        plan.citizens
            .iter()
            .filter(|c| c.din > 0)
            .map(|c| Posting::new(c.account_id.clone(), c.din)),
    );
    let cmd = PostCommand {
        id: Some(GENESIS_TX_ID.into()),
        idempotency_key: Some(GENESIS_KEY.into()),
        postings,
        memo: Some(plan.memo.clone()),
        at: Some(now_iso()),
    };
    let posted = dropagentx_domain::post(&ledger, &cmd).map_err(DbError::from)?;
    insert_tx_with_postings(&mut tx, &posted.tx, Some(GENESIS_KEY)).await?;

    // ردیف audit — ریز گپ −63.0 برای همیشه قابل اثبات می‌ماند.
    let meta = serde_json::json!({
        "legacyUsers": users.len(),
        "legacyTransactions": tx_count,
        "storedBalanceSum": plan.sum_credits,
        "perCitizenGenesis": plan.sum_din,
        "impliedByLegacyLedger": 187_735.8,
        "gap": -63.0,
        "gapItemization": "9×50 purchases without tx rows (−450) + manual owner credit (+387)",
        "source": source,
    })
    .to_string();
    insert_audit(&mut tx, "legacy-migration", "legacy.adjustment", Some("ledger"), Some(&meta))
        .await?;

    // ۳۰ محصول لگاسی — تحویل فقط از `link` (فایل‌های دیسک Railway برای همیشه رفتند).
    for p in products {
        let title: String = p.title.chars().take(120).collect();
        let description: String =
            p.description.clone().unwrap_or_default().chars().take(800).collect();
        let category = p.category.as_ref().map(|c| c.chars().take(60).collect::<String>());
        let delivery = p.link.as_ref().filter(|l| !l.is_empty()).cloned();
        sqlx::query(
            r#"INSERT INTO "Product" ("id", "sellerId", "title", "description", "priceDin",
                 "category", "deliveryPayload", "active", "createdAt")
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT("id") DO NOTHING"#,
        )
        .bind(format!("legacy-{}", p.id))
        .bind(format!("user:tg:{}", p.creator_id))
        .bind(title)
        .bind(description)
        .bind(p.price_credits.floor() as i64)
        .bind(category)
        .bind(delivery)
        .bind(p.is_active == 1)
        .bind(now_iso())
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    lines.push("[migrate] done.".into());
    let rec = reconcile(store, i64::try_from(products.len()).unwrap_or(i64::MAX)).await?;
    lines.extend(rec.lines());
    Ok(MigrateReport { lines, replayed: false, dry_run })
}
