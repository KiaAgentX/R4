//! کوئری‌های خواندن/نوشتن فاز ۴ — پورت پرس‌وجوهای `back/server/src/*.ts` که
//! فرمان دامنه نیستند: صف HITL، محصولات داشبورد، دنبالهٔ audit، مصرف AI و
//! بکاپ `VACUUM INTO`.
//!
//! قواعد ثابت فاز ۲ سر جایشان است:
//! * ترازها هرگز ذخیره نمی‌شوند — همیشه مشتق از `Posting` (ADR-003).
//! * هر فرمان پول‌محور داخل همان تراکنشِ `apply`/تخصیص HITL هم audit و هم
//!   outbox می‌نویسد.
//! * تصمیم HITL «تأیید ⇒ تسویه» اتمی است: یک تراکنش، یا هر دو یا هیچ‌کدام.
//!
//! خطاهای صف HITL عمداً به‌صورت [`dropagentx_domain::DomainError`] با کدهای
//! دامنه برمی‌گردند (`HITL_NOT_FOUND` و…) تا نگاشت ۴۰۴/۴۲۲ِ `contracts::error`
//! بدون تکرار، درست کار کند.

use dropagentx_domain::{
    DeliverEscrowCommand, DomainError, DomainErrorCode, DomainEvent, DomainResult, Escrow,
    LedgerState, PostedTransaction, ReleaseEscrowCommand,
};

use crate::error::{DbError, DbResult};
use crate::store::insert_audit;
use crate::store::{
    load_escrow_registry, load_ledger_state, now_iso, persist_outcome, ApplyOptions,
    CommandOutcome, V3Store,
};
use crate::wire::event_payload;

// ── صف HITL ─────────────────────────────────────────────────────────────────

/// ردیف خام درخواست HITL از جدول `HitlRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitlRow {
    /// شناسه
    pub id: String,
    /// نوع (`ESCROW_RELEASE`)
    pub kind: String,
    /// امانت مربوط
    pub escrow_id: String,
    /// مبلغ
    pub amount: i64,
    /// درخواست‌دهنده
    pub requester_id: String,
    /// یادداشت اختیاری
    pub note: Option<String>,
    /// وضعیت (`PENDING`/`APPROVED`/`REJECTED`/`EXPIRED`)
    pub status: String,
    /// زمان ایجاد ISO
    pub created_at: String,
    /// زمان انقضا ISO
    pub expires_at: String,
    /// تصمیم‌گیرنده
    pub decided_by: Option<String>,
    /// زمان تصمیم
    pub decided_at: Option<String>,
    /// یادداشت تصمیم
    pub decision_note: Option<String>,
}

/// خروجی تصمیم HITL — درخواست به‌روزشده + حاصل تسویه (فقط برای APPROVE).
#[derive(Debug, Clone)]
pub struct HitlDecision {
    /// درخواست پس از تصمیم
    pub request: HitlRow,
    /// امانت پس از تسویه (فقط approve)
    pub escrow: Option<Escrow>,
    /// تراکنش تسویه (فقط approve؛ در replay تراکنش اصلی)
    pub tx: Option<PostedTransaction>,
    /// آیا تسویه یک replay بود؟
    pub replayed: bool,
}

/// فرمان تسویه با کلید قطعی — `hitl:<id>:approve` (فاز ۳ مستند کرده).
fn settlement_key(hitl_id: &str) -> String {
    format!("hitl:{hitl_id}:approve")
}

/// ردیف HITL را از یک ردیف SQL خام می‌سازد.
fn hitl_row_from(row: sqlx::sqlite::SqliteRow) -> HitlRow {
    use sqlx::Row;
    HitlRow {
        id: row.get("id"),
        kind: row.get("kind"),
        escrow_id: row.get("escrowId"),
        amount: row.get::<i64, _>("amount"),
        requester_id: row.get("requesterId"),
        note: row.get("note"),
        status: row.get("status"),
        created_at: row.get("createdAt"),
        expires_at: row.get("expiresAt"),
        decided_by: row.get("decidedBy"),
        decided_at: row.get("decidedAt"),
        decision_note: row.get("decisionNote"),
    }
}

const HITL_SELECT: &str = r#"SELECT "id","kind","escrowId","amount","requesterId","note","status","createdAt","expiresAt","decidedBy","decidedAt","decisionNote" FROM "HitlRequest""#;

