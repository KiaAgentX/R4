//! ۱۹ فرمان بات — ۱۴ دستور کاربر + ۵ دستور ادمین (ADR-009 فاز ۵).
//!
//! تجزیه مثل تلگرام: `/cmd@BotName` → `Cmd` (پسوند بات و بزرگی‌حرف بی‌اثر)؛
//! برچسب‌های کیبورد شیشه‌ای هم به همین enum می‌رسند (نگاشت در [`crate::i18n`]).
//!
//! مجوز: دستورهای ادمین با گیت `Env::is_admin` fail-closed اجرا می‌شوند؛
//! گذارهای امانت در orchestrator نقش خریدار/فروشنده را چک می‌کنند و هر دو
//! backend ([`crate::memory`]) گارد را دوباره داخل فرمان اعمال می‌کند.

/// فرمان‌های بات.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cmd {
    // ── کاربر (۱۴) ──────────────────────────────────────────────────────────
    /// خوش‌آمد + کیبورد شیشه‌ای
    Start,
    /// راهنمای کامل فرمان‌ها
    Help,
    /// منو (کیبورد شیشه‌ای)
    Menu,
    /// موجودی کیف
    Balance,
    /// تاریخچهٔ تراکنش‌ها
    History,
    /// فهرست محصولات فعال
    Products,
    /// خرید — `/buy <productId>` یا `/buy <sellerTgId> <amount>`
    Buy,
    /// وضعیت امانت — `/status <escrowId>` یا آخرین امانت باز من
    Status,
    /// تأمین مالی امانت (فقط خریدار)
    Fund,
    /// ثبت تحویل (فقط فروشنده)
    Deliver,
    /// آزادسازی/تسویه (فقط خریدار) — ≥ ۵۰٬۰۰۰ Đ صف HITL
    Release,
    /// اختلاف (فقط خریدار)
    Dispute,
    /// لغو امانت CREATED (فقط خریدار)
    Cancel,
    /// تغییر زبان — `/lang fa|en`
    Lang,
    // ── ادمین (۵) ───────────────────────────────────────────────────────────
    /// صف HITL — درخواست‌های PENDING
    Queue,
    /// تأیید HITL → تسویهٔ اتمیک ۹۰/۱۰ در همان تراکنش
    Approve,
    /// رد HITL — هیچ پولی جابه‌جا نمی‌شود
    Reject,
    /// خلاصهٔ اقتصاد کشور
    Economy,
    /// انتهای audit log
    Audit,
}

impl Cmd {
    /// فرمان‌های کاربر — به ترتیب راهنما.
    pub const USER_COMMANDS: [Cmd; 14] = [
        Cmd::Start,
        Cmd::Help,
        Cmd::Menu,
        Cmd::Balance,
        Cmd::History,
        Cmd::Products,
        Cmd::Buy,
        Cmd::Status,
        Cmd::Fund,
        Cmd::Deliver,
        Cmd::Release,
        Cmd::Dispute,
        Cmd::Cancel,
        Cmd::Lang,
    ];

    /// فرمان‌های ادمین — به ترتیب راهنما.
    pub const ADMIN_COMMANDS: [Cmd; 5] =
        [Cmd::Queue, Cmd::Approve, Cmd::Reject, Cmd::Economy, Cmd::Audit];

    /// همهٔ ۱۹ فرمان — برای تجزیه و کمک‌حافظهٔ راهنما.
    pub const ALL: [Cmd; 19] = [
        Cmd::Start,
        Cmd::Help,
        Cmd::Menu,
        Cmd::Balance,
        Cmd::History,
        Cmd::Products,
        Cmd::Buy,
        Cmd::Status,
        Cmd::Fund,
        Cmd::Deliver,
        Cmd::Release,
        Cmd::Dispute,
        Cmd::Cancel,
        Cmd::Lang,
        Cmd::Queue,
        Cmd::Approve,
        Cmd::Reject,
        Cmd::Economy,
        Cmd::Audit,
    ];

    /// نام لاتین فرمان (بدون `/`) — همان چیزی که تلگرام می‌فرستد.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Cmd::Start => "start",
            Cmd::Help => "help",
            Cmd::Menu => "menu",
            Cmd::Balance => "balance",
            Cmd::History => "history",
            Cmd::Products => "products",
            Cmd::Buy => "buy",
            Cmd::Status => "status",
            Cmd::Fund => "fund",
            Cmd::Deliver => "deliver",
            Cmd::Release => "release",
            Cmd::Dispute => "dispute",
            Cmd::Cancel => "cancel",
            Cmd::Lang => "lang",
            Cmd::Queue => "queue",
            Cmd::Approve => "approve",
            Cmd::Reject => "reject",
            Cmd::Economy => "economy",
            Cmd::Audit => "audit",
        }
    }

    /// آیا دستور ادمین است؟ (گیت fail-closed در orchestrator)
    #[must_use]
    pub const fn is_admin(self) -> bool {
        matches!(self, Cmd::Queue | Cmd::Approve | Cmd::Reject | Cmd::Economy | Cmd::Audit)
    }

    /// تجزیهٔ متن فرمان — `/buy@DropAgentXBot args` → `Some(Cmd::Buy)`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Cmd> {
        let raw = text.strip_prefix('/')?;
        let token = raw.split(['@', ' ', '\n']).next()?;
        let token = token.trim();
        if token.is_empty() {
            return None;
        }
        let lower = token.to_ascii_lowercase();
        Cmd::ALL.iter().copied().find(|c| c.as_str() == lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nineteen_commands_user_and_admin_split() {
        assert_eq!(Cmd::USER_COMMANDS.len(), 14, "۱۴ دستور کاربر ADR-009");
        assert_eq!(Cmd::ADMIN_COMMANDS.len(), 5, "۵ دستور ادمین ADR-009");
        // یکتایی نام‌ها
        let mut names: Vec<&str> = Cmd::ALL.iter().map(|c| c.as_str()).collect();
        names.sort_unstable();
        let unique = names.windows(2).all(|w| w[0] != w[1]);
        assert!(unique, "نام فرمان‌ها یکتا هستند");
    }

    #[test]
    fn parses_command_with_bot_suffix_and_case() {
        assert_eq!(Cmd::parse("/balance"), Some(Cmd::Balance));
        assert_eq!(Cmd::parse("/Balance@DropAgentXBot"), Some(Cmd::Balance));
        assert_eq!(Cmd::parse("/BUY arg1 arg2"), Some(Cmd::Buy));
        assert_eq!(Cmd::parse("/audit"), Some(Cmd::Audit));
    }

    #[test]
    fn rejects_non_commands() {
        assert_eq!(Cmd::parse("سلام"), None);
        assert_eq!(Cmd::parse("/"), None);
        assert_eq!(Cmd::parse("/nope"), None);
        assert_eq!(Cmd::parse(""), None);
    }

    #[test]
    fn admin_gate_flags() {
        assert!(Cmd::Approve.is_admin());
        assert!(Cmd::Queue.is_admin());
        assert!(!Cmd::Balance.is_admin());
        assert!(!Cmd::Release.is_admin());
    }
}
