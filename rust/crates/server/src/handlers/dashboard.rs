//! هندلرهای داشبورد (command-center) — همهٔ مسیرهای این گروه پاکت
//! `{ ok: true, data: … }` برمی‌گردانند (بخشی از قرارداد — SPA هر شکل 2xx
//! دیگری را شکست می‌داند). داده‌ها همه از جدول‌های زندهٔ v3 مشتق می‌شوند؛
//! ریل‌های کویست/رزرو/تنظیمات تا رسیدن موتورشان خالی یا 501 صادقانه‌اند.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

use dropagentx_contracts::{
    ApprovalDecisionData, ApprovalDecisionRequest, ApprovalDecisionResponse, ApprovalListResponse,
    ApprovalView, AuditRow, BrandData, BrandResponse, CreateProductRequest, DashListData,
    DashListResponse, DashUserListData, DashUserListResponse, DashWalletListData,
    DashWalletListResponse, HttpFailure, LeaderboardData, LeaderboardResponse, LeaderboardType,
    OrderAction, OrderActionData, OrderActionRequest, OrderActionResponse, OrderListData,
    OrderListResponse, OverviewData, OverviewResponse, ProductListData, ProductListResponse,
    ProductMutationResponse, SecurityData, SecurityLayer, SecurityResponse, SeriesPoint,
    SettingEntry, SettingsListData, SettingsListResponse, TopProduct, UpdateProductRequest,
};
use dropagentx_db::{now_iso, queries::deliver_order, AuditRowRead, ProductFilter, ProductRow};
use dropagentx_domain::{cancel_escrow, refund_escrow, EscrowRefCommand, TREASURY_ACCOUNT_ID};

use crate::error::ApiResult;
use crate::middleware::require_admin;
use crate::state::AppState;
use crate::views::{
    approval_view, audit_row, dash_user, dash_wallet, leaderboard_entry, order_status_filter,
    order_view, product_view, recent_order_view,
};

// ── استخراج کوئری‌ها — همان coerce/پیش‌فرض‌های zod رجیستری ───────────────────

/// پارامترهای کوئری مشترک فهرست‌ها.
#[derive(Debug, Default, Deserialize)]
pub struct ListQuery {
    /// سقف صفحه
    pub limit: Option<i64>,
    /// از این‌جا
    pub offset: Option<i64>,
    /// جست‌وجو
    pub q: Option<String>,
    /// فیلتر وضعیت (orders/approvals)
    pub status: Option<String>,
    /// نوع لیدربورد
    pub r#type: Option<String>,
    /// کاربر هدف (`/ai/usage`)
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
}

/// اعتبارسنجی limit/offset با بازهٔ و پیش‌فرض رجیستری — خارج از بازه → 400
/// (مثل `z.coerce.number().int().min().max().default()`).
fn validate_paging(
    limit: Option<i64>,
    offset: Option<i64>,
    max: i64,
    default: i64,
) -> ApiResult<(i64, i64)> {
    let limit = limit.unwrap_or(default);
    let offset = offset.unwrap_or(0);
    if !(1..=max).contains(&limit) || offset < 0 {
        return Err(HttpFailure::invalid_request(1).into());
    }
    Ok((limit, offset))
}

/// نقشهٔ فروش واقعی هر محصول — یک کوئری گروهی به‌جای N+1.
async fn sales_map(state: &AppState) -> ApiResult<std::collections::HashMap<String, i64>> {
    Ok(state.store.sales_by_product().await?.into_iter().collect())
}

// ── برند ────────────────────────────────────────────────────────────────────

/// `GET /brand` — شل وایت‌لیبل از env. عمومی.
///
/// # Errors
/// هیچ.
pub async fn brand(State(state): State<Arc<AppState>>) -> Json<BrandResponse> {
    BrandResponse {
        ok: true,
        data: BrandData {
            name: state.config.brand_name.clone(),
            tagline: state.config.brand_tagline.clone(),
        },
    }
    .into()
}

