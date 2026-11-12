//! آداپتور webhook بات داخل آکسوم — مسیر additive فاز ۵.
//!
//! `POST /api/v1/bot/webhook` — خارج رجیستری ۴۶تایی TS (مثل تصمیم `/ai/chat`
//! فاز ۴): تلگرام بدنهٔ update می‌فرستد و پاسخ HTTP همان فرمان sendMessage است
//! (`{"method":"sendMessage",...}` — الگوی std/http فعلی؛ تلگرام خودش ارسال
//! می‌کند و رانر جدا لازم نیست).
//!
//! امنیت fail-closed دو لایه:
//! * بدون `BOT_WEBHOOK_SECRET` در کانفیگ → 503 `BOT_DISABLED` (مسیر اصلاً باز نیست)
//! * هدر `X-Telegram-Bot-Api-Secret-Token` با مقایسهٔ زمان‌ثابت → 401
//!   `BOT_UNAUTHORIZED` (همان helper کلید ادمین فاز ۳)
//!
//! retry امن: هر update فقط با message_id وارد orchestrator می‌شود و کلید
//! یکسان‌سازی `tg:{chat}:{msg}` از همان‌جا می‌آید — تکرار وب‌هوک دوبار پول
//! جابه‌جا نمی‌کند (معیار پذیرش فاز ۵).

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde_json::json;
use std::sync::Arc;

use dropagentx_bot::{handle_update, BotAction, WireUpdate};

use crate::bot_env::StoreEnv;
use crate::error::ApiResult;
use crate::state::AppState;

const SECRET_HEADER: &str = "x-telegram-bot-api-secret-token";

/// `POST /api/v1/bot/webhook` — آداپتور تلگرام (فاز ۵، additive).
///
/// # Errors
/// 503 `BOT_DISABLED` بدون کانفیگ؛ 401 `BOT_UNAUTHORIZED` با توکن غلط؛
/// 400 `INVALID_REQUEST` با بدنهٔ غیر-JSON.
pub async fn webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    // ۱) گیت کانفیگ — بدون توکن مخفی، مسیر اصلاً باز نیست
    let Some(secret) = state.config.bot_webhook_secret.as_deref() else {
        return Err(dropagentx_contracts::HttpFailure::bot_disabled().into());
    };
    // ۲) گیت تلگرام — مقایسهٔ زمان‌ثابت (helper فاز ۳: ≥۱۶ کاراکتر، fail-closed)
    let provided = headers.get(SECRET_HEADER).and_then(|v| v.to_str().ok());
    if !dropagentx_contracts::verify_kia_key(provided, Some(secret)) {
        return Err(dropagentx_contracts::HttpFailure::bot_unauthorized().into());
    }

    // ۳) تجزیهٔ update — مثل zod در لبهٔ HTTP
    let update: WireUpdate = crate::handlers::parse_body(&body)?;

    // ۴) فقط پیام متنیِ دارای فرستنده وارد orchestrator می‌شود؛ بقیه ack می‌شوند
    let Some(msg) = update.message.filter(|m| m.from.is_some() && m.text.is_some()) else {
        return Ok(Json(json!({})));
    };

    // ۵) قلب خالص — همان orchestrator تست‌های آفلاین
    let mut env = StoreEnv::from_state(&state);
    let actions = handle_update(&mut env, &msg).await;

    // ۶) پاسخ = فرمان sendMessage (تلگرام خودش اجرا می‌کند)؛ اگر چند action
    //    آمد فقط اولی در پاسخ می‌رود و بقیه در لاگ می‌مانند (برای فاز بعد).
    match actions.first() {
        Some(BotAction::Send { chat_id, text, keyboard }) => {
            if actions.len() > 1 {
                tracing::warn!(extra = actions.len() - 1, "multiple bot actions; first sent");
            }
            let mut payload = json!({ "method": "sendMessage", "chat_id": chat_id, "text": text });
            if let Some(kb) = keyboard {
                payload["reply_markup"] =
                    json!({ "keyboard": kb.keyboard, "resize_keyboard": kb.resize_keyboard });
            }
            Ok(Json(payload))
        }
        None => Ok(Json(json!({}))),
    }
}