impl V3Store {
    /// خواندن یک درخواست HITL — `None` یعنی وجود ندارد.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn get_hitl(&self, id: &str) -> DbResult<Option<HitlRow>> {
        let sql = format!("{HITL_SELECT} WHERE \"id\" = ?");
        let row = sqlx::query(&sql).bind(id).fetch_optional(self.pool_ref()).await?;
        Ok(row.map(hitl_row_from))
    }

    /// فهرست درخواست‌های HITL با فیلتر وضعیت اختیاری — جدیدترین اول.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn list_hitl(&self, status: Option<&str>) -> DbResult<Vec<HitlRow>> {
        let (sql, has_filter) = match status {
            Some(_) => (
                format!(
                    "{HITL_SELECT} WHERE \"status\" = ? ORDER BY \"createdAt\" DESC, \"id\" DESC"
                ),
                true,
            ),
            None => (format!("{HITL_SELECT} ORDER BY \"createdAt\" DESC, \"id\" DESC"), false),
        };
        let q = sqlx::query(&sql);
        let rows = if has_filter {
            q.bind(status.unwrap_or_default()).fetch_all(self.pool_ref()).await?
        } else {
            q.fetch_all(self.pool_ref()).await?
        };
        Ok(rows.into_iter().map(hitl_row_from).collect())
    }

    /// صف‌کردن درخواست HITL — `id` تکراری خطا نیست؛ همان درخواست موجود برمی‌گردد
    /// (replay-safe مثل `hitl.ts`). مبلغ از امانت کپی می‌شود؛ مسابقهٔ درج هم‌زمان
    /// با `ON CONFLICT DO NOTHING` + بازخوانی حل می‌شود.
    ///
    /// # Errors
    /// امانت ناموجود → `ESCROW_NOT_FOUND` (دامنه)؛ خطای SQL.
    pub async fn enqueue_hitl(&self, row: HitlRow, escrow_id: &str) -> DbResult<HitlRow> {
        // موجود بودن امانت — همان قرارداد TS (amount از امانت کپی می‌شود)
        let escrow = self.get_escrow(escrow_id).await?;
        if escrow.is_none() {
            return Err(DomainError::new(
                DomainErrorCode::EscrowNotFound,
                format!("Escrow {escrow_id} does not exist"),
            )
            .into());
        }
        let mut tx = self.pool_ref().begin().await?;
        sqlx::query(
            r#"INSERT INTO "HitlRequest"
               ("id","kind","escrowId","amount","requesterId","note","status","createdAt","expiresAt","decidedBy","decidedAt","decisionNote")
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?)
               ON CONFLICT("id") DO NOTHING"#,
        )
        .bind(&row.id)
        .bind(&row.kind)
        .bind(&row.escrow_id)
        .bind(row.amount)
        .bind(&row.requester_id)
        .bind(&row.note)
        .bind(&row.status)
        .bind(&row.created_at)
        .bind(&row.expires_at)
        .bind(&row.decided_by)
        .bind(&row.decided_at)
        .bind(&row.decision_note)
        .execute(&mut *tx)
        .await?;
        let sql = format!("{HITL_SELECT} WHERE \"id\" = ?");
        let existing =
            sqlx::query(&sql).bind(&row.id).fetch_one(&mut *tx).await.map(hitl_row_from)?;
        if existing.created_at == row.created_at && existing.status == "PENDING" {
            // این درج تازه بود — audit یک‌بار
            insert_audit(&mut tx, "server", "hitl.create", Some(&row.id), Some(&row.kind)).await?;
        }
        tx.commit().await?;
        Ok(existing)
    }

    /// رد کردن درخواست HITL — هیچ پولی جابه‌جا نمی‌شود.
    ///
    /// # Errors
    /// `HITL_NOT_FOUND` / `HITL_ALREADY_DECIDED` / `HITL_EXPIRED` (کد دامنه →
    /// 404/422 در لایهٔ قرارداد) یا خطای SQL.
    pub async fn reject_hitl(
        &self,
        id: &str,
        decided_by: &str,
        note: Option<&str>,
    ) -> DbResult<HitlRow> {
        let mut tx = self.pool_ref().begin().await?;
        let row = self.hitl_pending_guard_tx(&mut tx, id).await?;
        let now = now_iso();
        sqlx::query(
            r#"UPDATE "HitlRequest" SET "status"='REJECTED', "decidedBy"=?, "decidedAt"=?, "decisionNote"=?
               WHERE "id"=?"#,
        )
        .bind(decided_by)
        .bind(&now)
        .bind(note)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        insert_audit(&mut tx, decided_by, "hitl.reject", Some(id), note).await?;
        tx.commit().await?;
        let mut updated = row;
        updated.status = "REJECTED".into();
        updated.decided_by = Some(decided_by.into());
        updated.decided_at = Some(now);
        updated.decision_note = note.map(str::to_owned);
        Ok(updated)
    }

    /// تأیید درخواست HITL و اجرای تسویه «در همان تراکنش» — قلب فاز ۴.
    ///
    /// ترتیب گام‌ها روی یک `sqlx::Transaction`:
    /// ۱) گارد وضعیت (PENDING بودن) با `UPDATE … WHERE status='PENDING'` —
    ///    رقابت دو ادمین فقط یکی را عبور می‌دهد؛ دومی `HITL_ALREADY_DECIDED`.
    /// ۲) تسویه از مسیر دامنهٔ خالص (`release_escrow`) با کلید قطعی
    ///    `hitl:<id>:approve` — پول ۹۰/۱۰ دقیقاً یک‌بار جابه‌جا می‌شود.
    /// ۳) `persist_outcome` — Tx + Posting + Escrow + outbox + audit همه در
    ///    همان تراکنش.
    ///
    /// # Errors
    /// کدهای دامنه (`HITL_*`، `REQUIRES_HUMAN_APPROVAL` و…) یا خطای SQL —
    /// هر شکست کل تراکنش را برمی‌گرداند (هیچ پولی نمی‌ماند).
    pub async fn approve_hitl_and_settle(
        &self,
        id: &str,
        decided_by: &str,
        note: Option<&str>,
    ) -> DbResult<HitlDecision> {
        let mut tx = self.pool_ref().begin().await?;

        // ۱) گارد اتمی: فقط PENDING → APPROVED عبور می‌کند.
        let row = self.hitl_pending_guard_tx(&mut tx, id).await?;
        let now = now_iso();
        let updated = sqlx::query(
            r#"UPDATE "HitlRequest" SET "status"='APPROVED', "decidedBy"=?, "decidedAt"=?, "decisionNote"=?
               WHERE "id"=? AND "status"='PENDING'"#,
        )
        .bind(decided_by)
        .bind(&now)
        .bind(note)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() != 1 {
            // رقابت: ادمین دیگری زودتر تصمیم گرفت → خطای تایپ‌شده، بازپرداخت صفر.
            tx.rollback().await?;
            return Err(DomainError::new(
                DomainErrorCode::HitlAlreadyDecided,
                format!("HITL request {id} has already been decided"),
            )
            .into());
        }
        insert_audit(&mut tx, decided_by, "hitl.approve", Some(id), note).await?;

        // ۲+۳) تسویه از مسیر دامنهٔ خالص — همان تراکنش. گارد PENDING یعنی
        // مسیر replay در عمل ناممکن است؛ کلید قطعی فقط کمربند ایمنی رقابت است.
        let ledger = load_ledger_state(&mut tx).await?;
        let registry = load_escrow_registry(&mut tx, Some(&row.escrow_id)).await?;
        let settle_opts = ApplyOptions {
            actor: decided_by.into(),
            action: "hitl.approve".into(),
            target: Some(row.escrow_id.clone()),
            escrow_id: Some(row.escrow_id.clone()),
            idempotency_key: Some(settlement_key(id)),
        };
        let outcome = match (|| -> DbResult<CommandOutcome> {
            let released = dropagentx_domain::release_escrow(
                &ledger,
                &registry,
                &ReleaseEscrowCommand {
                    escrow_id: row.escrow_id.clone(),
                    idempotency_key: Some(settlement_key(id)),
                    at: Some(now.clone()),
                    approved_by_admin: true,
                },
            )?;
            Ok(CommandOutcome {
                ledger: released.ledger,
                tx: released.tx,
                escrow: Some(released.escrow),
                events: released.events.into_iter().map(DomainEvent::Escrow).collect(),
            })
        })() {
            Ok(o) => o,
            Err(e) => {
                tx.rollback().await?;
                return Err(e);
            }
        };
        persist_outcome(&mut tx, &settle_opts, &outcome).await?;
        tx.commit().await?;

        let mut request = row;
        request.status = "APPROVED".into();
        request.decided_by = Some(decided_by.into());
        request.decided_at = Some(now);
        request.decision_note = note.map(str::to_owned);
        Ok(HitlDecision { request, escrow: outcome.escrow, tx: outcome.tx, replayed: false })
    }

    /// گارد «PENDING» داخل تراکنش — خطاهای دامنه‌ای تایپ‌شده برمی‌گرداند.
    async fn hitl_pending_guard_tx(
        &self,
        exec: &mut sqlx::SqliteConnection,
        id: &str,
    ) -> DbResult<HitlRow> {
        let sql = format!("{HITL_SELECT} WHERE \"id\" = ?");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&mut *exec)
            .await?
            .map(hitl_row_from)
            .ok_or_else(|| {
            DomainError::new(
                DomainErrorCode::HitlNotFound,
                format!("HITL request {id} does not exist"),
            )
        })?;
        if row.status != "PENDING" {
            return Err(DomainError::new(
                DomainErrorCode::HitlAlreadyDecided,
                format!("HITL request {id} has already been decided ({})", row.status),
            )
            .into());
        }
        if row.expires_at.as_str() < now_iso().as_str() {
            // منقضی‌شده اما هنوز sweep نشده — همان معناشناسی انقضا.
            return Err(DomainError::new(
                DomainErrorCode::HitlExpired,
                format!("HITL request {id} expired at {}", row.expires_at),
            )
            .into());
        }
        Ok(row)
    }

    /// sweep انقضا — همهٔ PENDINGهای سررسیدگذشته → `EXPIRED`. خروجی: تعداد.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn sweep_expired_hitl(&self) -> DbResult<i64> {
        let now = now_iso();
        let mut tx = self.pool_ref().begin().await?;
        let res = sqlx::query(
            r#"UPDATE "HitlRequest" SET "status"='EXPIRED'
               WHERE "status"='PENDING' AND "expiresAt" < ?"#,
        )
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        let n = i64::try_from(res.rows_affected()).unwrap_or(i64::MAX);
        if n > 0 {
            insert_audit(&mut tx, "server", "hitl.sweep", None, Some(&format!("expired:{n}")))
                .await?;
        }
        tx.commit().await?;
        Ok(n)
    }
}

