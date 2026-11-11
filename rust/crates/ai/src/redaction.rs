//! ۹ قاعدهٔ redaction — قبل از ورود به LLM و قبل از خروج از پروسه (ADR-009 فاز ۶).
//!
//! هر تطبیق با برچسب `[REDACTED:<RULE>]` جایگزین می‌شود و شمرده می‌شود؛
//! شمارش در پاسخ (`AiChatResponse.redactions`) و در audit می‌نشیند. قاعده‌ها
//! به ترتیب اولویت اجرا می‌شوند تا قواعد پهن‌تر قواعد دقیق‌تر را نبلعند.

/// برچسب هر قاعده.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactRule {
    /// کلید خصوصی PEM
    Pem,
    /// هدر Bearer
    Bearer,
    /// توکن JWT
    Jwt,
    /// کلید API با پیشوند sk
    ApiKey,
    /// توکن بات تلگرام (id:hash)
    TelegramToken,
    /// کلید دسترسی AWS (AKIA/ASIA)
    AwsKey,
    /// شمارهٔ کارت
    Card,
    /// عبارت بازیابی (۱۲/۲۴ کلمه)
    SeedPhrase,
    /// رشتهٔ hex طولانی (کلید/امضا)
    Hex,
}

impl RedactRule {
    /// برچسب روی سیم — `[REDACTED:<LABEL>]`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            RedactRule::Pem => "PEM",
            RedactRule::Bearer => "BEARER",
            RedactRule::Jwt => "JWT",
            RedactRule::ApiKey => "API_KEY",
            RedactRule::TelegramToken => "TELEGRAM_TOKEN",
            RedactRule::AwsKey => "AWS_KEY",
            RedactRule::Card => "CARD",
            RedactRule::SeedPhrase => "SEED_PHRASE",
            RedactRule::Hex => "HEX",
        }
    }
}

/// نتیجهٔ redaction — متن پاک + تعداد جایگزینی‌ها.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionReport {
    /// متن پس از پاکسازی
    pub text: String,
    /// تعداد جایگزینی‌ها
    pub count: i64,
}

/// پاکسازی ورودی/خروجی با هر ۹ قاعده — روی سیم شمرده می‌شود.
/// الگوها یک‌بار با `LazyLock` کامپایل می‌شوند (بدون وابستگی بیرونی اضافه).
#[must_use]
pub fn redact(input: &str) -> RedactionReport {
    let mut text = input.to_owned();
    let mut count: i64 = 0;
    for (rule, re) in &*COMPILED {
        let replacement = format!("[REDACTED:{}]", rule.label());
        let n = i64::try_from(re.find_iter(&text).count()).unwrap_or(i64::MAX);
        text = re.replace_all(&text, replacement.as_str()).into_owned();
        count += n;
    }
    RedactionReport { text, count }
}

