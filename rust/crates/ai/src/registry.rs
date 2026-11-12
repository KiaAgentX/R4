//! رجیستری واحد — ۶ ایجنت Zenovix + ۱۶ مهارت KIA-OS (ADR-009 فاز ۶).
//!
//! «یک تعریف»: هویت/متادیتای fa/en، مهارت‌های مجاز هر ایجنت و پرامپت سیستمی
//! همه از همین جدول می‌آیند — نمای وایر `/ai/agents` و `/ai/skills` (فاز ۳/۴)
//! و گیت مجوز موتور فاز ۶ هر دو از این‌جا تغذیه می‌شوند تا رجیستری هرگز از
//! هندلر/موتور فاصله نگیرد (همان قاعدهٔ ADR-005).
//!
//! `allowedSkills` مشتق از همین رجیستری است؛ `allowed_agents: []` یعنی هیچ
//! ایجنتی مجاز به فراخوانی نیست (wallet_mint/wallet_burn — فقط داشبورد/شورا).

use dropagentx_contracts::{AiAgentName, AiAgentView, AiSkillKind, AiSkillView};

/// مشخصهٔ ایجنت.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSpec {
    /// نام
    pub name: AiAgentName,
    /// ایموجی
    pub emoji: &'static str,
    /// عنوان فارسی
    pub title_fa: &'static str,
    /// عنوان انگلیسی
    pub title_en: &'static str,
    /// توضیح (انگلیسی — مثل وایر فعلی)
    pub description: &'static str,
    /// پرامپت سیستمی فارسی
    pub system_fa: &'static str,
    /// پرامپت سیستمی انگلیسی
    pub system_en: &'static str,
    /// مهارت‌های مجاز — مشتق از رجیستری مهارت‌ها
    pub skills: &'static [&'static str],
}

/// مشخصهٔ مهارت.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSpec {
    /// شناسه (`wallet.balance`)
    pub id: &'static str,
    /// نوع مجوز
    pub kind: AiSkillKind,
    /// عنوان فارسی
    pub title_fa: &'static str,
    /// عنوان انگلیسی
    pub title_en: &'static str,
    /// توضیح
    pub description: &'static str,
    /// ایجنت‌های مجاز — خالی یعنی هیچ‌کس (fail-closed)
    pub allowed_agents: &'static [AiAgentName],
    /// فقط staff؟
    pub staff_only: bool,
}

/// کلیدواژه‌های روتینگ قطعی — (ایجنت، کلیدواژه‌های fa/en).
const ROUTING_KEYWORDS: &[(AiAgentName, &[&str])] = &[
    (
        AiAgentName::Sales,
        &["خرید", "بخرم", "محصول", "فروش", "قیمت", "سبد", "buy", "product", "price", "purchase"],
    ),
    (
        AiAgentName::Support,
        &[
            "اختلاف",
            "تحویل",
            "مشکل",
            "مرجوع",
            "پشتیبانی",
            "dispute",
            "deliver",
            "refund",
            "support",
            "problem",
        ],
    ),
    (
        AiAgentName::Analyst,
        &[
            "موجودی",
            "اقتصاد",
            "تاریخچه",
            "آمار",
            "تحلیل",
            "balance",
            "economy",
            "history",
            "stats",
            "analytics",
        ],
    ),
    (
        AiAgentName::Content,
        &["متن", "تبلیغ", "کپی", "بنویس", "محتوا", "content", "copy", "write", "announce"],
    ),
    (
        AiAgentName::Receptionist,
        &["سلام", "خوش", "منشور", "قوانین", "شروع", "hello", "hi", "charter", "welcome", "start"],
    ),
];

