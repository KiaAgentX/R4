//! قراردادهای وایرِ داشبورد (command-center) — پورت `dashboard.ts`.
//!
//! همهٔ مسیرهای داشبورد پاکت `{ ok: true, data: … }` را برمی‌گردانند — SPA هر
//! شکل 2xx دیگری را شکست می‌داند؛ پس پاکت بخشی از قرارداد است نه یک قراردادِ
//! اخلاقی. نکتهٔ ارز: دفتر v3 تک‌ارزی (دیین) است؛ UI دو اسلات درآمد دارد —
//! Stars ⭐ تا رسیدن ریل استارز صفر می‌ماند، credits اعداد دیین را می‌برد.

use serde::{Deserialize, Serialize};
use serde_json::Map;
use utoipa::ToSchema;

use crate::common::{Din, IsoDateTime};

/// شمارندهٔ غیرمنفی داشبورد — `dashCount` (int ≥ 0). NOTE: فیلدها عمداً `i64` خام‌اند
/// چون utoipa روی alias محدودیت‌های minimum/maximum را اعمال نمی‌کند (رفع‌شده با حاشیه‌نویسی مستقیم).
#[allow(dead_code)]
type DashCount = i64;

/// شخص (خریدار/کاربر) در نماها.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashPerson {
    /// نام نمایشی (nullable — صریح روی سیم)
    #[schema(required = true)]
    pub name: Option<String>,
    /// یوزرنیم تلگرام (nullable — صریح)
    #[schema(required = true)]
    pub username: Option<String>,
}

// ── برند ────────────────────────────────────────────────────────────────────

/// `brandResponseSchema` — `{ok:true, data:{name, tagline}}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrandResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: BrandData,
}

/// بدنهٔ برند.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrandData {
    /// نام برند
    #[schema(min_length = 1)]
    pub name: String,
    /// شعار
    pub tagline: String,
}

// ── overview ────────────────────────────────────────────────────────────────

/// `overviewResponseSchema` — KPIهای ۲۴ساعته + سری ۱۴روزه + برترین‌ها.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverviewResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: OverviewData,
}

/// بدنهٔ overview.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverviewData {
    /// کاربران
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub users: i64,
    /// محصولات
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub products: i64,
    /// سفارش‌ها
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub orders: i64,
    /// درآمد استارز (ریل آینده)
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub revenue_stars: i64,
    /// درآمد کریديت (دیین)
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub revenue_credits: i64,
    /// تأییدهای در انتظار (HITL)
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub pending_approvals: i64,
    /// فراخوانی‌های AI
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub ai_calls: i64,
    /// دقیقاً ۱۴ روز UTC آخر، صفرپر — چارت همین را می‌کشد
    pub series: Vec<SeriesPoint>,
    /// برترین محصولات
    pub top_products: Vec<TopProduct>,
    /// سفارش‌های اخیر
    pub recent_orders: Vec<RecentOrder>,
}

/// نقطهٔ سری ۱۴روزه.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeriesPoint {
    /// `YYYY-MM-DD`
    #[schema(pattern = r"^\d{4}-\d{2}-\d{2}$")]
    pub date: String,
    /// درآمد روز
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub revenue: i64,
    /// سفارش‌های روز
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub orders: i64,
}

/// محصول برتر.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TopProduct {
    /// عنوان
    pub title: String,
    /// تعداد فروش
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub sales_count: i64,
}

/// سفارش اخیر.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecentOrder {
    /// شناسه
    pub id: String,
    /// عنوان محصول (nullable)
    #[schema(required = true)]
    pub title: Option<String>,
    /// خریدار
    pub buyer: DashPerson,
    /// کل استارز
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub total_stars: i64,
    /// کل کریديت
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub total_credits: i64,
    /// وضعیت
    pub status: String,
    /// زمان ایجاد
    pub created_at: IsoDateTime,
}

// ── محصولات (مارکت) ─────────────────────────────────────────────────────────

/// وضعیت محصول.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProductStatus {
    /// فعال
    Active,
    /// آرشیو‌شده
    Archived,
}

