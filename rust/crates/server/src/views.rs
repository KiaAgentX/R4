//! تبدیل‌گرهای دامنه → وایر — تنها جایی که شکل DTOها ساخته می‌شود.
//!
//! قاعدهٔ فاز ۳ سر جایش است: فیلدهای `.nullable()` همیشه صریح روی سیم می‌مانند
//! (`null`)، فیلدهای `.optional()` غایب؛ وضعیت اسکرو `SCREAMING_SNAKE_CASE` و
//! وضعیت‌های UI داشبورد `lowercase`. هیچ هندلری مستقیم JSON نمی‌سازد.

use dropagentx_contracts::{
    ApprovalView, AuditRow, DashPerson, DashUser, DashWallet, EscrowStatus, EscrowView, HitlView,
    IsoDateTime, LeaderboardEntry, OrderView, ProductStatus, ProductView, RiskLevel,
};
use dropagentx_db::{AccountBalance, AuditRowRead, HitlRow, ProductRow};
use dropagentx_domain::{Charter, Escrow};

use crate::error::ApiFailure;

/// وضعیت امانت دامنه → وضعیت وایر (همان رشته‌ها).
#[must_use]
pub fn escrow_status_wire(status: dropagentx_domain::EscrowStatus) -> EscrowStatus {
    match status {
        dropagentx_domain::EscrowStatus::Created => EscrowStatus::Created,
        dropagentx_domain::EscrowStatus::Funded => EscrowStatus::Funded,
        dropagentx_domain::EscrowStatus::Delivered => EscrowStatus::Delivered,
        dropagentx_domain::EscrowStatus::Released => EscrowStatus::Released,
        dropagentx_domain::EscrowStatus::Disputed => EscrowStatus::Disputed,
        dropagentx_domain::EscrowStatus::Refunded => EscrowStatus::Refunded,
        dropagentx_domain::EscrowStatus::Cancelled => EscrowStatus::Cancelled,
        dropagentx_domain::EscrowStatus::Expired => EscrowStatus::Expired,
    }
}

/// i64 دامنه → `Din` وایر — دامنهٔ خالص بازهٔ `[0, 2^53−1]` را تضمین می‌کند؛
/// شاخهٔ دوم عملاً غیرقابل‌دسترس است و فقط برای کلن‌بودن تایپ لازم می‌شود.
#[must_use]
pub fn wire_din(raw: i64) -> dropagentx_contracts::Din {
    match dropagentx_contracts::Din::new(raw) {
        Some(d) => d,
        None => match dropagentx_contracts::Din::new(0) {
            Some(zero) => zero,
            None => unreachable!("Din::new(0) is valid by definition"),
        },
    }
}

/// نمای امانت وایر — فیلدهای nullable صریح.
#[must_use]
pub fn escrow_view(e: &Escrow) -> EscrowView {
    EscrowView {
        id: e.id.clone(),
        buyer_id: e.buyer_id.clone(),
        seller_id: e.seller_id.clone(),
        amount: wire_din(e.amount.as_i64()),
        product_ref: e.product_ref.clone(),
        status: escrow_status_wire(e.status),
        created_at: e.created_at.clone(),
        funded_tx_id: e.funded_tx_id.clone(),
        settle_tx_id: e.settle_tx_id.clone(),
        dispute_reason: e.dispute_reason.clone(),
        delivery_ref: e.delivery_ref.clone(),
        expires_at: e.expires_at.clone(),
    }
}

/// نمای حساب وایر — تراز مشتق‌شده (ADR-003).
#[must_use]
pub fn account_view(a: &AccountBalance) -> dropagentx_contracts::AccountView {
    dropagentx_contracts::AccountView {
        id: a.id.clone(),
        kind: a.kind.clone(),
        label: a.label.clone(),
        balance: a.balance,
        created_at: a.created_at.clone(),
    }
}