/// جدول قواعد به ترتیب اولویت — پهن‌ترین‌ها اول.
const RULES: &[(RedactRule, &str)] = &[
    // ۱. PEM — کل بلوک کلید خصوصی
    (
        RedactRule::Pem,
        r"-----BEGIN [A-Z ]*PRIVATE KEY-----(?s:.+?)-----END [A-Z ]*PRIVATE KEY-----",
    ),
    // ۲. JWT — سه قطعهٔ base64url جدا با نقطه
    (RedactRule::Jwt, r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\b"),
    // ۳. Bearer — هدر احراز
    (RedactRule::Bearer, r"\bBearer\s+[A-Za-z0-9._~+/=-]{8,}\b"),
    // ۴. کلید API با پیشوند sk (openai-سبک)
    (RedactRule::ApiKey, r"\bsk-[A-Za-z0-9_-]{16,}\b"),
    // ۵. توکن بات تلگرام — `123456789:AA…`
    (RedactRule::TelegramToken, r"\b[0-9]{6,12}:[A-Za-z0-9_-]{30,}\b"),
    // ۶. کلید AWS — AKIA/ASIA + ۱۶ حرف-رقم بزرگ
    (RedactRule::AwsKey, r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b"),
    // ۷. شمارهٔ کارت — ۱۳-۱۹ رقم با گروه‌بندی اختیاری
    (RedactRule::Card, r"\b(?:\d[ -]?){12,18}\d\b"),
    // ۸. عبارت بازیابی — ≥۱۱ کلمهٔ کوچک با فاصله
    (RedactRule::SeedPhrase, r"\b[a-z]{3,8}(?:\s+[a-z]{3,8}){11,23}\b"),
    // ۹. hex طولانی — ≥۶۴ کاراکتر (کلید/امضا/هش)
    (RedactRule::Hex, r"\b[0-9a-fA-F]{64,}\b"),
];

/// قواعد کامپایل‌شده — یک‌بار در عمر پروسه.
static COMPILED: std::sync::LazyLock<Vec<(RedactRule, regex::Regex)>> =
    std::sync::LazyLock::new(|| {
        RULES
            .iter()
            .filter_map(|(rule, pattern)| regex::Regex::new(pattern).ok().map(|re| (*rule, re)))
            .collect()
    });

#[cfg(test)]
mod tests {
    use super::*;

    fn count_of(input: &str) -> i64 {
        redact(input).count
    }

    #[test]
    fn pem_block_is_removed_whole() {
        let secret = "-----BEGIN PRIVATE KEY-----\nabc\ndef\n-----END PRIVATE KEY-----";
        let r = redact(&format!("before {secret} after"));
        assert_eq!(r.text, "before [REDACTED:PEM] after");
        assert_eq!(r.count, 1);
    }

    #[test]
    fn bearer_and_jwt_are_labeled() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV";
        let r = redact(&format!("Bearer abc.def.123 and {jwt}"));
        assert!(r.text.contains("[REDACTED:BEARER]"), "{}", r.text);
        assert!(r.text.contains("[REDACTED:JWT]"), "{}", r.text);
        assert_eq!(r.count, 2);
    }

    #[test]
    fn sk_key_and_telegram_bot_token() {
        let r = redact(
            "key sk-abc123456789012345678 and 7000000000:AAFKFDKqzUd-something-long-token-here",
        );
        assert!(r.text.contains("[REDACTED:API_KEY]"), "{}", r.text);
        assert!(r.text.contains("[REDACTED:TELEGRAM_TOKEN]"), "{}", r.text);
        assert_eq!(r.count, 2);
    }

    #[test]
    fn aws_key_card_and_hex() {
        let r = redact("AKIAIOSFODNN7EXAMPLE 4111 1111 1111 1111 and aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert!(r.text.contains("[REDACTED:AWS_KEY]"), "{}", r.text);
        assert!(r.text.contains("[REDACTED:CARD]"), "{}", r.text);
        assert!(r.text.contains("[REDACTED:HEX]"), "{}", r.text);
        assert_eq!(r.count, 3);
    }

    #[test]
    fn seed_phrase_of_twelve_words() {
        let seed = "word one two three four five six seven eight nine ten eleven twelve";
        let r = redact(seed);
        assert!(r.text.contains("[REDACTED:SEED_PHRASE]"), "{}", r.text);
    }

    #[test]
    fn normal_text_is_untouched_with_zero_count() {
        let r = redact("سلام، موجودی کیف من چقدر است؟ amount 40000 and user:tg:123");
        assert_eq!(r.count, 0);
        assert!(r.text.contains("40000"));
        assert!(r.text.contains("user:tg:123"), "شناسهٔ حساب redact نمی‌شود");
    }

    #[test]
    fn count_sums_across_rules() {
        let n = count_of("sk-abcdefghijklmnop1234 Bearer qq-token-99 4111111111111111");
        assert_eq!(n, 3, "API_KEY + BEARER + CARD = 3");
    }

    #[test]
    fn repeated_occurrences_each_count() {
        let n = count_of("sk-aaaaaaaaaaaaaaaaaaaa sk-bbbbbbbbbbbbbbbbbbbb");
        assert_eq!(n, 2);
    }
}
