//! هندلرهای سیستم — health عمیق، بکاپ و شفافیت اقتصاد (مادهٔ ۶ منشور).

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use dropagentx_contracts::{
    BackupCreated, Currency, DinSymbol, EconomySummary, HealthResponse, HealthService, HealthStatus,
};
use dropagentx_domain::Charter;

use crate::error::ApiResult;
use crate::middleware::require_admin;
use crate::state::AppState;
use crate::views::wire_din;

/// `GET /health` — liveness عمیق: سروری که به دفترش نمی‌رسد سالم نیست.
/// فیلدهای LLM فقط «وجود کانفیگ» را نشان می‌دهند (ADR-006) — هرگز کلید.
///
/// # Errors
/// دیتابیس در دسترس نیست → 503 `DB_DOWN`.
pub async fn health(State(state): State<Arc<AppState>>) -> ApiResult<Json<HealthResponse>> {
    // خوانش عمیق دفتر — گپ واقعی بین پروسه و دیتابیس را می‌گیرد
    state
        .store
        .economy_summary()
        .await
        .map_err(|_| dropagentx_contracts::HttpFailure::db_down())?;
    Ok(Json(HealthResponse {
        status: HealthStatus::Ok,
        service: HealthService::DropagentxV3Server,
        version: crate::VERSION.to_owned(),
        llm_configured: state.config.llm.is_some(),
        llm_model: state.config.llm.as_ref().map(|c| c.model.clone()),
        db: Some("ok".into()),
        bot: Some(if state.config.bot_token.is_some() { "ok" } else { "disabled" }.into()),
        ai: Some(if state.config.llm.is_some() { "ok" } else { "disabled" }.into()),
        uptime: Some(state.started_at.elapsed().as_secs_f64()),
    }))
}

/// `POST /system/backup` — اسنپ‌شات سازگار SQLite (`VACUUM INTO`) + هرس.
///
/// # Errors
/// فروشگاه درون‌حافظه‌ای → 503 `BACKUP_UNSUPPORTED`؛ شکست SQL → `BACKUP_FAILED`.
pub async fn create_backup(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> ApiResult<Json<BackupCreated>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    if state.store.file_path().is_none() {
        return Err(dropagentx_contracts::HttpFailure::backup_unsupported().into());
    }
    let info = state
        .store
        .backup_to_dir(&state.config.backup_dir, state.config.keep_backups)
        .await
        .map_err(|e| dropagentx_contracts::HttpFailure {
            status: 500,
            code: "BACKUP_FAILED".into(),
            message: format!("VACUUM INTO failed — {e}"),
            meta: None,
        })?;
    Ok(Json(BackupCreated {
        file: info.file,
        size_bytes: i64::try_from(info.size_bytes).unwrap_or(i64::MAX),
        kept: info.kept,
        created_at: info.created_at,
    }))
}

/// `GET /economy` — خلاصهٔ اقتصاد ملی؛ عمومی و فقط‌خواندنی (مادهٔ ۶).
///
/// # Errors
/// خطای SQL.
pub async fn get_economy(State(state): State<Arc<AppState>>) -> ApiResult<Json<EconomySummary>> {
    let s = state.store.economy_summary().await?;
    let minted_all_time = state.store.minted_all_time().await?;
    let burned_all_time = state.store.burned_all_time().await?;
    let account_count = state.store.account_count().await?;
    Ok(Json(EconomySummary {
        currency: Currency::Dnx,
        symbol: DinSymbol::Dinar,
        circulating_supply: wire_din(s.circulating_supply),
        treasury_balance: wire_din(s.treasury_balance),
        escrow_locked: wire_din(s.escrow_locked),
        minted_all_time: wire_din(minted_all_time),
        burned_all_time: wire_din(burned_all_time),
        minted_this_year: wire_din(s.minted_this_year),
        yearly_mint_cap: wire_din(Charter::YEARLY_MINT_CAP_DIN.as_i64()),
        genesis_supply: wire_din(Charter::GENESIS_SUPPLY_DIN.as_i64()),
        trial_balance_sum: s.trial_sum,
        balanced: s.balanced,
        account_count,
        open_escrows: s.open_escrows,
    }))
}