/// نمای درخواست HITL وایر.
///
/// # Errors
/// `HitlRow.status` ناشناخته → 500 `INTERNAL` (دادهٔ خراب، هرگز بی‌صدا رد نمی‌شود).
pub fn hitl_view(r: &HitlRow) -> Result<HitlView, ApiFailure> {
    let kind = match r.kind.as_str() {
        "ESCROW_RELEASE" => dropagentx_contracts::HitlKind::EscrowRelease,
        other => return Err(http_failure_corrupt(format!("unknown HITL kind {other:?}")).into()),
    };
    let status = match r.status.as_str() {
        "PENDING" => dropagentx_contracts::HitlStatus::Pending,
        "APPROVED" => dropagentx_contracts::HitlStatus::Approved,
        "REJECTED" => dropagentx_contracts::HitlStatus::Rejected,
        "EXPIRED" => dropagentx_contracts::HitlStatus::Expired,
        other => return Err(http_failure_corrupt(format!("unknown HITL status {other:?}")).into()),
    };
    Ok(HitlView {
        id: r.id.clone(),
        kind,
        escrow_id: r.escrow_id.clone(),
        amount: dropagentx_contracts::Din::new(r.amount)
            .ok_or_else(|| http_failure_corrupt("HITL amount outside safe range"))?,
        requester_id: r.requester_id.clone(),
        note: r.note.clone(),
        status,
        created_at: r.created_at.clone(),
        expires_at: r.expires_at.clone(),
        decided_by: r.decided_by.clone(),
        decided_at: r.decided_at.clone(),
        decision_note: r.decision_note.clone(),
    })
}

// سازندهٔ خطای دادهٔ خراب — 500 INTERNAL با پیام صادقانه
// (فیلدهای HttpFailure عمومی‌اند — سازندهٔ pub(crate) فاز ۳ از بیرون در دسترس نیست)
fn http_failure_corrupt(message: impl Into<String>) -> dropagentx_contracts::HttpFailure {
    dropagentx_contracts::HttpFailure {
        status: 500,
        code: "INTERNAL".into(),
        message: message.into(),
        meta: None,
    }
}

/// ردیف audit وایر — `status` همیشه `"ok"` (رویداد ثبت‌شده = اتفاق‌افتاده).
#[must_use]
pub fn audit_row(r: &AuditRowRead) -> AuditRow {
    AuditRow {
        id: r.id.clone(),
        actor: r.actor.clone(),
        action: r.action.clone(),
        target: r.target.clone(),
        status: "ok".into(),
        created_at: iso_or_epoch(&r.at),
    }
}

/// رشتهٔ زمان DB → `IsoDateTime` وایر؛ ورودی نامعتبر (غیرقابل‌تصور از
/// `now_iso`) به epoch دترمینیستیک دامنه می‌افتد — نمایش، نه پول.
#[must_use]
pub fn iso_or_epoch(raw: &str) -> IsoDateTime {
    match IsoDateTime::new(raw) {
        Some(v) => v,
        None => match IsoDateTime::new(dropagentx_domain::EPOCH_ISO) {
            Some(epoch) => epoch,
            None => unreachable!("EPOCH_ISO is a valid RFC3339 timestamp"),
        },
    }
}

/// نمای محصول وایر — فیلدهای dashboard-only ستون ندارند و صادقانه مشتق
/// می‌شوند: Stars ریل نیست (۰)، stock نامحدود (−۱)، featured ستون ندارد
/// (false)، salesCount از اسکروهای تسویه‌شده با `productRef` می‌آید.
#[must_use]
pub fn product_view(p: &ProductRow, sales_count: i64) -> ProductView {
    ProductView {
        id: p.id.clone(),
        seller_id: p.seller_id.clone(),
        title: p.title.clone(),
        description: p.description.clone(),
        category: p.category.clone().unwrap_or_default(),
        price_stars: 0,
        price_credits: p.price_din,
        stock: -1,
        sales_count,
        featured: false,
        status: if p.active { ProductStatus::Active } else { ProductStatus::Archived },
        created_at: iso_or_epoch(&p.created_at),
    }
}

/// سطح ریسک از آستانهٔ منشور (مادهٔ ۷) — ≥ ۵۰٬۰۰۰ Đ `high`، وگرنه `low`.
#[must_use]
fn risk_level(amount: i64) -> RiskLevel {
    if amount >= Charter::HITL_THRESHOLD_DIN.as_i64() {
        RiskLevel::High
    } else {
        RiskLevel::Low
    }
}

/// نمای تأیید داشبورد از ردیف HITL — خلاصهٔ انسانی + ریسک مشتق از منشور.
///
/// # Errors
/// همان [`hitl_view`] (kind/status ناشناخته).
pub fn approval_view(r: &HitlRow) -> Result<ApprovalView, ApiFailure> {
    let base = hitl_view(r)?;
    Ok(ApprovalView {
        id: base.id,
        r#type: r.kind.clone(),
        risk_level: risk_level(r.amount),
        summary: format!("Release {} Đ", r.amount),
        note: r.note.clone(),
        amount: r.amount,
        status: r.status.to_lowercase(),
        created_at: IsoDateTime::new(r.created_at.clone())
            .ok_or_else(|| http_failure_corrupt("HITL createdAt is not RFC3339"))?,
        expires_at: IsoDateTime::new(r.expires_at.clone()),
    })
}