// ── overview ────────────────────────────────────────────────────────────────

/// `GET /overview` — KPIها + سری ۱۴روزه + برترین‌ها + سفارش‌های اخیر؛ همه از
/// جدول‌های زنده. ادمین.
///
/// # Errors
/// خطای SQL.
pub async fn overview(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> ApiResult<Json<OverviewResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let users = state.store.count_users().await?;
    let products = state.store.count_products(&ProductFilter::default()).await?;
    let orders = state.store.count_escrows().await?;
    let pending_approvals = state.store.count_hitl("PENDING").await?;
    let ai_calls = state.store.ai_calls_total().await?;

    // سری ۱۴روزه — صفرپر دقیقاً ۱۴ روز UTC آخر (چارت همین را می‌کشد)
    let revenue = state.store.revenue_series_14d().await?;
    let mut by_day: HashMap<String, i64> = HashMap::new();
    for (day, amount) in revenue {
        by_day.insert(day, amount);
    }
    let escrows = state.store.list_escrows(None).await?;
    let mut orders_by_day: HashMap<String, i64> = HashMap::new();
    for e in &escrows {
        let day = e.created_at.chars().take(10).collect::<String>();
        *orders_by_day.entry(day).or_insert(0) += 1;
    }
    let mut series = Vec::with_capacity(14);
    for i in (0..14).rev() {
        let date = (chrono::Utc::now() - chrono::Duration::days(i)).format("%Y-%m-%d").to_string();
        let revenue = by_day.get(&date).copied().unwrap_or(0);
        let day_orders = orders_by_day.get(&date).copied().unwrap_or(0);
        series.push(SeriesPoint { date, revenue, orders: day_orders });
    }
    // KPI درآمد ۲۴ساعته = درآمد امروز (نقطهٔ آخر سری)
    let revenue_credits = series.last().map(|p| p.revenue).unwrap_or(0);

    // برترین محصولات بر اساس فروش واقعی (اسکروهای تسویه‌شده)
    let mut rows = state
        .store
        .list_products(&ProductFilter { limit: 200, offset: 0, ..ProductFilter::default() })
        .await?;
    let sales = sales_map(&state).await?;
    rows.sort_by(|a, b| {
        sales.get(&b.id).copied().unwrap_or(0).cmp(&sales.get(&a.id).copied().unwrap_or(0))
    });
    let top_products = rows
        .iter()
        .take(5)
        .map(|p| TopProduct {
            title: p.title.clone(),
            sales_count: sales.get(&p.id).copied().unwrap_or(0),
        })
        .collect();

    // سفارش‌های اخیر — ۱۰ امانت آخر
    let mut recent_orders = Vec::new();
    for e in escrows.iter().take(10) {
        let title = state.store.product_title(e.product_ref.as_deref()).await?;
        recent_orders.push(recent_order_view(e, title));
    }

    Ok(Json(OverviewResponse {
        ok: true,
        data: OverviewData {
            users,
            products,
            orders,
            revenue_stars: 0, // ریل استارز تا رسیدن ریل واقعی صفر می‌ماند
            revenue_credits,
            pending_approvals,
            ai_calls,
            series,
            top_products,
            recent_orders,
        },
    }))
}

// ── محصولات ─────────────────────────────────────────────────────────────────

