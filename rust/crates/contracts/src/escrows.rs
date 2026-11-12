//! قراردادهای امانت — ستون فقرات مارکت‌پلیس (ADR-004، مادهٔ ۸ منشور) — پورت `escrows.ts`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::{Din, EscrowStatus, Id, IdempotencyKey};

/// `createEscrowRequestSchema` — id غایب را سرور می‌سازد؛ تکراری رد می‌شود.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateEscrowRequest {
    /// شناسهٔ اختیاری (سرور می‌سازد؛ تکراری → خطا)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
    /// خریدار
    pub buyer_id: Id,
    /// فروشنده
    pub seller_id: Id,
    /// مبلغ دیین
    pub amount: Din,
    /// ارجاع محصول اختیاری (≤۱۲۸)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_length = 128)]
    pub product_ref: Option<String>,
    /// TTL ساعت (۱ تا ۷۲۰)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(minimum = 1_i64, maximum = 720_i64)]
    pub ttl_hours: Option<i64>,
    /// کلید یکسان‌سازی اختیاری
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
}

/// نمای امانت — `escrowViewSchema` (همهٔ فیلدهای nullable صریح روی سیم).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EscrowView {
    /// شناسه
    pub id: String,
    /// خریدار
    pub buyer_id: String,
    /// فروشنده
    pub seller_id: String,
    /// مبلغ
    pub amount: Din,
    /// ارجاع محصول (nullable — صریح روی سیم)
    #[schema(required = true)]
    pub product_ref: Option<String>,
    /// وضعیت
    pub status: EscrowStatus,
    /// زمان ایجاد
    pub created_at: String,
    /// تراکنش تأمین (nullable)
    #[schema(required = true)]
    pub funded_tx_id: Option<String>,
    /// تراکنش تسویه (nullable)
    #[schema(required = true)]
    pub settle_tx_id: Option<String>,
    /// دلیل اختلاف (nullable)
    #[schema(required = true)]
    pub dispute_reason: Option<String>,
    /// ارجاع تحویل (nullable)
    #[schema(required = true)]
    pub delivery_ref: Option<String>,
    /// زمان انقضا (nullable)
    #[schema(required = true)]
    pub expires_at: Option<String>,
}

/// `escrowListResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EscrowListResponse {
    /// ردیف‌ها
    pub items: Vec<EscrowView>,
}

/// `fundEscrowRequestSchema`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FundEscrowRequest {
    /// کلید یکسان‌سازی اختیاری
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
}

/// `deliverEscrowRequestSchema`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeliverEscrowRequest {
    /// ارجاع تحویل اختیاری (≤۲۵۶)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_length = 256)]
    pub delivery_ref: Option<String>,
    /// کلید یکسان‌سازی اختیاری
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
}

/// `releaseEscrowRequestSchema` — مادهٔ ۷: برای مبالغ ≥ ۵۰٬۰۰۰ Đ باید
/// `approvedByAdmin: true` **صریح** باشد؛ هرگز ضمنی.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseEscrowRequest {
    /// تأیید صریح ادمین (مادهٔ ۷)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_by_admin: Option<bool>,
    /// کلید یکسان‌سازی اختیاری
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
}

/// `disputeEscrowRequestSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DisputeEscrowRequest {
    /// دلیل (۱ تا ۵۰۰ کاراکتر)
    #[schema(min_length = 1, max_length = 500)]
    pub reason: String,
    /// کلید یکسان‌سازی اختیاری
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
}

/// `refundEscrowRequestSchema`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefundEscrowRequest {
    /// کلید یکسان‌سازی اختیاری
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
}

/// `cancelEscrowRequestSchema`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelEscrowRequest {
    /// کلید یکسان‌سازی اختیاری
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
}

/// `expireEscrowRequestSchema`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExpireEscrowRequest {
    /// کلید یکسان‌سازی اختیاری
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
}

/// `escrowActionResponseSchema` — پاسخ همهٔ اقدامات امانت.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EscrowActionResponse {
    /// نمای امانت پس از عملیات
    pub escrow: EscrowView,
    /// شناسهٔ تراکنش (nullable — لغو/ساخت پول جابه‌جا نمی‌کند)
    #[schema(required = true)]
    pub tx_id: Option<String>,
    /// آیا replay بود؟
    pub replayed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    fn sample_view() -> EscrowView {
        EscrowView {
            id: "escrow:01".into(),
            buyer_id: "user:tg:1".into(),
            seller_id: "user:tg:2".into(),
            amount: Din::new(60_000).unwrap(),
            product_ref: None,
            status: EscrowStatus::Released,
            created_at: "2026-02-01T00:00:00.000Z".into(),
            funded_tx_id: Some("tx:02".into()),
            settle_tx_id: Some("tx:03".into()),
            dispute_reason: None,
            delivery_ref: Some("ref-9".into()),
            expires_at: None,
        }
    }

    #[test]
    fn escrow_view_null_fields_stay_explicit() {
        let json = to_string(&sample_view()).unwrap();
        assert!(json.contains(r#""productRef":null"#));
        assert!(json.contains(r#""disputeReason":null"#));
        assert!(json.contains(r#""expiresAt":null"#));
        assert!(json.contains(r#""status":"RELEASED""#));
        let back: EscrowView = from_str(&json).unwrap();
        assert_eq!(back, sample_view());
    }

    #[test]
    fn empty_request_bodies_serialize_to_empty_object() {
        assert_eq!(to_string(&FundEscrowRequest::default()).unwrap(), "{}");
        assert_eq!(to_string(&CancelEscrowRequest::default()).unwrap(), "{}");
    }

    #[test]
    fn release_request_carries_explicit_admin_approval() {
        let json = r#"{"approvedByAdmin":true}"#;
        let req: ReleaseEscrowRequest = from_str(json).unwrap();
        assert_eq!(req.approved_by_admin, Some(true));
        assert_eq!(to_string(&req).unwrap(), json);
    }

    #[test]
    fn escrow_action_response_roundtrip() {
        let resp = EscrowActionResponse {
            escrow: sample_view(),
            tx_id: Some("tx:03".into()),
            replayed: false,
        };
        let json = to_string(&resp).unwrap();
        assert!(json.contains(r#""txId":"tx:03""#));
        assert!(json.contains(r#""replayed":false"#));
    }
}
