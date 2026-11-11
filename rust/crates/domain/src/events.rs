//! رویدادهای دامنه — تایپ‌شده، شناسه‌های دترمینیستیک، بدون ساعت پنهان.
//! `at` همیشه توسط فراخوان (فرمان هندلر یا ساعت آداپتور) تأمین می‌شود.

/// زمان پیش‌فرض دترمینیستیک وقتی فراخوان `at` نمی‌دهد.
/// دامنه ساعت واقعی ندارد — ساعت، مسئولیت آداپتور است.
pub const EPOCH_ISO: &str = "1970-01-01T00:00:00.000Z";

/// نوع رویداد اسکرو — ۸ گذار مجاز ماشین وضعیت.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EscrowEventKind {
    /// ساخته‌شدن امانت
    Created,
    /// تامین‌مالی (خریدار → اسکرو)
    Funded,
    /// تحویل کالا/خدمت
    Delivered,
    /// آزادسازی به نفع فروشنده + مالیات
    Released,
    /// اختلاف/دعوا
    Disputed,
    /// بازگشت وجه به خریدار
    Refunded,
    /// لغو قبل از تامین‌مالی
    Cancelled,
    /// انقضا با بازگشت خودکار
    Expired,
}

impl EscrowEventKind {
    /// رشته کانانیکی — یکسان با نسخه TS (برای شناسه رویداد و dual-run).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "EscrowCreated",
            Self::Funded => "EscrowFunded",
            Self::Delivered => "EscrowDelivered",
            Self::Released => "EscrowReleased",
            Self::Disputed => "EscrowDisputed",
            Self::Refunded => "EscrowRefunded",
            Self::Cancelled => "EscrowCancelled",
            Self::Expired => "EscrowExpired",
        }
    }
}

impl std::fmt::Display for EscrowEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// رویداد اسکرو — شناسه دترمینیستیک: `{escrowId}#{kind}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowEvent {
    /// شناسه دترمینیستیک رویداد
    pub id: String,
    /// زمان ISO (فراهم‌شده توسط فراخوان)
    pub at: String,
    /// نوع رویداد
    pub kind: EscrowEventKind,
    /// شناسه امانت
    pub escrow_id: String,
    /// خریدار (در Created)
    pub buyer_id: Option<String>,
    /// فروشنده (در Created)
    pub seller_id: Option<String>,
    /// مبلغ (Funded/Refunded/Expired/Created)
    pub amount: Option<i64>,
    /// شناسه تراکنش دفتر (Funded/Released/Refunded/Expired)
    pub tx_id: Option<String>,
    /// سهم خالص فروشنده (Released)
    pub seller_net: Option<i64>,
    /// مالیات خزانه (Released)
    pub tax: Option<i64>,
    /// دلیل اختلاف (Disputed)
    pub reason: Option<String>,
    /// شناسه همبستگی اختیاری برای trace
    pub correlation_id: Option<String>,
}

impl EscrowEvent {
    /// ساخت رویداد با شناسه دترمینیستیک — هر گذار حداکثر یک‌بار برای هر امانت
    /// رخ می‌دهد، پس `{escrowId}#{kind}` یکتا و قابل replay است.
    #[must_use]
    pub fn new(kind: EscrowEventKind, escrow_id: &str, at: &str) -> Self {
        Self {
            id: format!("{}#{}", escrow_id, kind.as_str()),
            at: at.to_owned(),
            kind,
            escrow_id: escrow_id.to_owned(),
            buyer_id: None,
            seller_id: None,
            amount: None,
            tx_id: None,
            seller_net: None,
            tax: None,
            reason: None,
            correlation_id: None,
        }
    }
}

/// رویداد ثبت تراکنش در دفتر.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionPostedEvent {
    /// شناسه دترمینیستیک: `{txId}#posted`
    pub id: String,
    /// زمان ISO
    pub at: String,
    /// شناسه تراکنش
    pub tx_id: String,
    /// یادداشت
    pub memo: Option<String>,
    /// کلید یکسان‌سازی
    pub idempotency_key: Option<String>,
    /// شناسه همبستگی
    pub correlation_id: Option<String>,
}

impl TransactionPostedEvent {
    /// ساخت رویداد با شناسه دترمینیستیک.
    #[must_use]
    pub fn new(tx_id: &str, at: &str) -> Self {
        Self {
            id: format!("{tx_id}#posted"),
            at: at.to_owned(),
            tx_id: tx_id.to_owned(),
            memo: None,
            idempotency_key: None,
            correlation_id: None,
        }
    }
}

/// هر رویداد دامنه — همان اجتماع `DomainEvent` در TS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    /// تراکنش دفتر ثبت شد
    TransactionPosted(TransactionPostedEvent),
    /// رویداد اسکرو
    Escrow(EscrowEvent),
}

impl DomainEvent {
    /// شناسه دترمینیستیک رویداد.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::TransactionPosted(e) => &e.id,
            Self::Escrow(e) => &e.id,
        }
    }

    /// زمان رویداد.
    #[must_use]
    pub const fn at(&self) -> &String {
        match self {
            Self::TransactionPosted(e) => &e.at,
            Self::Escrow(e) => &e.at,
        }
    }
}
