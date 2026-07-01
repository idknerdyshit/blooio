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

use std::sync::{Arc, Mutex};
use std::time::Duration;

use blooio::error::codes;
use blooio::resources::contacts::CreateContact;
use blooio::resources::groups::CreateGroup;
use blooio::resources::webhooks::CreateWebhook;
use blooio::{Client, ClientConfig, RequestOptions, RetryPolicy};
use wiremock::matchers::{body_json, header, header_exists, method, path, query_param};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

/// A client with fast, deterministic retries for exercising the retry loop
/// without slowing the test suite.
fn retrying_client(server: &MockServer, max_retries: u32) -> Client {
    Client::from_config(
        ClientConfig::new("test-key")
            .with_base_url(server.uri())
            .with_retry(
                RetryPolicy::default()
                    .with_max_retries(max_retries)
                    .with_base_delay(Duration::from_millis(1))
                    .with_jitter(false),
            ),
    )
    .unwrap()
}

async fn client(server: &MockServer) -> Client {
    Client::from_config(ClientConfig::new("test-key").with_base_url(server.uri())).unwrap()
}

#[derive(Debug, Clone)]
struct CaptureIdempotencyKey {
    values: Arc<Mutex<Vec<String>>>,
}

impl Match for CaptureIdempotencyKey {
    fn matches(&self, request: &Request) -> bool {
        let Some(value) = request.headers.get("idempotency-key") else {
            return false;
        };
        let Ok(value) = value.to_str() else {
            return false;
        };
        self.values.lock().unwrap().push(value.to_owned());
        true
    }
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
        .create(CreateContact::new("+15551234567"))
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
    assert_eq!(resp.ids(), vec!["m1"]);
}

#[tokio::test]
async fn request_options_add_headers_and_query_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/contacts"))
        .and(query_param("q", "alice"))
        .and(query_param("trace", "1"))
        .and(header("x-request-id", "req-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "contacts": [],
            "pagination": { "limit": 50, "offset": 0, "total": 0 }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server)
        .await
        .send_with_options(
            blooio::resources::contacts::ListContacts {
                q: Some("alice".into()),
                ..Default::default()
            },
            RequestOptions::new()
                .header("x-request-id", "req-123")
                .query("trace", "1"),
        )
        .await
        .unwrap();
    assert!(resp.contacts.is_empty());
}

#[tokio::test]
async fn request_options_base_url_overrides_url_only() {
    let client_server = MockServer::start().await;
    let override_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/me"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "0"))
        .up_to_n_times(1)
        .with_priority(1)
        .expect(1)
        .mount(&override_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "valid": true,
            "user_id": "override"
        })))
        .with_priority(2)
        .expect(1)
        .mount(&override_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "valid": true,
            "user_id": "client"
        })))
        .expect(1)
        .mount(&client_server)
        .await;

    let client = Client::from_config(
        ClientConfig::new("test-key")
            .with_base_url(client_server.uri())
            .with_retry(RetryPolicy::none()),
    )
    .unwrap();
    let response = client
        .send_with_options(
            blooio::resources::account::GetMe,
            RequestOptions::new()
                .base_url(format!("{}/", override_server.uri()))
                .retry(
                    RetryPolicy::default()
                        .with_max_retries(1)
                        .with_base_delay(Duration::from_millis(1))
                        .with_jitter(false),
                ),
        )
        .await
        .unwrap();
    assert_eq!(response.user_id.as_deref(), Some("override"));
    assert_eq!(client.config().base_url, client_server.uri());

    let response = client.account().get().await.unwrap();
    assert_eq!(response.user_id.as_deref(), Some("client"));
}

