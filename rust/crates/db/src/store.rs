//! آداپتور ماندگاری — «تنها» جایی که دامنهٔ خالص به حافظه می‌رسد (ADR-001/003).
//!
//! طراحی (دقیقاً همان الگوی `back/server/src/persistence.ts` با sqlx):
//!  - جدول `Posting` تنها منبع حقیقت است؛ ترازها همیشه با `SUM(amount)`
//!    مشتق می‌شوند و هیچ‌جا ذخیره نمی‌شوند.
//!  - هر فرمان داخل یک تراکنش تعاملی اجرا می‌شود:
//!    ۱. بارگذاری کامل وضعیت دفتر — حساب‌ها، ترازهای مشتق‌شده، تراکنش‌ها، کلیدهای یکسان‌سازی
//!    ۲. اجرای closure دامنهٔ خالص (بدون IO — همان توابع فاز ۱)
//!    ۳. درج تراکنش + ردیف‌ها، upsert اسکرو، رویدادهای outbox و audit
//!  - یکسان‌سازی: پیش‌چکِ replay داخل تراکنش مسیر عادی را کوتاه می‌کند؛
//!    مسابقهٔ همزمانی را قید UNIQUE روی `Tx.idempotencyKey` می‌بندد — یعنی
//!    «مسابقه‌ی retry دوبار پول جابه‌جا نمی‌کند». در برخورد، درخواست به یک
//!    خواندنِ صادقانهٔ replay تبدیل می‌شود (رفتار P2002 در TS).
//!
//! نکتهٔ مقیاس: v3 کل وضعیت را در هر فرمان بار می‌کند (اول درستی، بعد سرعت).
//! در مقیاس فعلی (~۱.۵k تراکنش) این کار در حد میکروثانیه است؛ مدل خواندن
//! جداگانه فقط وقتی دفتر چند مرتبه بزرگر شد.

use crate::error::{DbError, DbResult};
use crate::wire;
use dropagentx_domain::{
    escrow_account_id, Account, AccountKind, Din, DomainEvent, DomainResult, Escrow,
    EscrowRegistry, EscrowStatus, LedgerState, PostedTransaction, Posting, BURN_ACCOUNT_ID,
    MINT_ACCOUNT_ID, TREASURY_ACCOUNT_ID,
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, Sqlite, SqliteConnection, SqlitePool,
};
use std::{path::Path, time::Duration};

/// فاصلهٔ امن قبل از رد شدن به‌خاطر قفل نویسندهٔ SQLite.
const BUSY_TIMEOUT: Duration = Duration::from_secs(10);

/// آداپتور فروشگاه v3 — نقطهٔ تنها اتصال دامنه به SQLite.
#[derive(Debug, Clone)]
pub struct V3Store {
    pool: SqlitePool,
    /// مسیر فایل دیتابیس — `None` یعنی درون‌حافظه‌ای (بکاپ VACUUM INTO اعمال نمی‌شود).
    file_path: Option<String>,
}

/// خروجی closure دامنه — همان `CommandOutcome` در TS.
#[derive(Debug, Clone)]
pub struct CommandOutcome {
    /// وضعیت جدید دفتر (خروجی دامنه)
    pub ledger: LedgerState,
    /// تراکنش جدید (اگر فرمان پول جابه‌جا کرد)
    pub tx: Option<PostedTransaction>,
    /// اسکرو دست‌خورده (اگر فرمان اسکرو بود)
    pub escrow: Option<Escrow>,
    /// رویدادهای دامنه برای outbox
    pub events: Vec<DomainEvent>,
}

/// گزینه‌های اجرای فرمان — همان `ApplyOptions` در TS.
#[derive(Debug, Clone)]
pub struct ApplyOptions {
    /// اجراکننده (برای audit)
    pub actor: String,
    /// نام فرمان (برای audit)
    pub action: String,
    /// هدف اختیاری (برای audit)
    pub target: Option<String>,
    /// اسکرو هدف — مستند در reportها
    pub escrow_id: Option<String>,
    /// کلید یکسان‌سازی — همین که UNIQUE می‌شود در `Tx`
    pub idempotency_key: Option<String>,
}

