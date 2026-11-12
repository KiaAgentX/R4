//! هندلرهای HTTP — هر هندلر سه گام دارد: امنیت (گارد ادمین/نرخ از middleware
//! و [`require_admin`])، اعتبارسنجی بدنهٔ وایر ([`parse_body`] — مثل zod در
//! لبهٔ HTTP)، و اجرای دامنه از مسیر خالص + [`V3Store`].
//!
//! هیچ هندلری مستقیم JSON نمی‌سازد — پاسخ‌ها از DTOهای فاز ۳ ساخته می‌شوند تا
//! قرارداد هرگز از هندلر فاصله نگیرد (ADR-005).

pub mod accounts;
pub mod ai;
pub mod bot;
pub mod dashboard;
pub mod escrows;
pub mod hitl;
pub mod system;
pub mod wallet;

use serde::de::DeserializeOwned;

use crate::error::ApiFailure;

/// اعتبارسنجی بدنهٔ وایر — همان نقطهٔ zod در TS: بدنهٔ نامعتبر → 400
/// `INVALID_REQUEST` با شمارش issue (serde یک خطای ریشه‌ای می‌دهد — حداقلِ
/// صادقانه ۱). بدنهٔ خالی برای فرم‌های all-optional مثل `{}` تفسیر می‌شود.
///
/// # Errors
/// [`HttpFailure::invalid_request`] — وضعیت ۴۰۰ با `meta.issues`.
pub fn parse_body<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ApiFailure> {
    let raw: &[u8] = if bytes.iter().all(|b| b.is_ascii_whitespace()) { b"{}" } else { bytes };
    serde_json::from_slice(raw)
        .map_err(|_| dropagentx_contracts::HttpFailure::invalid_request(1).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dropagentx_contracts::FundEscrowRequest;

    #[test]
    fn empty_body_becomes_default_object_for_optional_forms() {
        let req: FundEscrowRequest = parse_body(b"").unwrap();
        assert_eq!(req, FundEscrowRequest::default());
    }

    #[test]
    fn invalid_body_is_invalid_request_400() {
        let err = parse_body::<FundEscrowRequest>(b"{\"idempotencyKey\":\"short\"}").unwrap_err();
        assert_eq!(err.status(), 400);
        assert_eq!(err.envelope().error.code, "INVALID_REQUEST");
    }
}
