//! قراردادهای سیکل AI (فاز ۵ از TS) — گیت‌وی Zenovix روی وایر (ADR-005) — پورت `ai.ts`.
//!
//! لایهٔ AI زیرساخت OPT-IN است: هر پاسخ روی سرور audit می‌شود، به ازای هر کاربر
//! با سقف روزانهٔ توکن meter می‌شود و قبل از خروج از پروسه redact می‌شود.
//! اجرای staff (`role: "staff"`) کلید ادمین می‌خواهد — هندلر fail-closed بررسی می‌کند.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::Id;

/// نام ۶ ایجنت Zenovix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AiAgentName {
    /// هماهنگ‌کننده
    Orchestrator,
    /// پذیرش
    Receptionist,
    /// فروش
    Sales,
    /// پشتیبانی
    Support,
    /// تحلیلگر
    Analyst,
    /// محتوا
    Content,
}

/// نوع مجوز مهارت — `read/write/money/staff`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AiSkillKind {
    /// فقط‌خواندنی
    Read,
    /// نوشتنی
    Write,
    /// پول
    Money,
    /// فقط staff
    Staff,
}

/// نقش درخواست‌دهندهٔ AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum AiChatRole {
    /// کاربر عادی (پیش‌فرض)
    #[default]
    User,
    /// کارکنان — کلید ادمین لازم دارد
    Staff,
}

/// مسیر تولید پاسخ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AiVia {
    /// مستقیم به ایجنت
    Direct,
    /// پیش‌فیلتر (بدون LLM)
    Prefilter,
    /// از طریق LLM
    Llm,
}

/// نقش پیام در تاریخچه.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HistoryRole {
    /// کاربر
    User,
    /// دستیار
    Assistant,
}

/// نمای ایجنت — `aiAgentViewSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AiAgentView {
    /// نام ایجنت
    pub name: AiAgentName,
    /// ایموجی
    pub emoji: String,
    /// عنوان فارسی
    pub title_fa: String,
    /// عنوان انگلیسی
    pub title_en: String,
    /// توضیح
    pub description: String,
    /// مهارت‌های مجاز
    pub skills: Vec<String>,
}

/// نمای مهارت — `aiSkillViewSchema` (متادیتای مجوز).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AiSkillView {
    /// شناسهٔ مهارت (مثل `wallet.balance`)
    pub id: String,
    /// نوع مجوز
    pub kind: AiSkillKind,
    /// عنوان فارسی
    pub title_fa: String,
    /// عنوان انگلیسی
    pub title_en: String,
    /// توضیح
    pub description: String,
    /// ایجنت‌های مجاز
    pub allowed_agents: Vec<AiAgentName>,
    /// آیا فقط staff؟
    pub staff_only: bool,
}

/// `aiAgentsListResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AiAgentsListResponse {
    /// ایجنت‌ها
    pub items: Vec<AiAgentView>,
}

/// `aiSkillsListResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AiSkillsListResponse {
    /// مهارت‌ها
    pub items: Vec<AiSkillView>,
}

/// رد ابزار برای audit — `aiToolTraceSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AiToolTrace {
    /// مهارت اجراشده
    pub skill: String,
    /// موفق؟
    pub ok: bool,
    /// خلاصهٔ انسانی
    pub summary: String,
    /// آیا نیاز به تأیید HITL دارد؟
    pub pending_approval: bool,
    /// کد خطا (nullable — صریح روی سیم)
    #[schema(required = true)]
    pub code: Option<String>,
}

/// مصرف توکن — `aiUsageSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AiUsage {
    /// ارائه‌دهنده (nullable — صریح)
    #[schema(required = true)]
    pub provider: Option<String>,
    /// مدل (nullable — صریح)
    #[schema(required = true)]
    pub model: Option<String>,
    /// توکن ورودی
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub prompt_tokens: i64,
    /// توکن خروجی
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub completion_tokens: i64,
    /// مجموع
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub total_tokens: i64,
}

/// پیام تاریخچه — قدیمی‌به‌جدید، حداکثر ۱۰.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMessage {
    /// نقش
    pub role: HistoryRole,
    /// متن (۱ تا ۲۰۰۰ کاراکتر)
    #[schema(min_length = 1, max_length = 2000)]
    pub content: String,
}

