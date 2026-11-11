//! خوانندهٔ دیتابیس لگاسی — SQL خام روی فایل بازیابی‌شدهٔ Railway.
//!
//! جدول‌های لگاسی (`users`، `products`، `transactions`) مدل v3 نیستند؛ مثل
//! `migrate-legacy.ts` با کوئری خام و `CAST(... AS TEXT)` می‌خوانیم چون
//! شناسه‌های لگاسی از Int32 بزرگ‌ترند. اتصال فقط‌خواندنی است — مهاجرت
//! هرگز روی منبع می‌نویسد.

use crate::error::{DbError, DbResult};
use crate::genesis::{LegacyProductRow, LegacyUserRow};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row,
};

/// تصویر کامل خوانده‌شده از لگاسی — ورودی مهاجرت و برنامهٔ genesis.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacySnapshot {
    /// همهٔ کاربران (مثل ۴۶۶ نفر روز-اول) مرتب بر اساس شناسه
    pub users: Vec<LegacyUserRow>,
    /// همهٔ محصولات (مثل ۳۰ محصول لگاسی) مرتب بر اساس شناسه
    pub products: Vec<LegacyProductRow>,
    /// تعداد ردیف‌های تراکنش لگاسی (مثل ۱٬۵۵۴) — فقط برای گزارش
    pub tx_count: i64,
}

/// خواندن تصویر لگاسی از فایل SQLite — فقط‌خواندنی.
///
/// # Errors
/// فایل نبود، جدول‌ها نبودند، یا SQLite باز نشد.
pub async fn read_legacy(path: &str) -> DbResult<LegacySnapshot> {
    let file = path.strip_prefix("file:").unwrap_or(path);
    if !std::path::Path::new(file).is_file() {
        return Err(DbError::Config(format!("legacy db not found: {file}")));
    }
    let opts = SqliteConnectOptions::new().filename(file).read_only(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .map_err(|e| DbError::Config(format!("cannot open legacy db {file}: {e}")))?;

    let mut users = Vec::new();
    for r in sqlx::query(
        "SELECT CAST(user_id AS TEXT) AS user_id, username, first_name, credits, is_banned \
         FROM users ORDER BY user_id",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| DbError::Config(format!("legacy users query failed: {e}")))?
    {
        users.push(LegacyUserRow {
            user_id: r.get("user_id"),
            username: r.get("username"),
            first_name: r.get("first_name"),
            credits: r.get("credits"),
            is_banned: r.get("is_banned"),
        });
    }

    let mut products = Vec::new();
    for r in sqlx::query(
        "SELECT CAST(id AS TEXT) AS id, CAST(creator_id AS TEXT) AS creator_id, title, \
         description, price_credits, category, link, is_active \
         FROM products ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| DbError::Config(format!("legacy products query failed: {e}")))?
    {
        products.push(LegacyProductRow {
            id: r.get("id"),
            creator_id: r.get("creator_id"),
            title: r.get("title"),
            description: r.get("description"),
            price_credits: r.get("price_credits"),
            category: r.get("category"),
            link: r.get("link"),
            is_active: r.get("is_active"),
        });
    }

    let tx_count: i64 = sqlx::query("SELECT COUNT(*) AS c FROM transactions")
        .fetch_one(&pool)
        .await
        .map_err(|e| DbError::Config(format!("legacy transactions query failed: {e}")))?
        .get("c");

    pool.close().await;
    Ok(LegacySnapshot { users, products, tx_count })
}
