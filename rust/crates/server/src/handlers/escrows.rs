//! هندلرهای امانت — ستون فقرات مارکت‌پلیس (ADR-004 + مادهٔ ۸ منشور).
//!
//! همهٔ گذارها از ماشین وضعیت دامنهٔ خالص می‌گذرند: replay با کلید یکسان‌سازی
//! هرگز دوبار پول جابه‌جا نمی‌کند، گذار نامعتبر ۴۲۲ است و مادهٔ ۷ برای
//! مبالغ ≥ ۵۰٬۰۰۰ Đ تأیید صریح ادمین می‌خواهد (یا مسیر HITL).

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use std::sync::Arc;

use dropagentx_contracts::{
    CancelEscrowRequest, CreateEscrowRequest, DeliverEscrowRequest, DisputeEscrowRequest,
    EscrowActionResponse, EscrowListResponse, EscrowView, ExpireEscrowRequest, FundEscrowRequest,
    RefundEscrowRequest, ReleaseEscrowRequest,
};
use dropagentx_db::{now_iso, ApplyOptions, ApplyResult, CommandOutcome};
use dropagentx_domain::{
    cancel_escrow, create_escrow, deliver_escrow, dispute_escrow, expire_escrow, fund_escrow,
    refund_escrow, release_escrow, CreateEscrowCommand, DeliverEscrowCommand, DisputeEscrowCommand,
    DomainEvent, DomainResult, EscrowOutcome, EscrowRefCommand, EscrowRegistry, EscrowStatus,
    LedgerState, ReleaseEscrowCommand,
};

use crate::error::{ApiFailure, ApiResult};
use crate::middleware::require_admin;
use crate::state::AppState;
use crate::views::escrow_view;

/// وضعیت وایر → وضعیت دامنه (واسطِ صریح به‌جای impl خارجی — قانون orphan).
fn wire_status_to_domain(s: dropagentx_contracts::EscrowStatus) -> EscrowStatus {
    match s {
        dropagentx_contracts::EscrowStatus::Created => EscrowStatus::Created,
        dropagentx_contracts::EscrowStatus::Funded => EscrowStatus::Funded,
        dropagentx_contracts::EscrowStatus::Delivered => EscrowStatus::Delivered,
        dropagentx_contracts::EscrowStatus::Released => EscrowStatus::Released,
        dropagentx_contracts::EscrowStatus::Disputed => EscrowStatus::Disputed,
        dropagentx_contracts::EscrowStatus::Refunded => EscrowStatus::Refunded,
        dropagentx_contracts::EscrowStatus::Cancelled => EscrowStatus::Cancelled,
        dropagentx_contracts::EscrowStatus::Expired => EscrowStatus::Expired,
    }
}

/// فیلتر وضعیت فهرست — رشتهٔ نامعتبر = 400 `INVALID_REQUEST` (مثل zod enum).
fn parse_status_filter(raw: &str) -> ApiResult<EscrowStatus> {
    serde_json::from_value::<dropagentx_contracts::EscrowStatus>(serde_json::Value::String(
        raw.to_owned(),
    ))
    .map(wire_status_to_domain)
    .map_err(|_| dropagentx_contracts::HttpFailure::invalid_request(1).into())
}

/// خروجی دامنه → خروجی فرمان فروشگاه.
fn into_outcome(o: EscrowOutcome) -> CommandOutcome {
    CommandOutcome {
        ledger: o.ledger,
        tx: o.tx,
        escrow: Some(o.escrow),
        events: o.events.into_iter().map(DomainEvent::Escrow).collect(),
    }
}

/// پاسخ استاندارد یک اقدام امانت — نمای امانت پس از گذار.
fn action_response(result: &ApplyResult) -> ApiResult<EscrowActionResponse> {
    let escrow =
        result.escrow.as_ref().ok_or(dropagentx_contracts::HttpFailure::internal_no_escrow())?;
    Ok(EscrowActionResponse {
        escrow: escrow_view(escrow),
        tx_id: result.tx.as_ref().map(|t| t.id.clone()),
        replayed: result.replayed,
    })
}

/// اجرای یک گذار اسکرو از مسیر دامنهٔ خالص داخل `apply` — نقطهٔ واحد audit/outbox.
async fn apply_escrow<F>(
    state: &AppState,
    escrow_id: &str,
    action: &'static str,
    idempotency_key: Option<String>,
    run: F,
) -> ApiResult<ApplyResult>
where
    F: FnOnce(&LedgerState, &EscrowRegistry) -> DomainResult<CommandOutcome>,
{
    state
        .store
        .apply(
            ApplyOptions {
                actor: "admin".into(),
                action: action.to_owned(),
                target: Some(escrow_id.to_owned()),
                escrow_id: Some(escrow_id.to_owned()),
                idempotency_key,
            },
            run,
        )
        .await
        .map_err(ApiFailure::from)
}

