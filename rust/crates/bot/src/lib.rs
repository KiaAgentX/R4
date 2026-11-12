//! `dropagentx-bot` — آداپتور تلگرام (فاز ۵ از ADR-009).
//!
//! معماری «orchestrator خالص» — همان الگوی فعلی:
//!
//! ```text
//! update تلگرام → [WireUpdate] → orchestrator (خالص، آفلاین‌تست‌پذیر)
//!     → [Vec<BotAction>] → رانر (webhook آکسوم یا polling توسعه)
//! ```
//!
//! | ماژول | نقش |
//! |---|---|
//! | [`wire`] | DTOهای وایر تلگرام + [BotAction] خروجی — بدون teloxide |
//! | [`cmd`] | ۱۴ دستور کاربر + ۵ دستور ادمین (۱۹ فرمان ADR-009) |
//! | [`i18n`] | fa/en + نگاشت کد خطای وایر به پیام محلی |
//! | [`keyboard`] | کیبورد شیشه‌ای — دکمه = متن = فرمان |
//! | [`env`] | قرارداد backend (`Env`) + انواع داده + خطای بات |
//! | [`memory`] | backend درون‌حافظه‌ای — آفلاین، معناشناسی عین `V3Store` |
//! | [`orchestrator`] | قلب خالص — authz، idempotency از message_id، پیام‌ها |
//!
//! صف HITL (مادهٔ ۷، آستانهٔ ۵۰٬۰۰۰ Đ) در هر دو backend با همان معناشناسی
//! سرور پیاده شده: شناسهٔ دترمیینیستیک، TTL ۲۴ ساعت، تسویهٔ اتمیک ۹۰/۱۰ با
//! کلید قطعی `hitl:<id>:approve`، تصمیم دوم → خطای تایپ‌شده با بازپرداخت صفر.
//!
//! رانرها بیرون این crate می‌نشینند: webhook روی `/api/v1/bot/webhook` داخل
//! آکسوم (crate سرور) و polling توسعه؛ teloxide فقط «رانر» است و قلب بات به
//! آن وابسته نیست.
#![forbid(unsafe_code)]

pub mod cmd;
pub mod env;
pub mod error;
pub mod i18n;
pub mod keyboard;
pub mod memory;
pub mod orchestrator;
pub mod runner;
pub mod wire;

/// شناسه کرِیت — برای smoke test و لاگ‌ها.
pub const NAME: &str = "dropagentx-bot";

/// نسخه کرِیت — از `Cargo.toml` خوانده می‌شود.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// سلام‌سنج — نشانهٔ زنده‌بودن سیم‌کشی.
#[must_use]
pub fn hello() -> &'static str {
    "dropagentx_bot · orchestrator ready"
}

// ---------- re-exports آرگونومیک ----------
pub use cmd::Cmd;
pub use env::{
    user_account, BotError, EconomyItem, Env, EscrowItem, HistoryItem, HitlItem, HitlOutcome,
    ProductItem, Transition, TransitionOutcome,
};
pub use error::RunnerError;
pub use i18n::Locale;
pub use memory::MemoryDomain;
pub use orchestrator::handle_update;
pub use wire::{BotAction, ReplyKeyboard, WireChat, WireMessage, WireUpdate, WireUser};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(NAME, "dropagentx-bot");
        assert!(!VERSION.is_empty());
        assert!(hello().ends_with("ready"));
    }

    #[test]
    fn nineteen_commands_are_wired() {
        assert_eq!(Cmd::ALL.len(), 19);
        assert_eq!(Cmd::USER_COMMANDS.len() + Cmd::ADMIN_COMMANDS.len(), Cmd::ALL.len());
    }
}