/// نمای محصول — `productViewSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductView {
    /// شناسه
    pub id: String,
    /// فروشنده
    pub seller_id: String,
    /// عنوان
    pub title: String,
    /// توضیح
    pub description: String,
    /// دسته
    pub category: String,
    /// قیمت استارز
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub price_stars: i64,
    /// قیمت کریديت
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub price_credits: i64,
    /// −۱ = نامحدود (قرارداد داشبورد)
    #[schema(minimum = -9_007_199_254_740_991_i64, maximum = 9_007_199_254_740_991_i64)]
    pub stock: i64,
    /// تعداد فروش
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub sales_count: i64,
    /// ویژه؟
    pub featured: bool,
    /// وضعیت
    pub status: ProductStatus,
    /// زمان ایجاد
    pub created_at: IsoDateTime,
}

/// `productListResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductListResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: ProductListData,
}

/// بدنهٔ لیست محصولات (paging).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductListData {
    /// ردیف‌ها
    pub items: Vec<ProductView>,
    /// کل
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub total: i64,
    /// سقف صفحه
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub limit: i64,
    /// از این‌جا
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub offset: i64,
}

/// `createProductRequestSchema`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateProductRequest {
    /// عنوان (۱ تا ۱۲۰ — اجباری)
    #[schema(min_length = 1, max_length = 120)]
    pub title: String,
    /// دسته (≤۴۰)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_length = 40)]
    pub category: Option<String>,
    /// قیمت استارز
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_stars: Option<Din>,
    /// قیمت کريديت
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_credits: Option<Din>,
    /// موجودی
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(minimum = -9_007_199_254_740_991_i64, maximum = 9_007_199_254_740_991_i64)]
    pub stock: Option<i64>,
    /// ویژه؟
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured: Option<bool>,
    /// توضیح (≤۸۰۰)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_length = 800)]
    pub description: Option<String>,
}

/// `updateProductRequestSchema` — partial + `active` (فرم ویرایش چک‌باکس فعال دارد).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProductRequest {
    /// عنوان
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(min_length = 1, max_length = 120)]
    pub title: Option<String>,
    /// دسته
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_length = 40)]
    pub category: Option<String>,
    /// قیمت استارز
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_stars: Option<Din>,
    /// قیمت کريديت
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_credits: Option<Din>,
    /// موجودی
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(minimum = -9_007_199_254_740_991_i64, maximum = 9_007_199_254_740_991_i64)]
    pub stock: Option<i64>,
    /// ویژه؟
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured: Option<bool>,
    /// توضیح
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_length = 800)]
    pub description: Option<String>,
    /// نگاشت به ستون `active`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

/// `productMutationResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductMutationResponse {
    /// همیشه true
    pub ok: bool,
    /// محصول
    pub data: ProductView,
}

// ── سفارش‌ها (نماهای اسکرو مارکت) ────────────────────────────────────────────

/// نمای سفارش — `orderViewSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderView {
    /// شناسه
    pub id: String,
    /// محصول
    pub product: String,
    /// خریدار
    pub buyer: DashPerson,
    /// کل استارز
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub total_stars: i64,
    /// کل کريديت
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub total_credits: i64,
    /// وضعیت
    pub status: String,
    /// زمان ایجاد
    pub created_at: IsoDateTime,
}

/// `orderListResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderListResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: OrderListData,
}

/// بدنهٔ لیست سفارش‌ها.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderListData {
    /// ردیف‌ها
    pub items: Vec<OrderView>,
    /// کل
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub total: i64,
}

/// اقدام سفارش — `orderActionRequestSchema`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum OrderAction {
    /// تحویل
    Deliver,
    /// لغو
    Cancel,
    /// بازپرداخت
    Refund,
}

/// درخواست اقدام سفارش.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderActionRequest {
    /// اقدام
    pub action: OrderAction,
}

/// `orderActionResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderActionResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: OrderActionData,
}

/// بدنهٔ اقدام سفارش.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderActionData {
    /// شناسه
    pub id: String,
    /// وضعیت پس از اقدام
    pub status: String,
}

// ── کاربران + کیف‌پول‌ها ────────────────────────────────────────────────────

/// نمای کاربر داشبورد — `dashUserSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashUser {
    /// شناسه
    pub id: String,
    /// نام
    pub name: String,
    /// یوزرنیم (nullable)
    #[schema(required = true)]
    pub username: Option<String>,
    /// نقش
    pub role: String,
    /// زبان (nullable)
    #[schema(required = true)]
    pub language: Option<String>,
    /// زمان ایجاد
    pub created_at: IsoDateTime,
}

