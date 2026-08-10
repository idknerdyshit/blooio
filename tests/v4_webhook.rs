//! V4 webhook envelope and shared-signature coverage.

#![cfg(all(feature = "api-v4", feature = "webhooks"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

const SECRET: &[u8] = b"whsec_test";

fn sign(timestamp: i64, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(SECRET).unwrap();
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    format!(
        "t={timestamp},v1={}",
        hex::encode(mac.finalize().into_bytes())
    )
}

#[test]
fn parses_v4_envelope_and_peeks_routing_fields() {
    let body = br#"{"id":"evt_1","type":"message.delivered","message_id":"msg_1","chat_id":"chat_1","occurred_at":1700000000,"data":{"status":"delivered"}}"#;
    let event = blooio::v4::webhook::WebhookEvent::parse(body).unwrap();
    assert_eq!(event.event_type, "message.delivered");
    assert_eq!(
        event.data.get("status"),
        Some(&serde_json::json!("delivered"))
    );
    let peek = blooio::v4::webhook::peek(body).unwrap();
    assert_eq!(peek.id.as_deref(), Some("evt_1"));
}

#[test]
fn parses_messaging_safety_events_forward_compatibly() {
    let state_changed = br#"{"id":"evt_safety_1","type":"safety.state_changed","occurred_at":1700000000,"data":{"tier":"warm","previous_tier":"new","action":"slow","previous_action":"queue","reasons":["volume"]}}"#;
    let event = blooio::v4::webhook::WebhookEvent::parse(state_changed).unwrap();
    assert_eq!(event.event_type, "safety.state_changed");
    assert_eq!(event.data.get("action"), Some(&serde_json::json!("slow")));
    assert_eq!(
        event.data.get("previous_action"),
        Some(&serde_json::json!("queue"))
    );

    let number_banned = br#"{"id":"evt_safety_2","type":"safety.number_banned","occurred_at":1700000001,"data":{"channel_id":"ch_1","phone_number":"+15551234567","allocation_type":"dedicated","banned_at":1700000001}}"#;
    let event = blooio::v4::webhook::WebhookEvent::parse(number_banned).unwrap();
    assert_eq!(event.event_type, "safety.number_banned");
    assert_eq!(
        event.data.get("allocation_type"),
        Some(&serde_json::json!("dedicated"))
    );
}

#[test]
fn parses_number_lifecycle_events_forward_compatibly() {
    let completed = br#"{"id":"evt_purchase_1","type":"number.purchase.completed","occurred_at":1700000002,"data":{"purchase_id":"purchase_1","status":"completed","phone_numbers":["+15551234567"],"channel_ids":["ch_1"]}}"#;
    let event = blooio::v4::webhook::WebhookEvent::parse(completed).unwrap();
    assert_eq!(event.event_type, "number.purchase.completed");
    assert_eq!(
        event.data.get("purchase_id"),
        Some(&serde_json::json!("purchase_1"))
    );

    let removed = br#"{"id":"evt_removed_1","type":"number.removed","occurred_at":1700000003,"data":{"phone_number":"+15551234567","channel_id":"ch_1","binding_id":"binding_1","reasons":["no_longer_needed"]}}"#;
    let event = blooio::v4::webhook::WebhookEvent::parse(removed).unwrap();
    assert_eq!(event.event_type, "number.removed");
    assert_eq!(
        event.data.get("reasons"),
        Some(&serde_json::json!(["no_longer_needed"]))
    );
}

#[test]
fn shared_signature_verification_accepts_authentic_body() {
    let body = br#"{"id":"evt_1","type":"poll.voted","occurred_at":1700000000,"data":{}}"#;
    let header = sign(1_700_000_000, body);
    blooio::v4::webhook::verify_at(SECRET, &header, body, 300, 1_700_000_000).unwrap();
    assert!(blooio::v4::webhook::verify_at(SECRET, &header, b"{}", 300, 1_700_000_000).is_err());
}
