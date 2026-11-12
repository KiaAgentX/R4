//! پاکت JSON به‌جای function-calling — قرارداد سخت وایر LLM (ADR-009 فاز ۶).
//!
//! LLM فقط یکی از دو پاکت را می‌تواند بدهد:
//! * `{"reply": "…"}` — پاسخ نهایی متنی
//! * `{"tool": "wallet.balance", "args": {…}} — درخواست اجرای مهارت
//!
//! متن خام هرگز عبور نمی‌کند: هر چیز غیر-JSON یا بی‌شکل → [`PacketError`]؛
//! دو پاکت خراب پیاپی در [`crate::engine`] شکست تایپ‌شده می‌دهد
//! (`AI_MALFORMED_PACKET` 502).

use serde::Deserialize;

use crate::error::PacketError;

/// پاکت معتبر LLM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    /// پاسخ نهایی
    Reply(String),
    /// درخواست ابزار
    Tool {
        /// شناسهٔ مهارت (مثل `wallet.balance`)
        skill: String,
        /// آرگومان‌ها (شیء؛ غایب = `{}`)
        args: serde_json::Value,
    },
}

/// شکل خام — untagged تا اولین واریانت منطبق برداشت شود (extra فیلدها بی‌اثر).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawPacket {
    Reply {
        /// پاسخ
        reply: String,
    },
    Tool {
        /// مهارت
        tool: String,
        /// آرگومان‌ها
        #[serde(default)]
        args: Option<serde_json::Value>,
    },
}

/// تجزیهٔ خروجی LLM به پاکت — سخت‌گیرانه (fail-closed).
///
/// # Errors
/// [`PacketError::RawText`] اگر JSON نباشد؛ [`PacketError::BadShape`] اگر
/// JSON باشد ولی `reply`/`tool` معتبر نداشته باشد.
pub fn parse_packet(text: &str) -> Result<Packet, PacketError> {
    let trimmed = text.trim();
    let Ok(raw) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return Err(PacketError::RawText);
    };
    if raw.as_object().is_none() {
        return Err(PacketError::RawText);
    }
    match serde_json::from_value::<RawPacket>(raw) {
        Ok(RawPacket::Reply { reply }) => {
            let reply = reply.trim().to_owned();
            if reply.is_empty() {
                return Err(PacketError::BadShape("reply must be non-empty".into()));
            }
            Ok(Packet::Reply(reply))
        }
        Ok(RawPacket::Tool { tool, args }) => {
            let skill = tool.trim().to_owned();
            if skill.is_empty() {
                return Err(PacketError::BadShape("tool must be non-empty".into()));
            }
            let args = args.unwrap_or_else(|| serde_json::json!({}));
            if !args.is_object() {
                return Err(PacketError::BadShape("args must be an object".into()));
            }
            Ok(Packet::Tool { skill, args })
        }
        Err(_) => Err(PacketError::BadShape(
            "expected {\"reply\": …} or {\"tool\": …, \"args\": …}".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PacketError::{BadShape, RawText};

    #[test]
    fn reply_packet_parses() {
        let p = parse_packet(r#"{"reply":"سلام! چطور می‌توانم کمک کنم؟"}"#).expect("reply");
        assert_eq!(p, Packet::Reply("سلام! چطور می‌توانم کمک کنم؟".into()));
    }

    #[test]
    fn tool_packet_parses_with_and_without_args() {
        let p = parse_packet(r#"{"tool":"wallet.balance","args":{"x":1}}"#).expect("tool+args");
        assert_eq!(
            p,
            Packet::Tool { skill: "wallet.balance".into(), args: serde_json::json!({"x":1}) }
        );
        let p2 = parse_packet(r#"{"tool":"economy.summary"}"#).expect("tool only");
        assert_eq!(
            p2,
            Packet::Tool { skill: "economy.summary".into(), args: serde_json::json!({}) }
        );
    }

    #[test]
    fn raw_text_never_passes() {
        assert_eq!(parse_packet("سلام، این پاسخ معمولی است."), Err(RawText));
        assert_eq!(parse_packet(""), Err(RawText));
        assert_eq!(parse_packet("[1,2,3]"), Err(RawText));
        assert_eq!(parse_packet("\"just a string\""), Err(RawText));
    }

    #[test]
    fn bad_shapes_are_typed() {
        assert!(matches!(parse_packet(r#"{"reply":123}"#), Err(BadShape(_))));
        assert!(matches!(parse_packet(r#"{"reply":""}"#), Err(BadShape(_))));
        assert!(matches!(parse_packet(r#"{"tool":""}"#), Err(BadShape(_))));
        assert!(matches!(parse_packet(r#"{"tool":"x","args":[1]}"#), Err(BadShape(_))));
        assert!(matches!(parse_packet(r#"{"foo":"bar"}"#), Err(BadShape(_))));
    }

    #[test]
    fn extra_fields_are_tolerated_like_ts() {
        let p = parse_packet(r#"{"reply":"ok","thinking":"…"}"#).expect("extra ok");
        assert_eq!(p, Packet::Reply("ok".into()));
    }
}
