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
async fn list_all_fetches_successive_pages() {
    // A full page (offset=0) keeps the paginator live; a short page (offset=50)
    // terminates it. `total` is null throughout, so termination relies purely on
    // the short-page guard.
    let server = MockServer::start().await;
    let full: Vec<_> = (0..50)
        .map(|i| serde_json::json!({ "id": format!("c{i}"), "name": "x" }))
        .collect();

    Mock::given(method("GET"))
        .and(path("/contacts"))
        .and(query_param("offset", "0"))
        .and(query_param("limit", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "contacts": full,
            "pagination": { "limit": 50, "offset": 0, "total": null }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/contacts"))
        .and(query_param("offset", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "contacts": [{ "id": "c50", "name": "x" }],
            "pagination": { "limit": 50, "offset": 50, "total": null }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let all = client(&server)
        .await
        .contacts()
        .list_all()
        .collect_all()
        .await
        .unwrap();
    assert_eq!(all.len(), 51);
}

#[tokio::test]
async fn malformed_body_maps_to_decode_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let err = client(&server).await.account().get().await.unwrap_err();
    assert!(matches!(err, blooio::Error::Decode(_)));
    // Decode is not an API error: the machine-readable accessors return None.
    assert_eq!(err.code(), None);
    assert_eq!(err.status(), None);
}

#[tokio::test]
async fn connection_refused_maps_to_transport_error() {
    // Port 1 is reserved and refuses connections immediately on localhost.
    let client = Client::from_config(
        ClientConfig::new("test-key")
            .with_base_url("http://127.0.0.1:1")
            .with_timeout(std::time::Duration::from_secs(2)),
    )
    .unwrap();

    let err = client.account().get().await.unwrap_err();
    assert!(matches!(err, blooio::Error::Transport(_)));
    assert_eq!(err.code(), None);
    assert_eq!(err.status(), None);
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

// ---------------------------------------------------------------------------
// Groups
// ---------------------------------------------------------------------------

#[tokio::test]
async fn groups_create_posts_body_and_returns_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/groups"))
        .and(header("content-type", "application/json"))
        .and(body_json(serde_json::json!({ "name": "Friends" })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "group_id": "g1",
            "name": "Friends"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server)
        .await
        .groups()
        .create("Friends", None, None)
        .await
        .unwrap();
    assert_eq!(resp["group_id"], "g1");
}

#[tokio::test]
async fn groups_get_returns_group() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/groups/g42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "group_id": "g42",
            "name": "Test Group",
            "member_count": 3
        })))
        .expect(1)
        .mount(&server)
        .await;

    let group = client(&server).await.groups().get("g42").await.unwrap();
    assert_eq!(group.group_id.as_deref(), Some("g42"));
    assert_eq!(group.name.as_deref(), Some("Test Group"));
}

#[tokio::test]
async fn groups_members_add_posts_contact_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/groups/g42/members"))
        .and(header("content-type", "application/json"))
        .and(body_json(serde_json::json!({ "contact_id": "c9" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "message": "Member added",
            "contact_created": false,
            "member": { "id": "c9", "contact_id": "c9", "identifier": "+15550009999" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server)
        .await
        .groups()
        .members("g42")
        .add("c9")
        .await
        .unwrap();
    assert_eq!(resp.message.as_deref(), Some("Member added"));
    assert_eq!(resp.contact_created, Some(false));
}

// ---------------------------------------------------------------------------
// Webhooks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn webhooks_create_posts_url_and_returns_webhook_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/webhooks"))
        .and(header("content-type", "application/json"))
        .and(body_json(
            serde_json::json!({ "webhook_url": "https://example.com/hook" }),
        ))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "webhook_id": "wh1",
            "webhook_url": "https://example.com/hook",
            "signing_secret": "sec_abc"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server)
        .await
        .webhooks()
        .create("https://example.com/hook", None, None)
        .await
        .unwrap();
    assert_eq!(resp.webhook_id.as_deref(), Some("wh1"));
    assert_eq!(resp.signing_secret.as_deref(), Some("sec_abc"));
}

#[tokio::test]
async fn webhooks_rotate_secret_posts_and_returns_new_secret() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/webhooks/wh1/secret/rotate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "webhook_id": "wh1",
            "signing_secret": "sec_new",
            "rotated_at": 1700000001,
            "rotation_count": 2
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server)
        .await
        .webhooks()
        .rotate_secret("wh1")
        .await
        .unwrap();
    assert_eq!(resp.webhook_id.as_deref(), Some("wh1"));
    assert_eq!(resp.signing_secret.as_deref(), Some("sec_new"));
    assert_eq!(resp.rotation_count, Some(2));
}

