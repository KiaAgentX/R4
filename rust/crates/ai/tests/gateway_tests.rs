//! تست‌های بستهٔ gateway — با fake-LLM بدون شبکه (معیار پذیرش فاز ۶).
//!
//! پوشش:
//! * پاکت سخت: reply/tool/پاکت خراب ×۲ = شکست تایپ‌شده؛ متن خام هرگز عبور نمی‌کند
//! * fallback قطعی: قطع primary → secondary جواب می‌دهد؛ همه قطع → AI_UPSTREAM
//! * گیت سقف: تمام → خطای تایپ‌شده **قبل از provider** (شمارندهٔ فراخوانی صفر)
//! * گیت مجوز: unknown/not-allowed/staff-only → refused trace، هرگز اجرا
//! * redaction: ورود (پرامپت) و خروج (reply) با شمارش
//! * usage: ثبت ردیف با outcome و توکن دقیق در هر چهار وضعیت ممیزی
//! * روتینگ: direct_agent / کلیدواژه fa/en / پیش‌فرض هماهنگ‌کننده

// تست‌بندی — unwrap/expect عمدی‌اند: شکست یعنی خود تست شکست خورده
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::sync::Mutex;

use dropagentx_ai::{
    AiUsageRecord, AnyProvider, AuditKind, ChatOutcome, FakeProvider, GateError, Gateway,
    ToolBackend, ToolCtx, ToolOutcome,
};
use dropagentx_contracts::{AiAgentName, AiChatRequest, HistoryMessage, HistoryRole, Id};

const ACTOR: &str = "user:tg:1001";
const CAP: i64 = 1_000_000;

/// backend درون‌حافظه‌ای — خواندنی/پولی/سقف، همه با شمارنده‌ها.
#[derive(Debug, Default)]
struct MemoryAiBackend {
    used: Mutex<i64>,
    records: Mutex<Vec<AiUsageRecord>>,
    balances: Mutex<BTreeMap<String, i64>>,
    escrow_status: Mutex<BTreeMap<String, String>>,
    hitl: Mutex<Vec<(String, String)>>,
}

impl MemoryAiBackend {
    fn seed_balance(&self, account: &str, amount: i64) {
        if let Ok(mut m) = self.balances.lock() {
            m.insert(account.to_owned(), amount);
        }
    }

    fn seed_escrow(&self, id: &str, status: &str) {
        if let Ok(mut m) = self.escrow_status.lock() {
            m.insert(id.to_owned(), status.to_owned());
        }
    }

    fn set_used(&self, value: i64) {
        if let Ok(mut u) = self.used.lock() {
            *u = value;
        }
    }

    fn hitl_len(&self) -> usize {
        self.hitl.lock().map(|h| h.len()).unwrap_or(0)
    }
}

