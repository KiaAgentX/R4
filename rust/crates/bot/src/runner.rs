//! رانر polling توسعه — teloxide فقط «رانر» است (ADR-009 §۸٫۵).
//!
//! ```text
//! update تلگرام → Message → WireMessage → orchestrator خالص → BotAction → teloxide send
//! ```
//!
//! این ماژول هیچ منطق کسب‌وکاری ندارد — authz، idempotency، صف HITL و i18n
//! همه در [`crate::orchestrator`] هستند؛ اینجا فقط تبدیل تایپ‌های teloxide به
//! DTOهای وایر و اجرای ارسال است. در تولید، webhook روی `/api/v1/bot/webhook`
//! (crate سرور) همین کار را بدون polling انجام می‌دهد.

use teloxide::prelude::*;
use teloxide::types::{ChatKind, KeyboardButton, KeyboardMarkup};

use crate::env::Env;
use crate::error::RunnerError;
use crate::orchestrator::handle_update;
use crate::wire::{BotAction, WireChat, WireMessage, WireUser};

/// تبدیل پیام teloxide به DTO وایر — فیلدهای غایب `None` می‌مانند (fail-closed).
#[must_use]
pub fn to_wire_message(msg: &Message) -> Option<WireMessage> {
    let text = msg.text().map(str::to_owned);
    text.as_ref()?;
    let from = msg.from.as_ref().map(|u| WireUser {
        id: i64::try_from(u.id.0).unwrap_or(-1),
        first_name: Some(u.first_name.clone()),
        username: u.username.clone(),
        language_code: u.language_code.as_ref().map(|l| l.as_str().to_owned()),
    });
    Some(WireMessage {
        message_id: i64::from(msg.id.0),
        from,
        chat: WireChat {
            id: msg.chat.id.0,
            chat_type: match msg.chat.kind {
                ChatKind::Public(_) => "public".into(),
                ChatKind::Private(_) => "private".into(),
            },
        },
        text,
    })
}

/// کیبورد شیشه‌ای → کیبورد teloxide.
fn to_telegram_keyboard(kb: &crate::wire::ReplyKeyboard) -> KeyboardMarkup {
    let rows = kb
        .keyboard
        .iter()
        .map(|row| row.iter().map(KeyboardButton::new).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    KeyboardMarkup::new(rows).resize_keyboard()
}

/// اجرای یک action خروجی با teloxide.
async fn send_action(bot: &Bot, action: &BotAction) -> Result<(), RunnerError> {
    match action {
        BotAction::Send { chat_id, text, keyboard } => {
            let mut req = bot.send_message(ChatId(*chat_id), text.clone());
            if let Some(kb) = keyboard {
                req = req.reply_markup(to_telegram_keyboard(kb));
            }
            req.await.map_err(|e| RunnerError::Telegram(e.to_string()))?;
            Ok(())
        }
    }
}

/// حلقهٔ polling توسعه — هر update → orchestrator → ارسال پاسخ‌ها.
///
/// `env` با `tokio::sync::Mutex` محافظت می‌شود چون متدهای [`Env`] `&mut self`
/// می‌گیرند و updateها ممکن است هم‌زمان برسند (سفارش تلگرام حفظ می‌شود چون
/// repl ترتیبی اجرا می‌کند، اما قفل برای فراخوان‌های بیرونی هم امن است).
///
/// # Errors
/// [`RunnerError::Telegram`] — خطای API تلگرام در ارسال.
pub async fn run_polling<E>(
    token: &str,
    env: std::sync::Arc<tokio::sync::Mutex<E>>,
) -> Result<(), RunnerError>
where
    E: Env + Send + Sync + 'static,
{
    let bot = Bot::new(token);
    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let env = env.clone();
        async move {
            let Some(wire) = to_wire_message(&msg) else {
                return Ok(());
            };
            let actions = {
                let mut guard = env.lock().await;
                handle_update(&mut *guard, &wire).await
            };
            for action in &actions {
                if let Err(e) = send_action(&bot, action).await {
                    tracing::error!(error = %e, chat_id = msg.chat.id.0, "bot send failed");
                }
            }
            Ok(())
        }
    })
    .await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_message_maps_to_wire() {
        let msg: Message = serde_json::from_value(serde_json::json!({
            "message_id": 7,
            "from": {"id": 1001, "is_bot": false, "first_name": "Kia", "language_code": "fa"},
            "chat": {"id": 1001, "type": "private", "first_name": "Kia"},
            "date": 1_700_000_000,
            "text": "/balance"
        }))
        .expect("valid telegram message");
        let wire = to_wire_message(&msg).expect("text message maps");
        assert_eq!(wire.message_id, 7);
        assert_eq!(wire.from.as_ref().expect("from").id, 1001);
        assert_eq!(wire.chat.id, 1001);
        assert_eq!(wire.text.as_deref(), Some("/balance"));
    }

    #[test]
    fn non_text_message_is_ignored() {
        let msg: Message = serde_json::from_value(serde_json::json!({
            "message_id": 8,
            "from": {"id": 1001, "is_bot": false, "first_name": "Kia"},
            "chat": {"id": 1001, "type": "private", "first_name": "Kia"},
            "date": 1_700_000_000,
            "sticker": {"file_id": "x", "width": 1, "height": 1}
        }))
        .expect("sticker message");
        assert!(to_wire_message(&msg).is_none(), "پیام بی‌متن وارد orchestrator نمی‌شود");
    }
}
