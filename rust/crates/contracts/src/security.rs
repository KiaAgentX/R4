//! اولیه‌های احراز (خالص، بدون وبرام) — پورت `server/src/auth.ts` + `guards.ts`.
//!
//! قواعد fail-closed:
//! * بررسی کلید ادمین با مقایسهٔ زمان‌ثابت (`subtle`)؛ کلیدِ تنظیم‌نشدهٔ سرور =
//!   ادمین **غیرفعال**، هرگز باز.
//! * Telegram initData طبق طرح رسمی WebApp HMAC-SHA256.
//! * rate-limit پنجرهٔ لغزان درون‌حافظه‌ای — همان معناشناسی TS (توزیع فاز ۴
//!   به‌عنوان middleware آکسوم، با همان بدنهٔ 429).

use std::collections::HashMap;
use std::time::Instant;

use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

// ── X-KIA-Key ───────────────────────────────────────────────────────────────

/// بررسی کلید ادمین — مقایسهٔ زمان‌ثابت.
///
/// * کلید سرور `None` یا کوتاه‌تر از ۱۶ کاراکتر → `false` (ادمین غیرفعال).
/// * هدر غایب/خالی → `false`.
/// * طول‌های متفاوت → `false` (بدون مقایسهٔ بایت‌به‌بایت — نشت زمانی ندارد).
#[must_use]
pub fn verify_kia_key(header_value: Option<&str>, server_key: Option<&str>) -> bool {
    let Some(server_key) = server_key else { return false };
    if server_key.len() < 16 {
        return false;
    }
    let Some(header) = header_value else { return false };
    if header.is_empty() {
        return false;
    }
    let a = header.as_bytes();
    let b = server_key.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

// ── Telegram initData (طرح رسمی WebApp) ─────────────────────────────────────

/// هویت مشتق‌شده از initData.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramIdentity {
    /// شناسهٔ عددی تلگرام به‌صورت رشته (`"123"`)
    pub user_id: String,
    /// یوزرنیم اختیاری
    pub username: Option<String>,
}

/// رکورد کاربر داخل initData (`user` = JSON کدشده در فرم).
#[derive(Debug, Deserialize)]
struct RawTelegramUser {
    id: Option<f64>,
    username: Option<String>,
}

/// اعتبارسنجی رسمی Telegram WebApp initData (HMAC-SHA256):
///
/// 1. `hash` را از پارامترها جدا کن.
/// 2. بقیهٔ پارامترها را sort کن و با `\n` بچسبان.
/// 3. `secret = SHA256(bot_token)`، `computed = HMAC-SHA256(secret, data_check_string)`.
/// 4. مقایسهٔ زمان‌ثابت با `hash` (hex).
///
/// هر گام شکست → `None` (fail-closed).
#[must_use]
pub fn verify_telegram_init_data(init_data: &str, bot_token: &str) -> Option<TelegramIdentity> {
    let mut params: Vec<(String, String)> = parse_urlencoded(init_data);
    let hash_pos = params.iter().position(|(k, _)| k == "hash")?;
    let hash = params.remove(hash_pos).1;

    params.sort_by(|a, b| a.0.cmp(&b.0));
    let data_check_string = params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut secret = Sha256::digest(bot_token.as_bytes());
    // SHA256 همیشه ۳۲ بایت است — کلید معتبر HMAC؛ با فراخوانی داخلی امن
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(&secret) else {
        return None;
    };
    mac.update(data_check_string.as_bytes());
    let computed = mac.finalize().into_bytes();
    secret.fill(0); // پاک‌سازی کلید مشتق‌شده از حافظه

    // مقایسهٔ زمان‌ثابت روی بایت‌های hex‌گشایی‌شده
    let expected = hex::decode(&hash).ok()?;
    if expected.len() != computed.len() {
        return None;
    }
    if expected.ct_eq(computed.as_slice()).unwrap_u8() != 1 {
        return None;
    }

    // کاربر: JSON داخل پارامتر user — id باید صحیح امن باشد
    let user_raw = params.iter().find(|(k, _)| k == "user").map(|(_, v)| v.clone())?;
    let user: RawTelegramUser = serde_json::from_str(&user_raw).ok()?;
    let id = user.id?;
    if !id.is_finite() || id < 0.0 || id.fract() != 0.0 || id > 9_007_199_254_740_991.0 {
        return None;
    }
    let username = user
        .username
        .filter(|u| !u.is_empty());
    Some(TelegramIdentity { user_id: format!("{}", id as i64), username })
}

