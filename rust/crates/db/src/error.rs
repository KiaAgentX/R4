//! مدل خطای لایهٔ ماندگاری — thiserror در کتابخانه، `anyhow` فقط در باینری‌ها.

use dropagentx_domain::DomainError;

/// خطای آداپتور persistence — همیشه با زمینهٔ کافی برای دیباگ.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// خطای sqlx (اتصال، کوئری، قید UNIQUE و…)
    #[error("sqlx: {0}")]
    Sql(#[from] sqlx::Error),
    /// خطای اجرای مهاجرت‌های نسخه‌دار اسکیما
    #[error("migrate: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    /// خطای دامنهٔ خالص (از اجرای closure داخل تراکنش)
    #[error("domain: {0}")]
    Domain(#[from] DomainError),
    /// پیکربندی نامعتبر (URL دیتابیس، مسیر لگاسی و…)
    #[error("config: {0}")]
    Config(String),
    /// دادهٔ خراب در فروشگاه (مثلاً `kind` حساب ناشناخته)
    #[error("corrupt store: {0}")]
    Corrupt(String),
    /// مهاجرت لگاسی به‌دلیل نقض ثبات رد شد (fail-closed)
    #[error("migration blocked: {0}")]
    Migration(String),
}

/// Result استاندارد لایهٔ db.
pub type DbResult<T> = Result<T, DbError>;