/// `GET /products` — مارکت برای command-center؛ عمومی با q/limit/offset.
///
/// # Errors
/// پجینگ خارج از بازه → 400؛ خطای SQL.
pub async fn list_products(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<ProductListResponse>> {
    let (limit, offset) = validate_paging(q.limit, q.offset, 100, 12)?;
    if let Some(term) = &q.q {
        if term.chars().count() > 80 {
            return Err(HttpFailure::invalid_request(1).into());
        }
    }
    let filter = ProductFilter { q: q.q, limit, offset, active_only: false };
    let total = state.store.count_products(&filter).await?;
    let sales = sales_map(&state).await?;
    let items = state
        .store
        .list_products(&filter)
        .await?
        .iter()
        .map(|p| product_view(p, sales.get(&p.id).copied().unwrap_or(0)))
        .collect();
    Ok(Json(ProductListResponse {
        ok: true,
        data: ProductListData { items, total, limit, offset },
    }))
}

/// `POST /products` — ساخت لیستین (فروشنده = خزانه؛ Stars ریل نیست). ادمین. ۲۰۱.
///
/// # Errors
/// 400 بدنهٔ نامعتبر؛ خطای SQL.
pub async fn create_product(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<(StatusCode, Json<ProductMutationResponse>)> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: CreateProductRequest = crate::handlers::parse_body(&body)?;
    // Stars/stock/featured ستون ندارند (رجیستری: فقط نمای داشبورد) — پذیرفته
    // می‌شوند اما ریل استارز هنوز نکاشته شده؛ credits روی `priceDin` می‌نشیند.
    let _ = req.price_stars;
    let _ = req.stock;
    let _ = req.featured;
    let row = ProductRow {
        id: format!("prod:{}", uuid::Uuid::now_v7()),
        seller_id: TREASURY_ACCOUNT_ID.into(),
        title: req.title,
        description: req.description.unwrap_or_default(),
        category: req.category, // None → NULL؛ نمای خروجی "" می‌دهد
        price_din: req.price_credits.map(|d| d.get()).unwrap_or(0),
        delivery_payload: None,
        active: true,
        created_at: now_iso(),
    };
    state.store.insert_product(&row).await?;
    Ok((
        StatusCode::CREATED,
        Json(ProductMutationResponse { ok: true, data: product_view(&row, 0) }),
    ))
}

/// `PATCH /products/{id}` — ویرایش جزئی لیستین. ادمین.
///
/// # Errors
/// 404 `PRODUCT_NOT_FOUND`؛ 400 بدنهٔ نامعتبر.
pub async fn update_product(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Json<ProductMutationResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: UpdateProductRequest = crate::handlers::parse_body(&body)?;
    let _ = req.price_stars; // ریل استارز — فقط نمای داشبورد
    let _ = req.stock;
    let _ = req.featured;
    let row = state
        .store
        .update_product(
            &id,
            req.title.as_deref(),
            req.category.as_deref(),
            req.price_credits.map(|d| d.get()),
            req.description.as_deref(),
            req.active,
        )
        .await?;
    let sales = sales_map(&state).await?;
    Ok(Json(ProductMutationResponse {
        ok: true,
        data: product_view(&row, sales.get(&row.id).copied().unwrap_or(0)),
    }))
}

/// `DELETE /products/{id}` — آرشیو نرم؛ ردیف برای ارجاع‌های دفتر می‌ماند. ادمین.
///
/// # Errors
/// 404 `PRODUCT_NOT_FOUND`.
pub async fn archive_product(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<ProductMutationResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let row = state.store.update_product(&id, None, None, None, None, Some(false)).await?;
    let sales = sales_map(&state).await?;
    Ok(Json(ProductMutationResponse {
        ok: true,
        data: product_view(&row, sales.get(&row.id).copied().unwrap_or(0)),
    }))
}

// ── سفارش‌ها (نماهای اسکرو) ─────────────────────────────────────────────────

/// `GET /orders` — امانت‌ها در شکل داشبورد با فیلتر وضعیت. ادمین.
///
/// # Errors
/// فیلتر نامعتبر → 400؛ خطای SQL.
pub async fn list_orders(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<OrderListResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let (limit, _offset) = validate_paging(q.limit, q.offset, 100, 20)?;
    let status_word = q.status.unwrap_or_else(|| "all".into());
    let escrows = state.store.list_escrows(order_status_filter(&status_word)).await?;
    let total = escrows.len() as i64;
    let mut items = Vec::new();
    for e in escrows.into_iter().take(limit as usize) {
        let title = state.store.product_title(e.product_ref.as_deref()).await?;
        items.push(order_view(&e, title));
    }
    Ok(Json(OrderListResponse { ok: true, data: OrderListData { items, total } }))
}

/// `POST /orders/{id}/status` — اقدام سفارش از مسیر دامنهٔ خالص:
/// deliver از FUNDED، cancel از CREATED، refund از DISPUTED. ادمین.
///
/// # Errors
/// ۴۲۲ گذار نامعتبر؛ 404 ناشناخته؛ 400 بدنهٔ نامعتبر.
pub async fn order_action(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Json<OrderActionResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: OrderActionRequest = crate::handlers::parse_body(&body)?;
    let escrow = match req.action {
        OrderAction::Deliver => deliver_order(&state.store, &id, None).await?,
        OrderAction::Cancel => state
            .store
            .apply(
                dropagentx_db::ApplyOptions {
                    actor: "admin".into(),
                    action: "escrow.cancel".into(),
                    target: Some(id.clone()),
                    escrow_id: Some(id.clone()),
                    idempotency_key: None,
                },
                |ledger, reg| {
                    cancel_escrow(
                        ledger,
                        reg,
                        &EscrowRefCommand {
                            escrow_id: id.clone(),
                            idempotency_key: None,
                            at: Some(now_iso()),
                        },
                    )
                    .map(|o| dropagentx_db::CommandOutcome {
                        ledger: o.ledger,
                        tx: o.tx,
                        escrow: Some(o.escrow),
                        events: o
                            .events
                            .into_iter()
                            .map(dropagentx_domain::DomainEvent::Escrow)
                            .collect(),
                    })
                },
            )
            .await?
            .escrow
            .ok_or(HttpFailure::internal_no_escrow())?,
        OrderAction::Refund => state
            .store
            .apply(
                dropagentx_db::ApplyOptions {
                    actor: "admin".into(),
                    action: "escrow.refund".into(),
                    target: Some(id.clone()),
                    escrow_id: Some(id.clone()),
                    idempotency_key: None,
                },
                |ledger, reg| {
                    refund_escrow(
                        ledger,
                        reg,
                        &EscrowRefCommand {
                            escrow_id: id.clone(),
                            idempotency_key: None,
                            at: Some(now_iso()),
                        },
                    )
                    .map(|o| dropagentx_db::CommandOutcome {
                        ledger: o.ledger,
                        tx: o.tx,
                        escrow: Some(o.escrow),
                        events: o
                            .events
                            .into_iter()
                            .map(dropagentx_domain::DomainEvent::Escrow)
                            .collect(),
                    })
                },
            )
            .await?
            .escrow
            .ok_or(HttpFailure::internal_no_escrow())?,
    };
    let status = crate::views::order_status_word(escrow.status);
    Ok(Json(OrderActionResponse { ok: true, data: OrderActionData { id: escrow.id, status } }))
}

// ── کاربران + کیف‌ها ────────────────────────────────────────────────────────

/// `GET /users` — حساب‌های کاربر برای تب wallets→users. ادمین.
///
/// # Errors
/// پجینگ خارج از بازه → 400؛ خطای SQL.
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<DashUserListResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let (limit, _) = validate_paging(q.limit, q.offset, 200, 100)?;
    let items = state
        .store
        .list_accounts_with_balances()
        .await?
        .iter()
        .filter(|a| a.kind == "user")
        .take(limit as usize)
        .map(dash_user)
        .collect();
    Ok(Json(DashUserListResponse { ok: true, data: DashUserListData { items } }))
}