#[tokio::test]
async fn request_options_retry_override_retries_transient_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/contacts"))
        .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "0"))
        .up_to_n_times(1)
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/contacts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "c1",
            "name": "Alice"
        })))
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(
        ClientConfig::new("test-key")
            .with_base_url(server.uri())
            .with_retry(RetryPolicy::none()),
    )
    .unwrap();
    let contact = client
        .send_with_options(
            CreateContact::new("+15550001111").name("Alice"),
            RequestOptions::new().retry(
                RetryPolicy::default()
                    .with_max_retries(1)
                    .with_base_delay(Duration::from_millis(1))
                    .with_jitter(false),
            ),
        )
        .await
        .unwrap();
    assert_eq!(contact.id.as_deref(), Some("c1"));
}

#[tokio::test]
async fn request_options_explicit_idempotency_key_is_preserved() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/contacts"))
        .and(header("idempotency-key", "explicit-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "c1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let contact = client(&server)
        .await
        .send_with_options(
            CreateContact::new("+15550001111"),
            RequestOptions::new().header("idempotency-key", "explicit-key"),
        )
        .await
        .unwrap();
    assert_eq!(contact.id.as_deref(), Some("c1"));
}

#[tokio::test]
async fn generated_idempotency_key_is_reused_across_retries() {
    let server = MockServer::start().await;
    let observed = Arc::new(Mutex::new(Vec::new()));
    let capture = CaptureIdempotencyKey {
        values: Arc::clone(&observed),
    };

    Mock::given(method("POST"))
        .and(path("/contacts"))
        .and(capture.clone())
        .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "0"))
        .up_to_n_times(1)
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/contacts"))
        .and(capture)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "c1"
        })))
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;

    retrying_client(&server, 1)
        .contacts()
        .create(CreateContact::new("+15550001111"))
        .await
        .unwrap();

    let values = observed.lock().unwrap();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0], values[1]);
}

#[tokio::test]
async fn request_options_timeout_applies_per_attempt() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(200))
                .set_body_json(serde_json::json!({ "valid": true, "user_id": "u1" })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let err = client(&server)
        .await
        .send_with_options(
            blooio::resources::account::GetMe,
            RequestOptions::new()
                .timeout(Duration::from_millis(20))
                .no_retry(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, blooio::Error::Transport(_)));
}

#[tokio::test]
async fn send_with_response_returns_decoded_output_and_raw_data() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-ratelimit-remaining", "9")
                .insert_header("x-secret", "server-secret")
                .set_body_json(serde_json::json!({ "valid": true, "user_id": "u1" })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let response = client(&server)
        .await
        .send_with_response(blooio::resources::account::GetMe)
        .await
        .unwrap();
    assert_eq!(response.output.user_id.as_deref(), Some("u1"));
    assert_eq!(response.meta.status, 200);
    assert_eq!(response.meta.rate_limit.unwrap().remaining, Some(9));
    assert_eq!(response.raw.status, 200);
    assert!(response.raw.headers.contains_key("x-secret"));
    assert!(response.raw.body.starts_with(b"{"));

    let dbg = format!("{response:?}");
    assert!(!dbg.contains("server-secret"));
    assert!(dbg.contains("REDACTED"));
}

#[tokio::test]
async fn put_replaces_resource() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/chats/chat1/background"))
        .and(header(
            "content-type",
            "multipart/form-data; boundary=blooio-form-boundary-0",
        ))
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
        .set_background(b"fake-png".to_vec())
        .await
        .unwrap();
    assert_eq!(resp.has_background, Some(true));
}

#[tokio::test]
async fn numbers_request_call_forwarding_posts_destination() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/me/numbers/%2B15551234567/call-forwarding"))
        .and(header("content-type", "application/json"))
        .and(body_json(
            serde_json::json!({ "forward_to": "+15559876543" }),
        ))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "success": true,
            "ticket_id": "tkt_abc123",
            "status": "open",
            "number": "+15551234567",
            "forward_to": "+15559876543"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client(&server)
        .await
        .numbers()
        .request_call_forwarding("+15551234567", "+15559876543")
        .await
        .unwrap();
    assert_eq!(resp.ticket_id.as_deref(), Some("tkt_abc123"));
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
            "code": "outbound_limit_reached",
            "limit": 10,
            "current": 10,
            "mode": "organization"
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
    assert_eq!(err.code(), Some(codes::OUTBOUND_LIMIT_REACHED));
    assert!(err.is_quota_error());
    assert!(!err.is_retryable());
    let blooio::Error::Api(api) = err else {
        panic!("expected api error");
    };
    assert_eq!(api.error(), Some("rate_limited"));
    assert_eq!(api.server_message(), Some("slow down"));
    assert_eq!(api.details().get("limit"), Some(&serde_json::json!(10)));
    assert_eq!(
        api.details().get("mode"),
        Some(&serde_json::json!("organization"))
    );
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
        .create(CreateGroup::new("Friends"))
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