// ── محصولات داشبورد ─────────────────────────────────────────────────────────

/// ردیف محصول از جدول `Product` — ستون‌ها یک‌به‌یک با `schema.prisma`:
/// فیلدهای dashboard-only (Stars/stock/featured/salesCount) ستون ندارند و در
/// نمای داشبورد مشتق می‌شوند (Stars ریل نیست — صفر؛ stock نامحدود؛
/// sales از اسکروهای تسویه‌شده با `productRef`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductRow {
    /// شناسه (`prod:<uuid>`)
    pub id: String,
    /// فروشنده
    pub seller_id: String,
    /// عنوان
    pub title: String,
    /// توضیح
    pub description: String,
    /// دسته
    pub category: Option<String>,
    /// قیمت به دیین — ستون واقعی `priceDin` (ریل کریدیت)
    pub price_din: i64,
    /// محتوای تحویل (درس لگاسی — داخل ردیف، نه دیسک)
    pub delivery_payload: Option<String>,
    /// فعال؟
    pub active: bool,
    /// زمان ایجاد ISO
    pub created_at: String,
}

/// فیلترهای فهرست محصولات — همان کوئری‌های TS (`q`/`limit`/`offset`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProductFilter {
    /// جست‌وجوی زیررشته‌ای در عنوان/توضیح
    pub q: Option<String>,
    /// سقف صفحه
    pub limit: i64,
    /// از این‌جا
    pub offset: i64,
    /// فقط فعال‌ها؟
    pub active_only: bool,
}

