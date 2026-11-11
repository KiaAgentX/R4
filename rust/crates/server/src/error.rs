//! پاکت خطای HTTP — پل بین خطاهای لایه‌ها و بدنهٔ وایر `{error:{code,message,meta?}}`.
//!
//! نگاشت honest (پورت `errors.ts` + کدهای صریح `api.ts`/`guards.ts`):
//! * [`HttpFailure`] (فاز ۳) — وضعیت و کد صریح دارد.
//! * [`DomainFailure`] — کد دامنه → 404/422/500 با `status_for_domain_code`.
//! * [`DbError`] — SQL/corrupt → 500 `INTERNAL`؛ ماندگاریِ ناتمام هرگز وضعیت را
//!   خراب نمی‌کند و کلاینت پیام صادقانه می‌گیرد.
//!
//! پاسخ 404 مسیرهای ناشناخته هم پاکت است (additive-only) — SPA و بات هرگز
//! بدنهٔ بی‌شکل نمی‌گیرند.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use dropagentx_contracts::{DomainFailure, ErrorEnvelope, HttpFailure};
use dropagentx_db::DbError;

/// خطای آمادهٔ HTTP — هر variant بدنهٔ وایر دقیق می‌دهد.
#[derive(Debug)]
pub enum ApiFailure {
    /// خطای صریح سرور (`HttpFailure` فاز ۳).
    Http(HttpFailure),
    /// خطای دامنهٔ خالص — نگاشت ۴۰۴/۴۲۲/۵۰۰ در لایهٔ قرارداد.
    Domain(DomainFailure),
    /// خطای زیرساخت ماندگاری.
    Db(DbError),
}

impl From<HttpFailure> for ApiFailure {
    fn from(f: HttpFailure) -> Self {
        Self::Http(f)
    }
}

impl From<DomainFailure> for ApiFailure {
    fn from(f: DomainFailure) -> Self {
        Self::Domain(f)
    }
}

impl From<dropagentx_domain::DomainError> for ApiFailure {
    fn from(e: dropagentx_domain::DomainError) -> Self {
        Self::Domain(DomainFailure::new(e))
    }
}

impl From<DbError> for ApiFailure {
    fn from(e: DbError) -> Self {
        match e {
            // خطای دامنهٔ خالص از داخل تراکنش — نگاشت صادقانهٔ ۴۰۴/۴۲۲ حفظ می‌شود
            DbError::Domain(domain_error) => Self::Domain(DomainFailure::new(domain_error)),
            other => Self::Db(other),
        }
    }
}

impl ApiFailure {
    /// وضعیت صادقانه.
    #[must_use]
    pub fn status(&self) -> u16 {
        match self {
            Self::Http(f) => f.status,
            Self::Domain(f) => f.status(),
            Self::Db(_) => 500,
        }
    }

    /// بدنهٔ وایر — عین شکل `{error:{…}}`.
    #[must_use]
    pub fn envelope(&self) -> ErrorEnvelope {
        match self {
            Self::Http(f) => f.to_envelope(),
            Self::Domain(f) => f.to_envelope(),
            Self::Db(e) => ErrorEnvelope {
                error: dropagentx_contracts::ApiError::new(
                    "INTERNAL",
                    format!("Persistence layer failed — {e}"),
                ),
            },
        }
    }
}

impl std::fmt::Display for ApiFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let env = self.envelope();
        write!(f, "{} — {}: {}", self.status(), env.error.code, env.error.message)
    }
}

impl IntoResponse for ApiFailure {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = self.envelope();
        // لاگ ساختاریافته — ردیف audit دیتابیس مسئول تاریخچه است؛ اینجا فقط trace
        let (code, message) = (&body.error.code, &body.error.message);
        tracing::warn!(status = self.status(), code = %code, message = %message, "api failure");
        (
            status,
            [(header::CONTENT_TYPE, "application/json")],
            serde_json::to_string(&body).unwrap_or_default(),
        )
            .into_response()
    }
}

/// Result استاندارد هندلرها.
pub type ApiResult<T> = Result<T, ApiFailure>;

#[cfg(test)]
mod tests {
    use super::*;
    use dropagentx_contracts::HttpFailure;
    use dropagentx_db::DbError;
    use dropagentx_domain::{DomainError, DomainErrorCode};

    #[test]
    fn http_failure_keeps_explicit_status() {
        let f = ApiFailure::Http(HttpFailure::admin_disabled());
        assert_eq!(f.status(), 503);
        assert_eq!(f.envelope().error.code, "ADMIN_DISABLED");
    }

    #[test]
    fn domain_failure_maps_honest_status() {
        let f = ApiFailure::from(DomainError::new(DomainErrorCode::EscrowNotFound, "x"));
        assert_eq!(f.status(), 404);
        let f = ApiFailure::from(DomainError::new(DomainErrorCode::AccountOverdraft, "x"));
        assert_eq!(f.status(), 422);
    }

    #[test]
    fn db_failure_is_internal_500() {
        let f = ApiFailure::Db(DbError::Corrupt("bad row".into()));
        assert_eq!(f.status(), 500);
        assert_eq!(f.envelope().error.code, "INTERNAL");
    }
}