/// ساخت initData معتبر برای تست‌ها — پورت `makeTestInitData`.
#[must_use]
pub fn make_test_init_data(bot_token: &str, user: &str, username: Option<&str>) -> String {
    let user_json = match username {
        Some(u) => format!(r#"{{"id":{user},"username":"{u}"}}"#),
        None => format!(r#"{{"id":{user}}}"#),
    };
    let mut params: Vec<(String, String)> = vec![
        ("user".into(), user_json),
        ("auth_date".into(), "1700000000".into()),
    ];
    params.sort_by(|a, b| a.0.cmp(&b.0));
    let data_check_string = params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("\n");
    let secret = Sha256::digest(bot_token.as_bytes());
    let mut mac = Hmac::<Sha256>::new_from_slice(&secret)
        .unwrap_or_else(|_| unreachable!("SHA256 digest is always a valid HMAC key length"));
    mac.update(data_check_string.as_bytes());
    let hash = hex::encode(mac.finalize().into_bytes());
    let pairs = params
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{pairs}&hash={hash}")
}

/// کدگذاری فرم ساده (مثل URLSearchParams — فقط ضروری‌ها).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' | b'{' | b'}' | b'"' | b':' | b',' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// تجزیهٔ فرم‌encoded — بدون وابستگی؛ + به فاصلهٔ URLSearchParams نگاشت نمی‌شود مگر در query.
fn parse_urlencoded(s: &str) -> Vec<(String, String)> {
    s.split('&')
        .filter(|p| !p.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (urldecode(k), urldecode(v))
        })
        .collect()
}

/// کدگشایی درصدی.
fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() + 1 && i + 2 < bytes.len() {
            let hex_part = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(v) = hex_part.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ── Rate limit — پنجرهٔ لغزان درون‌حافظه‌ای ──────────────────────────────────

/// اندازهٔ پنجره (۶۰ ثانیه) — مثل TS.
pub const RATE_WINDOW_SECS: u64 = 60;

/// limiter پنجرهٔ لغزان — یکی به ازای سرور (درون Mutex فاز ۴).
///
/// * تماس‌گیرندگان معتمد (بات/داشبورد — دارای کلید ادمین) باکت مشترک `admin` می‌گیرند.
/// * بقیه با ip واقعی کلید می‌خورند (X-Forwarded-For در فاز ۴).
/// * نگه‌داشت حافظه: بیش از ۱۰٬۰۰۰ کلید → پاک‌سازی (مثل TS).
#[derive(Debug, Default)]
pub struct RateLimiter {
    windows: HashMap<String, Vec<Instant>>,
    limit: usize,
}

impl RateLimiter {
    /// limiter جدید با سقف درخواست در دقیقه.
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self { windows: HashMap::new(), limit }
    }

    /// ثبت hit و تصمیم — `Ok(())` مجاز، `Err(HttpFailure)` یعنی 429 آماده.
    pub fn check(&mut self, key: &str) -> Result<(), crate::error::HttpFailure> {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(RATE_WINDOW_SECS);
        let hits = self.windows.entry(key.to_owned()).or_default();
        hits.retain(|t| now.duration_since(*t) < window);
        if hits.len() >= self.limit {
            return Err(crate::error::HttpFailure::rate_limited(
                i64::try_from(self.limit).unwrap_or(i64::MAX),
            ));
        }
        hits.push(now);
        if self.windows.len() > 10_000 {
            self.windows.clear(); // نگه‌دارندهٔ حافظهٔ خام — مثل TS
        }
        Ok(())
    }

    /// ریست کامل (تست‌ها).
    pub fn reset(&mut self) {
        self.windows.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "kia-admin-key-0123456789";

    #[test]
    fn kia_key_fail_closed_on_missing_or_short_server_key() {
        assert!(!verify_kia_key(Some(KEY), None)); // سرور بدون کلید → هرگز باز
        assert!(!verify_kia_key(Some(KEY), Some("short"))); // < ۱۶ کاراکتر
        assert!(!verify_kia_key(None, Some(KEY))); // هدر غایب
        assert!(!verify_kia_key(Some(""), Some(KEY))); // هدر خالی
        assert!(verify_kia_key(Some(KEY), Some(KEY)));
        assert!(!verify_kia_key(Some("kia-admin-key-0123456788"), Some(KEY)));
        assert!(!verify_kia_key(Some(KEY), Some(&KEY[..15])));
    }

    #[test]
    fn telegram_init_data_official_scheme_roundtrip() {
        let bot = "7000000000:TEST-TOKEN-abc";
        let data = make_test_init_data(bot, "12345", Some("ali"));
        let identity = verify_telegram_init_data(&data, bot).expect("valid initData must verify");
        assert_eq!(identity.user_id, "12345");
        assert_eq!(identity.username.as_deref(), Some("ali"));
    }

    #[test]
    fn telegram_init_data_rejects_tampering() {
        let bot = "7000000000:TEST-TOKEN-abc";
        let mut data = make_test_init_data(bot, "12345", None);
        // دستکاری hash
        let len = data.len();
        data.replace_range(len - 2..len, if data.ends_with("aa") { "bb" } else { "aa" });
        assert!(verify_telegram_init_data(&data, bot).is_none());
        // توکن اشتباه
        let data2 = make_test_init_data(bot, "99", None);
        assert!(verify_telegram_init_data(&data2, "7000000000:OTHER").is_none());
        // hash غایب
        let data3 = data2.split("&hash=").next().unwrap_or("").to_string();
        assert!(verify_telegram_init_data(&data3, bot).is_none());
        // user بدون id
        let mut mac = Hmac::<Sha256>::new_from_slice(&Sha256::digest(bot.as_bytes())).unwrap();
        mac.update(b"user=%7B%7D");
        let bogus = format!("user=%7B%7D&hash={}", hex::encode(mac.finalize().into_bytes()));
        assert!(verify_telegram_init_data(&bogus, bot).is_none());
    }

    #[test]
    fn rate_limiter_blocks_after_limit_and_separates_admin_bucket() {
        let mut lim = RateLimiter::new(3);
        assert!(lim.check("1.2.3.4").is_ok());
        assert!(lim.check("1.2.3.4").is_ok());
        assert!(lim.check("1.2.3.4").is_ok());
        let err = lim.check("1.2.3.4").unwrap_err();
        assert_eq!(err.status, 429);
        assert_eq!(err.code, "RATE_LIMITED");
        // ادمین باکت خودش را دارد — تماس معتمد متأثر از ip عمومی نیست
        assert!(lim.check("admin").is_ok());
        lim.reset();
    }
}
