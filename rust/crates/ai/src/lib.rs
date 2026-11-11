//! `dropagentx-ai` — گیت‌وی AI (فاز ۶ از ADR-009).
//!
//! معماری «gateway خالص» — همان الگوی فاز ۵:
//!
//! ```text
//! درخواست چت → [گیت سقف] → روتینگ ایجنت → redaction ورودی
//!     → fallback قطعی سه‌اسلات (primary→secondary→fallback)
//!     → پاکت JSON سخت ({"reply"} | {"tool","args"})
//!     → گیت مجوز رجیستری → backend مهارت → trace + مصرف توکن
//! ```
//!
//! | ماژول | نقش |
//! |---|---|
//! | [`registry`] | رجیستری واحد ۶ ایجنت + ۱۶ مهارت (fa/en + پرامپت سیستمی) |
//! | [`llm`] | سه‌اسلات ADR-008 — OpenAI-سازگار + FakeProvider تست |
//! | [`packet`] | پاکت JSON به‌جای function-calling — متن خام هرگز عبور نمی‌کند |
//! | [`redaction`] | ۹ قاعدهٔ redaction + شمارش برای audit |
//! | [`tools`] | قرارداد backend مهارت‌ها + گیت/ثبت سقف روزانه |
//! | [`engine`] | موتور چت — fallback، مجوز، trace، usage |
//!
//! ممیزی هر پاسخ `ai.ok|refused|failed|pending` در هندلر سرور نوشته می‌شود
//! (crate خالص SQL نمی‌داند — همان مرز فاز ۱/۲).
#![forbid(unsafe_code)]

pub mod engine;
pub mod error;
pub mod llm;
pub mod packet;
pub mod redaction;
pub mod registry;
pub mod tools;

/// شناسه کرِیت — برای smoke test و لاگ‌ها.
pub const NAME: &str = "dropagentx-ai";

/// نسخه کرِیت — از `Cargo.toml` خوانده می‌شود.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// سلام‌سنج — نشانهٔ زنده‌بودن سیم‌کشی.
#[must_use]
pub fn hello() -> &'static str {
    "dropagentx_ai · gateway ready"
}

// ---------- re-exports آرگونومیک ----------
pub use engine::{AuditKind, ChatOutcome, Gateway};
pub use error::{GateError, LlmError, PacketError};
pub use llm::{AnyProvider, FakeProvider, LlmProvider, LlmReply, OpenAiProvider};
pub use packet::Packet;
pub use redaction::{redact, RedactRule, RedactionReport};
pub use registry::{
    agent_of, agent_views, route_agent, skill_of, skill_views, AgentSpec, SkillSpec, AGENTS, SKILLS,
};
pub use tools::{AiUsageRecord, ToolBackend, ToolCtx, ToolOutcome};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(NAME, "dropagentx-ai");
        assert!(!VERSION.is_empty());
        assert!(hello().ends_with("ready"));
    }

    #[test]
    fn registry_shape_is_adr_locked() {
        assert_eq!(AGENTS.len(), 6, "۶ ایجنت Zenovix");
        assert_eq!(SKILLS.len(), 16, "۱۶ مهارت KIA-OS");
    }
}
