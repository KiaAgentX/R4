//! `dropagentx-agents` — تیم ایجنت‌ها — ۷ نقش، MCP، A2A، ناوگان (فاز ۶)
//!
//! فاز ۰: اسکلت سیم‌کشی‌شده؛ رفتار واقعی طبق ADR-009 در فازهای بعد اضافه می‌شود.
#![forbid(unsafe_code)]

/// شناسه کرِیت — برای smoke test و لاگ‌ها.
pub const NAME: &str = "dropagentx-agents";

/// نسخه کرِیت — از `Cargo.toml` خوانده می‌شود.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// سلام‌سنج فاز ۰ — نشانه زنده‌بودن سیم‌کشی (جای hello-handler).
#[must_use]
pub fn hello() -> &'static str {
    "dropagentx_agents · Agent-One is alive"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(NAME, "dropagentx-agents");
        assert!(!VERSION.is_empty());
        assert!(hello().ends_with("alive"));
    }
}
