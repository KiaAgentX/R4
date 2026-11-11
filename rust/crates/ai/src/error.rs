//! خطاهای crate AI — thiserror لایهٔ gateway (قاعدهٔ فاز ۰).

/// خطای ارائه‌دهندهٔ LLM.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LlmError {
    /// timeout فراخوانی (tokio::time — معادل AbortController)
    #[error("llm timeout")]
    Timeout,
    /// خطای HTTP (وضعیت + خلاصه — کلید هرگز در پیام نمی‌آید)
    #[error("llm http {status}: {detail}")]
    Http {
        /// کد وضعیت HTTP
        status: u16,
        /// خلاصهٔ صادقانه بدون نشت کلید
        detail: String,
    },
    /// پاسخ قابل تجزیه نبود
    #[error("llm bad response: {0}")]
    BadResponse(String),
    /// ارائه‌دهنده عملاً قطع است (fake/test)
    #[error("llm slot down")]
    Down,
}

/// خطای پاکت — متن خام هرگز عبور نمی‌کند.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PacketError {
    /// خروجی JSON نبود (متن خام)
    #[error("raw text is not a packet")]
    RawText,
    /// JSON بود ولی شکل پاکت نداشت
    #[error("packet shape invalid: {0}")]
    BadShape(String),
}

/// خطای گیت gateway — نگاشت صریح به کدهای وایر در هندلر.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GateError {
    /// سقف روزانهٔ توکن تمام شد — قبل از هر فراخوانی چک می‌شود
    #[error("ai cap exceeded: {used}/{cap}")]
    CapExceeded {
        /// مصرف امروز
        used: i64,
        /// سقف
        cap: i64,
    },
    /// دو پاکت خراب پیاپی — شکست تایپ‌شده
    #[error("malformed packets: {count}")]
    MalformedPacket {
        /// تعداد پاکت‌های خراب (همیشه ۲ در لحظهٔ خطا)
        count: usize,
    },
    /// هر سه اسلات شکست خوردند
    #[error("all llm slots failed: {attempts}")]
    AllSlotsFailed {
        /// خلاصهٔ کوتاه هر تلاش (بدون نشت کلید)
        attempts: String,
    },
    /// درخواست عملاً خالی است
    #[error("empty request")]
    EmptyRequest,
    /// شکست زیرساخت backend (SQL و…) — جزئیات بدون نشت کلید
    #[error("ai backend failure: {0}")]
    Backend(String),
}
