//! موتور gateway — روتینگ، fallback قطعی سه‌اسلات، پاکت سخت، گیت مجوز و سقف.
//!
//! جریان یک فراخوانی (ADR-009 فاز ۶):
//!
//! ```text
//! ۱. گیت سقف روزانه (قبل از هر فراخوانی — تمام → 429 typed)
//! ۲. روتینگ ایجنت (direct_agent یا کلیدواژه‌محور قطعی)
//! ۳. redaction ورودی (۹ قاعده + شمارش)
//! ۴. حلقهٔ اسلات‌ها primary→secondary→fallback:
//!      پاکت reply  → redaction خروجی → ai.ok
//!      پاکت tool   → گیت مجوز رجیستری (allowed_agents + staff_only)
//!                    → backend.execute → ok/pending/failed trace
//!      پاکت خراب  → شمارنده؛ دو پیاپی → شکست تایپ‌شده (متن خام هرگز عبور نمی‌کند)
//!      خطای اسلات → اسلات بعدی؛ هیچ → AI_UPSTREAM
//! ۵. ثبت مصرف توکن (همیشه — حتی refused/failed)
//! ```
//!
//! ممیزی هر پاسخ `ai.ok|refused|failed|pending` در هندلر سرور نوشته می‌شود.

use dropagentx_contracts::{
    AiAgentName, AiChatRequest, AiChatResponse, AiToolTrace, AiUsage, AiVia,
};

use crate::error::GateError;
use crate::llm::{AnyProvider, LlmProvider};
use crate::packet::{parse_packet, Packet};
use crate::redaction::redact;
use crate::registry::{agent_of, route_agent, skill_of};
use crate::tools::{AiUsageRecord, ToolBackend, ToolCtx, ToolOutcome};

/// گیت‌وی — اسلات‌های مرتب + سقف روزانه.
pub struct Gateway {
    /// اسلات‌ها به ترتیب اولویت (primary اول)
    slots: Vec<AnyProvider>,
    /// سقف روزانهٔ توکن per-user (`AI_DAILY_TOKEN_CAP`)
    cap: i64,
}

impl std::fmt::Debug for Gateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // اسلات‌ها بدون نشت کلید چاپ می‌شوند (Debug ارائه‌دهنده‌ها هم امن است)
        f.debug_struct("Gateway")
            .field(
                "slots",
                &self.slots.iter().map(|s| (s.slot(), s.provider(), s.model())).collect::<Vec<_>>(),
            )
            .field("cap", &self.cap)
            .finish()
    }
}

/// نتیجهٔ ممیزی هر پاسخ — دقیقاً چهار وضعیت ADR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditKind {
    /// پاسخ/ابزار موفق
    Ok,
    /// گیت مجوز رد کرد
    Refused,
    /// مهارت شکست خورد
    Failed,
    /// منتظر تأیید HITL
    Pending,
}

impl AuditKind {
    /// رشتهٔ audit — `ai.ok|refused|failed|pending`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            AuditKind::Ok => "ai.ok",
            AuditKind::Refused => "ai.refused",
            AuditKind::Failed => "ai.failed",
            AuditKind::Pending => "ai.pending",
        }
    }
}

/// خروجی یک چت — پاسخ وایر + نوع ممیزی.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatOutcome {
    /// پاسخ کامل وایر
    pub response: AiChatResponse,
    /// نوع ممیزی
    pub audit: AuditKind,
}

impl Gateway {
    /// ساخت gateway — اسلات‌ها به ترتیب اولویت (حداقل یکی).
    #[must_use]
    pub fn new(slots: Vec<AnyProvider>, cap: i64) -> Self {
        Self { slots, cap }
    }