impl ToolBackend for MemoryAiBackend {
    async fn execute(&self, ctx: &ToolCtx, skill: &str, args: &serde_json::Value) -> ToolOutcome {
        match skill {
            "wallet.balance" => {
                let balances = self.balances.lock().map(|m| m.clone()).unwrap_or_default();
                let b = balances.get(&ctx.actor).copied().unwrap_or(0);
                ToolOutcome::Data {
                    value: serde_json::json!({ "account": ctx.actor, "balance": b }),
                    summary: format!("Balance of {} is {b} Đ", ctx.actor),
                }
            }
            "escrow.status" => {
                let escrows = self.escrow_status.lock().map(|m| m.clone()).unwrap_or_default();
                let id = args.get("escrowId").and_then(|v| v.as_str()).unwrap_or("e1");
                match escrows.get(id) {
                    Some(status) => ToolOutcome::Data {
                        value: serde_json::json!({ "id": id, "status": status }),
                        summary: format!("Escrow {id} is {status}"),
                    },
                    None => ToolOutcome::Failed {
                        code: "ESCROW_NOT_FOUND".into(),
                        message: format!("Escrow {id} not found"),
                    },
                }
            }
            "escrow_refund" => {
                // معیار پذیرش: بازپرداخت ۶۰٬۰۰۰ → ردیف HITL؛ اسکرو دست‌نخورده
                let id = args.get("escrowId").and_then(|v| v.as_str()).unwrap_or("e1");
                if let Ok(mut h) = self.hitl.lock() {
                    h.push(("ESCROW_REFUND".into(), id.to_owned()));
                }
                ToolOutcome::Pending {
                    summary: format!("Refund of escrow {id} queued for human approval (HITL)."),
                }
            }
            "escrow_release" => {
                let id = args.get("escrowId").and_then(|v| v.as_str()).unwrap_or("e1");
                let escrows = self.escrow_status.lock().map(|m| m.clone()).unwrap_or_default();
                match escrows.get(id).map(String::as_str) {
                    Some("DELIVERED") | Some("DISPUTED") => ToolOutcome::Data {
                        value: serde_json::json!({ "id": id, "status": "RELEASED" }),
                        summary: format!("Escrow {id} settled 90/10."),
                    },
                    Some(other) => ToolOutcome::Failed {
                        code: "INVALID_TRANSITION".into(),
                        message: format!("Cannot release escrow in status {other}"),
                    },
                    None => ToolOutcome::Failed {
                        code: "ESCROW_NOT_FOUND".into(),
                        message: format!("Escrow {id} not found"),
                    },
                }
            }
            "ai.handoff" => {
                if let Ok(mut h) = self.hitl.lock() {
                    h.push(("HUMAN_HANDOFF".into(), ctx.actor.clone()));
                }
                ToolOutcome::Pending { summary: "Queued for the council (human handoff).".into() }
            }
            _ => ToolOutcome::Failed {
                code: "UNKNOWN_SKILL".into(),
                message: "unimplemented in fake backend".into(),
            },
        }
    }

    async fn tokens_today(&self, _user_id: &str) -> Result<i64, GateError> {
        Ok(self.used.lock().map(|u| *u).unwrap_or(0))
    }

    async fn record_usage(&self, record: AiUsageRecord) -> Result<(), GateError> {
        let total = record.total_tokens;
        if let Ok(mut r) = self.records.lock() {
            r.push(record);
        }
        if let Ok(mut u) = self.used.lock() {
            *u += total;
        }
        Ok(())
    }
}

fn request(text: &str) -> AiChatRequest {
    AiChatRequest {
        user_id: Id::new(ACTOR).expect("id"),
        init_data: None,
        role: Default::default(),
        text: text.to_owned(),
        language: "fa".into(),
        direct_agent: None,
        history: None,
        knowledge: None,
    }
}

fn ctx() -> ToolCtx {
    ToolCtx { actor: ACTOR.to_owned(), staff: false }
}

fn staff_ctx() -> ToolCtx {
    ToolCtx { actor: "dashboard:admin".to_owned(), staff: true }
}

fn gateway_with(fake: &FakeProvider) -> Gateway {
    Gateway::new(vec![AnyProvider::Fake(fake.clone())], CAP)
}

fn two_slots(primary: &FakeProvider, secondary: &FakeProvider) -> Gateway {
    Gateway::new(
        vec![AnyProvider::Fake(primary.clone()), AnyProvider::Fake(secondary.clone())],
        CAP,
    )
}

fn three_slots(
    primary: &FakeProvider,
    secondary: &FakeProvider,
    fallback: &FakeProvider,
) -> Gateway {
    Gateway::new(
        vec![
            AnyProvider::Fake(primary.clone()),
            AnyProvider::Fake(secondary.clone()),
            AnyProvider::Fake(fallback.clone()),
        ],
        CAP,
    )
}

const OK_REPLY: &str = r#"{"reply":"سلام! موجودی شما چک شد."}"#;

// ── پاکت reply — مسیر سالم ──────────────────────────────────────────────────