/// زمان انقضا از TTL ساعت — RFC3339 UTC با millis.
fn expires_from_ttl(ttl_hours: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::hours(ttl_hours))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// `POST /escrows` — ساخت امانت (CREATED؛ پول هنوز جابه‌جا نشده). ادمین. ۲۰۱.
///
/// # Errors
/// ۴۲۲ `ESCROW_ALREADY_EXISTS` / `INVALID_ACCOUNT_KIND`؛ 404 حساب ناشناخته.
pub async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<(StatusCode, Json<EscrowActionResponse>)> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: CreateEscrowRequest = crate::handlers::parse_body(&body)?;
    let id = req
        .id
        .as_ref()
        .map(|i| i.get().to_owned())
        .unwrap_or_else(|| format!("escrow:{}", uuid::Uuid::now_v7()));
    let expires_at = req.ttl_hours.map(expires_from_ttl);
    let buyer_id = req.buyer_id.get().to_owned();
    let seller_id = req.seller_id.get().to_owned();
    let amount = dropagentx_domain::Din::new(req.amount.get())?;
    let product_ref = req.product_ref.clone();
    let key = req.idempotency_key.as_ref().map(|k| k.get().to_owned());
    let escrow_id = id.clone();

    let result = apply_escrow(&state, &id, "escrow.create", key, move |ledger, reg| {
        create_escrow(
            ledger,
            reg,
            &CreateEscrowCommand {
                id: escrow_id,
                buyer_id,
                seller_id,
                amount,
                product_ref,
                expires_at,
                at: Some(now_iso()),
            },
        )
        .map(into_outcome)
    })
    .await?;
    let response = action_response(&result)?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// `GET /escrows` — فهرست امانت‌ها با فیلتر وضعیت اختیاری. ادمین.
///
/// # Errors
/// فیلتر نامعتبر → 400؛ خطای SQL.
pub async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<EscrowListResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let status = match params.get("status").map(String::as_str) {
        Some(raw) => Some(parse_status_filter(raw)?),
        None => None,
    };
    let items = state.store.list_escrows(status).await?.iter().map(escrow_view).collect();
    Ok(Json(EscrowListResponse { items }))
}

/// `GET /escrows/{id}` — خواندن یک امانت؛ عمومی (شفافیت مادهٔ ۶).
///
/// # Errors
/// 404 `ESCROW_NOT_FOUND`.
pub async fn get_one(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<EscrowView>> {
    state.store.get_escrow(&id).await?.map(|e| Json(escrow_view(&e))).ok_or_else::<ApiFailure, _>(
        || {
            dropagentx_domain::DomainError::new(
                dropagentx_domain::DomainErrorCode::EscrowNotFound,
                format!("Escrow {id} does not exist"),
            )
            .into()
        },
    )
}

/// `POST /escrows/{id}/fund` — قفل کردن پول خریدار در حساب اسکرو. ادمین.
///
/// # Errors
/// ۴۲۲ گذار نامعتبر / overdraft؛ 404 امانت ناشناخته.
pub async fn fund(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Json<EscrowActionResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: FundEscrowRequest = crate::handlers::parse_body(&body)?;
    let key = req.idempotency_key.as_ref().map(|k| k.get().to_owned());
    let escrow_id = id.clone();
    let closure_key = key.clone();
    let result = apply_escrow(&state, &id, "escrow.fund", key, move |ledger, reg| {
        fund_escrow(
            ledger,
            reg,
            &EscrowRefCommand { escrow_id, idempotency_key: closure_key, at: Some(now_iso()) },
        )
        .map(into_outcome)
    })
    .await?;
    Ok(Json(action_response(&result)?))
}

/// `POST /escrows/{id}/deliver` — ثبت تحویل؛ پول هنوز قفل است. ادمین.
///
/// # Errors
/// ۴۲۲ گذار نامعتبر؛ 404 ناشناخته.
pub async fn deliver(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Json<EscrowActionResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: DeliverEscrowRequest = crate::handlers::parse_body(&body)?;
    let key = req.idempotency_key.as_ref().map(|k| k.get().to_owned());
    let delivery_ref = req.delivery_ref.clone();
    let escrow_id = id.clone();
    let closure_key = key.clone();
    let result = apply_escrow(&state, &id, "escrow.deliver", key, move |ledger, reg| {
        deliver_escrow(
            ledger,
            reg,
            &DeliverEscrowCommand {
                escrow_id,
                idempotency_key: closure_key,
                at: Some(now_iso()),
                delivery_ref,
            },
        )
        .map(into_outcome)
    })
    .await?;
    Ok(Json(action_response(&result)?))
}

/// `POST /escrows/{id}/release` — تسویهٔ ۹۰/۱۰ (مادهٔ ۲)؛ مادهٔ ۷: مبالغ
/// ≥ ۵۰٬۰۰۰ Đ بدون `approvedByAdmin: true` صریح رد می‌شوند (۴۲۲).
///
/// # Errors
/// ۴۲۲ `REQUIRES_HUMAN_APPROVAL` / گذار نامعتبر؛ 404 ناشناخته.
pub async fn release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Json<EscrowActionResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: ReleaseEscrowRequest = crate::handlers::parse_body(&body)?;
    let key = req.idempotency_key.as_ref().map(|k| k.get().to_owned());
    let approved = req.approved_by_admin.unwrap_or(false);
    let escrow_id = id.clone();
    let closure_key = key.clone();
    let result = apply_escrow(&state, &id, "escrow.release", key, move |ledger, reg| {
        release_escrow(
            ledger,
            reg,
            &ReleaseEscrowCommand {
                escrow_id,
                idempotency_key: closure_key,
                at: Some(now_iso()),
                approved_by_admin: approved,
            },
        )
        .map(into_outcome)
    })
    .await?;
    Ok(Json(action_response(&result)?))
}

