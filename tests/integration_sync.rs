//! Mock-server integration tests for the blocking client (httpmock).

#![cfg(feature = "sync")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal
)]

use blooio::{BlockingClient, ClientConfig};
use httpmock::prelude::*;

fn client(server: &MockServer) -> BlockingClient {
    BlockingClient::from_config(ClientConfig::new("test-key").with_base_url(server.base_url()))
        .unwrap()
}

#[test]
fn get_sends_bearer_auth() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/me")
            .header("Authorization", "Bearer test-key");
        then.status(200)
            .json_body(serde_json::json!({ "valid": true, "user_id": "u1" }));
    });

    let me = client(&server).account().get().unwrap();
    m.assert();
    assert_eq!(me.user_id.as_deref(), Some("u1"));
}

#[test]
fn post_sends_json_body_and_content_type() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/contacts")
            .header("content-type", "application/json")
            .json_body(serde_json::json!({ "identifier": "+15551234567" }));
        then.status(201)
            .json_body(serde_json::json!({ "id": "c2", "identifier": "+15551234567" }));
    });

    let c = client(&server)
        .contacts()
        .create("+15551234567", None)
        .unwrap();
    m.assert();
    assert_eq!(c.id.as_deref(), Some("c2"));
}

#[test]
fn send_message_includes_idempotency_key() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/chats/chat1/messages")
            .header_exists("idempotency-key")
            .json_body(serde_json::json!({ "text": "hi" }));
        then.status(200)
            .json_body(serde_json::json!({ "message_id": "m1", "status": "sent" }));
    });

    let resp = client(&server).chat("chat1").send_text("hi").unwrap();
    m.assert();
    assert_eq!(resp.ids(), vec!["m1".to_string()]);
}

#[test]
fn delete_returns_deletion() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(DELETE).path("/contacts/c9");
        then.status(200)
            .json_body(serde_json::json!({ "success": true, "deleted_at": 1700000000_i64 }));
    });

    let resp = client(&server).contacts().delete("c9").unwrap();
    m.assert();
    assert_eq!(resp.success, Some(true));
}

#[test]
fn error_response_maps_to_api_error() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/chats/chat1/messages");
        then.status(429).json_body(serde_json::json!({
            "error": "rate_limited",
            "message": "slow down",
            "status": 429,
            "code": "outbound_limit_reached"
        }));
    });

    let err = client(&server).chat("chat1").send_text("hi").unwrap_err();
    assert_eq!(err.status(), Some(429));
    assert_eq!(err.code(), Some("outbound_limit_reached"));
}