#[tokio::test]
async fn reply_packet_returns_wire_response_with_usage() {
    let fake = FakeProvider::new("primary", "fake-openai", "model-x");
    fake.push_ok(OK_REPLY);
    let out: ChatOutcome = gateway_with(&fake)
        .chat(&MemoryAiBackend::default(), &ctx(), &request("xqzt"))
        .await
        .expect("ok");
    assert_eq!(out.audit, AuditKind::Ok);
    assert_eq!(out.response.agent, AiAgentName::Orchestrator);
    assert_eq!(out.response.provider, "fake-openai");
    assert_eq!(out.response.usage.model.as_deref(), Some("model-x"));
    assert!(out.response.usage.total_tokens > 0);
    assert_eq!(out.response.tool_traces.len(), 0);
    assert!(!out.response.pending_approval);
}

#[tokio::test]
async fn outgoing_reply_is_redacted_with_count() {
    let fake = FakeProvider::new("primary", "fake", "m");
    let leak = r#"{"reply":"your key is sk-abcdefghijklmnop1234 keep it safe"}"#;
    fake.push_ok(leak);
    let out = gateway_with(&fake)
        .chat(&MemoryAiBackend::default(), &ctx(), &request("hi"))
        .await
        .expect("ok");
    assert!(!out.response.reply.contains("sk-abcdefghijklmnop"), "{}", out.response.reply);
    assert!(out.response.reply.contains("[REDACTED:API_KEY]"));
    assert_eq!(out.response.redactions, 1);
}

#[tokio::test]
async fn incoming_text_is_redacted_before_provider() {
    let fake = FakeProvider::new("primary", "fake", "m");
    let backend = MemoryAiBackend::default();
    let _ = gateway_with(&fake)
        .chat(&backend, &ctx(), &request("my key sk-abcdefghijklmnop1234 and Bearer qq-token-99"))
        .await;
    let (_, user_prompt) = fake.last_call().expect("recorded");
    assert!(!user_prompt.contains("sk-abcdefghijklmnop"));
    assert!(user_prompt.contains("[REDACTED:API_KEY]"), "{user_prompt}");
    assert!(user_prompt.contains("[REDACTED:BEARER]"), "{user_prompt}");
}

// ── گیت سقف — قبل از provider ───────────────────────────────────────────────

#[tokio::test]
async fn cap_exceeded_fails_before_any_provider_call() {
    let fake = FakeProvider::new("primary", "fake", "m");
    let backend = MemoryAiBackend::default();
    backend.set_used(CAP);
    let err =
        gateway_with(&fake).chat(&backend, &ctx(), &request("سلام")).await.expect_err("capped");
    assert_eq!(err, GateError::CapExceeded { used: CAP, cap: CAP });
    assert_eq!(fake.call_count(), 0, "هیچ فراخوانی provider نشده");
}

#[tokio::test]
async fn usage_accumulates_and_hits_cap_on_second_call() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"reply":"first"}"#);
    let backend = MemoryAiBackend::default();
    let gw = Gateway::new(vec![AnyProvider::Fake(fake.clone())], 100);
    let first = gw.chat(&backend, &ctx(), &request("one")).await.expect("first ok");
    let used_after_first = first.response.usage.total_tokens;
    assert!(used_after_first > 0);
    let second = gw.chat(&backend, &ctx(), &request("two")).await;
    let capped = matches!(second, Err(GateError::CapExceeded { used, cap: 100 }) if used >= 100);
    assert!(capped, "second call must be capped: {second:?}");
    assert_eq!(fake.call_count(), 1, "فراخوانی دوم به provider نرسید");
}

// ── fallback قطعی سه‌اسلات ──────────────────────────────────────────────────

