//! شکل سیمی رویدادها — JSON بایت‌به‌بایت همان چیزی که `JSON.stringify` در
//! `back/domain/src/events.ts` می‌سازد، تا dual-run و مصرف‌کنندهٔ outbox
//! (بات فاز ۵) هر دو طرف TS و Rust یک داده ببینند.
//!
//! قواعد کپی از TS:
//!  - فیلدهای اختیاریِ `None` کلاً حذف می‌شوند (مثل `undefined` در
//!    `JSON.stringify`) — `null` نوشته نمی‌شود.
//!  - نام‌ها camelCase: `escrowId`، `sellerNet`، `idempotencyKey` و…
//!  - `kind` برای `TransactionPosted` ثابت است و برای اسکرو همان رشتهٔ
//!    کانانیکی `EscrowEventKind` (مثل `EscrowReleased`).
//!
//! تست‌های طلایی همین ماژول جلوی انحراف شکل را می‌گیرند؛ در فاز ۳ تولید
//! قرارداد (serde + utoipa) از همین شکل به‌عنوان منبع حقیقت استفاده می‌کند.

use dropagentx_domain::{DomainEvent, EscrowEvent, TransactionPostedEvent};

/// رشتهٔ `kind` رویداد — یکسان با TS (ستون `OutboxEvent.kind`).
#[must_use]
pub fn event_kind(event: &DomainEvent) -> &'static str {
    match event {
        DomainEvent::TransactionPosted(_) => "TransactionPosted",
        DomainEvent::Escrow(ev) => ev.kind.as_str(),
    }
}

/// JSON payload رویداد — شکل `JSON.stringify(event)` در TS.
#[must_use]
pub fn event_payload(event: &DomainEvent) -> String {
    let mut m = serde_json::Map::new();
    match event {
        DomainEvent::TransactionPosted(ev) => fill_transaction_posted(&mut m, ev),
        DomainEvent::Escrow(ev) => fill_escrow(&mut m, ev),
    }
    serde_json::Value::Object(m).to_string()
}

fn put(map: &mut serde_json::Map<String, serde_json::Value>, key: &str, v: serde_json::Value) {
    map.insert(key.to_owned(), v);
}

fn put_opt(map: &mut serde_json::Map<String, serde_json::Value>, key: &str, v: &Option<String>) {
    if let Some(s) = v {
        put(map, key, serde_json::Value::String(s.clone()));
    }
}

fn put_opt_num(map: &mut serde_json::Map<String, serde_json::Value>, key: &str, v: Option<i64>) {
    if let Some(n) = v {
        put(map, key, serde_json::Value::from(n));
    }
}

fn fill_transaction_posted(
    map: &mut serde_json::Map<String, serde_json::Value>,
    ev: &TransactionPostedEvent,
) {
    put(map, "kind", serde_json::Value::String("TransactionPosted".into()));
    put(map, "id", serde_json::Value::String(ev.id.clone()));
    put(map, "at", serde_json::Value::String(ev.at.clone()));
    put_opt(map, "correlationId", &ev.correlation_id);
    put(map, "txId", serde_json::Value::String(ev.tx_id.clone()));
    put_opt(map, "memo", &ev.memo);
    put_opt(map, "idempotencyKey", &ev.idempotency_key);
}

fn fill_escrow(map: &mut serde_json::Map<String, serde_json::Value>, ev: &EscrowEvent) {
    put(map, "kind", serde_json::Value::String(ev.kind.as_str().into()));
    put(map, "id", serde_json::Value::String(ev.id.clone()));
    put(map, "at", serde_json::Value::String(ev.at.clone()));
    put_opt(map, "correlationId", &ev.correlation_id);
    put(map, "escrowId", serde_json::Value::String(ev.escrow_id.clone()));
    put_opt(map, "buyerId", &ev.buyer_id);
    put_opt(map, "sellerId", &ev.seller_id);
    put_opt_num(map, "amount", ev.amount);
    put_opt(map, "txId", &ev.tx_id);
    put_opt_num(map, "sellerNet", ev.seller_net);
    put_opt_num(map, "tax", ev.tax);
    put_opt(map, "reason", &ev.reason);
}

#[cfg(test)]
mod tests {
    use super::*;
    use dropagentx_domain::{EscrowEventKind, TransactionPostedEvent, EPOCH_ISO};
    use serde_json::json;

    #[test]
    fn transaction_posted_shape_matches_ts() {
        let mut ev = TransactionPostedEvent::new("tx_1", EPOCH_ISO);
        ev.memo = Some("genesis".into());
        ev.idempotency_key = Some("legacy:genesis:v1".into());
        let payload = event_payload(&DomainEvent::TransactionPosted(ev));
        let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(
            v,
            json!({
                "kind": "TransactionPosted",
                "id": "tx_1#posted",
                "at": EPOCH_ISO,
                "txId": "tx_1",
                "memo": "genesis",
                "idempotencyKey": "legacy:genesis:v1"
            })
        );
        assert_eq!(
            event_kind(&DomainEvent::TransactionPosted(TransactionPostedEvent::new(
                "tx_1", EPOCH_ISO
            ))),
            "TransactionPosted"
        );
    }

    #[test]
    fn optional_fields_are_omitted_like_json_stringify() {
        let ev = EscrowEvent::new(EscrowEventKind::Delivered, "e1", EPOCH_ISO);
        let payload = event_payload(&DomainEvent::Escrow(ev));
        let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(
            v,
            json!({
                "kind": "EscrowDelivered",
                "id": "e1#EscrowDelivered",
                "at": EPOCH_ISO,
                "escrowId": "e1"
            })
        );
        // هیچ فیلد null یا فیلد اضافه‌ای نباید باشد
        assert!(v.get("amount").is_none());
        assert!(v.get("reason").is_none());
        assert_eq!(
            event_kind(&DomainEvent::Escrow(EscrowEvent::new(
                EscrowEventKind::Delivered,
                "e1",
                EPOCH_ISO
            ))),
            "EscrowDelivered"
        );
    }

    #[test]
    fn released_event_carries_seller_net_and_tax() {
        let mut ev = EscrowEvent::new(EscrowEventKind::Released, "e9", EPOCH_ISO);
        ev.seller_net = Some(360);
        ev.tax = Some(40);
        ev.tx_id = Some("tx_3".into());
        let v: serde_json::Value =
            serde_json::from_str(&event_payload(&DomainEvent::Escrow(ev))).unwrap();
        assert_eq!(v["sellerNet"], json!(360));
        assert_eq!(v["tax"], json!(40));
        assert_eq!(v["txId"], json!("tx_3"));
        assert_eq!(v["kind"], json!("EscrowReleased"));
    }
}