/// نتیجهٔ اجرای فرمان — همان `ApplyResult` در TS.
#[derive(Debug, Clone)]
pub struct ApplyResult {
    /// آیا این یک پخش مجدد بود؟ (پول دوباره جابه‌جا نشده)
    pub replayed: bool,
    /// تراکنش (جدید یا اصلی در replay)
    pub tx: Option<PostedTransaction>,
    /// اسکرو (اگر مربوط به فرمان بود)
    pub escrow: Option<Escrow>,
    /// وضعیت دفتر پس از اجرا
    pub ledger: LedgerState,
}

/// تراز حساب + متادیتا — برای گزارش‌ها و داشبورد.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountBalance {
    /// شناسه حساب
    pub id: String,
    /// رشتهٔ کانانیکی نوع (`user`، `escrow` و…)
    pub kind: String,
    /// برچسب اختیاری
    pub label: Option<String>,
    /// تراز مشتق‌شده از `SUM(Posting.amount)`
    pub balance: i64,
    /// زمان ساخت ISO
    pub created_at: String,
}

/// خلاصهٔ اقتصاد — همان `economySummary` در TS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomySummary {
    /// پول در دست شهروندان + اسکرو + خزانه
    pub circulating_supply: i64,
    /// تراز خزانه
    pub treasury_balance: i64,
    /// پول قفل‌شده در اسکروها
    pub escrow_locked: i64,
    /// Σ کیف کاربران — عدد پذیرش ۱۸۷,۶۷۲ Đ
    pub users_sum: i64,
    /// تراز حساب ضرب (هرگز مثبت نیست)
    pub mint_balance: i64,
    /// تراز حساب سوزاندن (هرگز منفی نیست)
    pub burn_balance: i64,
    /// ضرب سال جاری UTC
    pub minted_this_year: i64,
    /// Σ همه ترازها — تقدس: همیشه ۰
    pub trial_sum: i64,
    /// آیا `trial_sum == 0`؟
    pub balanced: bool,
    /// تعداد اسکروهای باز (CREATED/FUNDED/DELIVERED/DISPUTED)
    pub open_escrows: i64,
}

/// ردیف تاریخچهٔ کیف — سوخت مهارت `wallet_history` (فاز ۶).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentPosting {
    /// شناسه تراکنش
    pub tx_id: String,
    /// مقدار علامت‌دار
    pub amount: i64,
    /// یادداشت
    pub memo: Option<String>,
    /// زمان ISO
    pub at: String,
}

/// ردیف outbox برای دیسپچ (فاز ۵).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxRow {
    /// شناسه ردیف outbox
    pub id: String,
    /// رشتهٔ `kind` رویداد
    pub kind: String,
    /// JSON رویداد (شکل سیمی TS)
    pub payload: String,
    /// زمان ساخت ISO
    pub created_at: String,
    /// ارسال شده؟
    pub published: bool,
}

