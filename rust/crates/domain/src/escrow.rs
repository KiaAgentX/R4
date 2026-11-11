//! ADR-004 — اسکرو به‌عنوان ماشین وضعیت صریح روی دفتر.
//!
//! ```text
//! CREATED → FUNDED → DELIVERED → RELEASED
//!                    ↘ DISPUTED → REFUNDED | RELEASED (داوری)
//! CREATED → CANCELLED        (هیچ پولی جابه‌جا نشده)
//! FUNDED  → EXPIRED          (بازگشت خودکار در جاروب TTL)
//! ```
//!
//! مسیر پول (ماده ۸ — اسکرو اجباری برای بازار):
//!   fund    : خریدار → اسکرو
//!   release : اسکرو → فروشنده (خالص) + اسکرو → خزانه (مالیات ۱۰٪)
//!   refund  : اسکرو → خریدار (کامل، بدون مالیات)

use crate::charter::{requires_hitl, split_seller_tax, Charter};
use crate::errors::{DomainError, DomainErrorCode, DomainResult};
use crate::events::{EscrowEvent, EscrowEventKind, EPOCH_ISO};
use crate::ledger::{
    add_account, post, Account, AccountKind, LedgerState, PostCommand, PostedTransaction, Posting,
    TREASURY_ACCOUNT_ID,
};
use crate::money::Din;
use std::collections::BTreeMap;

/// وضعیت امانت — ۸ حالت ماشین.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EscrowStatus {
    /// ساخته‌شده، هنوز تامین‌نشده
    Created,
    /// تامین‌شده (پول در حساب اسکرو)
    Funded,
    /// تحویل‌شده
    Delivered,
    /// آزادشده (پایانی)
    Released,
    /// در اختلاف (پایانی نیست)
    Disputed,
    /// بازگشت‌داده‌شده (پایانی)
    Refunded,
    /// لغوشده (پایانی)
    Cancelled,
    /// منقضی‌شده (پایانی)
    Expired,
}

impl EscrowStatus {
    /// رشته کانانیکی — یکسان با نسخه TS.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "CREATED",
            Self::Funded => "FUNDED",
            Self::Delivered => "DELIVERED",
            Self::Released => "RELEASED",
            Self::Disputed => "DISPUTED",
            Self::Refunded => "REFUNDED",
            Self::Cancelled => "CANCELLED",
            Self::Expired => "EXPIRED",
        }
    }

    /// آیا وضعیت پایانی است؟ (هیچ گذاری از آن خارج نمی‌شود)
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Released | Self::Refunded | Self::Cancelled | Self::Expired)
    }
}

impl std::fmt::Display for EscrowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// وضعیت‌های پایانی ماشین.
pub const ESCROW_TERMINAL_STATUSES: [EscrowStatus; 4] = [
    EscrowStatus::Released,
    EscrowStatus::Refunded,
    EscrowStatus::Cancelled,
    EscrowStatus::Expired,
];

/// جدول گذار ماشین‌پذیر — مصرف هندلرها، تست‌ها و UI.
/// یک‌به‌یک با `ESCROW_TRANSITIONS` در TS.
#[must_use]
pub const fn escrow_transitions(status: EscrowStatus) -> &'static [EscrowEventKind] {
    match status {
        EscrowStatus::Created => &[EscrowEventKind::Funded, EscrowEventKind::Cancelled],
        EscrowStatus::Funded => {
            &[EscrowEventKind::Delivered, EscrowEventKind::Disputed, EscrowEventKind::Expired]
        }
        EscrowStatus::Delivered => &[EscrowEventKind::Released, EscrowEventKind::Disputed],
        EscrowStatus::Disputed => &[EscrowEventKind::Refunded, EscrowEventKind::Released],
        EscrowStatus::Released
        | EscrowStatus::Refunded
        | EscrowStatus::Cancelled
        | EscrowStatus::Expired => &[],
    }
}

/// یک امانت.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Escrow {
    /// شناسه یکتا
    pub id: String,
    /// حساب خریدار
    pub buyer_id: String,
    /// حساب فروشنده
    pub seller_id: String,
    /// مبلغ
    pub amount: Din,
    /// ارجاع محصول اختیاری
    pub product_ref: Option<String>,
    /// وضعیت فعلی
    pub status: EscrowStatus,
    /// زمان ساخت ISO
    pub created_at: String,
    /// شناسه تراکنش تامین‌مالی
    pub funded_tx_id: Option<String>,
    /// شناسه تراکنش تسویه (release/refund/expire)
    pub settle_tx_id: Option<String>,
    /// دلیل اختلاف
    pub dispute_reason: Option<String>,
    /// ارجاع تحویل
    pub delivery_ref: Option<String>,
    /// مهلت انقضا ISO
    pub expires_at: Option<String>,
}

/// ثبت‌همه امانت‌ها — تغییرناپذیر از دید فراخوان.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EscrowRegistry {
    /// امانت‌ها بر اساس شناسه
    pub escrows: BTreeMap<String, Escrow>,
}

/// رجیستری خالی.
#[must_use]
pub fn open_registry() -> EscrowRegistry {
    EscrowRegistry::default()
}

/// شناسه حساب دفتریِ یک امانت: `escrow:{id}`.
#[must_use]
pub fn escrow_account_id(escrow_id: &str) -> String {
    format!("escrow:{escrow_id}")
}

