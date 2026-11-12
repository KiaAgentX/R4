//! i18n fa/en — کاتالوگ پیام‌ها + نگاشت کد خطای وایر به پیام محلی.
//!
//! تصمیم ADR-009 فاز ۵: «i18n fa/en + نگاشت کد خطای وایر به پیام محلی».
//! لوکِیل پیش‌فرض `fa` است (جماعت هدف)، `language_code` کلاینت تلگرام hint
//! است و `/lang` انتخاب صریح را در session بات (Env) می‌نشاند.
//!
//! هر خطا ([`crate::env::BotError`]) به پیام محلی‌شده می‌رسد؛ کد وایر هرگز
//! مستقیم به کاربر نشان داده نمی‌شود (کد در audit/لاگ می‌ماند).

use dropagentx_domain::DomainErrorCode;

use crate::cmd::Cmd;
use crate::env::BotError;

/// زبان رابط بات.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    /// فارسی — پیش‌فرض
    Fa,
    /// English
    En,
}

impl Locale {
    /// از `language_code` تلگرام — `en*` → En، بقیه → Fa.
    #[must_use]
    pub fn from_language_code(code: Option<&str>) -> Self {
        match code.map(|c| c.trim().to_ascii_lowercase()) {
            Some(c) if c.starts_with("en") => Locale::En,
            _ => Locale::Fa,
        }
    }

    /// برچسب زبان برای پیام تعویض.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Locale::Fa => "فارسی",
            Locale::En => "English",
        }
    }

    /// زبان دیگر (toggle `/lang` بدون آرگومان).
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Locale::Fa => Locale::En,
            Locale::En => Locale::Fa,
        }
    }
}

/// فرمت Din — پوشش ایمن روی [`dropagentx_domain::format_din`]؛ مقادیر منفی
/// اینجا نمی‌آیند (سرخپست‌ها علامت را جدا مدیریت می‌کنند).
fn fmt_din(amount: i64) -> String {
    dropagentx_domain::Din::new(amount)
        .map(dropagentx_domain::format_din)
        .unwrap_or_else(|_| format!("{amount} \u{110}"))
}

// ── برچسب‌های کیبورد شیشه‌ای — برچسب = فرمان (نگاشت پایدار) ─────────────────

/// برچسب هر فرمان در کیبورد — fa و en.
#[must_use]
pub fn cmd_label(locale: Locale, cmd: Cmd) -> &'static str {
    match (locale, cmd) {
        (Locale::Fa, Cmd::Balance) => "💰 موجودی",
        (Locale::Fa, Cmd::History) => "📜 تاریخچه",
        (Locale::Fa, Cmd::Products) => "🛍 محصولات",
        (Locale::Fa, Cmd::Buy) => "🛒 خرید",
        (Locale::Fa, Cmd::Status) => "📦 وضعیت",
        (Locale::Fa, Cmd::Fund) => "🔐 تأمین مالی",
        (Locale::Fa, Cmd::Deliver) => "📤 تحویل",
        (Locale::Fa, Cmd::Release) => "🔓 آزادسازی",
        (Locale::Fa, Cmd::Dispute) => "⚖️ اختلاف",
        (Locale::Fa, Cmd::Cancel) => "✖️ لغو",
        (Locale::Fa, Cmd::Lang) => "🌐 زبان",
        (Locale::Fa, Cmd::Help) => "❓ راهنما",
        (Locale::Fa, Cmd::Queue) => "🗂 صف HITL",
        (Locale::Fa, Cmd::Economy) => "📊 اقتصاد",
        (Locale::En, Cmd::Balance) => "💰 Balance",
        (Locale::En, Cmd::History) => "📜 History",
        (Locale::En, Cmd::Products) => "🛍 Products",
        (Locale::En, Cmd::Buy) => "🛒 Buy",
        (Locale::En, Cmd::Status) => "📦 Status",
        (Locale::En, Cmd::Fund) => "🔐 Fund",
        (Locale::En, Cmd::Deliver) => "📤 Deliver",
        (Locale::En, Cmd::Release) => "🔓 Release",
        (Locale::En, Cmd::Dispute) => "⚖️ Dispute",
        (Locale::En, Cmd::Cancel) => "✖️ Cancel",
        (Locale::En, Cmd::Lang) => "🌐 Language",
        (Locale::En, Cmd::Help) => "❓ Help",
        (Locale::En, Cmd::Queue) => "🗂 HITL queue",
        (Locale::En, Cmd::Economy) => "📊 Economy",
        // فرمان‌های بدون دکمه (start/menu/help متن‌اند؛ approve/reject/audit
        // شناسه می‌خواهند — دکمهٔ امن ندارند)
        (_, Cmd::Start) => "/start",
        (_, Cmd::Menu) => "/menu",
        (_, Cmd::Approve) => "/approve",
        (_, Cmd::Reject) => "/reject",
        (_, Cmd::Audit) => "/audit",
    }
}