/// ۶ ایجنت — دادهٔ ثابت (همان شکل وایر فاز ۴ + پرامپت سیستمی فاز ۶).
pub static AGENTS: &[AgentSpec] = &[
    AgentSpec {
        name: AiAgentName::Orchestrator,
        emoji: "🧠",
        title_fa: "هماهنگ‌کننده",
        title_en: "Orchestrator",
        description: "Routes the request to the right specialist agent and merges the answer.",
        system_fa: "تو هماهنگ‌کنندهٔ کشور Agent-One هستی؛ اگر درخواست مال حوزهٔ دیگری بود با مهارت مناسب اجرایش کن و پاسخ کوتاه و دقیق بده.",
        system_en: "You are the Orchestrator of the Agent-One nation; route work to the right skill and answer briefly and precisely.",
        skills: &[
            "wallet.balance", "wallet.history", "escrow.create", "escrow.status",
            "escrow.deliver", "escrow.dispute", "escrow_refund", "escrow_release",
            "product.search", "kb.search", "ai.handoff",
        ],
    },
    AgentSpec {
        name: AiAgentName::Receptionist,
        emoji: "🛎️",
        title_fa: "پذیرش",
        title_en: "Receptionist",
        description: "Greets citizens, explains the charter and routes to the right agent.",
        system_fa: "تو پذیرش کشور هستی؛ منشور ۱۰ ماده‌ای را ساده توضیح می‌دهی و شهروند را به ایجنت درست راهنمایی می‌کنی.",
        system_en: "You are the Receptionist; explain the 10-article charter simply and guide citizens to the right agent.",
        skills: &["wallet.balance", "product.search", "kb.search", "ai.handoff"],
    },
    AgentSpec {
        name: AiAgentName::Sales,
        emoji: "🛒",
        title_fa: "فروش",
        title_en: "Sales",
        description: "Lists products, opens charter-compliant escrows for buyers.",
        system_fa: "تو فروش کشور هستی؛ محصولات را معرفی می‌کنی و برای خریدار امانت مطابق منشور می‌سازی.",
        system_en: "You are Sales; list products and open charter-compliant escrows for buyers.",
        skills: &["product.search", "escrow.create", "escrow.status"],
    },
    AgentSpec {
        name: AiAgentName::Support,
        emoji: "🧯",
        title_fa: "پشتیبانی",
        title_en: "Support",
        description: "Tracks escrow disputes, answers delivery questions.",
        system_fa: "تو پشتیبانی هستی؛ اختلاف‌های امانت را پیگیری می‌کنی و وضعیت تحویل را شفاف گزارش می‌دهی.",
        system_en: "You are Support; track escrow disputes and report delivery status transparently.",
        skills: &["escrow.status", "escrow.deliver", "escrow.dispute", "escrow_refund", "wallet.history", "kb.search"],
    },
    AgentSpec {
        name: AiAgentName::Analyst,
        emoji: "📊",
        title_fa: "تحلیلگر",
        title_en: "Analyst",
        description: "Reads the public economy summary and personal wallet history.",
        system_fa: "تو تحلیلگر هستی؛ خلاصهٔ اقتصاد عمومی و تاریخچهٔ کیف شخصی را گزارش می‌دهی — بدون ادعای عددی بی‌منبع.",
        system_en: "You are the Analyst; report the public economy summary and personal wallet history — never invent numbers.",
        skills: &["wallet.balance", "wallet.history", "economy.summary"],
    },
    AgentSpec {
        name: AiAgentName::Content,
        emoji: "✍️",
        title_fa: "محتوا",
        title_en: "Content",
        description: "Drafts product copy and announcements in fa/en.",
        system_fa: "تو تولیدکنندهٔ محتوا هستی؛ کپی محصول و اطلاعیه‌ها را در fa/en می‌نویسی.",
        system_en: "You are Content; draft product copy and announcements in fa/en.",
        skills: &["product.search", "kb.search", "kb.upsert"],
    },
];

