//! Outbox — پل رویدادهای دامنه به مصرف‌کننده‌های بعدی (بات فاز ۵).
//!
//! الگوی transactional outbox: رویدادها در همان تراکنشِ تغییر وضعیت مالی
//! درج می‌شوند — «اتفاق افتاده اما اعلام‌نشده» غیرممکن است. دیسپچ بعداً با
//! خواندن `published = 0` و علامت‌زدن انجام می‌شود.

use crate::error::DbResult;
use crate::store::{insert_audit, insert_outbox_events, OutboxRow};
use dropagentx_domain::DomainEvent;
use sqlx::Row;

/// درج رویدادها در outbox — داخل تراکنش جاری فراخوانی می‌شود.
///
/// # Errors
/// خطای SQL.
pub(crate) async fn enqueue(
    exec: &mut sqlx::SqliteConnection,
    events: &[DomainEvent],
) -> DbResult<()> {
    insert_outbox_events(exec, events).await
}

/// یک ردیف audit — داخل تراکنش جاری فراخوانی می‌شود.
///
/// # Errors
/// خطای SQL.
pub(crate) async fn audit(
    exec: &mut sqlx::SqliteConnection,
    actor: &str,
    action: &str,
    target: Option<&str>,
    meta: Option<&str>,
) -> DbResult<()> {
    insert_audit(exec, actor, action, target, meta).await
}

impl crate::store::V3Store {
    /// رویدادهای منتظرِ ارسال به ترتیب درج (rowid — ترتیب واقعی تولید).
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn pending_outbox(&self, limit: i64) -> DbResult<Vec<OutboxRow>> {
        let mut out = Vec::new();
        for r in sqlx::query(
            r#"SELECT "id", "kind", "payload", "createdAt", "published" FROM "OutboxEvent"
               WHERE "published" = 0 ORDER BY "rowid" ASC LIMIT ?"#,
        )
        .bind(limit)
        .fetch_all(self.pool_ref())
        .await?
        {
            out.push(OutboxRow {
                id: r.get("id"),
                kind: r.get("kind"),
                payload: r.get("payload"),
                created_at: r.get("createdAt"),
                published: r.get::<i64, _>("published") != 0,
            });
        }
        Ok(out)
    }

    /// علامت‌زدن رویدادها به‌عنوان ارسال‌شده — تعداد ردیف‌های تغییر یافته.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn mark_outbox_published(&self, ids: &[String]) -> DbResult<u64> {
        let mut n = 0;
        let mut tx = self.pool_ref().begin().await?;
        for id in ids {
            n += sqlx::query(r#"UPDATE "OutboxEvent" SET "published" = 1 WHERE "id" = ?"#)
                .bind(id)
                .execute(&mut *tx)
                .await?
                .rows_affected();
        }
        tx.commit().await?;
        Ok(n)
    }

    /// ثبت audit مستقل (خارج از `apply`) — مثل `legacy.adjustment`.
    ///
    /// # Errors
    /// خطای SQL.
    pub async fn audit(
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
}