    /// آیا سیکل فعال است؟ (بدون اسلات = AI_DISABLED در هندلر)
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        !self.slots.is_empty()
    }

    /// اجرای یک چت کامل — قلب فاز ۶.
    ///
    /// # Errors
    /// [`GateError::CapExceeded`] قبل از هر فراخوانی؛ [`GateError::MalformedPacket`]
    /// پس از دو پاکت خراب؛ [`GateError::AllSlotsFailed`] وقتی همهٔ اسلات‌ها شکست
    /// خوردند؛ [`GateError::EmptyRequest`] وقتی متن redact‌شده خالی شد.
    pub async fn chat<B: ToolBackend>(
        &self,
        backend: &B,
        ctx: &ToolCtx,
        req: &AiChatRequest,
    ) -> Result<ChatOutcome, GateError> {
        // ۱) گیت سقف — قبل از هر فراخوانی provider
        let used = backend.tokens_today(&ctx.actor).await?;
        if used >= self.cap && self.cap > 0 {
            return Err(GateError::CapExceeded { used, cap: self.cap });
        }

        // ۲) روتینگ ایجنت — direct یا کلیدواژه‌محور قطعی
        let agent = req.direct_agent.unwrap_or_else(|| route_agent(&req.text));
        let spec = agent_of(agent);
        let language = if req.language.starts_with("en") { "en" } else { "fa" };

        // ۳) redaction ورودی — قبل از رسیدن به هر provider
        let redacted_user = redact(&req.text);
        let mut redactions = redacted_user.count;

        let system = build_system_prompt(spec.name, language);
        let user = build_user_prompt(req, &redacted_user.text, language);

        // ۴) حلقهٔ اسلات‌ها — fallback قطعی + «دو پاکت خراب» شکست تایپ‌شده.
        // هر اسلات حداکثر دو تلاش می‌گیرد (پاکت خراب → همان اسلات دوباره سؤال
        // می‌شود)؛ دو خراب پیاپی در هر جای زنجیره = AI_MALFORMED_PACKET.
        let mut malformed: usize = 0;
        let mut attempts: Vec<String> = Vec::new();
        for slot in &self.slots {
            let mut done: Option<(crate::llm::LlmReply, Packet)> = None;
            for _ in 0..2u8 {
                match slot.complete(&system, &user).await {
                    Ok(r) => match parse_packet(&r.text) {
                        Ok(packet) => {
                            done = Some((r, packet));
                            break;
                        }
                        Err(_) => {
                            malformed += 1;
                            if malformed >= 2 {
                                self.record(
                                    backend,
                                    ctx,
                                    agent,
                                    slot,
                                    r.prompt_tokens,
                                    r.completion_tokens,
                                    AuditKind::Failed,
                                    redactions,
                                )
                                .await;
                                return Err(GateError::MalformedPacket { count: malformed });
                            }
                        }
                    },
                    Err(e) => {
                        attempts.push(format!("{}: {e}", slot.slot()));
                        break;
                    }
                }
            }
            if let Some((reply, packet)) = done {
                match packet {
                    Packet::Reply(text) => {
                        // redaction خروجی — متن پاسخ قبل از خروج از پروسه پاک می‌شود
                        let clean = redact(&text);
                        redactions += clean.count;
                        let response = AiChatResponse {
                            reply: clean.text,
                            agent,
                            via: if req.direct_agent.is_some() {
                                AiVia::Direct
                            } else {
                                AiVia::Llm
                            },
                            provider: slot.provider().to_owned(),
                            tool_traces: Vec::new(),
                            usage: AiUsage {
                                provider: Some(slot.provider().to_owned()),
                                model: Some(slot.model().to_owned()),
                                prompt_tokens: reply.prompt_tokens,
                                completion_tokens: reply.completion_tokens,
                                total_tokens: reply.prompt_tokens + reply.completion_tokens,
                            },
                            pending_approval: false,
                            redactions,
                        };
                        self.record(
                            backend,
                            ctx,
                            agent,
                            slot,
                            reply.prompt_tokens,
                            reply.completion_tokens,
                            AuditKind::Ok,
                            redactions,
                        )
                        .await;
                        return Ok(ChatOutcome { response, audit: AuditKind::Ok });
                    }
                    Packet::Tool { skill, args } => {
                        return self
                            .run_tool(
                                backend, ctx, req, agent, slot, reply, skill, args, redactions,
                            )
                            .await;
                    }
                }
            }
        }
        Err(GateError::AllSlotsFailed { attempts: attempts.join("; ") })
    }

    /// اجرای پاکت ابزار — گیت مجوز رجیستری (fail-closed) + backend + trace.
    // پارامترها یک قالب منسجم اند (زمینهٔ یک فراخوانی ابزار) — تصمیم ثبت‌شدهٔ فاز ۶.
    #[allow(clippy::too_many_arguments)]
    async fn run_tool<B: ToolBackend>(
        &self,
        backend: &B,
        ctx: &ToolCtx,
        req: &AiChatRequest,
        agent: AiAgentName,
        slot: &AnyProvider,
        reply: crate::llm::LlmReply,
        skill: String,
        args: serde_json::Value,
        redactions: i64,
    ) -> Result<ChatOutcome, GateError> {
        let total = reply.prompt_tokens + reply.completion_tokens;
        let via = if req.direct_agent.is_some() { AiVia::Direct } else { AiVia::Llm };

        // گیت مجوز — قبل از هر اجرا؛ رد = پیام صادقانه + trace، هرگز شکست خام
        let refused_code = match skill_of(&skill) {
            None => Some(("UNKNOWN_SKILL", "This skill does not exist in the registry.")),
            Some(spec) if !spec.allowed_agents.contains(&agent) => {
                Some(("SKILL_NOT_ALLOWED", "This agent is not allowed to run this skill."))
            }
            Some(spec) if spec.staff_only && !ctx.staff => {
                Some(("UNAUTHORIZED_STAFF", "This skill requires staff authorization."))
            }
            _ => None,
        };
        let (outcome, audit) = if let Some((code, message)) = refused_code {
            let trace = AiToolTrace {
                skill: skill.clone(),
                ok: false,
                summary: message.to_owned(),
                pending_approval: false,
                code: Some(code.to_owned()),
            };
            let response = AiChatResponse {
                reply: format!("⚠️ {message}"),
                agent,
                via,
                provider: slot.provider().to_owned(),
                tool_traces: vec![trace],
                usage: AiUsage {
                    provider: Some(slot.provider().to_owned()),
                    model: Some(slot.model().to_owned()),
                    prompt_tokens: reply.prompt_tokens,
                    completion_tokens: reply.completion_tokens,
                    total_tokens: total,
                },
                pending_approval: false,
                redactions,
            };
            (response, AuditKind::Refused)
        } else {
            // گیت دوم داخل backend (authz واقعی روی داده) — fail-closed
            let result = backend.execute(ctx, &skill, &args).await;
            let (trace, reply_text, audit, pending) = match result {
                ToolOutcome::Data { value: _, summary } => (
                    AiToolTrace {
                        skill: skill.clone(),
                        ok: true,
                        summary: summary.clone(),
                        pending_approval: false,
                        code: None,
                    },
                    summary,
                    AuditKind::Ok,
                    false,
                ),
                ToolOutcome::Pending { summary } => (
                    AiToolTrace {
                        skill: skill.clone(),
                        ok: true,
                        summary: summary.clone(),
                        pending_approval: true,
                        code: Some("REQUIRES_HUMAN_APPROVAL".into()),
                    },
                    summary,
                    AuditKind::Pending,
                    true,
                ),
                ToolOutcome::Failed { code, message } => (
                    AiToolTrace {
                        skill: skill.clone(),
                        ok: false,
                        summary: message.clone(),
                        pending_approval: false,
                        code: Some(code),
                    },
                    message,
                    AuditKind::Failed,
                    false,
                ),
            };
            let response = AiChatResponse {
                reply: reply_text,
                agent,
                via,
                provider: slot.provider().to_owned(),
                tool_traces: vec![trace],
                usage: AiUsage {
                    provider: Some(slot.provider().to_owned()),
                    model: Some(slot.model().to_owned()),
                    prompt_tokens: reply.prompt_tokens,
                    completion_tokens: reply.completion_tokens,
                    total_tokens: total,
                },
                pending_approval: pending,
                redactions,
            };
            (response, audit)
        };

        self.record(
            backend,
            ctx,
            agent,
            slot,
            reply.prompt_tokens,
            reply.completion_tokens,
            audit,
            redactions,
        )
        .await;
        Ok(ChatOutcome { response: outcome, audit })
    }

    /// ثبت ردیف مصرف — شکستش مسیر را نمی‌بندد (فقط لاگ).
    #[allow(clippy::too_many_arguments)]
    async fn record<B: ToolBackend>(
        &self,
        backend: &B,
        ctx: &ToolCtx,
        agent: AiAgentName,
        slot: &AnyProvider,
        prompt_tokens: i64,
        completion_tokens: i64,
        audit: AuditKind,
        _redactions: i64,
    ) {
        // شمارش redaction خودش در پاسخ وایر و audit سرور می‌نشیند
        let record = AiUsageRecord {
            user_id: ctx.actor.clone(),
            agent: agent.as_str().to_owned(),
            provider: slot.provider().to_owned(),
            model: slot.model().to_owned(),
            kind: "chat".into(),
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            outcome: audit.as_str().to_owned(),
        };
        if let Err(e) = backend.record_usage(record).await {
            tracing::warn!(error = %e, "ai usage recording failed");
        }
    }
}