const PRODUCT_SELECT: &str = r#"SELECT "id","sellerId","title","description","category","priceDin","deliveryPayload","active","createdAt" FROM "Product""#;

fn product_row_from(row: sqlx::sqlite::SqliteRow) -> ProductRow {
    use sqlx::Row;
    ProductRow {
        id: row.get("id"),
        seller_id: row.get("sellerId"),
        title: row.get("title"),
        description: row.get("description"),
        category: row.get("category"),
        price_din: row.get("priceDin"),
        delivery_payload: row.get("deliveryPayload"),
        active: row.get::<i64, _>("active") != 0,
        created_at: row.get("createdAt"),
    }
}

impl V3Store {
    /// فهرست محصولات با فیلتر — جدیدترین اول.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn list_products(&self, f: &ProductFilter) -> DbResult<Vec<ProductRow>> {
        let mut where_clauses: Vec<&str> = Vec::new();
        if f.active_only {
            where_clauses.push(r#""active" = 1"#);
        }
        if f.q.is_some() {
            where_clauses.push(r#"("title" LIKE ? ESCAPE '\' OR "description" LIKE ? ESCAPE '\')"#);
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };
        let sql = format!(
            "{PRODUCT_SELECT}{where_sql} ORDER BY \"createdAt\" DESC, \"id\" DESC LIMIT ? OFFSET ?"
        );
        // like بیرون از if نگه داشته می‌شود — bind قرضش را تا fetch_all نگه می‌دارد
        let like = f.q.as_ref().map(|qs| {
            let escaped = qs.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
            format!("%{escaped}%")
        });
        let mut q = sqlx::query(&sql);
        if let Some(pattern) = &like {
            q = q.bind(pattern).bind(pattern);
        }
        let rows = q.bind(f.limit).bind(f.offset).fetch_all(self.pool_ref()).await?;
        Ok(rows.into_iter().map(product_row_from).collect())
    }

    /// شمارش محصولات با همان فیلتر — برای `total`.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn count_products(&self, f: &ProductFilter) -> DbResult<i64> {
        let mut where_clauses: Vec<&str> = Vec::new();
        if f.active_only {
            where_clauses.push(r#""active" = 1"#);
        }
        if f.q.is_some() {
            where_clauses.push(r#"("title" LIKE ? ESCAPE '\' OR "description" LIKE ? ESCAPE '\')"#);
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };
        let sql = format!(r#"SELECT COUNT(*) AS c FROM "Product"{where_sql}"#);
        let like = f.q.as_ref().map(|qs| {
            let escaped = qs.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
            format!("%{escaped}%")
        });
        let mut q = sqlx::query(&sql);
        if let Some(pattern) = &like {
            q = q.bind(pattern).bind(pattern);
        }
        let row = q.fetch_one(self.pool_ref()).await?;
        use sqlx::Row;
        Ok(row.get("c"))
    }

    /// خواندن یک محصول.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn get_product(&self, id: &str) -> DbResult<Option<ProductRow>> {
        let sql = format!("{PRODUCT_SELECT} WHERE \"id\" = ?");
        let row = sqlx::query(&sql).bind(id).fetch_optional(self.pool_ref()).await?;
        Ok(row.map(product_row_from))
    }

    /// درج محصول — شناسه از قبل ساخته‌شده (`prod:<uuid>`).
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn insert_product(&self, p: &ProductRow) -> DbResult<()> {
        sqlx::query(
            r#"INSERT INTO "Product"
               ("id","sellerId","title","description","category","priceDin","deliveryPayload","active","createdAt")
               VALUES (?,?,?,?,?,?,?,?,?)"#,
        )
        .bind(&p.id)
        .bind(&p.seller_id)
        .bind(&p.title)
        .bind(&p.description)
        .bind(&p.category)
        .bind(p.price_din)
        .bind(&p.delivery_payload)
        .bind(i64::from(p.active))
        .bind(&p.created_at)
        .execute(self.pool_ref())
        .await?;
        self.audit_ext("server", "product.create", Some(&p.id), None).await?;
        Ok(())
    }

    /// به‌روزرسانی جزئی محصول — فقط فیلدهای `Some` اعمال می‌شوند. ستون‌های
    /// ویرایش‌پذیر: title/category/priceDin/description/active (خلاصهٔ مسیر
    /// رجیستری — stock و stars فقط نمای داشبوردند).
    ///
    /// # Errors
    /// محصول ناموجod → `PRODUCT_NOT_FOUND` (دامنه)؛ خطای SQL.
    pub async fn update_product(
        &self,
        id: &str,
        title: Option<&str>,
        category: Option<&str>,
        price_din: Option<i64>,
        description: Option<&str>,
        active: Option<bool>,
    ) -> DbResult<ProductRow> {
        let existing = self.get_product(id).await?.ok_or_else(|| {
            DomainError::new(
                DomainErrorCode::ProductNotFound,
                format!("Product {id} does not exist"),
            )
        })?;
        let merged = ProductRow {
            title: title.map(str::to_owned).unwrap_or_else(|| existing.title.clone()),
            category: category.map(str::to_owned).or_else(|| existing.category.clone()),
            price_din: price_din.unwrap_or(existing.price_din),
            description: description
                .map(str::to_owned)
                .unwrap_or_else(|| existing.description.clone()),
            active: active.unwrap_or(existing.active),
            ..existing.clone()
        };
        sqlx::query(
            r#"UPDATE "Product" SET "title"=?, "category"=?, "priceDin"=?,
               "description"=?, "active"=? WHERE "id"=?"#,
        )
        .bind(&merged.title)
        .bind(&merged.category)
        .bind(merged.price_din)
        .bind(&merged.description)
        .bind(i64::from(merged.active))
        .bind(id)
        .execute(self.pool_ref())
        .await?;
        self.audit_ext("server", "product.update", Some(id), None).await?;
        Ok(merged)
    }

    /// حذف محصول.
    ///
    /// # Errors
    /// محصول ناموجod → `PRODUCT_NOT_FOUND` (دامنه)؛ خطای SQL.
    pub async fn delete_product(&self, id: &str) -> DbResult<()> {
        let res = sqlx::query(r#"DELETE FROM "Product" WHERE "id"=?"#)
            .bind(id)
            .execute(self.pool_ref())
            .await?;
        if res.rows_affected() == 0 {
            return Err(DomainError::new(
                DomainErrorCode::ProductNotFound,
                format!("Product {id} does not exist"),
            )
            .into());
        }
        self.audit_ext("server", "product.delete", Some(id), None).await?;
        Ok(())
    }
}

// ── audit + مصرف AI + تجمیع‌های داشبورد ─────────────────────────────────────

/// ردیف audit برای نماهای داشبورد.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRowRead {
    /// شناسه
    pub id: String,
    /// عامل
    pub actor: String,
    /// اقدام
    pub action: String,
    /// هدف
    pub target: Option<String>,
    /// متادیتا (JSON رشته‌ای)
    pub meta: Option<String>,
    /// زمان
    pub at: String,
}

impl V3Store {
    /// فهرست audit — جدیدترین اول (نمونهٔ بزرگ‌ترین `limit` ۲۰۰).
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn list_audit(&self, limit: i64) -> DbResult<Vec<AuditRowRead>> {
        let rows = sqlx::query(
            r#"SELECT "id","actor","action","target","meta","at" FROM "AuditLog"
               ORDER BY "at" DESC, "id" DESC LIMIT ?"#,
        )
        .bind(limit)
        .fetch_all(self.pool_ref())
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            use sqlx::Row;
            out.push(AuditRowRead {
                id: r.get("id"),
                actor: r.get("actor"),
                action: r.get("action"),
                target: r.get("target"),
                meta: r.get("meta"),
                at: r.get("at"),
            });
        }
        Ok(out)
    }

    /// نوشتن audit خارج از فرمان دامنه (محصولات، sweep و…) — همان ستون‌های TS.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn audit_ext(
        &self,
        actor: &str,
        action: &str,
        target: Option<&str>,
        meta: Option<&str>,
    ) -> DbResult<()> {
        let mut tx = self.pool_ref().begin().await?;
        insert_audit(&mut tx, actor, action, target, meta).await?;
        tx.commit().await?;
        Ok(())
    }