/// کلمهٔ وضعیت داشبورد — واژگان فیلتر سفارش‌ها (pending/paid/delivered/refund
/// ed/cancelled)؛ وضعیت‌های بیرون فیلتر lowercase می‌شوند.
#[must_use]
pub fn order_status_word(status: dropagentx_domain::EscrowStatus) -> String {
    match status {
        dropagentx_domain::EscrowStatus::Created => "pending".into(),
        dropagentx_domain::EscrowStatus::Funded => "paid".into(),
        dropagentx_domain::EscrowStatus::Delivered => "delivered".into(),
        dropagentx_domain::EscrowStatus::Refunded => "refunded".into(),
        dropagentx_domain::EscrowStatus::Cancelled => "cancelled".into(),
        other => status_lowercase(other),
    }
}

/// رشتهٔ وضعیت وایر به‌صورت lowercase (`FUNDED` → `"funded"`) — برای نماهای UI.
#[must_use]
fn status_lowercase(status: dropagentx_domain::EscrowStatus) -> String {
    let wire = serde_json::to_string(&escrow_status_wire(status))
        .unwrap_or_else(|_| format!("{:?}", status));
    wire.trim_matches('"').to_lowercase()
}

/// نمای سفارش داشبورد از امانت — وضعیت lowercase، Stars صفر تا رسیدن ریل.
#[must_use]
pub fn order_view(e: &Escrow, product_title: Option<String>) -> OrderView {
    OrderView {
        id: e.id.clone(),
        product: product_title.unwrap_or_else(|| e.product_ref.clone().unwrap_or_default()),
        buyer: DashPerson { name: None, username: None },
        total_stars: 0,
        total_credits: e.amount.as_i64(),
        status: order_status_word(e.status),
        created_at: iso_or_epoch(&e.created_at),
    }
}

/// نمای سفارش اخیر برای overview — `title` nullable (ارجاع آزاد).
#[must_use]
pub fn recent_order_view(
    e: &Escrow,
    product_title: Option<String>,
) -> dropagentx_contracts::RecentOrder {
    dropagentx_contracts::RecentOrder {
        id: e.id.clone(),
        title: product_title.or_else(|| e.product_ref.clone()),
        buyer: DashPerson { name: None, username: None },
        total_stars: 0,
        total_credits: e.amount.as_i64(),
        status: order_status_word(e.status),
        created_at: iso_or_epoch(&e.created_at),
    }
}

/// نگاشت فیلتر داشبورد (`pending/paid/…`) → وضعیت امانت — `None` یعنی همه.
#[must_use]
pub fn order_status_filter(filter: &str) -> Option<dropagentx_domain::EscrowStatus> {
    match filter {
        "pending" => Some(dropagentx_domain::EscrowStatus::Created),
        "paid" => Some(dropagentx_domain::EscrowStatus::Funded),
        "delivered" => Some(dropagentx_domain::EscrowStatus::Delivered),
        "refunded" => Some(dropagentx_domain::EscrowStatus::Refunded),
        "cancelled" => Some(dropagentx_domain::EscrowStatus::Cancelled),
        _ => None,
    }
}

/// نمای کاربر داشبورد از حساب — نام از label (fallback: شناسه).
#[must_use]
pub fn dash_user(a: &AccountBalance) -> DashUser {
    DashUser {
        id: a.id.clone(),
        name: a.label.clone().unwrap_or_else(|| a.id.clone()),
        username: None,
        role: "user".into(),
        language: None,
        created_at: iso_or_epoch(&a.created_at),
    }
}

/// نمای کیف داشبورد — تراز مشتق + قفل اسکرو + credits (همان دیین).
#[must_use]
pub fn dash_wallet(a: &AccountBalance, locked: i64) -> DashWallet {
    DashWallet {
        name: a.label.clone().unwrap_or_else(|| a.id.clone()),
        username: None,
        role: "user".into(),
        balance: a.balance,
        locked,
        credits: a.balance,
    }
}