/// پرامپت سیستمی — نقش ایجنت + مهارت‌های مجاز + قرارداد پاکت + اعداد منشور.
fn build_system_prompt(agent: AiAgentName, language: &str) -> String {
    let spec = agent_of(agent);
    let base = if language == "en" { spec.system_en } else { spec.system_fa };
    let skills: Vec<String> = spec.skills.iter().map(|s| format!("- {s}")).collect();
    format!(
        "{base}\n\nAllowed skills:\n{}\n\nAnswer ONLY with a JSON packet:\n- {{\"reply\": \"…\"}} or\n- {{\"tool\": \"<skill>\", \"args\": {{…}}}}\nRaw text is never accepted.\nCharter facts: escrow settle = 90/10 (art. 2), HITL threshold = 50,000 Đ (art. 7), tax = 10% (art. 8). Language: {language}.",
        skills.join("\n")
    )
}

/// پرامپت کاربر — متن redact‌شده + تاریخچه + زمینهٔ دانش.
fn build_user_prompt(req: &AiChatRequest, redacted_text: &str, language: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("language: {language}\n"));
    if let Some(history) = &req.history {
        for m in history.iter().take(10) {
            let role = match m.role {
                dropagentx_contracts::HistoryRole::User => "user",
                dropagentx_contracts::HistoryRole::Assistant => "assistant",
            };
            out.push_str(&format!("history[{role}]: {}\n", redact(&m.content).text));
        }
    }
    if let Some(knowledge) = &req.knowledge {
        out.push_str(&format!("knowledge: {}\n", redact(knowledge).text));
    }
    out.push_str(&format!("user: {redacted_text}"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_contains_packet_contract_and_charter() {
        let p = build_system_prompt(AiAgentName::Sales, "fa");
        assert!(p.contains("{\"reply\""));
        assert!(p.contains("۵۰٬۰۰۰") || p.contains("50,000"));
        assert!(p.contains("product.search"));
        assert!(!p.contains("economy.summary"), "فقط مهارت‌های مجاز");
    }

    #[test]
    fn user_prompt_redacts_history_and_knowledge() {
        let req = AiChatRequest {
            user_id: dropagentx_contracts::Id::new("user:tg:1").expect("id"),
            init_data: None,
            role: Default::default(),
            text: "hi".into(),
            language: "fa".into(),
            direct_agent: None,
            history: Some(vec![dropagentx_contracts::HistoryMessage {
                role: dropagentx_contracts::HistoryRole::User,
                content: "my key sk-abcdefghijklmnop1234".into(),
            }]),
            knowledge: Some("Bearer zzzz-token-123456".into()),
        };
        let p = build_user_prompt(&req, "hi", "fa");
        assert!(p.contains("[REDACTED:API_KEY]"), "{p}");
        assert!(p.contains("[REDACTED:BEARER]"), "{p}");
        assert!(!p.contains("sk-abcdefghijklmnop"), "{p}");
    }
}
