//! خطای رانر بات — thiserror لایهٔ teloxide.
//!
//! در کتابخانه‌ها thiserror (قاعدهٔ فاز ۰)؛ anyhow فقط در باینری‌ها.

/// خطاهای رانر polling.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    /// خطای API تلگرام در ارسال (شبکه/مجوز/بدنهٔ نامعتبر).
    #[error("telegram send failed: {0}")]
    Telegram(String),

    /// خطای زیرساخت backend (SQL و…) — جزئیات در لاگ.
    #[error("bot backend failure: {0}")]
    Backend(String),
}