/// `GET /wallets` — ترازهای مشتق + قفل اسکرو به ازای هر حساب کاربر. ادمین.
///
/// # Errors
/// پجینگ خارج از بازه → 400؛ خطای SQL.
pub async fn list_wallets(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<DashWalletListResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let (limit, _) = validate_paging(q.limit, q.offset, 200, 100)?;
    let accounts = state.store.list_accounts_with_balances().await?;
    let mut items = Vec::new();
    for a in accounts.iter().filter(|a| a.kind == "user").take(limit as usize) {
        let locked = state.store.locked_for_buyer(&a.id).await?;
        items.push(dash_wallet(a, locked));
    }
    Ok(Json(DashWalletListResponse { ok: true, data: DashWalletListData { items } }))
}

// ── ریل‌های آینده + لیدربورد ────────────────────────────────────────────────

/// `GET /quests` — ریل کویست؛ خالی تا رسیدن موتور کویست. عمومی.
///
/// # Errors
/// هیچ.
pub async fn quests() -> Json<DashListResponse> {
    DashListResponse { ok: true, data: DashListData { items: Vec::new() } }.into()
}

/// `GET /bookings` — ریل رزرو؛ خالی تا رسیدن موتور رزرو. عمومی.
///
/// # Errors
/// هیچ.
pub async fn bookings() -> Json<DashListResponse> {
    DashListResponse { ok: true, data: DashListData { items: Vec::new() } }.into()
}