/// `POST /escrows/{id}/dispute` — اختلاف؛ اسکرو قفل می‌ماند. ادمین.
///
/// # Errors
/// ۴۲۲ گذار نامعتبر / دلیل خالی؛ 404 ناشناخته.
pub async fn dispute(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Json<EscrowActionResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: DisputeEscrowRequest = crate::handlers::parse_body(&body)?;
    let key = req.idempotency_key.as_ref().map(|k| k.get().to_owned());
    let reason = req.reason.clone();
    let escrow_id = id.clone();
    let closure_key = key.clone();
    let result = apply_escrow(&state, &id, "escrow.dispute", key, move |ledger, reg| {
        dispute_escrow(
            ledger,
            reg,
            &DisputeEscrowCommand {
                escrow_id,
                idempotency_key: closure_key,
                at: Some(now_iso()),
                reason,
            },
        )
        .map(into_outcome)
    })
    .await?;
    Ok(Json(action_response(&result)?))
}

/// `POST /escrows/{id}/refund` — بازگشت کامل به خریدار، بدون مالیات. ادمین.
///
/// # Errors
/// ۴۲۲ گذار نامعتبر (باید DISPUTED باشد)؛ 404 ناشناخته.
pub async fn refund(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Json<EscrowActionResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: RefundEscrowRequest = crate::handlers::parse_body(&body)?;
    let key = req.idempotency_key.as_ref().map(|k| k.get().to_owned());
    let escrow_id = id.clone();
    let closure_key = key.clone();
    let result = apply_escrow(&state, &id, "escrow.refund", key, move |ledger, reg| {
        refund_escrow(
            ledger,
            reg,
            &EscrowRefCommand { escrow_id, idempotency_key: closure_key, at: Some(now_iso()) },
        )
        .map(into_outcome)
    })
    .await?;
    Ok(Json(action_response(&result)?))
}

/// `POST /escrows/{id}/cancel` — لغو CREATED؛ هیچ پولی حرکت نکرده. ادمین.
///
/// # Errors
/// ۴۲۲ گذار نامعتبر؛ 404 ناشناخته.
pub async fn cancel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Json<EscrowActionResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: CancelEscrowRequest = crate::handlers::parse_body(&body)?;
    let key = req.idempotency_key.as_ref().map(|k| k.get().to_owned());
    let escrow_id = id.clone();
    let closure_key = key.clone();
    let result = apply_escrow(&state, &id, "escrow.cancel", key, move |ledger, reg| {
        cancel_escrow(
            ledger,
            reg,
            &EscrowRefCommand { escrow_id, idempotency_key: closure_key, at: Some(now_iso()) },
        )
        .map(into_outcome)
    })
    .await?;
    Ok(Json(action_response(&result)?))
}

/// `POST /escrows/{id}/expire` — sweep TTL؛ بازگشت خودکار FUNDED (مادهٔ ۹). ادمین.
///
/// # Errors
/// ۴۲۲ گذار نامعتبر (باید FUNDED باشد)؛ 404 ناشناخته.
pub async fn expire(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Json<EscrowActionResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: ExpireEscrowRequest = crate::handlers::parse_body(&body)?;
    let key = req.idempotency_key.as_ref().map(|k| k.get().to_owned());
    let escrow_id = id.clone();
    let closure_key = key.clone();
    let result = apply_escrow(&state, &id, "escrow.expire", key, move |ledger, reg| {
        expire_escrow(
            ledger,
            reg,
            &EscrowRefCommand { escrow_id, idempotency_key: closure_key, at: Some(now_iso()) },
        )
        .map(into_outcome)
    })
    .await?;
    Ok(Json(action_response(&result)?))
}