/// برچسب کیبورد → فرمان — مسیر رفت‌وبرگشتِ [`cmd_label`].
#[must_use]
pub fn label_to_cmd(label: &str) -> Option<Cmd> {
    let trimmed = label.trim();
    for locale in [Locale::Fa, Locale::En] {
        for cmd in Cmd::ALL {
            if cmd_label(locale, cmd) == trimmed {
                return Some(cmd);
            }
        }
    }
    None
}

// ── پیام‌ها ──────────────────────────────────────────────────────────────────

/// خوش‌آمد `/start`.
#[must_use]
pub fn welcome(locale: Locale, name: Option<&str>) -> String {
    let who = name.unwrap_or_default();
    match locale {
        Locale::Fa => format!(
            "سلام {who}! 👋\nبه «کشور Agent-One» خوش آمدی — منشور ۱۰ ماده‌ای، پول Din (Đ) و امانت ۸ مرحله‌ای.\n\nاز منو زیر استفاده کن؛ برای شروع خرید: /buy <شناسه محصول>"
        ),
        Locale::En => format!(
            "Hi {who}! 👋\nWelcome to the Agent-One nation — a 10-article charter, Din (Đ) money and an 8-state escrow.\n\nUse the menu below; to start buying: /buy <productId>"
        ),
    }
}

/// راهنمای کامل — ۱۴ دستور کاربر (+۵ ادمین برای ادمین‌ها).
#[must_use]
pub fn help(locale: Locale, show_admin: bool) -> String {
    let mut lines: Vec<String> = Vec::new();
    match locale {
        Locale::Fa => {
            lines.push("راهنمای بات Agent-One:".into());
            lines.push("/start — خوش‌آمد".into());
            lines.push("/menu — منو".into());
            lines.push("/balance — موجودی کیف".into());
            lines.push("/history — تاریخچهٔ تراکنش‌ها".into());
            lines.push("/products — محصولات فعال".into());
            lines.push("/buy <productId> | /buy <sellerTgId> <amount> — خرید".into());
            lines.push("/status <escrowId> — وضعیت امانت".into());
            lines.push("/fund [escrowId] — تأمین مالی (خریدار)".into());
            lines.push("/deliver [escrowId] — تحویل (فروشنده)".into());
            lines.push("/release [escrowId] — تسویه (خریدار؛ ≥۵۰٬۰۰۰ Đ صف HITL)".into());
            lines.push("/dispute [escrowId] — اختلاف (خریدار)".into());
            lines.push("/cancel [escrowId] — لغو (خریدار)".into());
            lines.push("/lang fa|en — زبان".into());
        }
        Locale::En => {
            lines.push("Agent-One bot guide:".into());
            lines.push("/start — welcome".into());
            lines.push("/menu — menu".into());
            lines.push("/balance — wallet balance".into());
            lines.push("/history — transaction history".into());
            lines.push("/products — active products".into());
            lines.push("/buy <productId> | /buy <sellerTgId> <amount> — buy".into());
            lines.push("/status <escrowId> — escrow status".into());
            lines.push("/fund [escrowId] — fund escrow (buyer)".into());
            lines.push("/deliver [escrowId] — deliver (seller)".into());
            lines.push("/release [escrowId] — settle (buyer; ≥50,000 Đ goes to HITL)".into());
            lines.push("/dispute [escrowId] — dispute (buyer)".into());
            lines.push("/cancel [escrowId] — cancel (buyer)".into());
            lines.push("/lang fa|en — language".into());
        }
    }
    if show_admin {
        match locale {
            Locale::Fa => {
                lines.push("— فرمان‌های ادمین —".into());
                lines.push("/queue — صف HITL".into());
                lines.push("/approve <hitlId> — تأیید → تسویهٔ ۹۰/۱۰".into());
                lines.push("/reject <hitlId> — رد".into());
                lines.push("/economy — خلاصهٔ اقتصاد".into());
                lines.push("/audit — انتهای audit".into());
            }
            Locale::En => {
                lines.push("— admin commands —".into());
                lines.push("/queue — HITL queue".into());
                lines.push("/approve <hitlId> — approve → 90/10 settle".into());
                lines.push("/reject <hitlId> — reject".into());
                lines.push("/economy — economy summary".into());
                lines.push("/audit — audit tail".into());
            }
        }
    }
    lines.join("\n")
}

