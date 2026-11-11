//! قراردادهای صف HITL (انسان-در-حلقه) — مادهٔ ۷ منشور — پورت `hitl.ts`.
//!
//! مبالغ ≥ ۵۰٬۰۰۰ Đ باید تأیید انسانی بگیرند. صف روی سرور است (منبع واحد
//! مشترک بات و داشبورد)؛ بات درخواست می‌سازد و ادمین تصمیم می‌گیرد. تأییدِ
//! `ESCROW_RELEASE` تسویه را «در همان تراکنش» اجرا می‌کند و کلید یکسان‌سازی
//! تسویه دترمینیستیک است (`hitl:<id>:approve`) تا retry هرگز دوباره نپردازد.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::{Din, Id};
use crate::escrows::EscrowView;

/// انواع درخواست HITL — فعلاً فقط تسویهٔ امانت.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HitlKind {
    /// تسویهٔ امانت
    EscrowRelease,
}

/// وضعیت درخواست HITL.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HitlStatus {
    /// در انتظار
    Pending,
    /// تأییدشده
    Approved,
    /// ردشده
    Rejected,
    /// منقضی‌شده
    Expired,
}

/// `createHitlRequestSchema` — id دترمینیستیک ساختِ replay-safe می‌دهد.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateHitlRequest {
    /// شناسهٔ اختیاری — تکراری → همان درخواست موجود (replay)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
    /// نوع
    pub kind: HitlKind,
    /// امانت مربوط
    pub escrow_id: Id,
    /// حساب درخواست‌دهنده (مثل `user:tg:123`) — برای ردیف audit
    pub requester_id: Id,
    /// متن آزاد برای تصمیم‌گیرنده (یوزرنیم، محصول، دلیل — ≤۵۰۰)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_length = 500)]
    pub note: Option<String>,
    /// TTL صف به ساعت (پیش‌فرض سرور ۲۴؛ PENDING سررسیدگذشته → EXPIRED)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(minimum = 1_i64, maximum = 720_i64)]
    pub ttl_hours: Option<i64>,
}

/// `hitlDecisionRequestSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HitlDecisionRequest {
    /// حساب تصمیم‌گیرنده (ردیف audit)
    pub decided_by: Id,
    /// یادداشت اختیاری (≤۵۰۰)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_length = 500)]
    pub note: Option<String>,
}

/// نمای درخواست HITL — `hitlViewSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HitlView {
    /// شناسه
    pub id: String,
    /// نوع
    pub kind: HitlKind,
    /// امانت مربوط
    pub escrow_id: String,
    /// مبلغ
    pub amount: Din,
    /// درخواست‌دهنده
    pub requester_id: String,
    /// یادداشت (nullable — صریح روی سیم)
    #[schema(required = true)]
    pub note: Option<String>,
    /// وضعیت
    pub status: HitlStatus,
    /// زمان ایجاد
    pub created_at: String,
    /// زمان انقضا
    pub expires_at: String,
    /// تصمیم‌گیرنده (nullable)
    #[schema(required = true)]
    pub decided_by: Option<String>,
    /// زمان تصمیم (nullable)
    #[schema(required = true)]
    pub decided_at: Option<String>,
    /// یادداشت تصمیم (nullable)
    #[schema(required = true)]
    pub decision_note: Option<String>,
}

/// `hitlListResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HitlListResponse {
    /// ردیف‌ها
    pub items: Vec<HitlView>,
}

/// `hitlSweepResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HitlSweepResponse {
    /// تعداد منقضی‌شده‌ها در این sweep
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub expired: i64,
}

/// `hitlActionResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HitlActionResponse {
    /// درخواست پس از تصمیم
    pub request: HitlView,
    /// نتیجهٔ تسویه — فقط برای APPROVED روی ESCROW_RELEASE؛ وگرنه null (صریح)
    #[schema(required = true)]
    pub escrow: Option<EscrowView>,
    /// شناسهٔ تراکنش (nullable — صریح)
    #[schema(required = true)]
    pub tx_id: Option<String>,
    /// آیا replay بود؟
    pub replayed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn hitl_view_null_fields_stay_explicit() {
        let json = r#"{"id":"hitl:01","kind":"ESCROW_RELEASE","escrowId":"escrow:01","amount":60000,"requesterId":"user:tg:1","note":null,"status":"PENDING","createdAt":"2026-02-01T00:00:00.000Z","expiresAt":"2026-02-02T00:00:00.000Z","decidedBy":null,"decidedAt":null,"decisionNote":null}"#;
        let v: HitlView = from_str(json).unwrap();
        assert_eq!(v.kind, HitlKind::EscrowRelease);
        assert_eq!(v.status, HitlStatus::Pending);
        assert_eq!(to_string(&v).unwrap(), json);
    }

    #[test]
    fn hitl_action_with_null_escrow_roundtrips() {
        let json = r#"{"request":{"id":"h","kind":"ESCROW_RELEASE","escrowId":"e","amount":1,"requesterId":"r","note":null,"status":"REJECTED","createdAt":"t","expiresAt":"t2","decidedBy":"admin","decidedAt":"d","decisionNote":"no"},"escrow":null,"txId":null,"replayed":false}"#;
        let v: HitlActionResponse = from_str(json).unwrap();
        assert_eq!(v.request.status, HitlStatus::Rejected);
        assert_eq!(v.escrow, None);
        assert!(to_string(&v).unwrap().contains(r#""escrow":null"#));
    }
}