/// خروجی عملیات اسکرو — وضعیت‌های جدید + رویدادها + تراکنش اختیاری.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowOutcome {
    /// وضعیت جدید دفتر
    pub ledger: LedgerState,
    /// رجیستری جدید
    pub registry: EscrowRegistry,
    /// نسخه به‌روزشده امانت
    pub escrow: Escrow,
    /// رویدادهای صادرشده (به‌ترتیب)
    pub events: Vec<EscrowEvent>,
    /// تراکنش دفتر (فقط برای عملیات پول‌محور)
    pub tx: Option<PostedTransaction>,
}

fn put_escrow(registry: &EscrowRegistry, escrow: Escrow) -> EscrowRegistry {
    let mut escrows = registry.escrows.clone();
    escrows.insert(escrow.id.clone(), escrow);
    EscrowRegistry { escrows }
}

fn load_escrow(registry: &EscrowRegistry, escrow_id: &str) -> DomainResult<Escrow> {
    registry.escrows.get(escrow_id).cloned().ok_or_else(|| {
        DomainError::new(
            DomainErrorCode::EscrowNotFound,
            format!("Escrow {escrow_id} does not exist"),
        )
    })
}

/// فرمان ساخت امانت.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateEscrowCommand {
    /// شناسه یکتای امانت
    pub id: String,
    /// حساب خریدار
    pub buyer_id: String,
    /// حساب فروشنده
    pub seller_id: String,
    /// مبلغ
    pub amount: Din,
    /// ارجاع محصول
    pub product_ref: Option<String>,
    /// مهلت انقضا ISO
    pub expires_at: Option<String>,
    /// زمان ISO (پیش‌فرض epoch دترمینیستیک)
    pub at: Option<String>,
}

/// فرمان با ارجاع به امانت موجود.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowRefCommand {
    /// شناسه امانت
    pub escrow_id: String,
    /// کلید یکسان‌سازی (برای عملیات پول‌محور)
    pub idempotency_key: Option<String>,
    /// زمان ISO
    pub at: Option<String>,
}

/// فرمان تحویل.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliverEscrowCommand {
    /// شناسه امانت
    pub escrow_id: String,
    /// کلید یکسان‌سازی
    pub idempotency_key: Option<String>,
    /// زمان ISO
    pub at: Option<String>,
    /// ارجاع تحویل
    pub delivery_ref: Option<String>,
}

/// فرمان آزادسازی — ماده ۷: برای مبلغ ≥ ۵۰,۰۰۰ Đ نیاز به تأیید ادمین دارد.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEscrowCommand {
    /// شناسه امانت
    pub escrow_id: String,
    /// کلید یکسان‌سازی
    pub idempotency_key: Option<String>,
    /// زمان ISO
    pub at: Option<String>,
    /// تأیید انسانی (ماده ۷)
    pub approved_by_admin: bool,
}

/// فرمان ثبت اختلاف.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeEscrowCommand {
    /// شناسه امانت
    pub escrow_id: String,
    /// کلید یکسان‌سازی
    pub idempotency_key: Option<String>,
    /// زمان ISO
    pub at: Option<String>,
    /// دلیل اختلاف (غیرخالی)
    pub reason: String,
}

/// ساخت امانت (CREATED) — فقط حساب اسکرو با تراز صفر اضافه می‌شود.
///
/// # Errors
/// `ESCROW_ALREADY_EXISTS`، `UNKNOWN_ACCOUNT`، `INVALID_ACCOUNT_KIND`،
/// `DUPLICATE_ACCOUNT`.
pub fn create_escrow(
    ledger: &LedgerState,
    registry: &EscrowRegistry,
    cmd: &CreateEscrowCommand,
) -> DomainResult<EscrowOutcome> {
    if registry.escrows.contains_key(&cmd.id) {
        return Err(DomainError::new(
            DomainErrorCode::EscrowAlreadyExists,
            format!("Escrow {} already exists", cmd.id),
        ));
    }
    let buyer = ledger.accounts.get(&cmd.buyer_id).ok_or_else(|| {
        DomainError::new(
            DomainErrorCode::UnknownAccount,
            format!("Buyer account {} does not exist", cmd.buyer_id),
        )
    })?;
    let seller = ledger.accounts.get(&cmd.seller_id).ok_or_else(|| {
        DomainError::new(
            DomainErrorCode::UnknownAccount,
            format!("Seller account {} does not exist", cmd.seller_id),
        )
    })?;
    if buyer.kind != AccountKind::User || seller.kind != AccountKind::User {
        return Err(DomainError::new(
            DomainErrorCode::InvalidAccountKind,
            "Buyer and seller must both be user accounts",
        ));
    }
    let escrow_account = Account {
        id: escrow_account_id(&cmd.id),
        kind: AccountKind::Escrow,
        label: Some(format!("escrow {}", cmd.id)),
    };
    let with_account = add_account(ledger, escrow_account)?;

    let at = cmd.at.clone().unwrap_or_else(|| EPOCH_ISO.to_owned());
    let escrow = Escrow {
        id: cmd.id.clone(),
        buyer_id: cmd.buyer_id.clone(),
        seller_id: cmd.seller_id.clone(),
        amount: cmd.amount,
        product_ref: cmd.product_ref.clone(),
        status: EscrowStatus::Created,
        created_at: at.clone(),
        funded_tx_id: None,
        settle_tx_id: None,
        dispute_reason: None,
        delivery_ref: None,
        expires_at: cmd.expires_at.clone(),
    };
    let mut event = EscrowEvent::new(EscrowEventKind::Created, &escrow.id, &at);
    event.buyer_id = Some(escrow.buyer_id.clone());
    event.seller_id = Some(escrow.seller_id.clone());
    event.amount = Some(escrow.amount.as_i64());
    Ok(EscrowOutcome {
        ledger: with_account,
        registry: put_escrow(registry, escrow.clone()),
        escrow,
        events: vec![event],
        tx: None,
    })
}