#[tokio::test]
async fn primary_down_secondary_answers() {
    let primary = FakeProvider::new("primary", "fake", "p");
    let secondary = FakeProvider::new("secondary", "fake-2", "s");
    primary.push_err(dropagentx_ai::LlmError::Down);
    secondary.push_ok(OK_REPLY);
    let out = two_slots(&primary, &secondary)
        .chat(&MemoryAiBackend::default(), &ctx(), &request("سلام"))
        .await
        .expect("secondary answers");
    assert_eq!(out.response.provider, "fake-2");
    assert_eq!(out.response.usage.model.as_deref(), Some("s"));
    assert_eq!(out.audit, AuditKind::Ok);
    assert_eq!(secondary.call_count(), 1);
    assert_eq!(primary.call_count(), 1);
}

#[tokio::test]
async fn primary_and_secondary_down_fallback_answers() {
    let primary = FakeProvider::new("primary", "fake", "p");
    let secondary = FakeProvider::new("secondary", "fake", "s");
    let fallback = FakeProvider::new("fallback", "fake-3", "f");
    primary.push_err(dropagentx_ai::LlmError::Timeout);
    secondary.push_err(dropagentx_ai::LlmError::Http { status: 500, detail: "boom".into() });
    fallback.push_ok(OK_REPLY);
    let out = three_slots(&primary, &secondary, &fallback)
        .chat(&MemoryAiBackend::default(), &ctx(), &request("سلام"))
        .await
        .expect("fallback answers");
    assert_eq!(out.response.provider, "fake-3");
}

#[tokio::test]
async fn all_slots_down_is_typed_upstream_error() {
    let primary = FakeProvider::new("primary", "fake", "p");
    let secondary = FakeProvider::new("secondary", "fake", "s");
    primary.push_err(dropagentx_ai::LlmError::Down);
    secondary.push_err(dropagentx_ai::LlmError::Down);
    let err = two_slots(&primary, &secondary)
        .chat(&MemoryAiBackend::default(), &ctx(), &request("سلام"))
        .await
        .expect_err("all down");
    match err {
        GateError::AllSlotsFailed { attempts } => {
            assert!(attempts.contains("primary"), "{attempts}");
            assert!(attempts.contains("secondary"), "{attempts}");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

// ── پاکت خراب ×۲ = شکست تایپ‌شده ────────────────────────────────────────────

#[tokio::test]
async fn two_malformed_packets_fail_typed() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok("this is raw text, not json");
    fake.push_ok(r#"{"reply": 42}"#);
    let err = gateway_with(&fake)
        .chat(&MemoryAiBackend::default(), &ctx(), &request("سلام"))
        .await
        .expect_err("malformed x2");
    assert_eq!(err, GateError::MalformedPacket { count: 2 });
    assert_eq!(fake.call_count(), 2);
}

#[tokio::test]
async fn one_malformed_then_good_slot_recovers() {
    let primary = FakeProvider::new("primary", "fake", "p");
    let secondary = FakeProvider::new("secondary", "fake", "s");
    primary.push_ok("raw text again");
    primary.push_ok(OK_REPLY); // تلاش دوم همان اسلات — پاسخ سالم
    secondary.push_ok(OK_REPLY);
    let out = two_slots(&primary, &secondary)
        .chat(&MemoryAiBackend::default(), &ctx(), &request("سلام"))
        .await
        .expect("recovers on retry");
    assert_eq!(out.audit, AuditKind::Ok);
    assert_eq!(out.response.provider, "fake");
    assert_eq!(primary.call_count(), 2, "دو تلاش در همان اسلات");
}

// ── ابزارها — گیت مجوز و trace ──────────────────────────────────────────────

#[tokio::test]
async fn tool_packet_executes_wallet_balance() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"wallet.balance","args":{}}"#);
    let backend = MemoryAiBackend::default();
    backend.seed_balance(ACTOR, 42_000);
    let out = gateway_with(&fake).chat(&backend, &ctx(), &request("موجودی")).await.expect("ok");
    assert_eq!(out.audit, AuditKind::Ok);
    let trace = &out.response.tool_traces[0];
    assert_eq!(trace.skill, "wallet.balance");
    assert!(trace.ok);
    assert!(!trace.pending_approval);
    assert!(trace.code.is_none());
    assert!(
        out.response.reply.contains("42,000") || out.response.reply.contains("42000"),
        "{}",
        out.response.reply
    );
    assert!(!out.response.pending_approval);
}

#[tokio::test]
async fn unknown_skill_is_refused_not_executed() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"wallet.nuke","args":{}}"#);
    let out = gateway_with(&fake)
        .chat(&MemoryAiBackend::default(), &ctx(), &request("x"))
        .await
        .expect("ok");
    assert_eq!(out.audit, AuditKind::Refused);
    assert_eq!(out.response.tool_traces[0].code.as_deref(), Some("UNKNOWN_SKILL"));
    assert!(!out.response.tool_traces[0].ok);
}