    /// مجموع مصرف توکن امروز (UTC) یک کاربر — برای `/ai/usage`.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn ai_tokens_today(&self, user_id: &str) -> DbResult<(i64, i64)> {
        let today = utc_today();
        let row = sqlx::query(
            r#"SELECT COALESCE(SUM("totalTokens"),0) AS total, COUNT(*) AS calls
               FROM "AiUsage" WHERE "userId" = ? AND substr("at",1,10) = ?"#,
        )
        .bind(user_id)
        .bind(&today)
        .fetch_one(self.pool_ref())
        .await?;
        use sqlx::Row;
        Ok((row.get("total"), row.get("calls")))
    }

    /// شمارش فراخوانی‌های AI (همه‌گاه) — برای overview.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn ai_calls_total(&self) -> DbResult<i64> {
        let row = sqlx::query(r#"SELECT COUNT(*) AS c FROM "AiUsage""#)
            .fetch_one(self.pool_ref())
            .await?;
        use sqlx::Row;
        Ok(row.get("c"))
    }

    /// تعداد حساب‌ها — برای `/economy` (accountCount).
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn account_count(&self) -> DbResult<i64> {
        let row = sqlx::query(r#"SELECT COUNT(*) AS c FROM "Account""#)
            .fetch_one(self.pool_ref())
            .await?;
        use sqlx::Row;
        Ok(row.get("c"))
    }

    /// تعداد حساب‌های کاربر (`kind='user'`) — KPI کاربران داشبورد.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn count_users(&self) -> DbResult<i64> {
        let row = sqlx::query(r#"SELECT COUNT(*) AS c FROM "Account" WHERE "kind" = 'user'"#)
            .fetch_one(self.pool_ref())
            .await?;
        use sqlx::Row;
        Ok(row.get("c"))
    }

    /// تعداد همهٔ امانت‌ها (هر وضعیت) — KPI سفارش‌های داشبورد.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn count_escrows(&self) -> DbResult<i64> {
        let row =
            sqlx::query(r#"SELECT COUNT(*) AS c FROM "Escrow""#).fetch_one(self.pool_ref()).await?;
        use sqlx::Row;
        Ok(row.get("c"))
    }

    /// تعداد درخواست‌های HITL در وضعیت داده‌شده — KPI تأییدهای در انتظار.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn count_hitl(&self, status: &str) -> DbResult<i64> {
        let row = sqlx::query(r#"SELECT COUNT(*) AS c FROM "HitlRequest" WHERE "status" = ?"#)
            .bind(status)
            .fetch_one(self.pool_ref())
            .await?;
        use sqlx::Row;
        Ok(row.get("c"))
    }

    /// کل ضرب‌شده از آغاز — قرینهٔ تراز حساب ضرب.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn minted_all_time(&self) -> DbResult<i64> {
        let row = sqlx::query(
            r#"SELECT COALESCE(-SUM("amount"),0) AS c FROM "Posting" WHERE "accountId" = (SELECT "id" FROM "Account" WHERE "kind"='mint' LIMIT 1)"#,
        )
        .fetch_one(self.pool_ref())
        .await?;
        use sqlx::Row;
        Ok(row.get("c"))
    }

    /// کل سوزانده‌شده از آغاز — تراز حساب سوزاندن.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn burned_all_time(&self) -> DbResult<i64> {
        let row = sqlx::query(
            r#"SELECT COALESCE(SUM("amount"),0) AS c FROM "Posting" WHERE "accountId" = (SELECT "id" FROM "Account" WHERE "kind"='burn' LIMIT 1)"#,
        )
        .fetch_one(self.pool_ref())
        .await?;
        use sqlx::Row;
        Ok(row.get("c"))
    }

    /// پول قفل‌شدهٔ یک خریدار — Σ مبلغ اسکروهای باز (FUNDED/DELIVERED/DISPUTED).
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn locked_for_buyer(&self, buyer_id: &str) -> DbResult<i64> {
        let row = sqlx::query(
            r#"SELECT COALESCE(SUM("amount"),0) AS c FROM "Escrow"
               WHERE "buyerId" = ? AND "status" IN ('FUNDED','DELIVERED','DISPUTED')"#,
        )
        .bind(buyer_id)
        .fetch_one(self.pool_ref())
        .await?;
        use sqlx::Row;
        Ok(row.get("c"))
    }

    /// فروش واقعی به ازای فروشنده — تعداد اسکروهای RELEASED (لیدربورد sales).
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn sales_by_seller(&self) -> DbResult<Vec<(String, i64)>> {
        let rows = sqlx::query(
            r#"SELECT "sellerId" AS s, COUNT(*) AS c FROM "Escrow"
               WHERE "status"='RELEASED' GROUP BY "sellerId" ORDER BY c DESC, s ASC"#,
        )
        .fetch_all(self.pool_ref())
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            use sqlx::Row;
            out.push((r.get("s"), r.get("c")));
        }
        Ok(out)
    }

    /// فروش واقعی به ازای محصول — تعداد اسکروهای RELEASED با `productRef`
    /// (salesCount نمای داشبورد — از جدول زندهٔ اسکرو مشتق می‌شود، ستون نیست).
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn sales_by_product(&self) -> DbResult<Vec<(String, i64)>> {
        let rows = sqlx::query(
            r#"SELECT "productRef" AS p, COUNT(*) AS c FROM "Escrow"
               WHERE "status"='RELEASED' AND "productRef" IS NOT NULL
               GROUP BY "productRef" ORDER BY c DESC"#,
        )
        .fetch_all(self.pool_ref())
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            use sqlx::Row;
            out.push((r.get("p"), r.get("c")));
        }
        Ok(out)
    }

    /// عنوان محصول بر اساس شناسه — نماهای سفارش (`None` اگر ارجاع آزاد بود).
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn product_title(&self, product_ref: Option<&str>) -> DbResult<Option<String>> {
        let Some(pref) = product_ref else { return Ok(None) };
        let row = sqlx::query(r#"SELECT "title" FROM "Product" WHERE "id" = ?"#)
            .bind(pref)
            .fetch_optional(self.pool_ref())
            .await?;
        use sqlx::Row;
        Ok(row.map(|r| r.get::<String, _>("title")))
    }

    /// سری ۱۴روزهٔ درآمد (تراز خزانه به تفکیک روز) — چارت overview.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn revenue_series_14d(&self) -> DbResult<Vec<(String, i64)>> {
        let rows = sqlx::query(
            r#"SELECT substr("Tx"."at",1,10) AS day, COALESCE(SUM("Posting"."amount"),0) AS revenue
               FROM "Posting" JOIN "Tx" ON "Tx"."id" = "Posting"."txId"
               WHERE "Posting"."accountId" = (SELECT "id" FROM "Account" WHERE "kind"='treasury' LIMIT 1)
                 AND "Posting"."amount" > 0
                 AND "Tx"."at" >= ?
               GROUP BY day ORDER BY day ASC"#,
        )
        .bind(utc_today_minus(13))
        .fetch_all(self.pool_ref())
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            use sqlx::Row;
            out.push((r.get("day"), r.get("revenue")));
        }
        Ok(out)
    }

    /// بکاپ سازگار SQLite با `VACUUM INTO` + هرس تا `keep` فایلِ جدیدترین.
    ///
    /// # Errors
    /// فروشگاه درون‌حافظه‌ای → [`crate::error::DbError::Config`] (لایهٔ سرور
    /// به `BACKUP_UNSUPPORTED` ۵۰۳ نگاشت می‌کند)؛ خطای SQL/فایل.
    pub async fn backup_to_dir(&self, dir: &std::path::Path, keep: usize) -> DbResult<BackupInfo> {
        let Some(src) = self.file_path() else {
            return Err(DbError::Config("in-memory store has no file to snapshot".into()));
        };
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| DbError::Config(format!("cannot create {}: {e}", dir.display())))?;
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let file_name = format!("backup-{stamp}.sqlite");
        let dest = dir.join(&file_name);
        if dest.exists() {
            // دو بکاپ در یک ثانیه — پسوند شماره‌دار
            let mut i = 1u32;
            loop {
                let alt = dir.join(format!("backup-{stamp}-{i}.sqlite"));
                if !alt.exists() {
                    return self.vacuum_into(&alt, src, dir, keep).await;
                }
                i += 1;
            }
        }
        self.vacuum_into(&dest, src, dir, keep).await
    }

    async fn vacuum_into(
        &self,
        dest: &std::path::Path,
        _src: &str,
        dir: &std::path::Path,
        keep: usize,
    ) -> DbResult<BackupInfo> {
        let dest_str = dest.display().to_string().replace('\'', "''");
        let sql = format!("VACUUM INTO '{dest_str}'");
        let res = sqlx::query(&sql).execute(self.pool_ref()).await;
        if let Err(e) = res {
            return Err(DbError::Sql(e));
        }
        let size = tokio::fs::metadata(dest)
            .await
            .map_err(|e| DbError::Config(format!("backup file missing: {e}")))?
            .len();
        // هرس — جدیدترین‌ها می‌مانند
        let mut entries: Vec<(std::path::PathBuf, std::time::SystemTime)> = Vec::new();
        let mut rd = tokio::fs::read_dir(dir)
            .await
            .map_err(|e| DbError::Config(format!("cannot read {}: {e}", dir.display())))?;
        while let Ok(Some(entry)) = rd.next_entry().await {
            let p = entry.path();
            if p.extension().is_some_and(|x| x == "sqlite") {
                if let Ok(meta) = entry.metadata().await {
                    if let Ok(modified) = meta.modified() {
                        entries.push((p, modified));
                    }
                }
            }
        }
        entries.sort_by_key(|(_, m)| *m);
        entries.reverse();
        let mut removed = 0u64;
        for (path, _) in entries.iter().skip(keep) {
            if tokio::fs::remove_file(path).await.is_ok() {
                removed += 1;
            }
        }
        let kept = i64::try_from(entries.len().saturating_sub(removed as usize)).unwrap_or(0);
        let created_at = now_iso();
        self.audit("server", "system.backup", Some(&dest.display().to_string()), None).await?;
        Ok(BackupInfo { file: dest.display().to_string(), size_bytes: size, kept, created_at })
    }
}