/// ۱۶ مهارت — دادهٔ ثابت (همان شکل وایر فاز ۴).
pub static SKILLS: &[SkillSpec] = &[
    SkillSpec {
        id: "wallet.balance",
        kind: AiSkillKind::Read,
        title_fa: "موجودی کیف",
        title_en: "Wallet balance",
        description: "Derived balance of the acting account.",
        allowed_agents: &[
            AiAgentName::Orchestrator,
            AiAgentName::Analyst,
            AiAgentName::Receptionist,
        ],
        staff_only: false,
    },
    SkillSpec {
        id: "wallet.history",
        kind: AiSkillKind::Read,
        title_fa: "تاریخچهٔ کیف",
        title_en: "Wallet history",
        description: "Recent postings of the acting account.",
        allowed_agents: &[AiAgentName::Orchestrator, AiAgentName::Analyst, AiAgentName::Support],
        staff_only: false,
    },
    SkillSpec {
        id: "escrow.status",
        kind: AiSkillKind::Read,
        title_fa: "وضعیت امانت",
        title_en: "Escrow status",
        description: "Read one escrow by id (public transparency).",
        allowed_agents: &[AiAgentName::Orchestrator, AiAgentName::Sales, AiAgentName::Support],
        staff_only: false,
    },
    SkillSpec {
        id: "product.search",
        kind: AiSkillKind::Read,
        title_fa: "جست‌وجوی مارکت",
        title_en: "Market search",
        description: "Search active market listings.",
        allowed_agents: &[
            AiAgentName::Orchestrator,
            AiAgentName::Sales,
            AiAgentName::Receptionist,
            AiAgentName::Content,
        ],
        staff_only: false,
    },
    SkillSpec {
        id: "kb.search",
        kind: AiSkillKind::Read,
        title_fa: "جست‌وجوی دانش",
        title_en: "Knowledge search",
        description: "Search the internal knowledge base.",
        allowed_agents: &[
            AiAgentName::Orchestrator,
            AiAgentName::Receptionist,
            AiAgentName::Support,
            AiAgentName::Content,
        ],
        staff_only: false,
    },
    SkillSpec {
        id: "economy.summary",
        kind: AiSkillKind::Read,
        title_fa: "خلاصهٔ اقتصاد",
        title_en: "Economy summary",
        description: "Public economy transparency summary (Article 6).",
        allowed_agents: &[AiAgentName::Orchestrator, AiAgentName::Analyst],
        staff_only: false,
    },
    SkillSpec {
        id: "escrow.create",
        kind: AiSkillKind::Write,
        title_fa: "ساخت امانت",
        title_en: "Create escrow",
        description: "Open a charter-compliant escrow for a buyer.",
        allowed_agents: &[AiAgentName::Orchestrator, AiAgentName::Sales],
        staff_only: false,
    },
    SkillSpec {
        id: "escrow.deliver",
        kind: AiSkillKind::Write,
        title_fa: "ثبت تحویل",
        title_en: "Deliver escrow",
        description: "Mark an escrow as delivered (money stays locked).",
        allowed_agents: &[AiAgentName::Orchestrator, AiAgentName::Support],
        staff_only: false,
    },
    SkillSpec {
        id: "escrow.dispute",
        kind: AiSkillKind::Write,
        title_fa: "ثبت اختلاف",
        title_en: "Open dispute",
        description: "Open a dispute with a reason (escrow stays locked).",
        allowed_agents: &[AiAgentName::Orchestrator, AiAgentName::Support],
        staff_only: false,
    },
    SkillSpec {
        id: "product.create",
        kind: AiSkillKind::Write,
        title_fa: "ساخت محصول",
        title_en: "Create product",
        description: "Create a house listing (seller = treasury).",
        allowed_agents: &[AiAgentName::Orchestrator, AiAgentName::Sales],
        staff_only: true,
    },
    SkillSpec {
        id: "kb.upsert",
        kind: AiSkillKind::Write,
        title_fa: "به‌روزرسانی دانش",
        title_en: "Upsert knowledge",
        description: "Add or update a knowledge base entry.",
        allowed_agents: &[AiAgentName::Orchestrator, AiAgentName::Content],
        staff_only: true,
    },
    SkillSpec {
        id: "escrow_refund",
        kind: AiSkillKind::Money,
        title_fa: "بازپرداخت امانت",
        title_en: "Escrow refund",
        description: "Arbitration refund — may require HITL approval.",
        allowed_agents: &[AiAgentName::Orchestrator, AiAgentName::Support],
        staff_only: true,
    },
    SkillSpec {
        id: "escrow_release",
        kind: AiSkillKind::Money,
        title_fa: "تسویهٔ امانت",
        title_en: "Escrow release",
        description: "Settle an escrow 90/10 — HITL gate at 50,000 Đ.",
        allowed_agents: &[AiAgentName::Orchestrator],
        staff_only: true,
    },
    SkillSpec {
        id: "wallet_mint",
        kind: AiSkillKind::Money,
        title_fa: "ضرب دیین",
        title_en: "Mint dinar",
        description: "Mint dinar into an account (yearly cap, admin only).",
        allowed_agents: &[],
        staff_only: true,
    },
    SkillSpec {
        id: "wallet_burn",
        kind: AiSkillKind::Money,
        title_fa: "سوزاندن دیین",
        title_en: "Burn dinar",
        description: "Burn dinar from an account (overdraft impossible).",
        allowed_agents: &[],
        staff_only: true,
    },
    SkillSpec {
        id: "ai.handoff",
        kind: AiSkillKind::Staff,
        title_fa: "ارجاع به انسان",
        title_en: "Human handoff",
        description: "Queue a HITL request for the council.",
        allowed_agents: &[AiAgentName::Orchestrator, AiAgentName::Receptionist],
        staff_only: false,
    },
];

/// ایجنت با نام.
#[must_use]
pub fn agent_of(name: AiAgentName) -> &'static AgentSpec {
    AGENTS.iter().find(|a| a.name == name).unwrap_or(&AGENTS[0])
}