#[tokio::test]
async fn skill_not_allowed_for_agent_is_refused() {
    let fake = FakeProvider::new("primary", "fake", "m");
    // economy.summary فقط Orchestrator/Analyst — Receptionist مجاز نیست
    fake.push_ok(r#"{"tool":"economy.summary","args":{}}"#);
    let mut req = request("اقتصاد");
    req.direct_agent = Some(AiAgentName::Receptionist);
    let out =
        gateway_with(&fake).chat(&MemoryAiBackend::default(), &ctx(), &req).await.expect("ok");
    assert_eq!(out.audit, AuditKind::Refused);
    assert_eq!(out.response.tool_traces[0].code.as_deref(), Some("SKILL_NOT_ALLOWED"));
    assert_eq!(out.response.agent, AiAgentName::Receptionist);
}

#[tokio::test]
async fn staff_only_skill_refused_for_citizen() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"escrow_refund","args":{"escrowId":"e1"}}"#);
    let out = gateway_with(&fake)
        .chat(&MemoryAiBackend::default(), &ctx(), &request("بازپرداخت"))
        .await
        .expect("ok");
    assert_eq!(out.audit, AuditKind::Refused);
    assert_eq!(out.response.tool_traces[0].code.as_deref(), Some("UNAUTHORIZED_STAFF"));
}

#[tokio::test]
async fn mint_is_refused_even_for_staff_no_agent_allowed() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"wallet_mint","args":{"accountId":"user:tg:1","amount":900}}"#);
    let out = gateway_with(&fake)
        .chat(&MemoryAiBackend::default(), &staff_ctx(), &request("mint"))
        .await
        .expect("ok");
    assert_eq!(out.audit, AuditKind::Refused);
    assert_eq!(out.response.tool_traces[0].code.as_deref(), Some("SKILL_NOT_ALLOWED"));
}

#[tokio::test]
async fn escrow_refund_by_staff_queues_hitl_and_escrow_untouched() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"escrow_refund","args":{"escrowId":"e60k"}}"#);
    let backend = MemoryAiBackend::default();
    backend.seed_escrow("e60k", "FUNDED");
    let out =
        gateway_with(&fake).chat(&backend, &staff_ctx(), &request("refund")).await.expect("ok");
    assert_eq!(out.audit, AuditKind::Pending);
    assert!(out.response.pending_approval);
    let trace = &out.response.tool_traces[0];
    assert!(trace.pending_approval);
    assert_eq!(trace.code.as_deref(), Some("REQUIRES_HUMAN_APPROVAL"));
    assert!(trace.ok);
    assert_eq!(backend.hitl_len(), 1, "ردیف HITL ثبت شد");
    assert_eq!(
        backend.escrow_status.lock().unwrap().get("e60k").map(String::as_str),
        Some("FUNDED"),
        "اسکرو هنوز FUNDED"
    );
}

