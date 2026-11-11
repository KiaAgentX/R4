//! وضعیت مشترک سرور — یک `Arc<AppState>` بین همهٔ هندلرها.
//!
//! دیتابیس ([`V3Store`]) نقطهٔ تنها اتصال دامنه به SQLite است؛ limiter نرخ
//! درون‌حافظه‌ای همان پنجرهٔ لغزان فاز ۳ را با Mutex پخش می‌کند و رجیستری
//! ایجنت/مهارت دادهٔ ثابت `/ai/agents` و `/ai/skills` است (فاز ۶ اجرای واقعی
//! مهارت‌ها را روی همین شناسه‌ها سیم می‌کند).

use std::sync::{Arc, Mutex};
use std::time::Instant;

use dropagentx_contracts::{AiAgentView, AiSkillView, RateLimiter};
use dropagentx_db::V3Store;

use crate::config::ServerConfig;

/// رجیستری ایجنت‌های Zenovix — دادهٔ ثابت (فاز ۶ موتور روتینگ را روی همین
/// نام‌ها می‌سازد). ۶ ایجنت با مهارت‌های مجاز خودشان.
#[must_use]
pub fn zenovix_agents() -> Vec<AiAgentView> {
    vec![
        AiAgentView {
            name: dropagentx_contracts::AiAgentName::Orchestrator,
            emoji: "🧠".into(),
            title_fa: "هماهنگ‌کننده".into(),
            title_en: "Orchestrator".into(),
            description: "Routes the request to the right specialist agent and merges the answer."
                .into(),
            skills: vec![
                "wallet.balance".into(),
                "wallet.history".into(),
                "escrow.create".into(),
                "escrow.status".into(),
                "product.search".into(),
                "kb.search".into(),
                "ai.handoff".into(),
            ],
        },
        AiAgentView {
            name: dropagentx_contracts::AiAgentName::Receptionist,
            emoji: "🛎️".into(),
            title_fa: "پذیرش".into(),
            title_en: "Receptionist".into(),
            description: "Greets citizens, explains the charter and routes to the right agent."
                .into(),
            skills: vec!["kb.search".into(), "product.search".into(), "ai.handoff".into()],
        },
        AiAgentView {
            name: dropagentx_contracts::AiAgentName::Sales,
            emoji: "🛒".into(),
            title_fa: "فروش".into(),
            title_en: "Sales".into(),
            description: "Lists products, opens charter-compliant escrows for buyers.".into(),
            skills: vec!["product.search".into(), "escrow.create".into(), "escrow.status".into()],
        },
        AiAgentView {
            name: dropagentx_contracts::AiAgentName::Support,
            emoji: "🧯".into(),
            title_fa: "پشتیبانی".into(),
            title_en: "Support".into(),
            description: "Tracks escrow disputes, answers delivery questions.".into(),
            skills: vec!["escrow.status".into(), "wallet.history".into(), "kb.search".into()],
        },
        AiAgentView {
            name: dropagentx_contracts::AiAgentName::Analyst,
            emoji: "📊".into(),
            title_fa: "تحلیلگر".into(),
            title_en: "Analyst".into(),
            description: "Reads the public economy summary and personal wallet history.".into(),
            skills: vec![
                "wallet.balance".into(),
                "wallet.history".into(),
                "economy.summary".into(),
            ],
        },
        AiAgentView {
            name: dropagentx_contracts::AiAgentName::Content,
            emoji: "✍️".into(),
            title_fa: "محتوا".into(),
            title_en: "Content".into(),
            description: "Drafts product copy and announcements in fa/en.".into(),
            skills: vec!["product.search".into(), "kb.search".into()],
        },
    ]
}