/// hint متن ناشناخته — منو را دوباره نشان می‌دهد.
#[must_use]
pub fn unknown_text(locale: Locale) -> String {
    match locale {
        Locale::Fa => "این را نمی‌فهمم — از منو زیر استفاده کن یا /help بزن.".into(),
        Locale::En => "I don't understand that — use the menu below or /help.".into(),
    }
}

/// تعویض زبان.
#[must_use]
pub fn lang_switched(locale: Locale) -> String {
    match locale {
        Locale::Fa => format!("زبان به {} تغییر کرد.", locale.name()),
        Locale::En => format!("Language switched to {}.", locale.name()),
    }
}

/// موجودی.
#[must_use]
pub fn balance_msg(locale: Locale, amount: i64) -> String {
    let din = fmt_din(amount);
    match locale {
        Locale::Fa => format!("💰 موجودی کیف شما: {din}"),
        Locale::En => format!("💰 Your wallet balance: {din}"),
    }
}

/// موجودی — تاریخچه.
#[must_use]
pub fn history_msg(
    locale: Locale,
    account: &str,
    items: &[(String, i64, Option<String>)],
) -> String {
    if items.is_empty() {
        return match locale {
            Locale::Fa => "📜 هنوز تراکنشی نداری.".into(),
            Locale::En => "📜 No transactions yet.".into(),
        };
    }
    let head = match locale {
        Locale::Fa => format!("📜 تاریخچهٔ {account}:"),
        Locale::En => format!("📜 History of {account}:"),
    };
    let body = items
        .iter()
        .map(|(tx, amount, memo)| {
            let sign = if *amount < 0 { "−" } else { "+" };
            let din = fmt_din(amount.abs());
            let m = memo.clone().unwrap_or_default();
            format!("• {tx}: {sign}{din} {m}").trim_end().to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{head}\n{body}")
}

/// فهرست محصولات.
#[must_use]
pub fn products_msg(locale: Locale, items: &[(String, String, i64)]) -> String {
    if items.is_empty() {
        return match locale {
            Locale::Fa => "🛍 فعلاً محصول فعالی نیست.".into(),
            Locale::En => "🛍 No active products yet.".into(),
        };
    }
    let head: &str = match locale {
        Locale::Fa => "🛍 محصولات فعال:",
        Locale::En => "🛍 Active products:",
    };
    let body = items
        .iter()
        .map(|(id, title, price)| format!("• {id} — {title} ({})", fmt_din(*price)))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{head}\n{body}")
}

/// نمای وضعیت امانت.
#[must_use]
pub fn escrow_msg(locale: Locale, e: &crate::env::EscrowItem) -> String {
    let din = fmt_din(e.amount);
    match locale {
        Locale::Fa => format!(
            "📦 امانت {}\nوضعیت: {}\nمبلغ: {din}\nخریدار: {}\nفروشنده: {}",
            e.id, e.status, e.buyer_id, e.seller_id
        ),
        Locale::En => format!(
            "📦 Escrow {}\nStatus: {}\nAmount: {din}\nBuyer: {}\nSeller: {}",
            e.id, e.status, e.buyer_id, e.seller_id
        ),
    }
}

/// امانت ساخته شد — راهنمای گام بعد.
#[must_use]
pub fn escrow_created(locale: Locale, e: &crate::env::EscrowItem) -> String {
    let din = fmt_din(e.amount);
    match locale {
        Locale::Fa => format!(
            "🛒 امانت {} ساخته شد ({din}).\nگام بعد: /fund {} — پول در امانت قفل می‌شود.",
            e.id, e.id
        ),
        Locale::En => format!(
            "🛒 Escrow {} created ({din}).\nNext: /fund {} — funds get locked in escrow.",
            e.id, e.id
        ),
    }
}

/// تأمین مالی شد.
#[must_use]
pub fn escrow_funded(locale: Locale, e: &crate::env::EscrowItem, replayed: bool) -> String {
    let din = fmt_din(e.amount);
    if replayed {
        return match locale {
            Locale::Fa => {
                format!("ℹ️ این پیام قبلاً پردازش شده — امانت {} همان‌جا تأمین شده بود.", e.id)
            }
            Locale::En => {
                format!("ℹ️ This message was already processed — escrow {} was funded then.", e.id)
            }
        };
    }
    match locale {
        Locale::Fa => format!(
            "🔐 امانت {e} تأمین شد ({din} قفل شد).\nگام بعد: فروشنده /deliver {e} می‌زند.",
            e = e.id
        ),
        Locale::En => format!(
            "🔐 Escrow {e} funded ({din} locked).\nNext: seller runs /deliver {e}.",
            e = e.id
        ),
    }
}

/// تحویل ثبت شد.
#[must_use]
pub fn escrow_delivered(locale: Locale, e: &crate::env::EscrowItem) -> String {
    match locale {
        Locale::Fa => format!(
            "📤 تحویل امانت {e} ثبت شد.\nگام بعد: خریدار /release {e} می‌زند (تسویهٔ ۹۰/۱۰).",
            e = e.id
        ),
        Locale::En => format!(
            "📤 Delivery recorded for escrow {e}.\nNext: buyer runs /release {e} (90/10 settle).",
            e = e.id
        ),
    }
}

/// تسویه شد — تقسیم دقیق ۹۰/۱۰.
#[must_use]
pub fn escrow_released(
    locale: Locale,
    e: &crate::env::EscrowItem,
    seller_net: i64,
    tax: i64,
) -> String {
    let net = fmt_din(seller_net);
    let tax_din = fmt_din(tax);
    match locale {
        Locale::Fa => format!(
            "🔓 امانت {} تسویه شد — دقیقاً ۹۰/۱۰ (مادهٔ ۲):\nسهم فروشنده: {net}\nمالیات خزانه (۱۰٪ مادهٔ ۸): {tax_din}",
            e.id
        ),
        Locale::En => format!(
            "🔓 Escrow {} settled — exactly 90/10 (art. 2):\nSeller share: {net}\nTreasury tax (10%, art. 8): {tax_din}",
            e.id
        ),
    }
}

/// اختلاف ثبت شد.
#[must_use]
pub fn escrow_disputed(locale: Locale, e: &crate::env::EscrowItem, reason: &str) -> String {
    match locale {
        Locale::Fa => {
            format!("⚖️ اختلاف روی امانت {} ثبت شد — پول قفل می‌ماند. دلیل: {reason}", e.id)
        }
        Locale::En => {
            format!("⚖️ Dispute recorded on escrow {} — funds stay locked. Reason: {reason}", e.id)
        }
    }
}

/// لغو شد.
#[must_use]
pub fn escrow_cancelled(locale: Locale, e: &crate::env::EscrowItem) -> String {
    match locale {
        Locale::Fa => format!("✖️ امانت {} لغو شد — پولی جابه‌جا نشده بود.", e.id),
        Locale::En => format!("✖️ Escrow {} cancelled — no money had moved.", e.id),
    }
}

/// صف HITL — درخواست ثبت شد (مادهٔ ۷).
#[must_use]
pub fn hitl_queued(locale: Locale, h: &crate::env::HitlItem) -> String {
    let din = fmt_din(h.amount);
    match locale {
        Locale::Fa => format!(
            "🛡 مادهٔ ۷ — تسویهٔ {} ({din}) بالای آستانهٔ ۵۰٬۰۰۰ Đ است و به تأیید انسانی نیاز دارد.\nدرخواست {} به صف HITL رفت (TTL ۲۴ ساعت).",
            h.escrow_id, h.id
        ),
        Locale::En => format!(
            "🛡 Art. 7 — settling {} ({din}) is above the 50,000 Đ threshold and needs human approval.\nRequest {} joined the HITL queue (24h TTL).",
            h.escrow_id, h.id
        ),
    }
}

/// فهرست صف HITL.
#[must_use]
pub fn hitl_list_msg(locale: Locale, items: &[crate::env::HitlItem]) -> String {
    if items.is_empty() {
        return match locale {
            Locale::Fa => "🗂 صف HITL خالی است.".into(),
            Locale::En => "🗂 HITL queue is empty.".into(),
        };
    }
    let head: &str = match locale {
        Locale::Fa => "🗂 صف HITL (PENDING):",
        Locale::En => "🗂 HITL queue (PENDING):",
    };
    let body = items
        .iter()
        .map(|h| {
            format!(
                "• {} — امانت {} ({}) — انقضا: {}",
                h.id,
                h.escrow_id,
                fmt_din(h.amount),
                h.expires_at
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{head}\n{body}")
}

/// تأیید شد → تسویهٔ ۹۰/۱۰.
#[must_use]
pub fn hitl_approved(locale: Locale, o: &crate::env::HitlOutcome) -> String {
    let net = fmt_din(o.seller_net.unwrap_or(0));
    let tax = fmt_din(o.tax.unwrap_or(0));
    match locale {
        Locale::Fa => format!(
            "✅ درخواست {} تأیید شد — تسویهٔ اتمیک در همان تراکنش:\nسهم فروشنده: {net}\nمالیات خزانه: {tax}",
            o.id
        ),
        Locale::En => format!(
            "✅ Request {} approved — atomic settle in the same transaction:\nSeller share: {net}\nTreasury tax: {tax}",
            o.id
        ),
    }
}

/// رد شد — هیچ پولی جابه‌جا نمی‌شود.
#[must_use]
pub fn hitl_rejected(locale: Locale, id: &str) -> String {
    match locale {
        Locale::Fa => format!("🚫 درخواست {id} رد شد — هیچ پولی جابه‌جا نمی‌شود."),
        Locale::En => format!("🚫 Request {id} rejected — no money moves."),
    }
}

/// خلاصهٔ اقتصاد.
#[must_use]
pub fn economy_msg(locale: Locale, e: &crate::env::EconomyItem) -> String {
    let fmt = |v: i64| fmt_din(v);
    match locale {
        Locale::Fa => format!(
            "📊 اقتصاد کشور:\nپول در گردش: {}\nخزانه: {}\nقفل در امانت: {}\nΣ کیف شهروندان: {}\nاسکروهای باز: {}\nتراز آزمایشی متوازن: {}",
            fmt(e.circulating_supply),
            fmt(e.treasury_balance),
            fmt(e.escrow_locked),
            fmt(e.users_sum),
            e.open_escrows,
            if e.balanced { "بله ✓" } else { "خیر ✗" }
        ),
        Locale::En => format!(
            "📊 Nation economy:\nCirculating: {}\nTreasury: {}\nLocked in escrow: {}\nΣ citizen wallets: {}\nOpen escrows: {}\nTrial balance even: {}",
            fmt(e.circulating_supply),
            fmt(e.treasury_balance),
            fmt(e.escrow_locked),
            fmt(e.users_sum),
            e.open_escrows,
            if e.balanced { "yes ✓" } else { "no ✗" }
        ),
    }
}

/// انتهای audit.
#[must_use]
pub fn audit_msg(locale: Locale, items: &[String]) -> String {
    if items.is_empty() {
        return match locale {
            Locale::Fa => "🧾 audit خالی است.".into(),
            Locale::En => "🧾 Audit log is empty.".into(),
        };
    }
    let head: &str = match locale {
        Locale::Fa => "🧾 انتهای audit:",
        Locale::En => "🧾 Audit tail:",
    };
    let body = items.iter().map(|i| format!("• {i}")).collect::<Vec<_>>().join("\n");
    format!("{head}\n{body}")
}

// ── نگاشت خطای وایر به پیام محلی ────────────────────────────────────────────

/// پیام محلی برای هر خطای بات — کد وایر → متن fa/en.
#[must_use]
pub fn error_msg(locale: Locale, err: &BotError) -> String {
    match err {
        BotError::Domain(code) => domain_error_msg(locale, *code),
        BotError::NotAdmin => match locale {
            Locale::Fa => "⛔ این فرمان فقط برای ادمین‌هاست.".into(),
            Locale::En => "⛔ This command is admin-only.".into(),
        },
        BotError::BuyerOnly => match locale {
            Locale::Fa => "⛔ فقط خریدارِ همین امانت مجاز به این کار است (fail-closed).".into(),
            Locale::En => "⛔ Only the escrow's buyer may do this (fail-closed).".into(),
        },
        BotError::SellerOnly => match locale {
            Locale::Fa => "⛔ فقط فروشندهٔ همین امانت مجاز به این کار است (fail-closed).".into(),
            Locale::En => "⛔ Only the escrow's seller may do this (fail-closed).".into(),
        },
        BotError::NotParty => match locale {
            Locale::Fa => "⛔ شما نه خریدار این امانت هستید نه فروشنده‌اش.".into(),
            Locale::En => "⛔ You are neither the buyer nor the seller of this escrow.".into(),
        },
        BotError::NoEligibleEscrow => match locale {
            Locale::Fa => {
                "🤷 امانت بازیِ متناسب با این کار پیدا نشد — شناسه را صریح بده: /fund <escrowId>"
                    .into()
            }
            Locale::En => {
                "🤷 No open escrow eligible for this — pass the id explicitly: /fund <escrowId>"
                    .into()
            }
        },
        BotError::EscrowNotFound => match locale {
            Locale::Fa => "🤷 امانت پیدا نشد — شناسه را با /status <escrowId> چک کن.".into(),
            Locale::En => "🤷 Escrow not found — check the id with /status <escrowId>.".into(),
        },
        BotError::BadArgs(usage) => match locale {
            Locale::Fa => format!("⚠️ آرگومان نامعتبر.\nاستفادهٔ درست: {usage}"),
            Locale::En => format!("⚠️ Invalid arguments.\nCorrect usage: {usage}"),
        },
        BotError::Backend => match locale {
            Locale::Fa => "🚨 خطای زیرساخت — چند لحظه بعد دوباره تلاش کن.".into(),
            Locale::En => "🚨 Backend error — please try again shortly.".into(),
        },
        BotError::UnknownCommand(_) => unknown_text(locale),
    }
}

/// نگاشت ۲۰ کد دامنه → پیام محلی (بدنهٔ فاز ۳: ۴۰۴/۴۲۲/۵۰۰ روی سیم؛
/// اینجا فقط متن کاربر).
#[must_use]
pub fn domain_error_msg(locale: Locale, code: DomainErrorCode) -> String {
    match (locale, code) {
        (Locale::Fa, DomainErrorCode::UnknownAccount) => {
            "🤷 حساب شما در کشور موجود نیست — اول ادمین برای شما کیف بسازد.".into()
        }
        (Locale::Fa, DomainErrorCode::EscrowNotFound) => {
            "🤷 امانت پیدا نشد — شناسه را چک کن.".into()
        }
        (Locale::Fa, DomainErrorCode::EscrowAlreadyExists) => {
            "🛡 امانت با این شناسه از قبل وجود دارد.".into()
        }
        (Locale::Fa, DomainErrorCode::InvalidTransition) => {
            "🚦 این گذار در وضعیت فعلی امانت مجاز نیست (ماشین ۸ حالته).".into()
        }
        (Locale::Fa, DomainErrorCode::RequiresHumanApproval) => {
            "🛡 این مبلغ بالای آستانهٔ ۵۰٬۰۰۰ Đ است — باید از صف HITL تأیید ادمین بگذرد.".into()
        }
        (Locale::Fa, DomainErrorCode::HitlNotFound) => {
            "🤷 درخواست HITL پیدا نشد — با /queue فهرست را ببین.".into()
        }
        (Locale::Fa, DomainErrorCode::HitlAlreadyDecided) => {
            "🚫 این درخواست قبلاً تصمیم‌گیری شده — تصمیم دوم بی‌اثر است (بازپرداخت صفر).".into()
        }
        (Locale::Fa, DomainErrorCode::HitlExpired) => {
            "⏰ درخواست HITL منقضی شده (TTL ۲۴ ساعت) — باید درخواست تازه ساخته شود.".into()
        }
        (Locale::Fa, DomainErrorCode::HitlNotRequired) => {
            "ℹ️ این مبلغ زیر آستانهٔ HITL است — مستقیم تسویه می‌شود.".into()
        }
        (Locale::Fa, DomainErrorCode::AccountOverdraft) => {
            "💸 موجودی کافی نیست — کیف هرگز منفی نمی‌شود.".into()
        }
        (Locale::Fa, DomainErrorCode::MintCapExceeded) => "🏦 سقف ضرب سالانهٔ مادهٔ ۳ پر است.".into(),
        (Locale::Fa, DomainErrorCode::ProductNotFound) => {
            "🤷 محصول پیدا نشد — با /products فهرست را ببین.".into()
        }
        (Locale::Fa, DomainErrorCode::InvalidAccountKind) => {
            "⛔ نوع حساب مجاز نیست — خریدار و فروشنده باید هر دو کاربر باشند.".into()
        }
        (Locale::Fa, DomainErrorCode::DuplicateAccount) => "🛡 این حساب قبلاً ساخته شده.".into(),
        (Locale::Fa, DomainErrorCode::NonIntegerAmount)
        | (Locale::Fa, DomainErrorCode::NegativeAmount)
        | (Locale::Fa, DomainErrorCode::AmountOutOfRange) => {
            "⚠️ مبلغ باید عدد صحیح مثبت در مرز امن (≤ ۲^۵۳−۱) باشد.".into()
        }
        (Locale::Fa, _) => "🚨 خطای داخلی — این خطا نباید به کاربر برسد؛ در audit ثبت شد.".into(),
        (Locale::En, DomainErrorCode::UnknownAccount) => {
            "🤷 Your account does not exist in the nation yet — ask an admin.".into()
        }
        (Locale::En, DomainErrorCode::EscrowNotFound) => {
            "🤷 Escrow not found — check the id.".into()
        }
        (Locale::En, DomainErrorCode::EscrowAlreadyExists) => {
            "🛡 An escrow with this id already exists.".into()
        }
        (Locale::En, DomainErrorCode::InvalidTransition) => {
            "🚦 This transition is not allowed in the escrow's current state (8-state machine)."
                .into()
        }
        (Locale::En, DomainErrorCode::RequiresHumanApproval) => {
            "🛡 This amount is above the 50,000 Đ threshold — it must pass the HITL queue.".into()
        }
        (Locale::En, DomainErrorCode::HitlNotFound) => {
            "🤷 HITL request not found — see /queue for the list.".into()
        }
        (Locale::En, DomainErrorCode::HitlAlreadyDecided) => {
            "🚫 This request was already decided — a second decision is void (zero refund).".into()
        }
        (Locale::En, DomainErrorCode::HitlExpired) => {
            "⏰ The HITL request expired (24h TTL) — a fresh request is needed.".into()
        }
        (Locale::En, DomainErrorCode::HitlNotRequired) => {
            "ℹ️ This amount is below the HITL threshold — it settles directly.".into()
        }
        (Locale::En, DomainErrorCode::AccountOverdraft) => {
            "💸 Insufficient balance — wallets never go negative.".into()
        }
        (Locale::En, DomainErrorCode::MintCapExceeded) => {
            "🏦 The yearly mint cap (art. 3) is exhausted.".into()
        }
        (Locale::En, DomainErrorCode::ProductNotFound) => {
            "🤷 Product not found — see /products for the list.".into()
        }
        (Locale::En, DomainErrorCode::InvalidAccountKind) => {
            "⛔ Invalid account kind — buyer and seller must both be users.".into()
        }
        (Locale::En, DomainErrorCode::DuplicateAccount) => "🛡 This account already exists.".into(),
        (Locale::En, DomainErrorCode::NonIntegerAmount)
        | (Locale::En, DomainErrorCode::NegativeAmount)
        | (Locale::En, DomainErrorCode::AmountOutOfRange) => {
            "⚠️ Amount must be a positive integer within the safe bound (≤ 2^53−1).".into()
        }
        (Locale::En, _) => {
            "🚨 Internal error — this should never reach the user; logged to audit.".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_code_mapping() {
        assert_eq!(Locale::from_language_code(Some("fa")), Locale::Fa);
        assert_eq!(Locale::from_language_code(Some("en-US")), Locale::En);
        assert_eq!(Locale::from_language_code(None), Locale::Fa, "پیش‌فرض fa");
        assert_eq!(Locale::from_language_code(Some("de")), Locale::Fa);
    }

    #[test]
    fn labels_roundtrip_fa_and_en() {
        for locale in [Locale::Fa, Locale::En] {
            for cmd in Cmd::ALL {
                let label = cmd_label(locale, cmd);
                assert_eq!(label_to_cmd(label), Some(cmd), "برچسب «{label}» باید برگردد");
            }
        }
    }

    #[test]
    fn labels_are_unique_within_locale() {
        for locale in [Locale::Fa, Locale::En] {
            let mut labels: Vec<&str> = Cmd::ALL.iter().map(|c| cmd_label(locale, *c)).collect();
            labels.sort_unstable();
            assert!(labels.windows(2).all(|w| w[0] != w[1]));
        }
    }

    #[test]
    fn error_map_covers_every_domain_code() {
        for code in [
            DomainErrorCode::NonIntegerAmount,
            DomainErrorCode::NegativeAmount,
            DomainErrorCode::AmountOutOfRange,
            DomainErrorCode::BpsOutOfRange,
            DomainErrorCode::TooFewPostings,
            DomainErrorCode::UnbalancedTransaction,
            DomainErrorCode::UnknownAccount,
            DomainErrorCode::InvalidAccountKind,
            DomainErrorCode::DuplicateAccount,
            DomainErrorCode::AccountOverdraft,
            DomainErrorCode::InvalidTransition,
            DomainErrorCode::EscrowNotFound,
            DomainErrorCode::EscrowAlreadyExists,
            DomainErrorCode::RequiresHumanApproval,
            DomainErrorCode::MintCapExceeded,
            DomainErrorCode::HitlNotFound,
            DomainErrorCode::HitlNotRequired,
            DomainErrorCode::HitlAlreadyDecided,
            DomainErrorCode::HitlExpired,
            DomainErrorCode::ProductNotFound,
        ] {
            let fa = domain_error_msg(Locale::Fa, code);
            let en = domain_error_msg(Locale::En, code);
            assert!(!fa.is_empty());
            assert!(!en.is_empty());
            assert_ne!(fa, en, "پیام fa و en باید متمایز باشند برای {code}");
        }
    }

    #[test]
    fn money_formats_with_thousand_separator() {
        let text = balance_msg(Locale::Fa, 187_672);
        assert!(text.contains("187,672 Đ"), "قالب Din فاز ۱ حفظ می‌شود: {text}");
    }
}
