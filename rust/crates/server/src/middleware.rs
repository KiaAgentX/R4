//! امنیت چندلایه روی سیم — پورت `guards.ts` به‌صورت middleware آکسوم.
//!
//! دو لایه، به همان ترتیب TS:
//! ۱. **نرخ** — پنجرهٔ لغزان ۶۰ثانیه‌ای؛ تماس‌گیرندهٔ دارای کلید ادمین معتبر باکت
//!    مشترک `admin` می‌گیرد، بقیه با کلید ip (X-Forwarded-For اول، سپس
//!    ConnectInfo، وگرنه `local`). بدنهٔ ۴۲۹ عین فاز ۳ است (`meta.limit`).
//! ۲. **ادمین** — fail-closed: کلید سرور تنظیم‌نشده → 503 `ADMIN_DISABLED`؛
//!    هدر غایب → 401 `ADMIN_KEY_REQUIRED`؛ نادرست → 401 `INVALID_ADMIN_KEY`.
//!    مقایسهٔ زمان‌ثابت از `contracts::security` (subtle).
//!
//! هویت عمومی AI (`initData`) در هندلر `/ai/chat` بررسی می‌شود — همین‌جا فقط
//! دو لایهٔ عمومی هر مسیر اعمال می‌شوند.

use axum::extract::{ConnectInfo, Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use std::net::SocketAddr;
use std::sync::Arc;

use dropagentx_contracts::{verify_kia_key, HttpFailure};

use crate::error::ApiFailure;
use crate::state::AppState;

/// کلید باکت ادمین — همهٔ تماس‌های معتمد یک باکت مشترک دارند (مثل TS).
const ADMIN_BUCKET: &str = "admin";

/// گارد ادمین — از هندلرها صدا زده می‌شود (هر مسیر `Security::Admin`).
///
/// # Errors
/// سه خطای صریح فاز ۳: `ADMIN_DISABLED`/`ADMIN_KEY_REQUIRED`/`INVALID_ADMIN_KEY`.
pub fn require_admin(headers: &HeaderMap, admin_key: Option<&str>) -> Result<(), ApiFailure> {
    let header_value = headers.get("x-kia-key").and_then(|v| v.to_str().ok());
    if admin_key.is_none() {
        return Err(HttpFailure::admin_disabled().into());
    }
    if header_value.is_none() {
        return Err(HttpFailure::admin_key_required().into());
    }
    if !verify_kia_key(header_value, admin_key) {
        return Err(HttpFailure::invalid_admin_key().into());
    }
    Ok(())
}

/// کلید باکت برای این درخواست — ادمین معتبر → باکت مشترک؛ بقیه → ip.
fn bucket_key(
    headers: &HeaderMap,
    admin_key: Option<&str>,
    connect_info: Option<SocketAddr>,
) -> String {
    let header_value = headers.get("x-kia-key").and_then(|v| v.to_str().ok());
    if verify_kia_key(header_value, admin_key) {
        return ADMIN_BUCKET.to_owned();
    }
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        // اولین ip زنجیره = کلاینت واقعی (پشت proxy)
        if let Some(first) = xff.split(',').next() {
            let trimmed = first.trim();
            if !trimmed.is_empty() {
                return format!("ip:{trimmed}");
            }
        }
    }
    if let Some(addr) = connect_info {
        return format!("ip:{}", addr.ip());
    }
    // oneshot تست‌ها و فراخوانی درون‌پردازشی — بدون ip
    "local".to_owned()
}

/// middleware نرخ — قبل از همهٔ مسیرهای `/api/v1` اعمال می‌شود.
///
/// # Errors
/// ۴۲۹ با بدنهٔ دقیق فاز ۳ (`RATE_LIMITED` + `meta.limit`).
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, ApiFailure> {
    let connect_info = request.extensions().get::<ConnectInfo<SocketAddr>>().map(|ci| ci.0);
    let key = bucket_key(request.headers(), state.config.admin_key.as_deref(), connect_info);
    let decision = {
        // قفل مسموم فقط پس از panic هندلر ممکن می‌شود — وضعیت limiter بحرانی
        // نیست؛ بازیابی امن است (همان معناشناسی best-effort TS).
        let mut limiter = state.limiter.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        limiter.check(&key)
    };
    if let Err(failure) = decision {
        tracing::warn!(bucket = %key, "rate limited");
        return Err(failure.into());
    }
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    const KEY: &str = "test-admin-key-0123456789";

    #[test]
    fn admin_guard_fail_closed_matrix() {
        let mut headers = HeaderMap::new();
        // سرور بدون کلید → 503 همیشه (هرگز open)
        assert_eq!(require_admin(&headers, None).unwrap_err().status(), 503);
        // هدر غایب → 401
        assert_eq!(require_admin(&headers, Some(KEY)).unwrap_err().status(), 401);
        // هدر نادرست → 401
        headers.insert("x-kia-key", "wrong-key-0123456789".parse().unwrap());
        assert_eq!(require_admin(&headers, Some(KEY)).unwrap_err().status(), 401);
        // هدر درست → عبور
        headers.insert("x-kia-key", KEY.parse().unwrap());
        assert!(require_admin(&headers, Some(KEY)).is_ok());
    }

    #[test]
    fn bucket_key_prefers_admin_then_xff_then_connect_info() {
        let mut headers = HeaderMap::new();
        assert_eq!(bucket_key(&headers, Some(KEY), None), "local");
        headers.insert("x-kia-key", KEY.parse().unwrap());
        assert_eq!(bucket_key(&headers, Some(KEY), None), "admin");
        headers.remove("x-kia-key");
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(bucket_key(&headers, Some(KEY), None), "ip:1.2.3.4");
        assert_eq!(
            bucket_key(&headers, Some(KEY), Some("9.9.9.9:80".parse().unwrap())),
            "ip:1.2.3.4"
        );
    }
}
