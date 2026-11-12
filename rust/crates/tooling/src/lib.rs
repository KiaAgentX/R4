//! `dropagentx-tooling` — ابزار — مهاجرت لگاسی، genesis_proof، dual_run،
//! audit_ledger، parity، loadtest (فاز ۲/۸).
//!
//! * [`parity`] — سوئیت parity فاز ۸: سناریوهای طلایی بایت‌به‌بایت + وضعیت DB
//! * [`dual_run`] — مغز مسیریابی dual-run: درصد ترافیک، سپر HITL، گیت parity
//! * [`proxy`] — پراکسی HTTP مسیریابی با نقاط کنترل `/_dualrun`
#![forbid(unsafe_code)]

pub mod dual_run;
pub mod parity;
pub mod proxy;

/// شناسه کرِیت — برای smoke test و لاگ‌ها.
pub const NAME: &str = "dropagentx-tooling";

/// نسخه کرِیت — از `Cargo.toml` خوانده می‌شود.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// سلام‌سنج فاز ۰ — نشانه زنده‌بودن سیم‌کشی (جای hello-handler).
#[must_use]
pub fn hello() -> &'static str {
    "dropagentx_tooling · Agent-One is alive"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(NAME, "dropagentx-tooling");
        assert!(!VERSION.is_empty());
        assert!(hello().ends_with("alive"));
    }
}