#[tokio::test]
async fn groups_members_list_all_fetches_successive_pages() {
    let server = MockServer::start().await;
    let full: Vec<_> = (0..50)
        .map(|i| serde_json::json!({ "id": format!("m{i}"), "contact_id": format!("c{i}") }))
        .collect();

    Mock::given(method("GET"))
        .and(path("/groups/g42/members"))
        .and(query_param("offset", "0"))
        .and(query_param("limit", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "group_id": "g42",
            "members": full,
            "pagination": { "limit": 50, "offset": 0, "total": null }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/groups/g42/members"))
        .and(query_param("offset", "50"))
        .and(query_param("limit", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "group_id": "g42",
            "members": [{ "id": "m50", "contact_id": "c50" }],
            "pagination": { "limit": 50, "offset": 50, "total": null }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let members = client(&server)
        .await
        .groups()
        .members("g42")
        .list_all()
        .collect_all()
        .await
        .unwrap();
    assert_eq!(members.len(), 51);
    assert_eq!(
        members.first().and_then(|m| m.contact_id.as_deref()),
        Some("c0")
    );
    assert_eq!(
        members.last().and_then(|m| m.contact_id.as_deref()),
        Some("c50")
    );
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
        .create(CreateWebhook::new("https://example.com/hook"))
        .await
        .unwrap();
    assert_eq!(resp.webhook_id.as_deref(), Some("wh1"));
    assert_eq!(
        resp.signing_secret.as_ref().map(|s| s.expose().as_str()),
        Some("sec_abc")
    );
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
    assert_eq!(
        resp.signing_secret.as_ref().map(|s| s.expose().as_str()),
        Some("sec_new")
    );
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

#[tokio::test]
async fn webhooks_logs_list_all_fetches_successive_pages() {
    let server = MockServer::start().await;
    let full: Vec<_> = (0..50)
        .map(|i| serde_json::json!({ "event_id": format!("evt{i}"), "response_status": 200 }))
        .collect();

    Mock::given(method("GET"))
        .and(path("/webhooks/wh1/logs"))
        .and(query_param("offset", "0"))
        .and(query_param("limit", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "logs": full,
            "pagination": { "limit": 50, "offset": 0, "total": null }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/webhooks/wh1/logs"))
        .and(query_param("offset", "50"))
        .and(query_param("limit", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "logs": [{ "event_id": "evt50", "response_status": 202 }],
            "pagination": { "limit": 50, "offset": 50, "total": null }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let logs = client(&server)
        .await
        .webhooks()
        .logs("wh1")
        .list_all()
        .collect_all()
        .await
        .unwrap();
    assert_eq!(logs.len(), 51);
    assert_eq!(
        logs.first().and_then(|log| log.event_id.as_deref()),
        Some("evt0")
    );
    assert_eq!(
        logs.last().and_then(|log| log.event_id.as_deref()),
        Some("evt50")
    );
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
        .batch(["+15550001111", "+15550002222"])
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

#[tokio::test]
async fn retries_unknown_429_then_succeeds() {
    let server = MockServer::start().await;
    // First attempt: unknown 429 with a Retry-After hint. Exhausted after one match.
    Mock::given(method("POST"))
        .and(path("/contacts"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_json(serde_json::json!({
                    "error": "rate_limited",
                    "code": "temporarily_rate_limited"
                })),
        )
        .up_to_n_times(1)
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    // Retry lands here.
    Mock::given(method("POST"))
        .and(path("/contacts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "c1",
            "name": "Alice"
        })))
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;

    let contact = retrying_client(&server, 2)
        .contacts()
        .create(CreateContact::new("+15550001111").name("Alice"))
        .await
        .unwrap();
    assert_eq!(contact.id.as_deref(), Some("c1"));
}

#[tokio::test]
async fn does_not_retry_documented_quota_429() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/contacts"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_json(serde_json::json!({
                    "error": "rate_limited",
                    "code": "new_conversation_limit_reached",
                    "plan_id": "plan_123",
                    "cap": 50,
                    "current": 50
                })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let err = retrying_client(&server, 2)
        .contacts()
        .create(CreateContact::new("+15550001111").name("Alice"))
        .await
        .unwrap_err();
    assert_eq!(err.status(), Some(429));
    assert_eq!(err.code(), Some(codes::NEW_CONVERSATION_LIMIT_REACHED));
    assert_eq!(err.retry_after(), Some(Duration::from_secs(0)));
    assert!(err.is_quota_error());
    assert!(!err.is_retryable());
    let blooio::Error::Api(api) = err else {
        panic!("expected api error");
    };
    assert_eq!(
        api.details().get("plan_id"),
        Some(&serde_json::json!("plan_123"))
    );
    assert_eq!(api.details().get("cap"), Some(&serde_json::json!(50)));
}

#[tokio::test]
async fn does_not_retry_client_error() {
    let server = MockServer::start().await;
    // A 400 is not retryable: exactly one request must be made.
    Mock::given(method("POST"))
        .and(path("/contacts"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "bad_request",
            "code": "invalid_number"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = retrying_client(&server, 3)
        .contacts()
        .create(CreateContact::new("nope"))
        .await
        .unwrap_err();
    assert_eq!(err.status(), Some(400));
    assert_eq!(err.code(), Some("invalid_number"));
}

#[tokio::test]
async fn gives_up_after_exhausting_retries() {
    let server = MockServer::start().await;
    // Always 503; with max_retries = 2 the client makes 3 attempts total.
    Mock::given(method("POST"))
        .and(path("/contacts"))
        .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "0"))
        .expect(3)
        .mount(&server)
        .await;

    let err = retrying_client(&server, 2)
        .contacts()
        .create(CreateContact::new("+15550002222"))
        .await
        .unwrap_err();
    assert_eq!(err.status(), Some(503));
    assert_eq!(err.retry_after(), Some(Duration::from_secs(0)));
}

#[tokio::test]
async fn send_with_meta_surfaces_rate_limit_headers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-ratelimit-limit", "100")
                .insert_header("x-ratelimit-remaining", "42")
                .set_body_json(serde_json::json!({ "valid": true, "user_id": "u1" })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = retrying_client(&server, 0);
    let (me, meta) = client
        .send_with_meta(blooio::resources::account::GetMe)
        .await
        .unwrap();
    assert_eq!(me.user_id.as_deref(), Some("u1"));
    let rl = meta.rate_limit.expect("rate-limit headers present");
    assert_eq!(rl.limit, Some(100));
    assert_eq!(rl.remaining, Some(42));
}

#[tokio::test]
async fn paginator_stream_yields_items_across_pages() {
    use futures::StreamExt;

    let server = MockServer::start().await;
    let full: Vec<_> = (0..50)
        .map(|i| serde_json::json!({ "id": format!("c{i}"), "name": "x" }))
        .collect();
    Mock::given(method("GET"))
        .and(path("/contacts"))
        .and(query_param("offset", "0"))
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

    let c = client(&server).await;
    let stream = c.contacts().list_all().stream();
    futures::pin_mut!(stream);
    let mut ids = Vec::new();
    while let Some(item) = stream.next().await {
        ids.push(item.unwrap().id.unwrap());
    }
    assert_eq!(ids.len(), 51);
    assert_eq!(ids.first().map(String::as_str), Some("c0"));
    assert_eq!(ids.last().map(String::as_str), Some("c50"));
}