/// تامین‌مالی امانت: خریدار → اسکرو (CREATED → FUNDED).
///
/// # Errors
/// `ESCROW_NOT_FOUND`، `INVALID_TRANSITION`، خطاهای دفتر.
pub fn fund_escrow(
    ledger: &LedgerState,
    registry: &EscrowRegistry,
    cmd: &EscrowRefCommand,
) -> DomainResult<EscrowOutcome> {
    let escrow = load_escrow(registry, &cmd.escrow_id)?;
    if escrow.status != EscrowStatus::Created {
        return Err(DomainError::new(
            DomainErrorCode::InvalidTransition,
            format!("Cannot fund escrow in status {}", escrow.status),
        ));
    }
    let posted = post(
        ledger,
        &PostCommand {
            id: None,
            idempotency_key: cmd.idempotency_key.clone(),
            postings: vec![
                Posting::new(escrow.buyer_id.clone(), -escrow.amount.as_i64()),
                Posting::new(escrow_account_id(&escrow.id), escrow.amount.as_i64()),
            ],
            memo: Some(format!("fund escrow {}", escrow.id)),
            at: cmd.at.clone(),
        },
    )?;
    let funded = Escrow {
        funded_tx_id: Some(posted.tx.id.clone()),
        status: EscrowStatus::Funded,
        ..escrow.clone()
    };
    let mut event = EscrowEvent::new(EscrowEventKind::Funded, &escrow.id, &posted.tx.at);
    event.amount = Some(escrow.amount.as_i64());
    event.tx_id = Some(posted.tx.id.clone());
    Ok(EscrowOutcome {
        ledger: posted.state,
        registry: put_escrow(registry, funded.clone()),
        escrow: funded,
        events: vec![event],
        tx: Some(posted.tx),
    })
}

/// تحویل کالا/خدمت (FUNDED → DELIVERED) — پولی جابه‌جا نمی‌شود.
///
/// # Errors
/// `ESCROW_NOT_FOUND`، `INVALID_TRANSITION`.
pub fn deliver_escrow(
    ledger: &LedgerState,
    registry: &EscrowRegistry,
    cmd: &DeliverEscrowCommand,
) -> DomainResult<EscrowOutcome> {
    let escrow = load_escrow(registry, &cmd.escrow_id)?;
    if escrow.status != EscrowStatus::Funded {
        return Err(DomainError::new(
            DomainErrorCode::InvalidTransition,
            format!("Cannot deliver escrow in status {}", escrow.status),
        ));
    }
    let at = cmd.at.clone().unwrap_or_else(|| EPOCH_ISO.to_owned());
    let delivered = Escrow {
        status: EscrowStatus::Delivered,
        delivery_ref: cmd.delivery_ref.clone(),
        ..escrow.clone()
    };
    let event = EscrowEvent::new(EscrowEventKind::Delivered, &escrow.id, &at);
    Ok(EscrowOutcome {
        ledger: ledger.clone(),
        registry: put_escrow(registry, delivered.clone()),
        escrow: delivered,
        events: vec![event],
        tx: None,
    })
}

/// آزادسازی: اسکرو → فروشنده (خالص) + خزانه (مالیات ۱۰٪).
/// ماده ۷ — مبلغ ≥ ۵۰,۰۰۰ Đ بدون `approved_by_admin` رد می‌شود.
///
/// # Errors
/// `ESCROW_NOT_FOUND`، `INVALID_TRANSITION`، `REQUIRES_HUMAN_APPROVAL`،
/// خطاهای دفتر/منشور.
pub fn release_escrow(
    ledger: &LedgerState,
    registry: &EscrowRegistry,
    cmd: &ReleaseEscrowCommand,
) -> DomainResult<EscrowOutcome> {
    let escrow = load_escrow(registry, &cmd.escrow_id)?;
    if escrow.status != EscrowStatus::Delivered && escrow.status != EscrowStatus::Disputed {
        return Err(DomainError::new(
            DomainErrorCode::InvalidTransition,
            format!("Cannot release escrow in status {}", escrow.status),
        ));
    }
    if requires_hitl(escrow.amount) && !cmd.approved_by_admin {
        return Err(DomainError::new(
            DomainErrorCode::RequiresHumanApproval,
            format!(
                "Release of {} requires human approval (HITL threshold {} Đ)",
                escrow.amount.as_i64(),
                Charter::HITL_THRESHOLD_DIN.as_i64()
            ),
        ));
    }
    let split = split_seller_tax(escrow.amount)?;
    let posted = post(
        ledger,
        &PostCommand {
            id: None,
            idempotency_key: cmd.idempotency_key.clone(),
            postings: vec![
                Posting::new(escrow_account_id(&escrow.id), -escrow.amount.as_i64()),
                Posting::new(escrow.seller_id.clone(), split.seller.as_i64()),
                Posting::new(TREASURY_ACCOUNT_ID, split.tax.as_i64()),
            ],
            memo: Some(format!("release escrow {} (tax {})", escrow.id, split.tax.as_i64())),
            at: cmd.at.clone(),
        },
    )?;
    let released = Escrow {
        status: EscrowStatus::Released,
        settle_tx_id: Some(posted.tx.id.clone()),
        ..escrow.clone()
    };
    let mut event = EscrowEvent::new(EscrowEventKind::Released, &escrow.id, &posted.tx.at);
    event.tx_id = Some(posted.tx.id.clone());
    event.seller_net = Some(split.seller.as_i64());
    event.tax = Some(split.tax.as_i64());
    Ok(EscrowOutcome {
        ledger: posted.state,
        registry: put_escrow(registry, released.clone()),
        escrow: released,
        events: vec![event],
        tx: Some(posted.tx),
    })
}

