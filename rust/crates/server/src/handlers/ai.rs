//! هندلرهای سیکل AI — گیت‌های هویت/مجوز fail-closed (پورت دقیق ADR-006) +
//! رجیستری ایجنت/مهارت. موتور روتینگ و مهارت‌ها در فاز ۶ روی همین گیت‌ها سیم
//! می‌شوند؛ تا آن‌روز تماس معتبر → 503 `AI_DISABLED` با پیام صادقانه.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use std::collections::HashMap;
use std::sync::Arc;

use dropagentx_contracts::{
    verify_kia_key, verify_telegram_init_data, AiAgentsListResponse, AiChatRequest, AiChatResponse,
    AiChatRole, AiSkillsListResponse, AiUsageView, HttpFailure,
};

use crate::error::{ApiFailure, ApiResult};
use crate::middleware::require_admin;
use crate::state::{kia_os_skills, zenovix_agents, AppState};

/// `GET /ai/agents` — ۶ ایجنت Zenovix با مهارت‌های مجاز هرکدام. عمومی.
///
/// # Errors
/// هیچ — دادهٔ ثابت.
pub async fn agents() -> Json<AiAgentsListResponse> {
    Json(AiAgentsListResponse { items: zenovix_agents() })
}

/// `GET /ai/skills` — ۱۶ مهارت KIA-OS با متادیتای مجوز. عمومی.
///
/// # Errors
/// هیچ — دادهٔ ثابت.
pub async fn skills() -> Json<AiSkillsListResponse> {
    Json(AiSkillsListResponse { items: kia_os_skills() })
}

/// `GET /ai/usage` — مصرف روزانهٔ یک کاربر در برابر سقف. ادمین.
///
/// # Errors
/// `userId` غایب → 400؛ خطای SQL.
pub async fn usage(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<AiUsageView>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let Some(user_id) = params.get("userId") else {
        return Err(HttpFailure::invalid_request(1).into());
    };
    let (used, _calls) = state.store.ai_tokens_today(user_id).await?;
    let cap = state.config.ai_daily_token_cap;
    Ok(Json(AiUsageView {
        user_id: user_id.clone(),
        used_today: used,
        cap,
        remaining_today: (cap - used).max(0),
    }))
}

/// `POST /ai/chat` — گیت‌های هویت به همان ترتیب TS + موتور فاز ۶:
///
/// ۱. `role: staff` بدون کلید ادمین معتبر → 401 `UNAUTHORIZED`
/// ۲. تماس‌گیرندهٔ بدون کلید: توکن بات تنظیم‌نشده → 503 `AI_IDENTITY_DISABLED`؛
///    initData غایب/نامعتبر → 401 `AI_IDENTITY_REQUIRED` — هویت هرگز از بدنه.
/// ۳. گیت‌وی AI نصب‌نشده → 503 `AI_DISABLED` (ADR-006 همه‌یا-هیچ در بوت).
/// ۴. موتور: سقف روزانه → 429 `AI_CAP_EXCEEDED`؛ دو پاکت خراب → 502
///    `AI_MALFORMED_PACKET`؛ همهٔ اسلات‌ها قطع → 502 `AI_UPSTREAM`.
/// ۵. ممیزی هر پاسخ: `ai.ok|refused|failed|pending` در AuditLog.
///
/// # Errors
/// گیت‌های بالا؛ 400 بدنهٔ نامعتبر.
pub async fn chat(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Json<AiChatResponse>> {
    let req: AiChatRequest = crate::handlers::parse_body(&body)?;
    let admin_ok = verify_kia_key(
        headers.get("x-kia-key").and_then(|v| v.to_str().ok()),
        state.config.admin_key.as_deref(),
    );

    if req.role == AiChatRole::Staff && !admin_ok {
        return Err(HttpFailure::unauthorized_staff().into());
    }

    let acting_account = if admin_ok {
        // تماس معتمد (بات/داشبورد) — userId بدنه معتبر است
        req.user_id.get().to_owned()
    } else {
        // هویت عمومی فقط از initData مشتق می‌شود — هرگز از بدنه
        let Some(bot_token) = state.config.bot_token.as_deref() else {
            return Err(HttpFailure::ai_identity_disabled().into());
        };
        let Some(init_data) = req.init_data.as_deref() else {
            return Err(HttpFailure::ai_identity_required().into());
        };
        match verify_telegram_init_data(init_data, bot_token) {
            Some(identity) => format!("user:tg:{}", identity.user_id),
            None => return Err(HttpFailure::ai_identity_required().into()),
        }
    };

    // گیت کانفیگ — gateway در بوت از اسلات‌های ADR-008 ساخته می‌شود
    let Some(gateway) = state.ai_gateway.as_ref() else {
        return Err(HttpFailure::ai_disabled().into());
    };

    // فاز ۶ — موتور کامل: سقف، fallback، پاکت سخت، مجوز، redaction، usage
    let ctx = dropagentx_ai::ToolCtx {
        actor: acting_account.clone(),
        staff: admin_ok && req.role == AiChatRole::Staff,
    };
    let tools = crate::ai_tools::AiTools::new(state.store.clone());
    let outcome: dropagentx_ai::ChatOutcome =
        gateway.chat(&tools, &ctx, &req).await.map_err(|e| -> ApiFailure {
            match e {
                dropagentx_ai::GateError::CapExceeded { used, cap } => {
                    HttpFailure::ai_cap_exceeded(used, cap).into()
                }
                dropagentx_ai::GateError::MalformedPacket { count } => {
                    let _ = count;
                    HttpFailure::ai_malformed_packet().into()
                }
                dropagentx_ai::GateError::AllSlotsFailed { attempts } => {
                    tracing::warn!(attempts, "all AI slots failed");
                    HttpFailure::ai_upstream(
                        "AI_UPSTREAM",
                        "Every LLM slot failed — see server logs.",
                    )
                    .into()
                }
                dropagentx_ai::GateError::Backend(detail) => {
                    tracing::error!(detail, "AI backend failure");
                    HttpFailure::internal("AI tool backend failed.").into()
                }
                dropagentx_ai::GateError::EmptyRequest => HttpFailure::invalid_request(1).into(),
            }
        })?;

    // ممیزی هر پاسخ — چهار وضعیت ADR با زمینهٔ کامل (بدون متن کاربر)
    let agent = outcome.response.agent.as_str();
    let meta = format!(
        r#"{{"provider":"{}","model":{},"totalTokens":{},"redactions":{},"pending":{},"tools":{}}}"#,
        outcome.response.provider,
        serde_json::to_string(&outcome.response.usage.model).unwrap_or_default(),
        outcome.response.usage.total_tokens,
        outcome.response.redactions,
        outcome.response.pending_approval,
        serde_json::to_string(
            &outcome
                .response
                .tool_traces
                .iter()
                .map(|t| (t.skill.as_str(), t.ok))
                .collect::<Vec<_>>()
        )
        .unwrap_or_default(),
    );
    if let Err(e) = state
        .store
        .audit_ext(&acting_account, outcome.audit.as_str(), Some(agent), Some(&meta))
        .await
    {
        tracing::warn!(error = %e, "ai audit write failed");
    }

    Ok(Json(outcome.response))
}