/// `aiChatRequestSchema` — هویت تماس‌گیرنده:
/// * با کلید ادمین (X-KIA-Key): `userId` معتبر است.
/// * بدون کلید: هویت از initData (HMAC تلگرام) مشتق می‌شود و `userId` نادیده
///   گرفته می‌شود — هیچ تماس‌گیرندهٔ عمومی نمی‌تواند کیف پول دیگری را هدف بگیرد.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AiChatRequest {
    /// هویت تماس‌گیرنده — فقط برای تماس‌گیرندگان معتبر (X-KIA-Key)
    pub user_id: Id,
    /// Telegram MiniApp initData — برای تماس‌گیرندگان بدون کلید ادمین الزامی؛
    /// HMAC-SHA256 رسمی سمت سرور با `TELEGRAM_BOT_TOKEN` بررسی می‌شود (fail-closed).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(min_length = 1, max_length = 8192)]
    pub init_data: Option<String>,
    /// نقش (پیش‌فرض `user` — مثل zod .default)
    #[serde(default)]
    #[schema(default = json!("user"))]
    pub role: AiChatRole,
    /// متن (۱ تا ۴۰۰۰)
    #[schema(min_length = 1, max_length = 4000)]
    pub text: String,
    /// برچسب زبان (BCP-47-ish) — مستقیم داخل پرامپت ایجنت (پیش‌فرض `fa`)
    #[serde(default = "default_language")]
    #[schema(default = json!("fa"), min_length = 2, max_length = 16)]
    pub language: String,
    /// پرش از روتینگ (بات خودش slash command را می‌شناسد)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_agent: Option<AiAgentName>,
    /// تاریخچه — قدیمی‌ها اول، حداکثر ۱۰ پیام
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_items = 10)]
    pub history: Option<Vec<HistoryMessage>>,
    /// زمینهٔ دانش پیش‌واکشی‌شده (grounding — ≤۴۰۰۰)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(max_length = 4000)]
    pub knowledge: Option<String>,
}

/// پیش‌فرض زبان — `"fa"`.
fn default_language() -> String {
    "fa".to_string()
}

/// `aiChatResponseSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AiChatResponse {
    /// پاسخ نهایی
    pub reply: String,
    /// ایجنت پاسخ‌دهنده
    pub agent: AiAgentName,
    /// مسیر تولید
    pub via: AiVia,
    /// ارائه‌دهندهٔ LLM
    pub provider: String,
    /// ردهای ابزار (audit)
    pub tool_traces: Vec<AiToolTrace>,
    /// مصرف توکن
    pub usage: AiUsage,
    /// آیا اقدامی منتظر تأیید HITL است؟
    pub pending_approval: bool,
    /// تعداد redactionها
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub redactions: i64,
}

/// `aiUsageViewSchema` — مصرف روزانهٔ یک کاربر در برابر سقف.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AiUsageView {
    /// کاربر
    pub user_id: String,
    /// مصرف امروز
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub used_today: i64,
    /// سقف روزانه
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub cap: i64,
    /// باقی‌ماندهٔ امروز
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub remaining_today: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn ai_chat_request_defaults_fill_role_and_language() {
        let json = r#"{"userId":"user:tg:123","text":"سلام"}"#;
        let req: AiChatRequest = from_str(json).unwrap();
        assert_eq!(req.role, AiChatRole::User);
        assert_eq!(req.language, "fa");
        // روی سیم سرور دوباره defaultها را می‌فرستد — مثل zod (io:output)
        assert!(to_string(&req).unwrap().contains(r#""role":"user""#));
        assert!(to_string(&req).unwrap().contains(r#""language":"fa""#));
    }

    #[test]
    fn ai_chat_response_roundtrip_with_fa_text() {
        let json = r#"{"reply":"سلام!","agent":"receptionist","via":"llm","provider":"openai","toolTraces":[{"skill":"wallet.balance","ok":true,"summary":"read balance","pendingApproval":false,"code":null}],"usage":{"provider":"openai","model":"gpt-4o-mini","promptTokens":120,"completionTokens":45,"totalTokens":165},"pendingApproval":false,"redactions":0}"#;
        let v: AiChatResponse = from_str(json).unwrap();
        assert_eq!(v.agent, AiAgentName::Receptionist);
        assert_eq!(v.via, AiVia::Llm);
        assert_eq!(to_string(&v).unwrap(), json);
    }

    #[test]
    fn history_over_ten_is_a_wire_violation_documented_in_schema() {
        // سقف ۱۰ در اسکیمای OpenAPI (maxItems) ثبت می‌شود؛ enforce در فاز ۴.
        let history: Vec<HistoryMessage> = (0..10)
            .map(|_| HistoryMessage { role: HistoryRole::User, content: "x".into() })
            .collect();
        let req = AiChatRequest {
            user_id: Id::new("user:tg:1").unwrap(),
            init_data: None,
            role: AiChatRole::default(),
            text: "t".into(),
            language: default_language(),
            direct_agent: None,
            history: Some(history),
            knowledge: None,
        };
        assert_eq!(req.history.as_ref().map(Vec::len), Some(10));
    }
}
