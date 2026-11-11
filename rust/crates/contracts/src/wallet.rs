//! قراردادهای کیف پول — ضرب و سوزاندن (ماده‌های ۳ و ۴ منشور) — پورت `wallet.ts`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::{Din, Id, IdempotencyKey};

/// درخواست ضرب — `mintRequestSchema` (سقف سالانه در فاز ۴ اعمال می‌شود).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MintRequest {
    /// حساب مقصد
    pub account_id: Id,
    /// مبلغ دیین
    pub amount: Din,
    /// یادداشت اختیاری (≤۲۰۰)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_length = 200)]
    pub memo: Option<String>,
    /// کلید یکسان‌سازی اختیاری
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
}

/// درخواست سوزاندن — `burnRequestSchema` (overdraft ناممکن است).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BurnRequest {
    /// حساب مبدأ
    pub account_id: Id,
    /// مبلغ دیین
    pub amount: Din,
    /// یادداشت اختیاری (≤۲۰۰)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_length = 200)]
    pub memo: Option<String>,
    /// کلید یکسان‌سازی اختیاری
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
}

/// پاسخ جابه‌جایی کیف پول — `walletMoveResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WalletMoveResponse {
    /// شناسهٔ تراکنش
    pub tx_id: String,
    /// حساب
    pub account_id: String,
    /// موجودی پس از عملیات
    #[schema(minimum = -9_007_199_254_740_991_i64, maximum = 9_007_199_254_740_991_i64)]
    pub balance: i64,
    /// آیا این پاسخ replay بود؟
    pub replayed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn mint_request_wire_shape_matches_ts() {
        let json = r#"{"accountId":"user:ali","amount":500,"memo":"welcome","idempotencyKey":"mint-ali-1"}"#;
        let req: MintRequest = from_str(json).unwrap();
        assert_eq!(req.amount, Din::new(500).unwrap());
        assert_eq!(
            req.idempotency_key.as_ref().map(ToString::to_string),
            Some("mint-ali-1".into())
        );
        assert_eq!(to_string(&req).unwrap(), json);
    }

    #[test]
    fn burn_request_omits_optionals() {
        let req = BurnRequest {
            account_id: Id::new("user:x").unwrap(),
            amount: Din::new(250).unwrap(),
            memo: None,
            idempotency_key: None,
        };
        assert_eq!(to_string(&req).unwrap(), r#"{"accountId":"user:x","amount":250}"#);
    }
}