#[tokio::test]
async fn release_of_created_escrow_is_typed_failure() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"escrow_release","args":{"escrowId":"e1"}}"#);
    let backend = MemoryAiBackend::default();
    backend.seed_escrow("e1", "CREATED");
    let out =
        gateway_with(&fake).chat(&backend, &staff_ctx(), &request("release")).await.expect("ok");
    assert_eq!(out.audit, AuditKind::Failed);
    assert_eq!(out.response.tool_traces[0].code.as_deref(), Some("INVALID_TRANSITION"));
    assert!(!out.response.pending_approval);
}

#[tokio::test]
async fn handoff_queues_for_council() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"ai.handoff","args":{"note":"کمک می‌خواهم"}}"#);
    let backend = MemoryAiBackend::default();
    let out =
        gateway_with(&fake).chat(&backend, &ctx(), &request("انسان می‌خواهم")).await.expect("ok");
    assert_eq!(out.audit, AuditKind::Pending);
    assert!(out.response.pending_approval);
    assert_eq!(backend.hitl_len(), 1);
}

// ── usage/metering ──────────────────────────────────────────────────────────

#[tokio::test]
async fn usage_recorded_with_outcome_and_agent_lowercase() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"wallet.balance","args":{}}"#);
    let mut req = request("موجودی");
    req.direct_agent = Some(AiAgentName::Analyst);
    let backend = MemoryAiBackend::default();
    backend.seed_balance(ACTOR, 5);
    let _ = gateway_with(&fake).chat(&backend, &ctx(), &req).await.expect("ok");
    let records = backend.records.lock().unwrap().clone();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].agent, "analyst");
    assert_eq!(records[0].outcome, "ai.ok");
    assert_eq!(records[0].kind, "chat");
    assert!(records[0].total_tokens > 0);
    assert_eq!(records[0].total_tokens, records[0].prompt_tokens + records[0].completion_tokens);
}

#[tokio::test]
async fn refused_outcome_is_recorded_as_ai_refused() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"wallet.nuke","args":{}}"#);
    let backend = MemoryAiBackend::default();
    let _ = gateway_with(&fake).chat(&backend, &ctx(), &request("x")).await.expect("ok");
    let records = backend.records.lock().unwrap().clone();
    assert_eq!(records.len(), 1, "refused هم مصرف provider دارد");
    assert_eq!(records[0].outcome, "ai.refused");
}

#[tokio::test]
async fn failed_tool_outcome_recorded_as_ai_failed() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"escrow_release","args":{"escrowId":"nope"}}"#);
    let backend = MemoryAiBackend::default();
    let out = gateway_with(&fake).chat(&backend, &staff_ctx(), &request("x")).await.expect("ok");
    assert_eq!(out.audit, AuditKind::Failed);
    assert_eq!(backend.records.lock().unwrap()[0].outcome, "ai.failed");
}

#[tokio::test]
async fn pending_outcome_recorded_as_ai_pending() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(r#"{"tool":"ai.handoff","args":{}}"#);
    let backend = MemoryAiBackend::default();
    let _ = gateway_with(&fake).chat(&backend, &ctx(), &request("x")).await.expect("ok");
    assert_eq!(backend.records.lock().unwrap()[0].outcome, "ai.pending");
}

// ── روتینگ و via ────────────────────────────────────────────────────────────

#[tokio::test]
async fn direct_agent_sets_via_direct_and_skips_routing() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(OK_REPLY);
    let mut req = request("whatever");
    req.direct_agent = Some(AiAgentName::Sales);
    let out =
        gateway_with(&fake).chat(&MemoryAiBackend::default(), &ctx(), &req).await.expect("ok");
    assert_eq!(out.response.agent, AiAgentName::Sales);
    assert_eq!(out.response.via, dropagentx_contracts::AiVia::Direct);
}

#[tokio::test]
async fn keyword_routing_fa_selects_sales_and_via_llm() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(OK_REPLY);
    let out = gateway_with(&fake)
        .chat(&MemoryAiBackend::default(), &ctx(), &request("می‌خواهم این محصول را بخرم"))
        .await
        .expect("ok");
    assert_eq!(out.response.agent, AiAgentName::Sales);
    assert_eq!(out.response.via, dropagentx_contracts::AiVia::Llm);
}

