//! `dropagentx-bot` — آداپتور تلگرام — teloxide، ۲۷ هندلر، ۲۵ زبان (فاز ۵)
//!
//! فاز ۰: اسکلت سیم‌کشی‌شده؛ رفتار واقعی طبق ADR-009 در فازهای بعد اضافه می‌شود.
#![forbid(unsafe_code)]

/// شناسه کرِیت — برای smoke test و لاگ‌ها.
pub const NAME: &str = "dropagentx-bot";

/// نسخه کرِیت — از `Cargo.toml` خوانده می‌شود.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// سلام‌سنج فاز ۰ — نشانه زنده‌بودن سیم‌کشی (جای hello-handler).
#[must_use]
pub fn hello() -> &'static str {
    "dropagentx_bot · Agent-One is alive"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(NAME, "dropagentx-bot");
        assert!(!VERSION.is_empty());
        assert!(hello().ends_with("alive"));
    }
}