/// نتیجهٔ بکاپ — پورت `BackupCreated`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupInfo {
    /// مسیر مطلق اسنپ‌شات
    pub file: String,
    /// حجم به بایت
    pub size_bytes: u64,
    /// تعداد نگه‌داشته‌شده‌ها بعد از هرس
    pub kept: i64,
    /// زمان ایجاد ISO
    pub created_at: String,
}

/// تاریخ امروز UTC به شکل `YYYY-MM-DD`.
fn utc_today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// تاریخِ «امروز منهای n روز» UTC — برای بازهٔ سری ۱۴روزه.
fn utc_today_minus(n: u32) -> String {
    (chrono::Utc::now() - chrono::Duration::days(i64::from(n))).format("%Y-%m-%d").to_string()
}

// silence unused-import warnings for re-exported helpers used only in docs
#[allow(unused_imports)]
use crate::store::RecentPosting;

/// رویدادهای دامنه به شکل JSON سیمی — برای درج دستی در outbox (فاز ۵).
pub fn wire_event(event: &DomainEvent) -> (String, String) {
    (crate::wire::event_kind(event).to_owned(), event_payload(event))
}

/// بازخوانی وضعیت کامل دفتر + یک امانت — برای مسیرهای ترکیبی آینده.
pub async fn load_world(
    store: &V3Store,
    escrow_id: Option<&str>,
) -> DbResult<(LedgerState, Option<Escrow>)> {
    let mut conn = store.acquire_conn().await?;
    let ledger = load_ledger_state(&mut conn).await?;
    let escrow = load_escrow_opt_pub(&mut conn, escrow_id).await?;
    Ok((ledger, escrow))
}