/// رجیستری ۱۶ مهارت KIA-OS — متادیتای مجوز (فاز ۶ گیت fail-closed قبل از اجرا
/// را روی همین `kind`ها می‌سازد؛ `staff_only` با کلید ادمین گیت می‌خورد).
#[must_use]
pub fn kia_os_skills() -> Vec<AiSkillView> {
    use dropagentx_contracts::{AiAgentName as A, AiSkillKind as K};
    let mut v = Vec::new();
    let mut push =
        |id: &str, kind: K, fa: &str, en: &str, desc: &str, agents: &[A], staff: bool| {
            v.push(AiSkillView {
                id: id.into(),
                kind,
                title_fa: fa.into(),
                title_en: en.into(),
                description: desc.into(),
                allowed_agents: agents.to_vec(),
                staff_only: staff,
            });
        };
    // خواندنی‌ها
    push(
        "wallet.balance",
        K::Read,
        "موجودی کیف",
        "Wallet balance",
        "Derived balance of the acting account.",
        &[A::Orchestrator, A::Analyst, A::Receptionist],
        false,
    );
    push(
        "wallet.history",
        K::Read,
        "تاریخچهٔ کیف",
        "Wallet history",
        "Recent postings of the acting account.",
        &[A::Orchestrator, A::Analyst, A::Support],
        false,
    );
    push(
        "escrow.status",
        K::Read,
        "وضعیت امانت",
        "Escrow status",
        "Read one escrow by id (public transparency).",
        &[A::Orchestrator, A::Sales, A::Support],
        false,
    );
    push(
        "product.search",
        K::Read,
        "جست‌وجوی مارکت",
        "Market search",
        "Search active market listings.",
        &[A::Orchestrator, A::Sales, A::Receptionist, A::Content],
        false,
    );
    push(
        "kb.search",
        K::Read,
        "جست‌وجوی دانش",
        "Knowledge search",
        "Search the internal knowledge base.",
        &[A::Orchestrator, A::Receptionist, A::Support, A::Content],
        false,
    );
    push(
        "economy.summary",
        K::Read,
        "خلاصهٔ اقتصاد",
        "Economy summary",
        "Public economy transparency summary (Article 6).",
        &[A::Orchestrator, A::Analyst],
        false,
    );
    // نوشتنی‌ها
    push(
        "escrow.create",
        K::Write,
        "ساخت امانت",
        "Create escrow",
        "Open a charter-compliant escrow for a buyer.",
        &[A::Orchestrator, A::Sales],
        false,
    );
    push(
        "escrow.deliver",
        K::Write,
        "ثبت تحویل",
        "Deliver escrow",
        "Mark an escrow as delivered (money stays locked).",
        &[A::Orchestrator, A::Support],
        false,
    );
    push(
        "escrow.dispute",
        K::Write,
        "ثبت اختلاف",
        "Open dispute",
        "Open a dispute with a reason (escrow stays locked).",
        &[A::Orchestrator, A::Support],
        false,
    );
    push(
        "product.create",
        K::Write,
        "ساخت محصول",
        "Create product",
        "Create a house listing (seller = treasury).",
        &[A::Orchestrator, A::Sales],
        true,
    );
    push(
        "kb.upsert",
        K::Write,
        "به‌روزرسانی دانش",
        "Upsert knowledge",
        "Add or update a knowledge base entry.",
        &[A::Orchestrator, A::Content],
        true,
    );
    // پولی‌ها — همیشه HITL-gated
    push(
        "escrow_refund",
        K::Money,
        "بازپرداخت امانت",
        "Escrow refund",
        "Arbitration refund — may require HITL approval.",
        &[A::Orchestrator, A::Support],
        true,
    );
    push(
        "escrow_release",
        K::Money,
        "تسویهٔ امانت",
        "Escrow release",
        "Settle an escrow 90/10 — HITL gate at 50,000 Đ.",
        &[A::Orchestrator],
        true,
    );
    push(
        "wallet_mint",
        K::Money,
        "ضرب دیین",
        "Mint dinar",
        "Mint dinar into an account (yearly cap, admin only).",
        &[],
        true,
    );
    push(
        "wallet_burn",
        K::Money,
        "سوزاندن دیین",
        "Burn dinar",
        "Burn dinar from an account (overdraft impossible).",
        &[],
        true,
    );
    // دستوری‌ها
    push(
        "ai.handoff",
        K::Staff,
        "ارجاع به انسان",
        "Human handoff",
        "Queue a HITL request for the council.",
        &[A::Orchestrator, A::Receptionist],
        false,
    );
    v
}

/// وضعیت مشترک همهٔ هندلرها.
#[derive(Debug)]
pub struct AppState {
    /// فروشگاه v3 — نقطهٔ تنها اتصال دامنه به SQLite.
    pub store: V3Store,
    /// پیکربندی بوت (تغییرناپذیر).
    pub config: ServerConfig,
    /// limiter نرخ پنجرهٔ لغزان — یکی به ازای سرور.
    pub limiter: Mutex<RateLimiter>,
    /// لحظهٔ شروع — uptime در health.
    pub started_at: Instant,
}

impl AppState {
    /// ساخت وضعیت — `started_at` همین‌جا قفل می‌شود.
    #[must_use]
    pub fn new(store: V3Store, config: ServerConfig) -> Self {
        Self {
            limiter: Mutex::new(RateLimiter::new(config.rate_limit)),
            store,
            config,
            started_at: Instant::now(),
        }
    }

    /// وضعیت اشتراکی — `Arc` برای کلون‌شدن داخل هر هندلر.
    #[must_use]
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}