/// ثبت اختلاف (FUNDED|DELIVERED → DISPUTED) — دلیل غیرخالی الزامی است.
///
/// # Errors
/// `ESCROW_NOT_FOUND`، `INVALID_TRANSITION`.
pub fn dispute_escrow(
    ledger: &LedgerState,
    registry: &EscrowRegistry,
    cmd: &DisputeEscrowCommand,
) -> DomainResult<EscrowOutcome> {
    let escrow = load_escrow(registry, &cmd.escrow_id)?;
    if escrow.status != EscrowStatus::Funded && escrow.status != EscrowStatus::Delivered {
        return Err(DomainError::new(
            DomainErrorCode::InvalidTransition,
            format!("Cannot dispute escrow in status {}", escrow.status),
        ));
    }
    if cmd.reason.trim().is_empty() {
        return Err(DomainError::new(
            DomainErrorCode::InvalidTransition,
            "A dispute requires a non-empty reason",
        ));
    }
    let at = cmd.at.clone().unwrap_or_else(|| EPOCH_ISO.to_owned());
    let disputed = Escrow {
        status: EscrowStatus::Disputed,
        dispute_reason: Some(cmd.reason.clone()),
        ..escrow.clone()
    };
    let mut event = EscrowEvent::new(EscrowEventKind::Disputed, &escrow.id, &at);
    event.reason = Some(cmd.reason.clone());
    Ok(EscrowOutcome {
        ledger: ledger.clone(),
        registry: put_escrow(registry, disputed.clone()),
        escrow: disputed,
        events: vec![event],
        tx: None,
    })
}

/// بازگشت کامل وجه به خریدار، بدون مالیات (DISPUTED → REFUNDED).
///
/// # Errors
/// `ESCROW_NOT_FOUND`، `INVALID_TRANSITION`، خطاهای دفتر.
pub fn refund_escrow(
    ledger: &LedgerState,
    registry: &EscrowRegistry,
    cmd: &EscrowRefCommand,
) -> DomainResult<EscrowOutcome> {
    let escrow = load_escrow(registry, &cmd.escrow_id)?;
    if escrow.status != EscrowStatus::Disputed {
        return Err(DomainError::new(
            DomainErrorCode::InvalidTransition,
            format!("Cannot refund escrow in status {} (must be DISPUTED)", escrow.status),
        ));
    }
    let posted = post(
        ledger,
        &PostCommand {
            id: None,
            idempotency_key: cmd.idempotency_key.clone(),
            postings: vec![
                Posting::new(escrow_account_id(&escrow.id), -escrow.amount.as_i64()),
                Posting::new(escrow.buyer_id.clone(), escrow.amount.as_i64()),
            ],
            memo: Some(format!("refund escrow {}", escrow.id)),
            at: cmd.at.clone(),
        },
    )?;
    let refunded = Escrow {
        status: EscrowStatus::Refunded,
        settle_tx_id: Some(posted.tx.id.clone()),
        ..escrow.clone()
    };
    let mut event = EscrowEvent::new(EscrowEventKind::Refunded, &escrow.id, &posted.tx.at);
    event.tx_id = Some(posted.tx.id.clone());
    event.amount = Some(escrow.amount.as_i64());
    Ok(EscrowOutcome {
        ledger: posted.state,
        registry: put_escrow(registry, refunded.clone()),
        escrow: refunded,
        events: vec![event],
        tx: Some(posted.tx),
    })
}

/// لغو قبل از تامین‌مالی (CREATED → CANCELLED) — هیچ پولی حرکت نمی‌کند.
///
/// # Errors
/// `ESCROW_NOT_FOUND`، `INVALID_TRANSITION`.
pub fn cancel_escrow(
    ledger: &LedgerState,
    registry: &EscrowRegistry,
    cmd: &EscrowRefCommand,
) -> DomainResult<EscrowOutcome> {
    let escrow = load_escrow(registry, &cmd.escrow_id)?;
    if escrow.status != EscrowStatus::Created {
        return Err(DomainError::new(
            DomainErrorCode::InvalidTransition,
            format!("Cannot cancel escrow in status {} (only CREATED)", escrow.status),
        ));
    }
    let at = cmd.at.clone().unwrap_or_else(|| EPOCH_ISO.to_owned());
    let cancelled = Escrow { status: EscrowStatus::Cancelled, ..escrow.clone() };
    let event = EscrowEvent::new(EscrowEventKind::Cancelled, &escrow.id, &at);
    Ok(EscrowOutcome {
        ledger: ledger.clone(),
        registry: put_escrow(registry, cancelled.clone()),
        escrow: cancelled,
        events: vec![event],
        tx: None,
    })
}