async fn load_escrow_opt_pub(
    exec: &mut sqlx::SqliteConnection,
    escrow_id: Option<&str>,
) -> DbResult<Option<Escrow>> {
    crate::store::load_escrow_opt(exec, escrow_id).await
}

#[allow(clippy::needless_pass_by_value)]
fn _event_payload_witness(ev: &DomainEvent) -> String {
    event_payload(ev)
}

/// تسویهٔ تحویل سفارش از مسیر دامنه — برای `POST /orders/{id}/status`.
///
/// # Errors
/// خطای دامنه یا SQL.
pub async fn deliver_order(
    store: &V3Store,
    escrow_id: &str,
    delivery_ref: Option<&str>,
) -> DbResult<Escrow> {
    store
        .apply(
            ApplyOptions {
                actor: "server".into(),
                action: "escrow.deliver".into(),
                target: Some(escrow_id.to_owned()),
                escrow_id: Some(escrow_id.to_owned()),
                idempotency_key: None,
            },
            |ledger, reg| {
                let o = dropagentx_domain::deliver_escrow(
                    ledger,
                    reg,
                    &DeliverEscrowCommand {
                        escrow_id: escrow_id.to_owned(),
                        idempotency_key: None,
                        at: Some(now_iso()),
                        delivery_ref: delivery_ref.map(str::to_owned),
                    },
                )?;
                Ok(CommandOutcome {
                    ledger: o.ledger,
                    tx: o.tx,
                    escrow: Some(o.escrow),
                    events: o.events.into_iter().map(DomainEvent::Escrow).collect(),
                })
            },
        )
        .await?
        .escrow
        .ok_or_else(|| DbError::Corrupt("deliver completed without escrow".into()))
}

/// DomainResult برای re-export داخلی — کدهای دامنه در امضاهای بالا.
pub type QDomainResult<T> = DomainResult<T>;