/// ردیف لیدربورد — `sales` واقعی، ریل‌های xp/ref تا رسیدن موتور کویست صفر.
#[must_use]
pub fn leaderboard_entry(rank: i64, name: String, sales: i64) -> LeaderboardEntry {
    LeaderboardEntry { rank, name, username: None, xp: 0, sales, refs: 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dropagentx_domain::EscrowStatus as D;

    fn escrow(status: D, amount: i64) -> Escrow {
        Escrow {
            id: "escrow:e1".into(),
            buyer_id: "user:b".into(),
            seller_id: "user:s".into(),
            amount: dropagentx_domain::Din::new(amount).unwrap(),
            product_ref: Some("prod:1".into()),
            status,
            created_at: "2026-02-01T00:00:00.000Z".into(),
            funded_tx_id: None,
            settle_tx_id: None,
            dispute_reason: None,
            delivery_ref: None,
            expires_at: None,
        }
    }

    #[test]
    fn escrow_view_keeps_nulls_explicit_and_status_screaming() {
        let v = escrow_view(&escrow(D::Created, 400));
        assert_eq!(v.status, EscrowStatus::Created);
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains(r#""status":"CREATED""#));
        assert!(json.contains(r#""disputeReason":null"#));
    }

    #[test]
    fn order_view_maps_dashboard_words_and_credits() {
        let v = order_view(&escrow(D::Funded, 60_000), Some("VIP".into()));
        assert_eq!(v.status, "paid", "FUNDED == paid in dashboard vocabulary");
        assert_eq!(v.total_credits, 60_000);
        assert_eq!(v.total_stars, 0);
        assert_eq!(v.product, "VIP");
        let released = order_view(&escrow(D::Released, 1), None);
        assert_eq!(released.status, "released", "outside filter vocabulary → lowercase wire word");
    }

    #[test]
    fn order_status_filter_maps_dashboard_words() {
        assert_eq!(order_status_filter("paid"), Some(D::Funded));
        assert_eq!(order_status_filter("pending"), Some(D::Created));
        assert_eq!(order_status_filter("all"), None);
        assert_eq!(order_status_filter("nonsense"), None);
    }

    #[test]
    fn approval_view_risk_derives_from_charter_threshold() {
        let mut row = HitlRow {
            id: "hitl:h1".into(),
            kind: "ESCROW_RELEASE".into(),
            escrow_id: "escrow:e1".into(),
            amount: 60_000,
            requester_id: "user:tg:1".into(),
            note: None,
            status: "PENDING".into(),
            created_at: "2026-02-01T00:00:00.000Z".into(),
            expires_at: "2026-02-02T00:00:00.000Z".into(),
            decided_by: None,
            decided_at: None,
            decision_note: None,
        };
        let v = approval_view(&row).unwrap();
        assert_eq!(v.risk_level, RiskLevel::High);
        assert_eq!(v.summary, "Release 60000 Đ");
        assert_eq!(v.status, "pending");
        row.amount = 100;
        let v = approval_view(&row).unwrap();
        assert_eq!(v.risk_level, RiskLevel::Low);
    }

    #[test]
    fn hitl_view_rejects_corrupt_kind_as_internal() {
        let row = HitlRow {
            kind: "WAT".into(),
            id: "h".into(),
            escrow_id: "e".into(),
            amount: 1,
            requester_id: "r".into(),
            note: None,
            status: "PENDING".into(),
            created_at: "t".into(),
            expires_at: "t".into(),
            decided_by: None,
            decided_at: None,
            decision_note: None,
        };
        assert_eq!(hitl_view(&row).unwrap_err().status(), 500);
    }

    #[test]
    fn product_view_maps_active_to_status() {
        let p = ProductRow {
            id: "prod:1".into(),
            seller_id: "treasury:nation".into(),
            title: "VIP".into(),
            description: "d".into(),
            category: None,
            price_din: 1_500,
            delivery_payload: None,
            active: true,
            created_at: "2026-02-01T00:00:00.000Z".into(),
        };
        let v = product_view(&p, 7);
        assert_eq!(v.status, ProductStatus::Active);
        assert_eq!(v.category, "");
        assert_eq!(v.stock, -1);
        assert_eq!(v.price_credits, 1_500);
        assert_eq!(v.price_stars, 0);
        assert_eq!(v.sales_count, 7);
        assert!(!v.featured);
    }

    #[test]
    fn dash_wallet_carries_balance_locked_credits() {
        let a = AccountBalance {
            id: "user:tg:1".into(),
            kind: "user".into(),
            label: Some("علی".into()),
            balance: 900,
            created_at: "t".into(),
        };
        let w = dash_wallet(&a, 100);
        assert_eq!(w.balance, 900);
        assert_eq!(w.locked, 100);
        assert_eq!(w.credits, 900);
        assert_eq!(w.name, "علی");
    }
}
