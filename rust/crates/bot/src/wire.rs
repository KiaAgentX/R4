//! DTOهای وایر تلگرام — کمینه‌ای که orchestrator می‌بیند.
//!
//! این ماژول «بدون teloxide» است (تصمیم ADR-009 §۸٫۵): قلب بات فقط این
//! ساختارها را می‌شناسد تا آفلاین و قطعی تست شود؛ هر رانر (webhook آکسوم یا
//! polling توسعه) مسئول تبدیل update واقعی تلگرام به [`WireUpdate`] و اجرای
//! [`BotAction`]هاست — همان الگوی «update → orchestrator → actions → send».
//!
//! کلیدهای ناگهانی فیلدهای غایب تلگرام با `Option` صریح مدل شده‌اند (بدون
//! `default` پنهان) تا fail-closed بماند: پیام بدون فرستنده هرگز فرمان اجرا
//! نمی‌کند.

use serde::{Deserialize, Serialize};

/// یک update تلگرام — فقط فیلدهای مصرفی بات.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireUpdate {
    /// شناسه یکتای update (`update_id` تلگرام)
    pub update_id: i64,
    /// پیام — برای انواع دیگر (callback_query و…) فعلاً `None`
    pub message: Option<WireMessage>,
}

/// پیام ورودی بات — از update تلگرام.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireMessage {
    /// شناسه پیام در چت — مبنای کلید یکسان‌سازی (retry وب‌هوک امن)
    pub message_id: i64,
    /// فرستنده — برای پست‌های کانال غایب است؛ orchestrator بدون او اجرا نمی‌کند
    pub from: Option<WireUser>,
    /// چت مقصد
    pub chat: WireChat,
    /// متن پیام — برای عکس/استیکر و… `None`
    pub text: Option<String>,
}

/// فرستندهٔ پیام.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireUser {
    /// شناسهٔ عددی تلگرام — مبنای حساب `user:tg:<id>` (فاز ۲)
    pub id: i64,
    /// نام نمایشی
    pub first_name: Option<String>,
    /// یوزرنیم
    pub username: Option<String>,
    /// کد زبان کلاینت (`fa`، `en`) — hint لوکِیل
    pub language_code: Option<String>,
}

/// چت مقصد.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireChat {
    /// شناسهٔ چت
    pub id: i64,
    /// نوع چت (`private`، `group`، …) — فیلد `type` تلگرام
    #[serde(rename = "type")]
    pub chat_type: String,
}

impl WireMessage {
    /// متنtrimmed پیام — `None` برای پیام بی‌متن.
    #[must_use]
    pub fn command_text(&self) -> Option<&str> {
        self.text.as_deref().map(str::trim).filter(|t| !t.is_empty())
    }
}

/// اقدام خروجی orchestrator — رانر مسئول اجرای واقعی ارسال است.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotAction {
    /// ارسال پیام متنی با کیبورد اختیاری.
    Send {
        /// چت مقصد
        chat_id: i64,
        /// متن پیام (لوکِیل‌شده)
        text: String,
        /// کیبورد شیشه‌ای اختیاری
        keyboard: Option<ReplyKeyboard>,
    },
}

/// کیبورد شیشه‌ای — ردیف‌هایی از برچسب دکمه؛ فشردن دکمه متن همان برچسب را
/// می‌فرستد و orchestrator آن را مثل فرمان resolve می‌کند (همان الگوی فعلی).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplyKeyboard {
    /// ردیف‌های دکمه (برچسب‌ها)
    pub keyboard: Vec<Vec<String>>,
    /// کیبورد جمع‌شوندهٔ بومی تلگرام
    pub resize_keyboard: bool,
}

impl ReplyKeyboard {
    /// کیبورد از ردیف‌های برچسب.
    #[must_use]
    pub fn new(rows: &[&[&str]]) -> Self {
        Self {
            keyboard: rows
                .iter()
                .map(|row| row.iter().map(|b| (*b).to_owned()).collect::<Vec<_>>())
                .collect(),
            resize_keyboard: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_parses_minimal_telegram_payload() {
        let raw = serde_json::json!({
            "update_id": 42,
            "message": {
                "message_id": 7,
                "from": {"id": 1001, "first_name": "Kia", "language_code": "fa"},
                "chat": {"id": 1001, "type": "private"},
                "text": "/balance"
            }
        });
        let update: WireUpdate = serde_json::from_value(raw).expect("valid telegram update");
        let msg = update.message.expect("message present");
        assert_eq!(msg.message_id, 7);
        assert_eq!(msg.from.as_ref().expect("from").id, 1001);
        assert_eq!(msg.chat.chat_type, "private");
        assert_eq!(msg.command_text(), Some("/balance"));
    }

    #[test]
    fn missing_from_and_text_stay_none_fail_closed() {
        let raw = serde_json::json!({
            "update_id": 1,
            "message": {"message_id": 2, "chat": {"id": -100, "type": "channel"}}
        });
        let msg: WireMessage =
            serde_json::from_value(raw["message"].clone()).expect("channel post");
        assert!(msg.from.is_none(), "بدون فرستنده — fail-closed");
        assert!(msg.text.is_none());
        assert!(msg.command_text().is_none());
    }

    #[test]
    fn reply_keyboard_serializes_telegram_shape() {
        let kb = ReplyKeyboard::new(&[&["💰 موجودی", "📜 تاریخچه"], &["🌐 زبان"]]);
        let json = serde_json::to_value(&kb).expect("keyboard serializes");
        assert_eq!(json["resize_keyboard"], serde_json::json!(true));
        assert_eq!(json["keyboard"][0][0], serde_json::json!("💰 موجودی"));
    }
}