// ── پرامپت و زبان ───────────────────────────────────────────────────────────

#[tokio::test]
async fn english_language_builds_english_system_prompt() {
    let fake = FakeProvider::new("primary", "fake", "m");
    let mut req = request("xqzt");
    req.language = "en".into();
    let _ = gateway_with(&fake).chat(&MemoryAiBackend::default(), &ctx(), &req).await.expect("ok");
    let (system, _) = fake.last_call().expect("recorded");
    assert!(system.contains("You are the Orchestrator"), "{system}");
    assert!(system.contains("Language: en"));
}

#[tokio::test]
async fn fa_language_builds_fa_system_prompt() {
    let fake = FakeProvider::new("primary", "fake", "m");
    let _ = gateway_with(&fake)
        .chat(&MemoryAiBackend::default(), &ctx(), &request("xqzt"))
        .await
        .expect("ok");
    let (system, _) = fake.last_call().expect("recorded");
    assert!(system.contains("هماهنگ‌کننده"), "{system}");
}

#[tokio::test]
async fn history_is_included_and_redacted() {
    let fake = FakeProvider::new("primary", "fake", "m");
    let mut req = request("continue");
    req.history = Some(vec![HistoryMessage {
        role: HistoryRole::User,
        content: "my token 7000000000:AAFKFDKqzUd-something-long-token-here".into(),
    }]);
    let _ = gateway_with(&fake).chat(&MemoryAiBackend::default(), &ctx(), &req).await.expect("ok");
    let (_, user_prompt) = fake.last_call().expect("recorded");
    assert!(user_prompt.contains("history[user]"));
    assert!(user_prompt.contains("[REDACTED:TELEGRAM_TOKEN]"), "{user_prompt}");
    assert!(!user_prompt.contains("AAFKFDKqzUd"));
}

#[tokio::test]
async fn knowledge_grounding_is_redacted_and_included() {
    let fake = FakeProvider::new("primary", "fake", "m");
    let mut req = request("سوال");
    req.knowledge =
        Some("charter article 2: seller 90% tax 10% — internal key sk-abcdefghijklmnop1234".into());
    let _ = gateway_with(&fake).chat(&MemoryAiBackend::default(), &ctx(), &req).await.expect("ok");
    let (_, user_prompt) = fake.last_call().expect("recorded");
    assert!(user_prompt.contains("charter article 2"));
    assert!(user_prompt.contains("[REDACTED:API_KEY]"));
}

// ── حالت‌های حاشیه‌ای ────────────────────────────────────────────────────────

#[tokio::test]
async fn empty_slots_gateway_reports_all_failed() {
    let gw = Gateway::new(Vec::new(), CAP);
    let err =
        gw.chat(&MemoryAiBackend::default(), &ctx(), &request("x")).await.expect_err("no slots");
    assert!(matches!(err, GateError::AllSlotsFailed { .. }));
}

#[tokio::test]
async fn is_enabled_reflects_slots() {
    let fake = FakeProvider::new("primary", "fake", "m");
    assert!(gateway_with(&fake).is_enabled());
    assert!(!Gateway::new(Vec::new(), CAP).is_enabled());
}

#[tokio::test]
async fn cap_zero_means_no_gate() {
    let fake = FakeProvider::new("primary", "fake", "m");
    fake.push_ok(OK_REPLY);
    let backend = MemoryAiBackend::default();
    backend.set_used(i64::MAX / 2);
    let gw = Gateway::new(vec![AnyProvider::Fake(fake.clone())], 0);
    let out = gw.chat(&backend, &ctx(), &request("x")).await;
    assert!(out.is_ok(), "cap=0 یعنی گیت خاموش: {out:?}");
}
