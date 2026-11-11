//! قرارداد backend مهارت‌ها — پل gateway به دامنه/فروشگاه (فاز ۶).
//!
//! گیت مجوز دولایه است: موتور در [`crate::engine`] مجوز رجیستری را چک می‌کند
//! (allowed_agents + staff_only) و backend واقعی در crate سرور گارد دوم را
//! داخل اجرای هر مهارت اعمال می‌کند (actor فقط خودش — هیچ کاربری کیف دیگری
//! را هدف نمی‌گیرد؛ staff با کلید ادمین).
//!
//! مصرف توکن هم از همین قرارداد می‌گذرد: گیت سقف روزانه قبل از فراخوانی و
//! ثبت ردیف `AiUsage` بعد از آن — بدون SQL داخل crate خالص.

use std::future::Future;

use serde::Deserialize;

use crate::error::GateError;

/// زمینهٔ تماس‌گیرنده — هویت قبلاً در هندلر تأیید شده (initData/کلید ادمین).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCtx {
    /// حساب عامل (`user:tg:<id>` یا حساب داشبورد)
    pub actor: String,
    /// آیا عامل staff معتبر است؟ (کلید ادمین + role: staff)
    pub staff: bool,
}

/// خروجی اجرای مهارت.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolOutcome {
    /// دادهٔ واقعی + خلاصهٔ انسانی برای reply
    Data {
        /// دادهٔ ساختاریافته (برای کلاینت‌ها)
        value: serde_json::Value,
        /// خلاصهٔ یک‌خطی
        summary: String,
    },
    /// درخواست صف HITL شد — هیچ پولی جابه‌جا نشده
    Pending {
        /// خلاصهٔ یک‌خطی
        summary: String,
    },
    /// شکست تایپ‌شدهٔ مهارت (کد دامنه/اعتبارسنجی)
    Failed {
        /// کد وایر (مثل `INVALID_TRANSITION`)
        code: String,
        /// پیام صادقانه
        message: String,
    },
}

/// ردیف مصرف توکن — برای جدول `AiUsage`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiUsageRecord {
    /// کاربر
    pub user_id: String,
    /// ایجنت
    pub agent: String,
    /// ارائه‌دهنده
    pub provider: String,
    /// مدل
    pub model: String,
    /// نوع سیکل (`chat`)
    pub kind: String,
    /// توکن ورودی
    pub prompt_tokens: i64,
    /// توکن خروجی
    pub completion_tokens: i64,
    /// مجموع
    pub total_tokens: i64,
    /// نتیجهٔ ممیزی (`ai.ok`/`ai.refused`/`ai.failed`/`ai.pending`)
    pub outcome: String,
}

/// backend مهارت‌ها + metering — پیاده‌سازی واقعی در crate سرور.
pub trait ToolBackend: Send + Sync {
    /// اجرای یک مهارت — گارد دوم authz داخل خودش.
    fn execute(
        &self,
        ctx: &ToolCtx,
        skill: &str,
        args: &serde_json::Value,
    ) -> impl Future<Output = ToolOutcome> + Send;

    /// مصرف توکن امروز کاربر — گیت سقف قبل از هر فراخوانی LLM.
    fn tokens_today(&self, user_id: &str) -> impl Future<Output = Result<i64, GateError>> + Send;

    /// ثبت ردیف مصرف — بعد از هر فراخوانی (حتی refused/failed).
    fn record_usage(
        &self,
        record: AiUsageRecord,
    ) -> impl Future<Output = Result<(), GateError>> + Send;
}