/// انقضای امانت تامین‌شده با بازگشت خودکار کامل (ماده ۹).
///
/// # Errors
/// `ESCROW_NOT_FOUND`، `INVALID_TRANSITION`، خطاهای دفتر.
pub fn expire_escrow(
    ledger: &LedgerState,
    registry: &EscrowRegistry,
    cmd: &EscrowRefCommand,
) -> DomainResult<EscrowOutcome> {
    let escrow = load_escrow(registry, &cmd.escrow_id)?;
    if escrow.status != EscrowStatus::Funded {
        return Err(DomainError::new(
            DomainErrorCode::InvalidTransition,
            format!("Cannot expire escrow in status {} (must be FUNDED)", escrow.status),
        ));
    }
    let posted = post(
        ledger,
        &PostCommand {
            id: None,
            idempotency_key: cmd.idempotency_key.clone(),
            postings: vec![
                Posting::new(escrow_account_id(&escrow.id), -escrow.amount.as_i64()),
                Posting::new(escrow.buyer_id.clone(), escrow.amount.as_i64()),
            ],
            memo: Some(format!("expire-refund escrow {}", escrow.id)),
            at: cmd.at.clone(),
        },
    )?;
    let expired = Escrow {
        status: EscrowStatus::Expired,
        settle_tx_id: Some(posted.tx.id.clone()),
        ..escrow.clone()
    };
    let mut event = EscrowEvent::new(EscrowEventKind::Expired, &escrow.id, &posted.tx.at);
    event.tx_id = Some(posted.tx.id.clone());
    event.amount = Some(escrow.amount.as_i64());
    Ok(EscrowOutcome {
        ledger: posted.state,
        registry: put_escrow(registry, expired.clone()),
        escrow: expired,
        events: vec![event],
        tx: Some(posted.tx),
    })
}

/// جاروب TTL: امانت‌های FUNDEDِ گذشته از مهلت که هنوز پول در اسکرو دارند.
#[must_use]
pub fn find_expired(registry: &EscrowRegistry, now_iso: &str) -> Vec<Escrow> {
    let Some(now) = parse_iso_unix(now_iso) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for escrow in registry.escrows.values() {
        if escrow.status != EscrowStatus::Funded {
            continue;
        }
        if let Some(exp) = &escrow.expires_at {
            if let Some(t) = parse_iso_unix(exp) {
                if t <= now {
                    out.push(escrow.clone());
                }
            }
        }
    }
    out
}