/// `GET /leaderboard` — sales واقعی (اسکروهای تسویه‌شده)؛ xp/ref با موتور کویست.
///
/// # Errors
/// فیلتر نامعتبر → 400؛ خطای SQL.
pub async fn leaderboard(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<LeaderboardResponse>> {
    let (limit, _) = validate_paging(q.limit, q.offset, 50, 25)?;
    let board: LeaderboardType = match q.r#type.as_deref() {
        Some("sales") => LeaderboardType::Sales,
        Some("ref") => LeaderboardType::Ref,
        _ => LeaderboardType::Xp, // پیش‌فرض رجیستری
    };
    let items = match board {
        LeaderboardType::Sales => state
            .store
            .sales_by_seller()
            .await?
            .into_iter()
            .take(limit as usize)
            .enumerate()
            .map(|(i, (name, sales))| leaderboard_entry(i as i64 + 1, name, sales))
            .collect(),
        LeaderboardType::Xp | LeaderboardType::Ref => Vec::new(),
    };
    Ok(Json(LeaderboardResponse { ok: true, data: LeaderboardData { items } }))
}

// ── تأییدها (نماهای HITL) ───────────────────────────────────────────────────

/// فیلتر وضعیت تأییدها → وضعیت HITL دیتابیس.
fn approval_status_filter(word: &str) -> ApiResult<Option<&'static str>> {
    match word {
        "pending" => Ok(Some("PENDING")),
        "approved" => Ok(Some("APPROVED")),
        "rejected" => Ok(Some("REJECTED")),
        "expired" => Ok(Some("EXPIRED")),
        "all" => Ok(None),
        _ => Err(HttpFailure::invalid_request(1).into()),
    }
}

/// `GET /approvals` — صف HITL در شکل داشبورد؛ پیش‌فرض pending. ادمین.
///
/// # Errors
/// فیلتر نامعتبر → 400؛ خطای SQL.
pub async fn list_approvals(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<ApprovalListResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let word = q.status.unwrap_or_else(|| "pending".into());
    let status = approval_status_filter(&word)?;
    let items = state
        .store
        .list_hitl(status)
        .await?
        .iter()
        .map(approval_view)
        .collect::<Result<Vec<ApprovalView>, _>>()?;
    Ok(Json(ApprovalListResponse {
        ok: true,
        data: dropagentx_contracts::ApprovalListData { items },
    }))
}