/// مهارت با شناسه — `None` یعنی مهارت ناشناخته (fail-closed).
#[must_use]
pub fn skill_of(id: &str) -> Option<&'static SkillSpec> {
    SKILLS.iter().find(|s| s.id == id)
}

/// روتینگ قطعی — کلیدواژه‌محور (fa/en)؛ بدون امتیاز = هماهنگ‌کننده.
/// `direct_agent` درخواست، این روتینگ را می‌پراند.
#[must_use]
pub fn route_agent(text: &str) -> AiAgentName {
    let lower = text.to_lowercase();
    let mut best: Option<(usize, AiAgentName)> = None;
    for (agent, keywords) in ROUTING_KEYWORDS {
        let score = keywords.iter().filter(|k| lower.contains(*k)).count();
        if score > 0 && best.as_ref().is_none_or(|(b, _)| score > *b) {
            best = Some((score, *agent));
        }
    }
    best.map(|(_, a)| a).unwrap_or(AiAgentName::Orchestrator)
}

/// نمای وایر ایجنت‌ها — از رجیستری واحد (عین خروجی فاز ۴).
#[must_use]
pub fn agent_views() -> Vec<AiAgentView> {
    AGENTS
        .iter()
        .map(|a| AiAgentView {
            name: a.name,
            emoji: a.emoji.into(),
            title_fa: a.title_fa.into(),
            title_en: a.title_en.into(),
            description: a.description.into(),
            skills: a.skills.iter().map(|s| (*s).to_owned()).collect(),
        })
        .collect()
}

/// نمای وایر مهارت‌ها — از رجیستری واحد (عین خروجی فاز ۴).
#[must_use]
pub fn skill_views() -> Vec<AiSkillView> {
    SKILLS
        .iter()
        .map(|s| AiSkillView {
            id: s.id.into(),
            kind: s.kind,
            title_fa: s.title_fa.into(),
            title_en: s.title_en.into(),
            description: s.description.into(),
            allowed_agents: s.allowed_agents.to_vec(),
            staff_only: s.staff_only,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dropagentx_contracts::AiAgentName::*;

    #[test]
    fn six_agents_sixteen_skills() {
        assert_eq!(AGENTS.len(), 6);
        assert_eq!(SKILLS.len(), 16);
    }

    #[test]
    fn agent_skill_lists_match_skill_registry() {
        // هر مهارتِ فهرست‌شده در ایجنت باید در رجیستری باشد و ایجنت را مجاز بداند
        for a in AGENTS {
            for s in a.skills {
                let spec = skill_of(s).unwrap_or_else(|| panic!("skill {s} not registered"));
                assert!(
                    spec.allowed_agents.contains(&a.name),
                    "skill {s} does not allow agent {}",
                    a.name.as_str()
                );
            }
        }
    }

    #[test]
    fn money_skills_with_no_agents_are_never_callable() {
        for id in ["wallet_mint", "wallet_burn"] {
            let s = skill_of(id).expect("registered");
            assert!(s.allowed_agents.is_empty(), "{id} باید بدون ایجنت مجاز باشد");
            assert_eq!(s.kind, AiSkillKind::Money);
        }
    }

    #[test]
    fn staff_only_flags_match_wire_shape() {
        assert!(skill_of("product.create").unwrap().staff_only);
        assert!(skill_of("kb.upsert").unwrap().staff_only);
        assert!(skill_of("escrow_refund").unwrap().staff_only);
        assert!(skill_of("escrow_release").unwrap().staff_only);
        assert!(!skill_of("ai.handoff").unwrap().staff_only);
        assert!(!skill_of("wallet.balance").unwrap().staff_only);
    }

    #[test]
    fn routing_is_deterministic_fa_and_en() {
        assert_eq!(route_agent("می‌خوام این محصول رو بخرم"), Sales);
        assert_eq!(route_agent("escrow dispute please"), Support);
        assert_eq!(route_agent("موجودی من چقدره؟"), Analyst);
        assert_eq!(route_agent("سلام!"), Receptionist);
        assert_eq!(route_agent("xqzt"), Orchestrator, "بدون امتیاز → هماهنگ‌کننده");
        assert_eq!(route_agent("buy"), Sales, "فقط یک کلیدواژه کافی است");
    }

    #[test]
    fn unknown_skill_is_none() {
        assert!(skill_of("nope.nope").is_none());
    }

    #[test]
    fn views_match_phase4_wire_shape() {
        let views = agent_views();
        assert_eq!(views.len(), 6);
        assert_eq!(views[0].name, Orchestrator);
        assert_eq!(views[0].emoji, "🧠");
        assert_eq!(skill_views().len(), 16);
    }
}