/// `dashUserListResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashUserListResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: DashUserListData,
}

/// بدنهٔ لیست کاربران.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashUserListData {
    /// ردیف‌ها
    pub items: Vec<DashUser>,
}

/// نمای کیف‌پول داشبورد — `dashWalletSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashWallet {
    /// نام
    pub name: String,
    /// یوزرنیم (nullable)
    #[schema(required = true)]
    pub username: Option<String>,
    /// نقش
    pub role: String,
    /// موجودی
    #[schema(minimum = -9_007_199_254_740_991_i64, maximum = 9_007_199_254_740_991_i64)]
    pub balance: i64,
    /// قفل‌شده در اسکرو
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub locked: i64,
    /// کريديت
    #[schema(minimum = -9_007_199_254_740_991_i64, maximum = 9_007_199_254_740_991_i64)]
    pub credits: i64,
}

/// `dashWalletListResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashWalletListResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: DashWalletListData,
}

/// بدنهٔ لیست کیف‌پول‌ها.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashWalletListData {
    /// ردیف‌ها
    pub items: Vec<DashWallet>,
}

// ── کویست‌ها / لیدربورد / رزروها ─────────────────────────────────────────────

/// `dashListResponseSchema` — ریل‌های خالی آینده (کویست/رزرو) — رکورد آزاد.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashListResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: DashListData,
}

/// بدنهٔ لیست آزاد — `Record<string, unknown>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashListData {
    /// رکوردهای آزاد
    #[schema(value_type = Vec<Object>)]
    pub items: Vec<Map<String, serde_json::Value>>,
}

/// نوع لیدربورد — `leaderboardQuerySchema`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum LeaderboardType {
    /// امتیاز تجربه (ریل آینده — پیش‌فرض)
    #[default]
    Xp,
    /// فروش (واقعی)
    Sales,
    /// رفرال (ریل آینده)
    Ref,
}

/// `leaderboardResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: LeaderboardData,
}

/// بدنهٔ لیدربورد.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardData {
    /// ردیف‌ها
    pub items: Vec<LeaderboardEntry>,
}

/// ردیف لیدربورد.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardEntry {
    /// رتبه
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub rank: i64,
    /// نام
    pub name: String,
    /// یوزرنیم (nullable)
    #[schema(required = true)]
    pub username: Option<String>,
    /// XP
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub xp: i64,
    /// فروش
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub sales: i64,
    /// رفرال‌ها
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub refs: i64,
}

// ── تأییدها (نماهای HITL) ───────────────────────────────────────────────────

/// سطح ریسک — از آستانهٔ منشور مشتق می‌شود.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// کم
    Low,
    /// متوسط
    Medium,
    /// زیاد (≥ ۵۰٬۰۰۰ Đ)
    High,
}

/// نمای تأیید — `approvalViewSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalView {
    /// شناسه
    pub id: String,
    /// نوع (`type` روی وایر — رزرو‌شدهٔ Rust با raw identifier)
    pub r#type: String,
    /// سطح ریسک
    pub risk_level: RiskLevel,
    /// خلاصهٔ انسانی
    pub summary: String,
    /// یادداشت (nullable)
    #[schema(required = true)]
    pub note: Option<String>,
    /// مبلغ
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub amount: i64,
    /// وضعیت (lowercase برای UI)
    pub status: String,
    /// زمان ایجاد
    pub created_at: IsoDateTime,
    /// زمان انقضا (nullable)
    #[schema(required = true)]
    pub expires_at: Option<IsoDateTime>,
}

/// فیلتر وضعیت تأییدها — `approvalListQuerySchema`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatusFilter {
    /// در انتظار (پیش‌فرض)
    #[default]
    Pending,
    /// تأییدشده
    Approved,
    /// ردشده
    Rejected,
    /// منقضی
    Expired,
    /// همه
    All,
}

/// `approvalListResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalListResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: ApprovalListData,
}

/// بدنهٔ لیست تأییدها.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalListData {
    /// ردیف‌ها
    pub items: Vec<ApprovalView>,
}

/// `approvalDecisionRequestSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalDecisionRequest {
    /// تأیید یا رد
    pub approve: bool,
    /// یادداشت (≤۵۰۰)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_length = 500)]
    pub note: Option<String>,
}

/// `approvalDecisionResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalDecisionResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: ApprovalDecisionData,
}

