//! هندلرهای صف HITL (انسان-در-حلقه) — مادهٔ ۷ منشور.
//!
//! تأیید `ESCROW_RELEASE` تسویهٔ ۹۰/۱۰ را «در همان تراکنش» اجرا می‌کند
//! (`approve_hitl_and_settle` فاز ۲)؛ کلید تسویه دترمینیستیک است
//! (`hitl:<id>:approve`) تا retry هرگز دوباره نپردازد. تصمیم دوم برای همان
//! درخواست → خطای تایپ‌شده با بازپرداخت صفر.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use std::sync::Arc;

use dropagentx_contracts::{
    CreateHitlRequest, HitlActionResponse, HitlDecisionRequest, HitlListResponse,
    HitlSweepResponse, HitlView,
};
use dropagentx_db::{now_iso, HitlRow};

use crate::error::{ApiFailure, ApiResult};
use crate::middleware::require_admin;
use crate::state::AppState;
use crate::views::hitl_view;

/// `POST /hitl` — صف‌کردن درخواست تأیید انسانی؛ مبلغ از امانت کپی می‌شود و
/// id تکراری همان درخواست موجود را برمی‌گرداند (replay). ادمین. ۲۰۱.
///
/// # Errors
/// 404 `ESCROW_NOT_FOUND`؛ 400 بدنهٔ نامعتبر.
pub async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<(StatusCode, Json<HitlView>)> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: CreateHitlRequest = crate::handlers::parse_body(&body)?;

    // مبلغ فقط از امانت — درخواست‌دهنده حق اظهار عدد ندارد
    let escrow =
        state.store.get_escrow(req.escrow_id.get()).await?.ok_or_else::<ApiFailure, _>(|| {
            dropagentx_domain::DomainError::new(
                dropagentx_domain::DomainErrorCode::EscrowNotFound,
                format!("Escrow {} does not exist", req.escrow_id.get()),
            )
            .into()
        })?;

    let id = req
        .id
        .as_ref()
        .map(|i| i.get().to_owned())
        .unwrap_or_else(|| format!("hitl:{}", uuid::Uuid::now_v7()));
    let ttl = req.ttl_hours.unwrap_or(24);
    let row = HitlRow {
        id,
        kind: "ESCROW_RELEASE".into(),
        escrow_id: escrow.id.clone(),
        amount: escrow.amount.as_i64(),
        requester_id: req.requester_id.get().to_owned(),
        note: req.note,
        status: "PENDING".into(),
        created_at: now_iso(),
        expires_at: (chrono::Utc::now() + chrono::Duration::hours(ttl))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        decided_by: None,
        decided_at: None,
        decision_note: None,
    };
    let queued = state.store.enqueue_hitl(row, escrow.id.as_str()).await?;
    Ok((StatusCode::CREATED, Json(hitl_view(&queued)?)))
}

/// `GET /hitl` — فهرست درخواست‌ها با فیلتر وضعیت اختیاری. ادمین.
///
/// # Errors
/// فیلتر نامعتبر → 400؛ خطای SQL.
pub async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<HitlListResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let status = params.get("status").map(String::as_str);
    // فیلتر enum مثل zod — مقدار خارج PENDING/APPROVED/REJECTED/EXPIRED → 400
    if let Some(s) = status {
        serde_json::from_value::<dropagentx_contracts::HitlStatus>(serde_json::Value::String(
            s.to_owned(),
        ))
        .map_err(|_| dropagentx_contracts::HttpFailure::invalid_request(1))?;
    }
    let items = state
        .store
        .list_hitl(status)
        .await?
        .iter()
        .map(hitl_view)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(HitlListResponse { items }))
}

/// `GET /hitl/{id}` — خواندن یک درخواست. ادمین.
///
/// # Errors
/// 404 `HITL_NOT_FOUND`.
pub async fn get_one(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<HitlView>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    state
        .store
        .get_hitl(&id)
        .await?
        .map(|r| hitl_view(&r).map(Json))
        .transpose()?
        .ok_or_else::<ApiFailure, _>(|| {
            dropagentx_domain::DomainError::new(
                dropagentx_domain::DomainErrorCode::HitlNotFound,
                format!("HITL request {id} does not exist"),
            )
            .into()
        })
}

/// `POST /hitl/{id}/approve` — تأیید → تسویهٔ ۹۰/۱۰ در همان تراکنش. ادمین.
///
/// # Errors
/// ۴۲۲ `HITL_ALREADY_DECIDED` / `HITL_EXPIRED` / کدهای تسویه؛ 404 ناشناخته.
pub async fn approve(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Json<HitlActionResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: HitlDecisionRequest = crate::handlers::parse_body(&body)?;
    let decision =
        state.store.approve_hitl_and_settle(&id, req.decided_by.get(), req.note.as_deref()).await?;
    Ok(Json(HitlActionResponse {
        request: hitl_view(&decision.request)?,
        escrow: decision.escrow.as_ref().map(crate::views::escrow_view),
        tx_id: decision.tx.as_ref().map(|t| t.id.clone()),
        replayed: decision.replayed,
    }))
}

/// `POST /hitl/{id}/reject` — رد؛ هیچ پولی جابه‌جا نمی‌شود. ادمین.
///
/// # Errors
/// ۴۲۲ `HITL_ALREADY_DECIDED`؛ 404 ناشناخته.
pub async fn reject(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Json<HitlActionResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: HitlDecisionRequest = crate::handlers::parse_body(&body)?;
    let row = state.store.reject_hitl(&id, req.decided_by.get(), req.note.as_deref()).await?;
    Ok(Json(HitlActionResponse {
        request: hitl_view(&row)?,
        escrow: None,
        tx_id: None,
        replayed: false,
    }))
}

/// `POST /hitl/sweep` — انقضای همهٔ PENDINGهای سررسیدگذشته (بات روی interval).
///
/// # Errors
/// خطای SQL.
pub async fn sweep(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> ApiResult<Json<HitlSweepResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let expired = state.store.sweep_expired_hitl().await?;
    Ok(Json(HitlSweepResponse { expired }))
}