/// زمان ISO-8601 UTC با میلی‌ثانیه — قرارداد `at` دامنه (قالب `EPOCH_ISO`).
#[must_use]
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// شروع سال UTC — برای سقف ضرب سالانه.
fn year_start_iso() -> DbResult<String> {
    use chrono::{Datelike, TimeZone};
    let y = chrono::Utc::now().year();
    let start = chrono::Utc
        .with_ymd_and_hms(y, 1, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| DbError::Config(format!("invalid year start for {y}")))?;
    Ok(start.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

fn kind_from_str(s: &str) -> DbResult<AccountKind> {
    match s {
        "user" => Ok(AccountKind::User),
        "escrow" => Ok(AccountKind::Escrow),
        "treasury" => Ok(AccountKind::Treasury),
        "mint" => Ok(AccountKind::Mint),
        "burn" => Ok(AccountKind::Burn),
        other => Err(DbError::Corrupt(format!("unknown account kind {other:?}"))),
    }
}

fn status_from_str(s: &str) -> DbResult<EscrowStatus> {
    match s {
        "CREATED" => Ok(EscrowStatus::Created),
        "FUNDED" => Ok(EscrowStatus::Funded),
        "DELIVERED" => Ok(EscrowStatus::Delivered),
        "RELEASED" => Ok(EscrowStatus::Released),
        "DISPUTED" => Ok(EscrowStatus::Disputed),
        "REFUNDED" => Ok(EscrowStatus::Refunded),
        "CANCELLED" => Ok(EscrowStatus::Cancelled),
        "EXPIRED" => Ok(EscrowStatus::Expired),
        other => Err(DbError::Corrupt(format!("unknown escrow status {other:?}"))),
    }
}

/// آیا این خطا نقض UNIQUE روی `Tx.idempotencyKey` است؟
fn is_idempotency_conflict(err: &DbError) -> bool {
    match err {
        DbError::Sql(sqlx::Error::Database(db)) => {
            db.constraint().is_some_and(|c| c == "Tx.idempotencyKey")
                || db.message().contains("UNIQUE constraint failed: Tx.idempotencyKey")
        }
        _ => false,
    }
}

// ── کوئری‌های مشترک — روی یک اتصال مشخص (داخل تراکنش یا خام) ─────────────────

pub(crate) async fn load_ledger_state(exec: &mut SqliteConnection) -> DbResult<LedgerState> {
    let mut accounts = std::collections::BTreeMap::new();
    let mut balances = std::collections::BTreeMap::new();
    let rows = sqlx::query(r#"SELECT "id", "kind", "label" FROM "Account" ORDER BY "id""#)
        .fetch_all(&mut *exec)
        .await?;
    for r in rows {
        let id: String = r.get("id");
        let kind = kind_from_str(&r.get::<String, _>("kind"))?;
        let label: Option<String> = r.get("label");
        balances.insert(id.clone(), 0);
        accounts.insert(id.clone(), Account { id, kind, label });
    }
    // ترازها = مشتق از ردیف‌ها (ADR-003) — حساب بدون ردیف صفر می‌ماند.
    let sums = sqlx::query(
        r#"SELECT "accountId", SUM("amount") AS "s" FROM "Posting" GROUP BY "accountId""#,
    )
    .fetch_all(&mut *exec)
    .await?;
    for r in sums {
        balances.insert(r.get::<String, _>("accountId"), r.get::<i64, _>("s"));
    }

    let mut txs = std::collections::BTreeMap::new();
    let mut idempotency = std::collections::BTreeMap::new();
    let tx_rows = sqlx::query(r#"SELECT "id", "idempotencyKey", "memo", "at" FROM "Tx""#)
        .fetch_all(&mut *exec)
        .await?;
    let mut by_tx: std::collections::BTreeMap<String, Vec<Posting>> =
        std::collections::BTreeMap::new();
    for r in sqlx::query(r#"SELECT "txId", "accountId", "amount" FROM "Posting" ORDER BY "id" ASC"#)
        .fetch_all(&mut *exec)
        .await?
    {
        by_tx
            .entry(r.get::<String, _>("txId"))
            .or_default()
            .push(Posting { account_id: r.get("accountId"), amount: r.get("amount") });
    }
    for r in tx_rows {
        let id: String = r.get("id");
        let key: Option<String> = r.get("idempotencyKey");
        if let Some(k) = &key {
            idempotency.insert(k.clone(), id.clone());
        }
        let postings = by_tx.remove(&id).unwrap_or_default();
        txs.insert(
            id.clone(),
            PostedTransaction {
                id,
                idempotency_key: key,
                postings,
                memo: r.get("memo"),
                at: r.get("at"),
            },
        );
    }
    Ok(LedgerState { accounts, balances, txs, idempotency })
}

pub(crate) async fn load_escrow_registry(
    exec: &mut SqliteConnection,
    escrow_id: Option<&str>,
) -> DbResult<EscrowRegistry> {
    // انحراف آگاهانه از TS: ثبت‌نام کامل بارگذاری می‌شود (در TS فقط اسکروی
    // هدف)، تا `EscrowAlreadyExists` در دامنه واقعاً قابل کشف باشد. در مقیاس
    // فعلی جدول اسکرو کوچک است.
    let rows = match escrow_id {
        Some(id) => {
            sqlx::query(r#"SELECT * FROM "Escrow" WHERE "id" = ?"#)
                .bind(id)
                .fetch_all(&mut *exec)
                .await?
        }
        None => sqlx::query(r#"SELECT * FROM "Escrow""#).fetch_all(&mut *exec).await?,
    };
    let mut escrows = std::collections::BTreeMap::new();
    for r in rows {
        let escrow = row_to_escrow(&r)?;
        escrows.insert(escrow.id.clone(), escrow);
    }
    Ok(EscrowRegistry { escrows })
}

pub(crate) fn row_to_escrow(r: &sqlx::sqlite::SqliteRow) -> DbResult<Escrow> {
    Ok(Escrow {
        id: r.get("id"),
        buyer_id: r.get("buyerId"),
        seller_id: r.get("sellerId"),
        amount: Din::new(r.get::<i64, _>("amount"))?,
        product_ref: r.get("productRef"),
        status: status_from_str(&r.get::<String, _>("status"))?,
        created_at: r.get("createdAt"),
        funded_tx_id: r.get("fundedTxId"),
        settle_tx_id: r.get("settleTxId"),
        dispute_reason: r.get("disputeReason"),
        delivery_ref: r.get("deliveryRef"),
        expires_at: r.get("expiresAt"),
    })
}

pub(crate) async fn tx_by_key(
    exec: &mut SqliteConnection,
    key: &str,
) -> DbResult<Option<PostedTransaction>> {
    let Some(r) = sqlx::query(
        r#"SELECT "id", "idempotencyKey", "memo", "at" FROM "Tx" WHERE "idempotencyKey" = ?"#,
    )
    .bind(key)
    .fetch_optional(&mut *exec)
    .await?
    else {
        return Ok(None);
    };
    let id: String = r.get("id");
    let postings: Vec<Posting> = sqlx::query(
        r#"SELECT "accountId", "amount" FROM "Posting" WHERE "txId" = ? ORDER BY "id" ASC"#,
    )
    .bind(&id)
    .fetch_all(&mut *exec)
    .await?
    .into_iter()
    .map(|p| Posting { account_id: p.get("accountId"), amount: p.get("amount") })
    .collect();
    Ok(Some(PostedTransaction {
        idempotency_key: r.get("idempotencyKey"),
        postings,
        memo: r.get("memo"),
        at: r.get("at"),
        id,
    }))
}

pub(crate) async fn load_escrow_opt(
    exec: &mut SqliteConnection,
    escrow_id: Option<&str>,
) -> DbResult<Option<Escrow>> {
    let Some(id) = escrow_id else { return Ok(None) };
    sqlx::query(r#"SELECT * FROM "Escrow" WHERE "id" = ?"#)
        .bind(id)
        .fetch_optional(&mut *exec)
        .await?
        .as_ref()
        .map(row_to_escrow)
        .transpose()
}

pub(crate) async fn insert_account(
    exec: &mut SqliteConnection,
    id: &str,
    kind: AccountKind,
    label: Option<&str>,
) -> DbResult<()> {
    sqlx::query(
        r#"INSERT INTO "Account" ("id", "kind", "label", "createdAt") VALUES (?, ?, ?, ?)
           ON CONFLICT("id") DO NOTHING"#,
    )
    .bind(id)
    .bind(kind.as_str())
    .bind(label)
    .bind(now_iso())
    .execute(&mut *exec)
    .await?;
    Ok(())
}

pub(crate) async fn insert_tx_with_postings(
    exec: &mut SqliteConnection,
    txn: &PostedTransaction,
    idempotency_override: Option<&str>,
) -> DbResult<()> {
    let key = idempotency_override.map(str::to_owned).or_else(|| txn.idempotency_key.clone());
    sqlx::query(r#"INSERT INTO "Tx" ("id", "idempotencyKey", "memo", "at") VALUES (?, ?, ?, ?)"#)
        .bind(&txn.id)
        .bind(key)
        .bind(&txn.memo)
        .bind(&txn.at)
        .execute(&mut *exec)
        .await?;
    for p in &txn.postings {
        sqlx::query(r#"INSERT INTO "Posting" ("txId", "accountId", "amount") VALUES (?, ?, ?)"#)
            .bind(&txn.id)
            .bind(&p.account_id)
            .bind(p.amount)
            .execute(&mut *exec)
            .await?;
    }
    Ok(())
}

pub(crate) async fn upsert_escrow(exec: &mut SqliteConnection, escrow: &Escrow) -> DbResult<()> {
    insert_account(
        exec,
        &escrow_account_id(&escrow.id),
        AccountKind::Escrow,
        Some(&format!("escrow {}", escrow.id)),
    )
    .await?;
    sqlx::query(
        r#"INSERT INTO "Escrow" ("id", "buyerId", "sellerId", "amount", "productRef",
             "status", "createdAt", "fundedTxId", "settleTxId", "disputeReason",
             "deliveryRef", "expiresAt")
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT("id") DO UPDATE SET
             "buyerId" = excluded."buyerId", "sellerId" = excluded."sellerId",
             "amount" = excluded."amount", "productRef" = excluded."productRef",
             "status" = excluded."status", "createdAt" = excluded."createdAt",
             "fundedTxId" = excluded."fundedTxId", "settleTxId" = excluded."settleTxId",
             "disputeReason" = excluded."disputeReason", "deliveryRef" = excluded."deliveryRef",
             "expiresAt" = excluded."expiresAt""#,
    )
    .bind(&escrow.id)
    .bind(&escrow.buyer_id)
    .bind(&escrow.seller_id)
    .bind(escrow.amount.as_i64())
    .bind(&escrow.product_ref)
    .bind(escrow.status.as_str())
    .bind(&escrow.created_at)
    .bind(&escrow.funded_tx_id)
    .bind(&escrow.settle_tx_id)
    .bind(&escrow.dispute_reason)
    .bind(&escrow.delivery_ref)
    .bind(&escrow.expires_at)
    .execute(&mut *exec)
    .await?;
    Ok(())
}

pub(crate) async fn insert_outbox_events(
    exec: &mut SqliteConnection,
    events: &[DomainEvent],
) -> DbResult<()> {
    for e in events {
        sqlx::query(
            r#"INSERT INTO "OutboxEvent" ("id", "kind", "payload", "createdAt", "published")
               VALUES (?, ?, ?, ?, 0)"#,
        )
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(wire::event_kind(e))
        .bind(wire::event_payload(e))
        .bind(now_iso())
        .execute(&mut *exec)
        .await?;
    }
    Ok(())
}

pub(crate) async fn insert_audit(
    exec: &mut SqliteConnection,
    actor: &str,
    action: &str,
    target: Option<&str>,
    meta: Option<&str>,
) -> DbResult<()> {
    sqlx::query(
        r#"INSERT INTO "AuditLog" ("id", "actor", "action", "target", "meta", "at")
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(uuid::Uuid::now_v7().to_string())
    .bind(actor)
    .bind(action)
    .bind(target)
    .bind(meta)
    .bind(now_iso())
    .execute(&mut *exec)
    .await?;
    Ok(())
}

/// درج خروجی یک فرمان دامنه داخل تراکنش باز — در فاز ۴ مسیر تصمیم HITL
/// («تأیید ⇒ تسویه در همان تراکنش») هم از همین نقطهٔ واحد عبور می‌کند.
pub(crate) async fn persist_outcome(
    exec: &mut SqliteConnection,
    opts: &ApplyOptions,
    outcome: &CommandOutcome,
) -> DbResult<()> {
    if let Some(escrow) = &outcome.escrow {
        upsert_escrow(exec, escrow).await?;
    }
    if let Some(txn) = &outcome.tx {
        // کلید فرمان بر کلید دامنه مقدم است — همان `options.idempotencyKey ?? ...` در TS
        insert_tx_with_postings(exec, txn, opts.idempotency_key.as_deref()).await?;
    }
    crate::outbox::enqueue(exec, &outcome.events).await?;
    let meta = outcome.tx.as_ref().map(|t| serde_json::json!({ "txId": t.id }).to_string());
    crate::outbox::audit(exec, &opts.actor, &opts.action, opts.target.as_deref(), meta.as_deref())
        .await?;
    Ok(())
}

impl V3Store {
    /// باز کردن با URL هدف — `file:./db/v3.db`، `sqlite::memory:` یا مسیر خام.
    ///
    /// # Errors
    /// مسیر/URL نامعتبر یا خطای باز شدن SQLite.
    pub async fn connect(url: &str) -> DbResult<Self> {
        let normalized = url.strip_prefix("file:").unwrap_or(url);
        let in_memory = normalized == ":memory:" || normalized == "sqlite::memory:";
        let mut opts = if in_memory {
            // درون‌حافظه‌ای + اشتراک حافظه پنهان — با تک‌اتصالِ زیر، دیتابیس
            // تا پایان عمر pool یکپارچه می‌ماند.
            SqliteConnectOptions::new().in_memory(true).shared_cache(true)
        } else {
            let path = Path::new(normalized);
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        DbError::Config(format!("cannot create dir {}: {e}", parent.display()))
                    })?;
                }
            }
            SqliteConnectOptions::new().filename(normalized)
        };
        opts = opts.create_if_missing(true).foreign_keys(true).busy_timeout(BUSY_TIMEOUT);
        // ::memory: با چند اتصال = چند دیتابیس جدا؛ تک‌اتصال می‌گذاریم.
        let max = if in_memory { 1 } else { 4 };
        let pool = SqlitePoolOptions::new().max_connections(max).connect_with(opts).await?;
        let file_path = if in_memory { None } else { Some(normalized.to_owned()) };
        Ok(Self { pool, file_path })
    }

    /// فروشگاه درون‌حافظه‌ای — برای تست‌ها.
    ///
    /// # Errors
    /// خطای باز شدن SQLite (نمی‌افتد در عمل).
    pub async fn connect_in_memory() -> DbResult<Self> {
        Self::connect("sqlite::memory:").await
    }

    /// باز کردن فقط‌خواندنی — برای ممیزی خارجی (genesis-proof).
    ///
    /// # Errors
    /// فایل وجود نداشته باشد یا SQLite باز نکند.
    pub async fn connect_read_only(path: &str) -> DbResult<Self> {
        let file = path.strip_prefix("file:").unwrap_or(path);
        let opts =
            SqliteConnectOptions::new().filename(file).read_only(true).busy_timeout(BUSY_TIMEOUT);
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await?;
        Ok(Self { pool, file_path: Some(file.to_owned()) })
    }

    /// آیا این فروشگاه روی فایل است؟ — بکاپ `VACUUM INTO` فقط برای فروشگاه
    /// فایل‌محور معنا دارد (`BACKUP_UNSUPPORTED` برای درون‌حافظه‌ای).
    #[must_use]
    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    /// مهاجرت‌های نسخه‌دار اسکیما — `migrations/0001_init.sql` (idempotent؛
    /// روی دیتابیس ساخته‌شده توسط Prisma هم بی‌اثر اجرا می‌شود).
    ///
    /// # Errors
    /// خطای اجرای مهاجرت.
    pub async fn migrate(&self) -> DbResult<()> {
        sqlx::migrate!().run(&self.pool).await?;
        Ok(())
    }

    /// دسترسی خام به pool — برای ماژول‌های هم‌کرِیت (migrate/proof) و تست.
    pub(crate) fn pool_ref(&self) -> &SqlitePool {
        &self.pool
    }

    /// یک اتصال از pool — برای توالی کوئری‌های چندمرحله‌ای خارج از `apply`.
    ///
    /// # Errors
    /// خطای اتصال.
    pub(crate) async fn acquire_conn(&self) -> DbResult<sqlx::pool::PoolConnection<Sqlite>> {
        Ok(self.pool.acquire().await?)
    }

    /// شروع تراکنش روی pool — برای مهاجرت لگاسی (همه یا هیچ).
    ///
    /// # Errors
    /// خطای اتصال.
    pub(crate) async fn begin_tx(&self) -> DbResult<sqlx::Transaction<'static, Sqlite>> {
        Ok(self.pool.begin().await?)
    }

    /// ساخت حساب اگر وجود ندارد — ثبت‌نام شهروند جدید (فاز ۵ از این استفاده می‌کند).
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn create_account_if_missing(
        &self,
        id: &str,
        kind: AccountKind,
        label: Option<&str>,
    ) -> DbResult<()> {
        let mut tx = self.pool.begin().await?;
        insert_account(&mut tx, id, kind, label).await?;
        tx.commit().await?;
        Ok(())
    }

    /// حساب‌های سیستمی باید قبل از هر جابه‌جایی پول باشند (مادهٔ ۴ منشور).
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn ensure_system_accounts(&self) -> DbResult<()> {
        let defs = [
            (MINT_ACCOUNT_ID, AccountKind::Mint, "منبع عرضه (ضرب)"),
            (TREASURY_ACCOUNT_ID, AccountKind::Treasury, "خزانه ملی"),
            (BURN_ACCOUNT_ID, AccountKind::Burn, "مقصد سوزاندن"),
        ];
        let mut tx = self.pool.begin().await?;
        for (id, kind, label) in defs {
            insert_account(&mut tx, id, kind, Some(label)).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// قلب آداپتور — بارگذاری → دامنهٔ خالص → درج، همه در یک تراکنش.
    ///
    /// # Errors
    /// خطای SQL، یا خطای دامنه (که تراکنش را کامل پس می‌گیرد — خطا هرگز
    /// وضعیت را خراب نمی‌کند).
    pub async fn apply<F>(&self, opts: ApplyOptions, run: F) -> DbResult<ApplyResult>
    where
        F: FnOnce(&LedgerState, &EscrowRegistry) -> DomainResult<CommandOutcome>,
    {
        let mut tx = self.pool.begin().await?;

        // ۱) پیش‌چک replay — مسیر عادیِ امن برای retryهای بات/API.
        if let Some(key) = opts.idempotency_key.as_deref() {
            if let Some(existing) = tx_by_key(&mut tx, key).await? {
                let ledger = load_ledger_state(&mut tx).await?;
                let escrow = load_escrow_opt(&mut tx, opts.escrow_id.as_deref()).await?;
                tx.commit().await?;
                tracing::info!(key, action = %opts.action, "apply replayed");
                return Ok(ApplyResult { replayed: true, tx: Some(existing), escrow, ledger });
            }
        }

        // ۲) بارگذاری کامل وضعیت.
        let ledger = load_ledger_state(&mut tx).await?;
        let registry = load_escrow_registry(&mut tx, opts.escrow_id.as_deref()).await?;

        // ۳) دامنهٔ خالص — بدون IO.
        let outcome = match run(&ledger, &registry) {
            Ok(o) => o,
            Err(e) => {
                tx.rollback().await?;
                return Err(e.into());
            }
        };

        // ۴) درج + commit.
        match persist_outcome(&mut tx, &opts, &outcome).await {
            Ok(()) => tx.commit().await?,
            Err(e) => {
                tx.rollback().await?;
                if is_idempotency_conflict(&e) {
                    // مسابقه: نویسندهٔ همزمان همان کلید را زودتر ثبت کرد —
                    // قید UNIQUE نجاتمان داد؛ تبدیل به خواندنِ صادقانهٔ replay.
                    if let Some(key) = opts.idempotency_key.as_deref() {
                        let mut conn = self.pool.acquire().await?;
                        if let Some(existing) = tx_by_key(&mut conn, key).await? {
                            let ledger = load_ledger_state(&mut conn).await?;
                            let escrow =
                                load_escrow_opt(&mut conn, opts.escrow_id.as_deref()).await?;
                            tracing::info!(key, "apply replayed after unique conflict");
                            return Ok(ApplyResult {
                                replayed: true,
                                tx: Some(existing),
                                escrow,
                                ledger,
                            });
                        }
                    }
                }
                return Err(e);
            }
        }

        Ok(ApplyResult {
            replayed: false,
            tx: outcome.tx,
            escrow: outcome.escrow,
            ledger: outcome.ledger,
        })
    }

    // ── خواندن‌های فقط-خواندنی ─────────────────────────────────────────────

    /// تراز یک حساب — `None` یعنی حساب وجود ندارد.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn balance_of(&self, account_id: &str) -> DbResult<Option<i64>> {
        let Some(_) = sqlx::query(r#"SELECT "id" FROM "Account" WHERE "id" = ?"#)
            .bind(account_id)
            .fetch_optional(&self.pool)
            .await?
        else {
            return Ok(None);
        };
        let sum: Option<i64> =
            sqlx::query(r#"SELECT SUM("amount") AS "s" FROM "Posting" WHERE "accountId" = ?"#)
                .bind(account_id)
                .fetch_one(&self.pool)
                .await?
                .get("s");
        Ok(Some(sum.unwrap_or(0)))
    }

    /// همه حساب‌ها با تراز مشتق‌شده — گزارش آشتی.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn list_accounts_with_balances(&self) -> DbResult<Vec<AccountBalance>> {
        let mut out = Vec::new();
        let rows = sqlx::query(
            r#"SELECT "id", "kind", "label", "createdAt" FROM "Account" ORDER BY "id" ASC"#,
        )
        .fetch_all(&self.pool)
        .await?;
        for r in rows {
            out.push(AccountBalance {
                id: r.get("id"),
                kind: r.get("kind"),
                label: r.get("label"),
                balance: 0,
                created_at: r.get("createdAt"),
            });
        }
        let sums = sqlx::query(
            r#"SELECT "accountId", SUM("amount") AS "s" FROM "Posting" GROUP BY "accountId""#,
        )
        .fetch_all(&self.pool)
        .await?;
        for r in sums {
            let id: String = r.get("accountId");
            if let Some(a) = out.iter_mut().find(|a| a.id == id) {
                a.balance = r.get("s");
            }
        }
        Ok(out)
    }

    /// خلاصهٔ اقتصاد — همان `economySummary` در TS (برای داشبورد و آشتی).
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn economy_summary(&self) -> DbResult<EconomySummary> {
        let accounts = self.list_accounts_with_balances().await?;
        let mut s = EconomySummary {
            circulating_supply: 0,
            treasury_balance: 0,
            escrow_locked: 0,
            users_sum: 0,
            mint_balance: 0,
            burn_balance: 0,
            minted_this_year: self.minted_this_year().await?,
            trial_sum: 0,
            balanced: false,
            open_escrows: self.open_escrow_count().await?,
        };
        for a in &accounts {
            s.trial_sum += a.balance;
            match a.kind.as_str() {
                "user" => s.users_sum += a.balance,
                "escrow" => s.escrow_locked += a.balance,
                "treasury" => s.treasury_balance += a.balance,
                "mint" => s.mint_balance += a.balance,
                "burn" => s.burn_balance += a.balance,
                _ => return Err(DbError::Corrupt(format!("unknown kind {:?}", a.kind))),
            }
        }
        s.circulating_supply = s.users_sum + s.escrow_locked + s.treasury_balance;
        s.balanced = s.trial_sum == 0;
        Ok(s)
    }

    /// تعداد اسکروهای باز.
    async fn open_escrow_count(&self) -> DbResult<i64> {
        let n: i64 = sqlx::query(
            r#"SELECT COUNT(*) AS "c" FROM "Escrow"
               WHERE "status" IN ('CREATED', 'FUNDED', 'DELIVERED', 'DISPUTED')"#,
        )
        .fetch_one(&self.pool)
        .await?
        .get("c");
        Ok(n)
    }

    /// ضرب سال جاری UTC — قرارداد سقف منشور.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn minted_this_year(&self) -> DbResult<i64> {
        let sum: Option<i64> = sqlx::query(
            r#"SELECT SUM(p."amount") AS "s" FROM "Posting" p
               JOIN "Tx" t ON t."id" = p."txId"
               WHERE p."accountId" = ? AND t."at" >= ?"#,
        )
        .bind(MINT_ACCOUNT_ID)
        .bind(year_start_iso()?)
        .fetch_one(&self.pool)
        .await?
        .get("s");
        Ok(-sum.unwrap_or(0))
    }

    /// آخرین ردیف‌های یک حساب — سوخت `wallet_history`.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn recent_postings_for(
        &self,
        account_id: &str,
        limit: i64,
    ) -> DbResult<Vec<RecentPosting>> {
        let mut out = Vec::new();
        for r in sqlx::query(
            r#"SELECT p."txId" AS "txId", p."amount" AS "amount", t."memo" AS "memo", t."at" AS "at"
               FROM "Posting" p JOIN "Tx" t ON t."id" = p."txId"
               WHERE p."accountId" = ? ORDER BY p."id" DESC LIMIT ?"#,
        )
        .bind(account_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?
        {
            out.push(RecentPosting {
                tx_id: r.get("txId"),
                amount: r.get("amount"),
                memo: r.get("memo"),
                at: r.get("at"),
            });
        }
        Ok(out)
    }

    /// یک اسکرو با شناسه.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn get_escrow(&self, id: &str) -> DbResult<Option<Escrow>> {
        sqlx::query(r#"SELECT * FROM "Escrow" WHERE "id" = ?"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(row_to_escrow)
            .transpose()
    }

    /// فهرست اسکروها — اختیاری با فیلتر وضعیت، حداکثر ۲۰۰ (مثل TS).
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn list_escrows(&self, status: Option<EscrowStatus>) -> DbResult<Vec<Escrow>> {
        let rows = match status {
            Some(st) => sqlx::query(
                r#"SELECT * FROM "Escrow" WHERE "status" = ? ORDER BY "createdAt" DESC LIMIT 200"#,
            )
            .bind(st.as_str())
            .fetch_all(&self.pool)
            .await?,
            None => {
                sqlx::query(r#"SELECT * FROM "Escrow" ORDER BY "createdAt" DESC LIMIT 200"#)
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        rows.iter().map(row_to_escrow).collect()
    }
}