/// بدنهٔ تصمیم تأیید.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalDecisionData {
    /// شناسهٔ درخواست
    pub id: String,
    /// وضعیت پس از تصمیم
    pub status: String,
    /// امانت مربوط (nullable)
    #[schema(required = true)]
    pub escrow_id: Option<String>,
    /// تراکنش تسویه (nullable)
    #[schema(required = true)]
    pub tx_id: Option<String>,
}

// ── تنظیمات + امنیت ─────────────────────────────────────────────────────────

/// `settingsListResponseSchema` — KV ران‌تایم (تا رسیدن مدل KV خالی).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SettingsListResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: SettingsListData,
}

/// بدنهٔ لیست تنظیمات.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SettingsListData {
    /// جفت‌های کلید/مقدار
    pub items: Vec<SettingEntry>,
}

/// یک تنظیم.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SettingEntry {
    /// کلید
    pub key: String,
    /// مقدار
    pub value: String,
}

/// `settingsPutRequestSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPutRequest {
    /// کلید (۱ تا ۱۲۸)
    #[schema(min_length = 1, max_length = 128)]
    pub key: String,
    /// مقدار (≤۲۰۰۰)
    #[schema(max_length = 2000)]
    pub value: String,
}

/// `securityResponseSchema` — لایه‌های دفاع زنده + دنبالهٔ audit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecurityResponse {
    /// همیشه true
    pub ok: bool,
    /// بدنه
    pub data: SecurityData,
}

/// بدنهٔ امنیت.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecurityData {
    /// لایه‌های دفاعی
    pub layers: Vec<SecurityLayer>,
    /// ردیف‌های audit
    pub audit: Vec<AuditRow>,
}

/// یک لایهٔ دفاعی.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecurityLayer {
    /// شناسه
    pub id: String,
    /// نام
    pub name: String,
    /// توضیح (اختیاری)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// فعال؟
    pub active: bool,
}

/// یک ردیف audit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditRow {
    /// شناسه
    pub id: String,
    /// عامل
    pub actor: String,
    /// اقدام
    pub action: String,
    /// هدف (nullable)
    #[schema(required = true)]
    pub target: Option<String>,
    /// وضعیت
    pub status: String,
    /// زمان
    pub created_at: IsoDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};
    use serde_json::json;

    #[test]
    fn envelope_ok_literal_roundtrips() {
        let json = r#"{"ok":true,"data":{"name":"DropAgentX","tagline":"Empire"}}"#;
        let v: BrandResponse = from_str(json).unwrap();
        assert!(v.ok);
        assert_eq!(to_string(&v).unwrap(), json);
    }

    #[test]
    fn dash_list_accepts_free_records() {
        let json = r#"{"ok":true,"data":{"items":[{"anything":1,"nested":{"x":"y"}}]}}"#;
        let v: DashListResponse = from_str(json).unwrap();
        assert_eq!(v.data.items.len(), 1);
        assert_eq!(v.data.items[0].get("anything"), Some(&json!(1)));
    }

    #[test]
    fn product_view_camel_case_and_status() {
        let json = r#"{"id":"prod:1","sellerId":"treasury","title":"VIP","description":"d","category":"c","priceStars":0,"priceCredits":1500,"stock":-1,"salesCount":7,"featured":true,"status":"active","createdAt":"2026-02-01T00:00:00.000Z"}"#;
        let v: ProductView = from_str(json).unwrap();
        assert_eq!(v.status, ProductStatus::Active);
        assert_eq!(v.stock, -1);
        assert_eq!(to_string(&v).unwrap(), json);
    }

    #[test]
    fn approval_view_lowercase_risk_and_status() {
        let json = r#"{"id":"hitl:01","type":"ESCROW_RELEASE","riskLevel":"high","summary":"Release 60000 Đ","note":null,"amount":60000,"status":"pending","createdAt":"2026-02-01T00:00:00.000Z","expiresAt":"2026-02-02T00:00:00.000Z"}"#;
        let v: ApprovalView = from_str(json).unwrap();
        assert_eq!(v.risk_level, RiskLevel::High);
        assert_eq!(to_string(&v).unwrap(), json);
    }

    #[test]
    fn create_product_request_omits_every_optional_when_unset() {
        let req = CreateProductRequest { title: "VIP".into(), ..Default::default() };
        assert_eq!(to_string(&req).unwrap(), r#"{"title":"VIP"}"#);
    }
}