/// تبدیل ISO-8601 UTC (`YYYY-MM-DDTHH:MM:SS(.mmm)?Z`) به ثانیه یونیکس —
/// بدون وابستگی خارجی؛ فقط برای مقایسه مهلت‌ها.
#[must_use]
fn parse_iso_unix(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> { s.get(from..to)?.parse::<i64>().ok() };
    let year = num(0, 4)?;
    let month = num(5, 7)? as u32;
    let day = num(8, 10)? as u32;
    let hour = num(11, 13)?;
    let minute = num(14, 16)?;
    let second = num(17, 19)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// الگوریتم `days_from_civil` (Hinnant) — تبدیل تاریخ میلادی به شمارش روز.
const fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (i64_from_u32(m) + 9) % 12; // [0, 11]
    let doy = (153 * mp + 2) / 5 + i64_from_u32(d) - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// `i64::from` معادل — سازگار با `const fn`.
const fn i64_from_u32(v: u32) -> i64 {
    v as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{
        balance_of, circulating_supply, open_ledger, trial_balance, BURN_ACCOUNT_ID,
        MINT_ACCOUNT_ID,
    };

    const MINT: &str = MINT_ACCOUNT_ID;
    const TREASURY: &str = TREASURY_ACCOUNT_ID;
    const BUYER: &str = "user:buyer";
    const SELLER: &str = "user:seller";
    const T0: &str = "2026-01-01T09:00:00.000Z";

    fn d(n: i64) -> Din {
        Din::new(n).unwrap()
    }

    fn setup() -> (LedgerState, EscrowRegistry) {
        let l = open_ledger();
        let l = add_account(&l, Account { id: MINT.into(), kind: AccountKind::Mint, label: None })
            .unwrap();
        let l = add_account(
            &l,
            Account { id: TREASURY.into(), kind: AccountKind::Treasury, label: None },
        )
        .unwrap();
        let l = add_account(
            &l,
            Account { id: BURN_ACCOUNT_ID.into(), kind: AccountKind::Burn, label: None },
        )
        .unwrap();
        let l = add_account(
            &l,
            Account {
                id: BUYER.into(), kind: AccountKind::User, label: Some("خریدار".into())
            },
        )
        .unwrap();
        let l = add_account(
            &l,
            Account {
                id: SELLER.into(), kind: AccountKind::User, label: Some("فروشنده".into())
            },
        )
        .unwrap();
        let l = post(
            &l,
            &PostCommand {
                id: None,
                idempotency_key: None,
                postings: vec![Posting::new(MINT, -1_000), Posting::new(BUYER, 1_000)],
                memo: None,
                at: None,
            },
        )
        .unwrap()
        .state;
        (l, open_registry())
    }

    /// create → fund → deliver یک‌جا (امانت آماده تسویه + رویدادهای تجمعی)
    fn funded(
        ledger: &LedgerState,
        registry: &EscrowRegistry,
        id: &str,
        amount: i64,
        expires_at: Option<&str>,
    ) -> (LedgerState, EscrowRegistry, Vec<EscrowEvent>) {
        let a = create_escrow(
            ledger,
            registry,
            &CreateEscrowCommand {
                id: id.into(),
                buyer_id: BUYER.into(),
                seller_id: SELLER.into(),
                amount: d(amount),
                product_ref: None,
                expires_at: expires_at.map(str::to_owned),
                at: Some(T0.into()),
            },
        )
        .unwrap();
        let b = fund_escrow(
            &a.ledger,
            &a.registry,
            &EscrowRefCommand {
                escrow_id: id.into(),
                idempotency_key: Some(format!("fund:{id}")),
                at: None,
            },
        )
        .unwrap();
        let c = deliver_escrow(
            &b.ledger,
            &b.registry,
            &DeliverEscrowCommand {
                escrow_id: id.into(),
                idempotency_key: None,
                at: None,
                delivery_ref: Some(format!("payload:{id}")),
            },
        )
        .unwrap();
        let mut events = a.events;
        events.extend(b.events);
        events.extend(c.events);
        (c.ledger, c.registry, events)
    }

    #[test]
    fn runs_the_happy_path_fund_100_seller_90_treasury_10() {
        let (l, reg) = setup();
        let created = create_escrow(
            &l,
            &reg,
            &CreateEscrowCommand {
                id: "e1".into(),
                buyer_id: BUYER.into(),
                seller_id: SELLER.into(),
                amount: d(100),
                product_ref: Some("prod:1".into()),
                expires_at: None,
                at: Some(T0.into()),
            },
        )
        .unwrap();
        assert_eq!(created.escrow.status, EscrowStatus::Created);
        assert_eq!(balance_of(&created.ledger, &escrow_account_id("e1")).unwrap(), 0);

        let funded_e = fund_escrow(
            &created.ledger,
            &created.registry,
            &EscrowRefCommand {
                escrow_id: "e1".into(),
                idempotency_key: Some("fund:e1".into()),
                at: None,
            },
        )
        .unwrap();
        assert_eq!(funded_e.escrow.status, EscrowStatus::Funded);
        assert_eq!(balance_of(&funded_e.ledger, BUYER).unwrap(), 900);
        assert_eq!(balance_of(&funded_e.ledger, &escrow_account_id("e1")).unwrap(), 100);

        let delivered = deliver_escrow(
            &funded_e.ledger,
            &funded_e.registry,
            &DeliverEscrowCommand {
                escrow_id: "e1".into(),
                idempotency_key: None,
                at: None,
                delivery_ref: Some("payload:e1".into()),
            },
        )
        .unwrap();
        assert_eq!(delivered.escrow.status, EscrowStatus::Delivered);

        let released = release_escrow(
            &delivered.ledger,
            &delivered.registry,
            &ReleaseEscrowCommand {
                escrow_id: "e1".into(),
                idempotency_key: Some("settle:e1".into()),
                at: None,
                approved_by_admin: false,
            },
        )
        .unwrap();
        assert_eq!(released.escrow.status, EscrowStatus::Released);
        assert_eq!(balance_of(&released.ledger, BUYER).unwrap(), 900);
        assert_eq!(balance_of(&released.ledger, SELLER).unwrap(), 90);
        assert_eq!(balance_of(&released.ledger, TREASURY).unwrap(), 10);
        assert_eq!(balance_of(&released.ledger, &escrow_account_id("e1")).unwrap(), 0);
        assert!(trial_balance(&released.ledger).balanced);
        assert_eq!(circulating_supply(&released.ledger), 1_000);
    }

    #[test]
    fn emits_deterministic_events_in_order() {
        let (l, reg) = setup();
        let (l, reg, events) = funded(&l, &reg, "e1", 100, None);
        let released = release_escrow(
            &l,
            &reg,
            &ReleaseEscrowCommand {
                escrow_id: "e1".into(),
                idempotency_key: None,
                at: None,
                approved_by_admin: false,
            },
        )
        .unwrap();
        let mut kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
        kinds.extend(released.events.iter().map(|e| e.kind.as_str()));
        assert_eq!(kinds, ["EscrowCreated", "EscrowFunded", "EscrowDelivered", "EscrowReleased"]);
        assert_eq!(events[0].id, "e1#EscrowCreated");
        assert_eq!(released.events[0].id, "e1#EscrowReleased");
        assert_eq!(released.events[0].tax, Some(10));
        assert_eq!(released.events[0].seller_net, Some(90));
    }

    #[test]
    fn gates_big_releases_behind_hitl_at_50k() {
        let (l, reg) = setup();
        let l = post(
            &l,
            &PostCommand {
                id: None,
                idempotency_key: None,
                postings: vec![Posting::new(MINT, -60_000), Posting::new(BUYER, 60_000)],
                memo: None,
                at: None,
            },
        )
        .unwrap()
        .state;
        let created = create_escrow(
            &l,
            &reg,
            &CreateEscrowCommand {
                id: "big".into(),
                buyer_id: BUYER.into(),
                seller_id: SELLER.into(),
                amount: d(60_000),
                product_ref: None,
                expires_at: None,
                at: None,
            },
        )
        .unwrap();
        let f = fund_escrow(
            &created.ledger,
            &created.registry,
            &EscrowRefCommand { escrow_id: "big".into(), idempotency_key: None, at: None },
        )
        .unwrap();
        let del = deliver_escrow(
            &f.ledger,
            &f.registry,
            &DeliverEscrowCommand {
                escrow_id: "big".into(),
                idempotency_key: None,
                at: None,
                delivery_ref: None,
            },
        )
        .unwrap();
        let blocked = release_escrow(
            &del.ledger,
            &del.registry,
            &ReleaseEscrowCommand {
                escrow_id: "big".into(),
                idempotency_key: None,
                at: None,
                approved_by_admin: false,
            },
        );
        assert_eq!(blocked.unwrap_err().code, DomainErrorCode::RequiresHumanApproval);
        let allowed = release_escrow(
            &del.ledger,
            &del.registry,
            &ReleaseEscrowCommand {
                escrow_id: "big".into(),
                idempotency_key: None,
                at: None,
                approved_by_admin: true,
            },
        )
        .unwrap();
        assert_eq!(balance_of(&allowed.ledger, SELLER).unwrap(), 54_000);
        assert_eq!(balance_of(&allowed.ledger, TREASURY).unwrap(), 6_000);
    }

    #[test]
    fn runs_dispute_refund_end_to_end() {
        let (l, reg) = setup();
        let c = create_escrow(
            &l,
            &reg,
            &CreateEscrowCommand {
                id: "e2".into(),
                buyer_id: BUYER.into(),
                seller_id: SELLER.into(),
                amount: d(40),
                product_ref: None,
                expires_at: None,
                at: None,
            },
        )
        .unwrap();
        let f = fund_escrow(
            &c.ledger,
            &c.registry,
            &EscrowRefCommand { escrow_id: "e2".into(), idempotency_key: None, at: None },
        )
        .unwrap();
        let disputed = dispute_escrow(
            &f.ledger,
            &f.registry,
            &DisputeEscrowCommand {
                escrow_id: "e2".into(),
                idempotency_key: None,
                at: None,
                reason: "کالا ناقص بود".into(),
            },
        )
        .unwrap();
        assert_eq!(disputed.escrow.status, EscrowStatus::Disputed);
        let refunded = refund_escrow(
            &disputed.ledger,
            &disputed.registry,
            &EscrowRefCommand { escrow_id: "e2".into(), idempotency_key: None, at: None },
        )
        .unwrap();
        assert_eq!(refunded.escrow.status, EscrowStatus::Refunded);
        assert_eq!(balance_of(&refunded.ledger, BUYER).unwrap(), 1_000);
        assert_eq!(balance_of(&refunded.ledger, SELLER).unwrap(), 0);
        assert_eq!(balance_of(&refunded.ledger, TREASURY).unwrap(), 0);
        assert!(trial_balance(&refunded.ledger).balanced);
    }

    #[test]
    fn allows_arbitration_to_release_disputed_to_seller() {
        let (l, reg) = setup();
        let c = create_escrow(
            &l,
            &reg,
            &CreateEscrowCommand {
                id: "e3".into(),
                buyer_id: BUYER.into(),
                seller_id: SELLER.into(),
                amount: d(40),
                product_ref: None,
                expires_at: None,
                at: None,
            },
        )
        .unwrap();
        let f = fund_escrow(
            &c.ledger,
            &c.registry,
            &EscrowRefCommand { escrow_id: "e3".into(), idempotency_key: None, at: None },
        )
        .unwrap();
        let disputed = dispute_escrow(
            &f.ledger,
            &f.registry,
            &DisputeEscrowCommand {
                escrow_id: "e3".into(),
                idempotency_key: None,
                at: None,
                reason: "ادعای خریدار".into(),
            },
        )
        .unwrap();
        let released = release_escrow(
            &disputed.ledger,
            &disputed.registry,
            &ReleaseEscrowCommand {
                escrow_id: "e3".into(),
                idempotency_key: None,
                at: None,
                approved_by_admin: false,
            },
        )
        .unwrap();
        assert_eq!(released.escrow.status, EscrowStatus::Released);
        assert_eq!(balance_of(&released.ledger, SELLER).unwrap(), 36);
        assert_eq!(balance_of(&released.ledger, TREASURY).unwrap(), 4);
    }

    #[test]
    fn cancels_before_funding_without_moving_money() {
        let (l, reg) = setup();
        let c = create_escrow(
            &l,
            &reg,
            &CreateEscrowCommand {
                id: "e4".into(),
                buyer_id: BUYER.into(),
                seller_id: SELLER.into(),
                amount: d(70),
                product_ref: None,
                expires_at: None,
                at: None,
            },
        )
        .unwrap();
        let cancelled = cancel_escrow(
            &c.ledger,
            &c.registry,
            &EscrowRefCommand { escrow_id: "e4".into(), idempotency_key: None, at: None },
        )
        .unwrap();
        assert_eq!(cancelled.escrow.status, EscrowStatus::Cancelled);
        assert_eq!(balance_of(&cancelled.ledger, BUYER).unwrap(), 1_000);
        assert_eq!(balance_of(&cancelled.ledger, SELLER).unwrap(), 0);
        assert_eq!(balance_of(&cancelled.ledger, &escrow_account_id("e4")).unwrap(), 0);
        assert!(cancelled.tx.is_none());
    }

    #[test]
    fn expires_funded_escrows_past_ttl_and_auto_refunds() {
        let (l, reg) = setup();
        let c = create_escrow(
            &l,
            &reg,
            &CreateEscrowCommand {
                id: "e5".into(),
                buyer_id: BUYER.into(),
                seller_id: SELLER.into(),
                amount: d(40),
                product_ref: None,
                expires_at: Some("2026-01-05T00:00:00.000Z".into()),
                at: None,
            },
        )
        .unwrap();
        let f = fund_escrow(
            &c.ledger,
            &c.registry,
            &EscrowRefCommand { escrow_id: "e5".into(), idempotency_key: None, at: None },
        )
        .unwrap();
        let due = find_expired(&f.registry, "2026-01-06T00:00:00.000Z");
        assert_eq!(due.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["e5"]);
        let expired = expire_escrow(
            &f.ledger,
            &f.registry,
            &EscrowRefCommand { escrow_id: "e5".into(), idempotency_key: None, at: None },
        )
        .unwrap();
        assert_eq!(expired.escrow.status, EscrowStatus::Expired);
        assert_eq!(balance_of(&expired.ledger, BUYER).unwrap(), 1_000);
        assert_eq!(balance_of(&expired.ledger, &escrow_account_id("e5")).unwrap(), 0);
        assert!(find_expired(&expired.registry, "2026-01-06T00:00:00.000Z").is_empty());
    }

    #[test]
    fn rejects_every_illegal_transition_fail_closed() {
        let (l, reg) = setup();
        // امانت ناشناخته
        assert!(fund_escrow(
            &l,
            &reg,
            &EscrowRefCommand { escrow_id: "ghost".into(), idempotency_key: None, at: None }
        )
        .is_err());
        let c = create_escrow(
            &l,
            &reg,
            &CreateEscrowCommand {
                id: "e6".into(),
                buyer_id: BUYER.into(),
                seller_id: SELLER.into(),
                amount: d(20),
                product_ref: None,
                expires_at: None,
                at: None,
            },
        )
        .unwrap();
        // تحویل قبل از تامین
        assert!(deliver_escrow(
            &c.ledger,
            &c.registry,
            &DeliverEscrowCommand {
                escrow_id: "e6".into(),
                idempotency_key: None,
                at: None,
                delivery_ref: None
            }
        )
        .is_err());
        // آزادسازی قبل از تامین
        assert!(release_escrow(
            &c.ledger,
            &c.registry,
            &ReleaseEscrowCommand {
                escrow_id: "e6".into(),
                idempotency_key: None,
                at: None,
                approved_by_admin: false
            }
        )
        .is_err());
        // لغو بعد از تامین
        let f = fund_escrow(
            &c.ledger,
            &c.registry,
            &EscrowRefCommand { escrow_id: "e6".into(), idempotency_key: None, at: None },
        )
        .unwrap();
        assert!(cancel_escrow(
            &f.ledger,
            &f.registry,
            &EscrowRefCommand { escrow_id: "e6".into(), idempotency_key: None, at: None }
        )
        .is_err());
        // بازگشت بدون اختلاف
        assert!(refund_escrow(
            &f.ledger,
            &f.registry,
            &EscrowRefCommand { escrow_id: "e6".into(), idempotency_key: None, at: None }
        )
        .is_err());
        // آزادسازی قبل از تحویل
        assert!(release_escrow(
            &f.ledger,
            &f.registry,
            &ReleaseEscrowCommand {
                escrow_id: "e6".into(),
                idempotency_key: None,
                at: None,
                approved_by_admin: false
            }
        )
        .is_err());
        // ساخت تکراری
        assert!(create_escrow(
            &l,
            &f.registry,
            &CreateEscrowCommand {
                id: "e6".into(),
                buyer_id: BUYER.into(),
                seller_id: SELLER.into(),
                amount: d(20),
                product_ref: None,
                expires_at: None,
                at: None
            }
        )
        .is_err());
        // خریدار باید حساب user باشد
        assert!(create_escrow(
            &l,
            &reg,
            &CreateEscrowCommand {
                id: "e7".into(),
                buyer_id: TREASURY.into(),
                seller_id: SELLER.into(),
                amount: d(20),
                product_ref: None,
                expires_at: None,
                at: None
            }
        )
        .is_err());
        // اختلاف نیاز به دلیل دارد
        assert!(dispute_escrow(
            &f.ledger,
            &f.registry,
            &DisputeEscrowCommand {
                escrow_id: "e6".into(),
                idempotency_key: None,
                at: None,
                reason: "   ".into()
            }
        )
        .is_err());
    }

    #[test]
    fn carries_the_transition_table_for_ui() {
        assert!(escrow_transitions(EscrowStatus::Created).contains(&EscrowEventKind::Funded));
        assert!(escrow_transitions(EscrowStatus::Funded).contains(&EscrowEventKind::Expired));
        assert_eq!(
            escrow_transitions(EscrowStatus::Disputed),
            [EscrowEventKind::Refunded, EscrowEventKind::Released]
        );
        for terminal in ESCROW_TERMINAL_STATUSES {
            assert!(escrow_transitions(terminal).is_empty());
            assert!(terminal.is_terminal());
        }
    }

    #[test]
    fn parses_iso_dates_for_ttl_sweep() {
        // مرزهای الگوریتم تاریخ — بدون وابستگی خارجی
        assert_eq!(parse_iso_unix("2026-01-01T00:00:00.000Z"), Some(1_767_225_600));
        assert_eq!(parse_iso_unix("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(parse_iso_unix("2026-13-01T00:00:00Z"), None);
        assert_eq!(parse_iso_unix("نامعتبر"), None);
        // 2026-01-05 <= 2026-01-06
        assert!(
            parse_iso_unix("2026-01-05T00:00:00.000Z").unwrap()
                <= parse_iso_unix("2026-01-06T00:00:00.000Z").unwrap()
        );
    }
}