#[tokio::test]
async fn webhooks_logs_list_returns_logs() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/webhooks/wh1/logs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "logs": [
                { "event_id": "evt1", "response_status": 200, "scope": "message" }
            ],
            "pagination": { "total": 1, "limit": 50, "offset": 0, "returned": 1, "has_more": false }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server)
        .await
        .webhooks()
        .logs("wh1")
        .list()
        .await
        .unwrap();
    assert_eq!(resp.logs.len(), 1);
    assert_eq!(resp.logs[0].event_id.as_deref(), Some("evt1"));
    assert_eq!(resp.logs[0].response_status, Some(200));
}

// ---------------------------------------------------------------------------
// Location
// ---------------------------------------------------------------------------

#[tokio::test]
async fn location_list_returns_friends() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/location/contacts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "friends": [
                { "handle": "+15550001111", "status": "sharing", "last_updated": 1700000000 }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server).await.location().list().await.unwrap();
    assert_eq!(resp.friends.len(), 1);
    assert_eq!(resp.friends[0].handle.as_deref(), Some("+15550001111"));
}

#[tokio::test]
async fn location_refresh_posts_and_returns_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/location/contacts/refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true,
            "friends": [
                { "handle": "+15550001111", "status": "sharing", "last_updated": 1700000010 }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server).await.location().refresh().await.unwrap();
    assert_eq!(resp.success, Some(true));
    let friends = resp.friends.unwrap();
    assert_eq!(friends.len(), 1);
    assert_eq!(friends[0].handle.as_deref(), Some("+15550001111"));
}

// ---------------------------------------------------------------------------
// Phone numbers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn phone_numbers_lookup_sends_query_param() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/phone-numbers/lookup"))
        .and(query_param("number", "+15550001111"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "input": "+15550001111",
            "valid": true,
            "e164": "+15550001111",
            "country": "US"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server)
        .await
        .phone_numbers()
        .lookup("+15550001111")
        .await
        .unwrap();
    assert_eq!(resp.valid, Some(true));
    assert_eq!(resp.e164.as_deref(), Some("+15550001111"));
    assert_eq!(resp.country.as_deref(), Some("US"));
}

#[tokio::test]
async fn phone_numbers_batch_posts_numbers_and_returns_results() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/phone-numbers/batch"))
        .and(header("content-type", "application/json"))
        .and(body_json(serde_json::json!({
            "numbers": ["+15550001111", "+15550002222"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                { "input": "+15550001111", "valid": true, "e164": "+15550001111" },
                { "input": "+15550002222", "valid": true, "e164": "+15550002222" }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server)
        .await
        .phone_numbers()
        .batch(vec!["+15550001111".into(), "+15550002222".into()])
        .await
        .unwrap();
    assert_eq!(resp.results.len(), 2);
    assert_eq!(resp.results[0].e164.as_deref(), Some("+15550001111"));
    assert_eq!(resp.results[1].valid, Some(true));
}

#[tokio::test]
async fn custom_user_agent_is_sent() {
    // Verifies ClientConfig::with_user_agent threads through to the executor.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .and(header("user-agent", "my-app/9.9"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "valid": true,
            "user_id": "u1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(
        ClientConfig::new("test-key")
            .with_base_url(server.uri())
            .with_user_agent("my-app/9.9"),
    )
    .unwrap();
    // The mock only matches when the User-Agent header is correct; a mismatch
    // would return 404 and make this call fail.
    client.account().get().await.unwrap();
}
