//! Mock-server integration tests for the async client (wiremock).
//!
//! These exercise a representative operation per HTTP method plus an error
//! path, and assert the `Authorization`, `Content-Type`, and `Idempotency-Key`
//! headers are sent.

#![cfg(feature = "async")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unused_async,
    clippy::print_stdout,
    clippy::unreadable_literal
)]

use blooio::{Client, ClientConfig};
use wiremock::matchers::{body_json, header, header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn client(server: &MockServer) -> Client {
    Client::from_config(ClientConfig::new("test-key").with_base_url(server.uri())).unwrap()
}

#[tokio::test]
async fn get_sends_bearer_auth() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "valid": true,
            "user_id": "u1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let me = client(&server).await.account().get().await.unwrap();
    assert_eq!(me.user_id.as_deref(), Some("u1"));
    assert_eq!(me.valid, Some(true));
}

#[tokio::test]
async fn get_passes_query_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/contacts"))
        .and(query_param("limit", "2"))
        .and(query_param("q", "alice"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "contacts": [{ "id": "c1", "name": "Alice" }],
            "pagination": { "limit": 2, "offset": 0, "total": 1 }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server)
        .await
        .contacts()
        .list_with(blooio::resources::contacts::ListContacts {
            limit: Some(2),
            q: Some("alice".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(resp.contacts.len(), 1);
}

#[tokio::test]
async fn post_sends_json_body_and_content_type() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/contacts"))
        .and(header("content-type", "application/json"))
        .and(body_json(
            serde_json::json!({ "identifier": "+15551234567" }),
        ))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "c2",
            "identifier": "+15551234567"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let c = client(&server)
        .await
        .contacts()
        .create("+15551234567", None)
        .await
        .unwrap();
    assert_eq!(c.id.as_deref(), Some("c2"));
}

#[tokio::test]
async fn send_message_includes_idempotency_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chats/chat1/messages"))
        .and(header_exists("idempotency-key"))
        .and(body_json(serde_json::json!({ "text": "hi" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "message_id": "m1",
            "status": "sent"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server)
        .await
        .chat("chat1")
        .send_text("hi")
        .await
        .unwrap();
    assert_eq!(resp.ids(), vec!["m1".to_string()]);
}

#[tokio::test]
async fn put_replaces_resource() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/chats/chat1/background"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "chat_id": "chat1",
            "has_background": true,
            "changed": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server)
        .await
        .chat("chat1")
        .set_background("data:image/png;base64,AAAA")
        .await
        .unwrap();
    assert_eq!(resp.has_background, Some(true));
}

#[tokio::test]
async fn delete_returns_deletion() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/contacts/c9"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true,
            "deleted_at": 1700000000
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server).await.contacts().delete("c9").await.unwrap();
    assert_eq!(resp.success, Some(true));
}

#[tokio::test]
async fn error_response_maps_to_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chats/chat1/messages"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "error": "rate_limited",
            "message": "slow down",
            "status": 429,
            "code": "outbound_limit_reached"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .await
        .chat("chat1")
        .send_text("hi")
        .await
        .unwrap_err();
    assert_eq!(err.status(), Some(429));
    assert_eq!(err.code(), Some("outbound_limit_reached"));
}
