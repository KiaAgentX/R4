//! هندلرهای کیف پول — ضرب (مادهٔ ۳: سقف سالانه) و سوزاندن (مادهٔ ۴: overdraft
//! ناممکن). هر فرمان از مسیر دامنهٔ خالص + `apply` می‌گذرد؛ replay هرگز دوبار
//! پول جابه‌جا نمی‌کند (قید UNIQUE + پیش‌چک).

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use dropagentx_contracts::{BurnRequest, MintRequest, WalletMoveResponse};
use dropagentx_db::{now_iso, ApplyOptions, ApplyResult, CommandOutcome, V3Store};
use dropagentx_domain::{
    assert_mint_within_cap, post, Din, DomainEvent, DomainResult, EscrowRegistry, LedgerState,
    PostCommand, Posting, TransactionPostedEvent, BURN_ACCOUNT_ID, MINT_ACCOUNT_ID,
};

use crate::error::{ApiFailure, ApiResult};
use crate::middleware::require_admin;
use crate::state::AppState;

/// شروع سال UTC جاری — قرارداد سقف منشور (مادهٔ ۳). مقایسهٔ رشته‌ای RFC3339
/// با فرمت یکسان millis+Z معادل مقایسهٔ زمانی است.
fn year_start_iso() -> String {
    use chrono::{Datelike, TimeZone};
    let y = chrono::Utc::now().year();
    chrono::Utc
        .with_ymd_and_hms(y, 1, 1, 0, 0, 0)
        .single()
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_else(|| format!("{y}-01-01T00:00:00.000Z"))
}

/// ضرب سال جاری از وضعیت لودشده داخل تراکنش — دقیق و بدون مسابقه: هر فرمان
/// کل دفتر را در همان تراکنش می‌بیند (الگوی فاز ۲).
fn minted_this_year_locked(ledger: &LedgerState) -> i64 {
    let year_start = year_start_iso();
    let mut sum = 0i64;
    for tx in ledger.txs.values() {
        if tx.at.as_str() >= year_start.as_str() {
            for p in &tx.postings {
                if p.account_id == MINT_ACCOUNT_ID && p.amount < 0 {
                    sum += -p.amount;
                }
            }
        }
    }
    sum
}

/// بدنهٔ فرمان ضرب/سوزاندن.
struct MoveCmd {
    account_id: String,
    amount: Din,
    memo: Option<String>,
    key: Option<String>,
}

async fn wallet_move(store: &V3Store, cmd: MoveCmd, mint: bool) -> Result<ApplyResult, ApiFailure> {
    let key = cmd.key.clone();
    let account_id = cmd.account_id.clone();
    let amount = cmd.amount;
    let memo = cmd.memo.clone().unwrap_or_else(|| {
        if mint {
            format!("mint {amount}")
        } else {
            format!("burn {amount}")
        }
    });
    let action = if mint { "wallet.mint" } else { "wallet.burn" };
    let outcome = store
        .apply(
            ApplyOptions {
                actor: "admin".into(),
                action: action.into(),
                target: Some(account_id.clone()),
                escrow_id: None,
                idempotency_key: key.clone(),
            },
            |ledger: &LedgerState, _reg: &EscrowRegistry| -> DomainResult<CommandOutcome> {
                // مادهٔ ۳ — سقف ضرب سالانه فقط روی مسیر mint؛ خوانش از همان
                // وضعیتِ لودشده داخل تراکنش — دو ضرب هم‌زمان هرگز از سقف می‌گذرند.
                if mint {
                    let minted = minted_this_year_locked(ledger);
                    assert_mint_within_cap(Din::new(minted)?, amount)?;
                }
                let postings = if mint {
                    vec![
                        Posting::new(MINT_ACCOUNT_ID, -amount.as_i64()),
                        Posting::new(account_id.clone(), amount.as_i64()),
                    ]
                } else {
                    vec![
                        Posting::new(account_id.clone(), -amount.as_i64()),
                        Posting::new(BURN_ACCOUNT_ID, amount.as_i64()),
                    ]
                };
                let posted = post(
                    ledger,
                    &PostCommand {
                        id: None,
                        idempotency_key: key.clone(),
                        postings,
                        memo: Some(memo),
                        at: Some(now_iso()),
                    },
                )?;
                let event = TransactionPostedEvent::new(&posted.tx.id, &posted.tx.at);
                // memo/key/correlation را کامل می‌کنیم — شکل سیمی رویداد فاز ۵
                let event = TransactionPostedEvent {
                    memo: posted.tx.memo.clone(),
                    idempotency_key: posted.tx.idempotency_key.clone(),
                    ..event
                };
                Ok(CommandOutcome {
                    ledger: posted.state,
                    tx: Some(posted.tx),
                    escrow: None,
                    events: vec![DomainEvent::TransactionPosted(event)],
                })
            },
        )
        .await?;
    Ok(outcome)
}

/// `POST /wallet/mint` — ضرب دیین به یک حساب. ادمین.
///
/// # Errors
/// ۴۲۲ `MINT_CAP_EXCEEDED` (مادهٔ ۳)؛ 404 `UNKNOWN_ACCOUNT`؛ 400 بدنهٔ نامعتبر.
pub async fn mint(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Json<WalletMoveResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: MintRequest = crate::handlers::parse_body(&body)?;
    let amount = Din::new(req.amount.get())?;
    let cmd = MoveCmd {
        account_id: req.account_id.get().to_owned(),
        amount,
        memo: req.memo,
        key: req.idempotency_key.as_ref().map(|k| k.get().to_owned()),
    };
    let result = wallet_move(&state.store, cmd, true).await?;
    to_move_response(&result, req.account_id.get())
}

/// `POST /wallet/burn` — سوزاندن دیین از یک حساب؛ overdraft ناممکن (مادهٔ ۴).
///
/// # Errors
/// ۴۲۲ `ACCOUNT_OVERDRAFT`؛ 404 `UNKNOWN_ACCOUNT`؛ 400 بدنهٔ نامعتبر.
pub async fn burn(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Json<WalletMoveResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: BurnRequest = crate::handlers::parse_body(&body)?;
    let amount = Din::new(req.amount.get())?;
    let cmd = MoveCmd {
        account_id: req.account_id.get().to_owned(),
        amount,
        memo: req.memo,
        key: req.idempotency_key.as_ref().map(|k| k.get().to_owned()),
    };
    let result = wallet_move(&state.store, cmd, false).await?;
    to_move_response(&result, req.account_id.get())
}

fn to_move_response(result: &ApplyResult, account_id: &str) -> ApiResult<Json<WalletMoveResponse>> {
    let tx = result.tx.as_ref().ok_or(dropagentx_contracts::HttpFailure::internal_no_tx())?;
    let balance = result.ledger.balances.get(account_id).copied().unwrap_or(0);
    Ok(Json(WalletMoveResponse {
        tx_id: tx.id.clone(),
        account_id: account_id.to_owned(),
        balance,
        replayed: result.replayed,
    }))
}