/// `POST /approvals/{id}/decide` — تصمیم ادمین؛ approve تسویه را در همان
/// تراکنش اجرا می‌کند (۹۰/۱۰)، reject پول را قفل نگه می‌دارد. ادمین.
///
/// # Errors
/// ۴۲۲ `HITL_ALREADY_DECIDED`؛ 404 ناشناخته؛ 400 بدنهٔ نامعتبر.
pub async fn approval_decision(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Json<ApprovalDecisionResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let req: ApprovalDecisionRequest = crate::handlers::parse_body(&body)?;
    // هویت تصمیم‌گیرنده روی این مسیر «داشبورد» است — ردیف audit همان را می‌برد
    const ACTOR: &str = "dashboard";
    let data = if req.approve {
        let d = state.store.approve_hitl_and_settle(&id, ACTOR, req.note.as_deref()).await?;
        ApprovalDecisionData {
            id: d.request.id,
            status: "approved".into(),
            escrow_id: Some(d.request.escrow_id),
            tx_id: d.tx.map(|t| t.id),
        }
    } else {
        let row = state.store.reject_hitl(&id, ACTOR, req.note.as_deref()).await?;
        ApprovalDecisionData {
            id: row.id,
            status: "rejected".into(),
            escrow_id: Some(row.escrow_id),
            tx_id: None,
        }
    };
    Ok(Json(ApprovalDecisionResponse { ok: true, data }))
}

// ── تنظیمات + امنیت ─────────────────────────────────────────────────────────

/// `GET /settings` — KV ران‌تایم؛ خالی تا رسیدن مدل KV. ادمین.
///
/// # Errors
/// 401/503 بدون کلید ادمین معتبر.
pub async fn list_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> ApiResult<Json<SettingsListResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    Ok(Json(SettingsListResponse {
        ok: true,
        data: SettingsListData { items: Vec::<SettingEntry>::new() },
    }))
}

/// `PUT /settings` — 501 صادقانه تا رسیدن مدل KV (بهتر از no-op خاموش).
///
/// # Errors
/// همیشه 501 `NOT_IMPLEMENTED`؛ 401 بدون کلید ادمین.
pub async fn put_setting(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Json<SettingsListResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let _req: dropagentx_contracts::SettingsPutRequest = crate::handlers::parse_body(&body)?;
    Err(HttpFailure::not_implemented().into())
}

/// `GET /security` — لایه‌های دفاع زنده (فلگ‌های واقعی کانفیگ) + دنبالهٔ audit.
///
/// # Errors
/// خطای SQL.
pub async fn security(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> ApiResult<Json<SecurityResponse>> {
    require_admin(&headers, state.config.admin_key.as_deref())?;
    let layers = vec![
        SecurityLayer {
            id: "kia-key".into(),
            name: "X-KIA-Key admin gate".into(),
            description: Some(
                "Fail-closed admin key on every privileged route — constant-time compare.".into(),
            ),
            active: state.config.admin_key.is_some(),
        },
        SecurityLayer {
            id: "rate-limit".into(),
            name: "Sliding-window rate limit".into(),
            description: Some(format!(
                "{} requests/minute — separate trusted bucket for admin callers.",
                state.config.rate_limit
            )),
            active: true,
        },
        SecurityLayer {
            id: "telegram-init-data".into(),
            name: "Telegram initData HMAC".into(),
            description: Some(
                "Official WebApp HMAC-SHA256 verification for public AI callers.".into(),
            ),
            active: state.config.bot_token.is_some(),
        },
        SecurityLayer {
            id: "audit-log".into(),
            name: "Audit trail".into(),
            description: Some(
                "Every money command writes an audit row inside the same transaction.".into(),
            ),
            active: true,
        },
        SecurityLayer {
            id: "ledger-invariants".into(),
            name: "Ledger invariants".into(),
            description: Some(
                "Double-entry Σ=0, overdraft-impossible balances, derived-only balances.".into(),
            ),
            active: true,
        },
    ];
    let audit = state
        .store
        .list_audit(20)
        .await?
        .iter()
        .map(|r: &AuditRowRead| audit_row(r))
        .collect::<Vec<AuditRow>>();
    Ok(Json(SecurityResponse { ok: true, data: SecurityData { layers, audit } }))
}
