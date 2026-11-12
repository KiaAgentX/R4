//! هندلرهای حساب‌ها — ثبت‌نام شهروند، فهرست ادمین و موجودی عمومی.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use std::sync::Arc;

use dropagentx_contracts::{
    AccountListResponse, AccountView, BalanceResponse, CreateAccountRequest,
};
use dropagentx_domain::{DomainError, DomainErrorCode};

use crate::error::{ApiFailure, ApiResult};
use crate::middleware::require_admin;
use crate::state::AppState;
use crate::views::{account_view, wire_din};

/// پیشوندهای سیستمی — هیچ حسابی این‌ها را نمی‌گیرد (حفاظت عرضه، مادهٔ ۴).
const RESERVED_PREFIXES: [&str; 4] = ["mint:", "treasury:", "burn:", "escrow:"];

/// `POST /accounts` — ثبت حساب کاربر (موجودی از ۰ شروع می‌شود). ادمین.
///
/// # Errors
/// ۴۲۲ `RESERVED_ID` / `DUPLICATE_ACCOUNT`؛ ۴۰۰ بدنهٔ نامعتبر.
pub async fn create_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<(StatusCode, Json<AccountView>)> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: CreateAccountRequest = crate::handlers::parse_body(&body)?;
    let id = req.id.get();

    if RESERVED_PREFIXES.iter().any(|p| id.starts_with(p)) {
        return Err(dropagentx_contracts::HttpFailure::reserved_id().into());
    }
    if state.store.balance_of(id).await?.is_some() {
        return Err(dropagentx_contracts::HttpFailure::duplicate_account(id).into());
    }
    state
        .store
        .create_account_if_missing(id, dropagentx_domain::AccountKind::User, req.label.as_deref())
        .await?;

    // خواندن صادقانهٔ ردیف — createdAt واقعی دیتابیس، نه حدس پروسه
    let account = state
        .store
        .list_accounts_with_balances()
        .await?
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else::<ApiFailure, _>(|| {
            DomainError::new(DomainErrorCode::UnknownAccount, format!("Account {id} vanished"))
                .into()
        })?;
    Ok((StatusCode::CREATED, Json(account_view(&account))))
}

/// `GET /accounts` — فهرست همهٔ حساب‌ها با تراز مشتق‌شده. ادمین.
///
/// # Errors
/// خطای SQL.
pub async fn list_accounts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> ApiResult<Json<AccountListResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let items = state.store.list_accounts_with_balances().await?.iter().map(account_view).collect();
    Ok(Json(AccountListResponse { items }))
}

/// `GET /accounts/{id}/balance` — موجودی مشتق‌شده؛ عمومی (شفافیت مادهٔ ۶).
///
/// # Errors
/// حساب ناشناخته → 404 `UNKNOWN_ACCOUNT`.
pub async fn get_balance(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<BalanceResponse>> {
    let balance = state.store.balance_of(&id).await?.ok_or_else::<ApiFailure, _>(|| {
        DomainError::new(DomainErrorCode::UnknownAccount, format!("Account {id} does not exist"))
            .into()
    })?;
    Ok(Json(BalanceResponse { account_id: id, balance: wire_din(balance) }))
}
