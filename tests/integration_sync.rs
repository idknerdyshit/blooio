//! Mock-server integration tests for the blocking client (httpmock).

#![cfg(feature = "sync")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal
)]

use std::time::Duration;

use blooio::resources::contacts::CreateContact;
use blooio::resources::groups::CreateGroup;
use blooio::resources::webhooks::CreateWebhook;
use blooio::{BlockingClient, ClientConfig, Operation, RequestOptions, RetryPolicy};
use httpmock::prelude::*;

fn client(server: &MockServer) -> BlockingClient {
    BlockingClient::from_config(ClientConfig::new("test-key").with_base_url(server.base_url()))
        .unwrap()
}

#[derive(Debug, Clone)]
struct HeadHealth;

impl Operation for HeadHealth {
    type Output = ();
    const METHOD: http::Method = http::Method::HEAD;

    fn path(&self) -> String {
        "/health".into()
    }
}

#[test]
fn retries_transient_5xx_until_budget_exhausted() {
    let server = MockServer::start();
    // Always 503 with a zero Retry-After so the loop runs without real delay.
    let m = server.mock(|when, then| {
        when.method(POST).path("/contacts");
        then.status(503).header("retry-after", "0");
    });

    let client = BlockingClient::from_config(
        ClientConfig::new("test-key")
            .with_base_url(server.base_url())
            .with_retry(
                RetryPolicy::default()
                    .with_max_retries(2)
                    .with_base_delay(Duration::from_millis(1))
                    .with_jitter(false),
            ),
    )
    .unwrap();

    let err = client
        .contacts()
        .create(CreateContact::new("+15550002222"))
        .unwrap_err();
    // max_retries = 2 → 3 total attempts.
    m.assert_hits(3);
    assert_eq!(err.status(), Some(503));
    assert_eq!(err.retry_after(), Some(Duration::from_secs(0)));
}

#[test]
fn does_not_retry_when_policy_is_none() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/contacts");
        then.status(503);
    });

    let client = BlockingClient::from_config(
        ClientConfig::new("test-key")
            .with_base_url(server.base_url())
            .with_retry(RetryPolicy::none()),
    )
    .unwrap();

    let err = client
        .contacts()
        .create(CreateContact::new("+15550003333"))
        .unwrap_err();
    m.assert_hits(1);
    assert_eq!(err.status(), Some(503));
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
fn send_supports_head_operations() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method("HEAD").path("/health");
        then.status(204);
    });

    client(&server).send(HeadHealth).unwrap();
    m.assert();
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
        .create(CreateContact::new("+15551234567"))
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
    assert_eq!(resp.ids(), vec!["m1"]);
}

#[test]
fn request_options_add_headers_and_query_params() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/contacts")
            .query_param("q", "alice")
            .query_param("trace", "1")
            .header("x-request-id", "req-123");
        then.status(200).json_body(serde_json::json!({
            "contacts": [],
            "pagination": { "limit": 50, "offset": 0, "total": 0 }
        }));
    });

    let resp = client(&server)
        .send_with_options(
            blooio::resources::contacts::ListContacts {
                q: Some("alice".into()),
                ..Default::default()
            },
            RequestOptions::new()
                .header("x-request-id", "req-123")
                .query("trace", "1"),
        )
        .unwrap();
    m.assert();
    assert!(resp.contacts.is_empty());
}

#[test]
fn request_options_base_url_overrides_url_only() {
    let client_server = MockServer::start();
    let override_server = MockServer::start();
    let override_mock = override_server.mock(|when, then| {
        when.method(GET)
            .path("/me")
            .header("Authorization", "Bearer test-key");
        then.status(503).header("retry-after", "0");
    });
    let client_mock = client_server.mock(|when, then| {
        when.method(GET)
            .path("/me")
            .header("Authorization", "Bearer test-key");
        then.status(200)
            .json_body(serde_json::json!({ "valid": true, "user_id": "client" }));
    });

    let client = BlockingClient::from_config(
        ClientConfig::new("test-key")
            .with_base_url(client_server.base_url())
            .with_retry(RetryPolicy::none()),
    )
    .unwrap();
    let err = client
        .send_with_options(
            blooio::resources::account::GetMe,
            RequestOptions::new()
                .base_url(format!("{}/", override_server.base_url()))
                .retry(
                    RetryPolicy::default()
                        .with_max_retries(1)
                        .with_base_delay(Duration::from_millis(1))
                        .with_jitter(false),
                ),
        )
        .unwrap_err();
    assert_eq!(err.status(), Some(503));
    override_mock.assert_hits(2);
    assert_eq!(client.config().base_url, client_server.base_url());

    let response = client.account().get().unwrap();
    client_mock.assert();
    assert_eq!(response.user_id.as_deref(), Some("client"));
}

#[test]
fn request_options_retry_override_retries_transient_error() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/me");
        then.status(503).header("retry-after", "0");
    });

    let client = BlockingClient::from_config(
        ClientConfig::new("test-key")
            .with_base_url(server.base_url())
            .with_retry(RetryPolicy::none()),
    )
    .unwrap();
    let err = client
        .send_with_options(
            blooio::resources::account::GetMe,
            RequestOptions::new().retry(
                RetryPolicy::default()
                    .with_max_retries(1)
                    .with_base_delay(Duration::from_millis(1))
                    .with_jitter(false),
            ),
        )
        .unwrap_err();
    m.assert_hits(2);
    assert_eq!(err.status(), Some(503));
}

#[test]
fn request_options_explicit_idempotency_key_is_preserved() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/contacts")
            .header("idempotency-key", "explicit-key");
        then.status(200)
            .json_body(serde_json::json!({ "id": "c1" }));
    });

    let contact = client(&server)
        .send_with_options(
            CreateContact::new("+15550001111"),
            RequestOptions::new().header("idempotency-key", "explicit-key"),
        )
        .unwrap();
    m.assert();
    assert_eq!(contact.id.as_deref(), Some("c1"));
}

#[test]
fn generated_idempotency_key_is_sent_across_retries() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/contacts")
            .header_exists("idempotency-key");
        then.status(503).header("retry-after", "0");
    });

    let client = BlockingClient::from_config(
        ClientConfig::new("test-key")
            .with_base_url(server.base_url())
            .with_retry(
                RetryPolicy::default()
                    .with_max_retries(1)
                    .with_base_delay(Duration::from_millis(1))
                    .with_jitter(false),
            ),
    )
    .unwrap();
    let err = client
        .contacts()
        .create(CreateContact::new("+15550001111"))
        .unwrap_err();
    assert_eq!(err.status(), Some(503));
    m.assert_hits(2);
}

#[test]
fn request_options_timeout_applies_per_attempt() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/me");
        then.status(200)
            .delay(Duration::from_millis(200))
            .json_body(serde_json::json!({ "valid": true, "user_id": "u1" }));
    });

    let err = client(&server)
        .send_with_options(
            blooio::resources::account::GetMe,
            RequestOptions::new()
                .timeout(Duration::from_millis(20))
                .no_retry(),
        )
        .unwrap_err();
    assert!(matches!(err, blooio::Error::Transport(_)));
    assert_eq!(m.hits(), 1);
}

#[test]
fn send_with_response_returns_decoded_output_and_raw_data() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/me");
        then.status(200)
            .header("x-ratelimit-remaining", "9")
            .header("x-secret", "server-secret")
            .json_body(serde_json::json!({ "valid": true, "user_id": "u1" }));
    });

    let response = client(&server)
        .send_with_response(blooio::resources::account::GetMe)
        .unwrap();
    m.assert();
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

#[test]
fn put_background_uses_multipart_content_type() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(PUT).path("/chats/chat1/background").header(
            "content-type",
            "multipart/form-data; boundary=blooio-form-boundary-0",
        );
        then.status(200).json_body(serde_json::json!({
            "chat_id": "chat1",
            "has_background": true,
            "changed": true
        }));
    });

    let resp = client(&server)
        .chat("chat1")
        .set_background(b"fake-png".to_vec())
        .unwrap();
    m.assert();
    assert_eq!(resp.has_background, Some(true));
}

#[test]
fn numbers_request_call_forwarding_posts_destination() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/me/numbers/%2B15551234567/call-forwarding")
            .header("content-type", "application/json")
            .json_body(serde_json::json!({ "forward_to": "+15559876543" }));
        then.status(201).json_body(serde_json::json!({
            "success": true,
            "ticket_id": "tkt_abc123",
            "status": "open",
            "number": "+15551234567",
            "forward_to": "+15559876543"
        }));
    });

    let resp = client(&server)
        .numbers()
        .request_call_forwarding("+15551234567", "+15559876543")
        .unwrap();
    m.assert();
    assert_eq!(resp.ticket_id.as_deref(), Some("tkt_abc123"));
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
fn list_all_iterator_fetches_successive_pages() {
    // Drives the blocking paginator through its `Iterator` impl: a full page
    // keeps it live, a short page terminates it. `total` is null throughout.
    let server = MockServer::start();
    let full: Vec<_> = (0..50)
        .map(|i| serde_json::json!({ "id": format!("c{i}"), "name": "x" }))
        .collect();

    let p1 = server.mock(|when, then| {
        when.method(GET)
            .path("/contacts")
            .query_param("offset", "0")
            .query_param("limit", "50");
        then.status(200).json_body(serde_json::json!({
            "contacts": full,
            "pagination": { "limit": 50, "offset": 0, "total": null }
        }));
    });
    let p2 = server.mock(|when, then| {
        when.method(GET)
            .path("/contacts")
            .query_param("offset", "50");
        then.status(200).json_body(serde_json::json!({
            "contacts": [{ "id": "c50", "name": "x" }],
            "pagination": { "limit": 50, "offset": 50, "total": null }
        }));
    });

    let c = client(&server);
    let mut total = 0usize;
    for page in c.contacts().list_all() {
        total += page.unwrap().len();
    }
    p1.assert();
    p2.assert();
    assert_eq!(total, 51);
}

#[test]
fn malformed_body_maps_to_decode_error() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/me");
        then.status(200).body("not json");
    });

    let err = client(&server).account().get().unwrap_err();
    assert!(matches!(err, blooio::Error::Decode(_)));
    assert_eq!(err.code(), None);
    assert_eq!(err.status(), None);
}

#[test]
fn connection_refused_maps_to_transport_error() {
    let client = BlockingClient::from_config(
        ClientConfig::new("test-key")
            .with_base_url("http://127.0.0.1:1")
            .with_timeout(std::time::Duration::from_secs(2)),
    )
    .unwrap();

    let err = client.account().get().unwrap_err();
    assert!(matches!(err, blooio::Error::Transport(_)));
    assert_eq!(err.code(), None);
    assert_eq!(err.status(), None);
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

// ---------------------------------------------------------------------------
// Groups
// ---------------------------------------------------------------------------

#[test]
fn groups_create_posts_to_groups() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/groups")
            .header("content-type", "application/json")
            .json_body(serde_json::json!({ "name": "Test Group" }));
        then.status(201)
            .json_body(serde_json::json!({ "group_id": "g1", "name": "Test Group" }));
    });

    let resp = client(&server)
        .groups()
        .create(CreateGroup::new("Test Group"))
        .unwrap();
    m.assert();
    // CreateGroup returns Json (serde_json::Value)
    assert_eq!(resp["group_id"], "g1");
}

#[test]
fn groups_get_fetches_by_id() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/groups/g42");
        then.status(200).json_body(serde_json::json!({
            "group_id": "g42",
            "name": "My Group",
            "member_count": 3
        }));
    });

    let group = client(&server).groups().get("g42").unwrap();
    m.assert();
    assert_eq!(group.group_id.as_deref(), Some("g42"));
    assert_eq!(group.name.as_deref(), Some("My Group"));
}

#[test]
fn groups_members_add_posts_contact_id() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/groups/g1/members")
            .header("content-type", "application/json")
            .json_body(serde_json::json!({ "contact_id": "c99" }));
        then.status(200).json_body(serde_json::json!({
            "message": "Member added",
            "contact_created": false
        }));
    });

    let resp = client(&server).groups().members("g1").add("c99").unwrap();
    m.assert();
    assert_eq!(resp.message.as_deref(), Some("Member added"));
    assert_eq!(resp.contact_created, Some(false));
}

#[test]
fn groups_members_list_all_iterator_fetches_successive_pages() {
    let server = MockServer::start();
    let full: Vec<_> = (0..50)
        .map(|i| serde_json::json!({ "id": format!("m{i}"), "contact_id": format!("c{i}") }))
        .collect();

    let p1 = server.mock(|when, then| {
        when.method(GET)
            .path("/groups/g1/members")
            .query_param("offset", "0")
            .query_param("limit", "50");
        then.status(200).json_body(serde_json::json!({
            "group_id": "g1",
            "members": full,
            "pagination": { "limit": 50, "offset": 0, "total": null }
        }));
    });
    let p2 = server.mock(|when, then| {
        when.method(GET)
            .path("/groups/g1/members")
            .query_param("offset", "50")
            .query_param("limit", "50");
        then.status(200).json_body(serde_json::json!({
            "group_id": "g1",
            "members": [{ "id": "m50", "contact_id": "c50" }],
            "pagination": { "limit": 50, "offset": 50, "total": null }
        }));
    });

    let c = client(&server);
    let mut total = 0usize;
    for page in c.groups().members("g1").list_all() {
        total += page.unwrap().len();
    }

    p1.assert();
    p2.assert();
    assert_eq!(total, 51);
}

// ---------------------------------------------------------------------------
// Webhooks
// ---------------------------------------------------------------------------

#[test]
fn webhooks_create_posts_url() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/webhooks")
            .header("content-type", "application/json")
            .json_body(serde_json::json!({ "webhook_url": "https://example.com/hook" }));
        then.status(201).json_body(serde_json::json!({
            "webhook_id": "wh1",
            "webhook_url": "https://example.com/hook",
            "signing_secret": "secret123"
        }));
    });

    let resp = client(&server)
        .webhooks()
        .create(CreateWebhook::new("https://example.com/hook"))
        .unwrap();
    m.assert();
    assert_eq!(resp.webhook_id.as_deref(), Some("wh1"));
    assert_eq!(
        resp.signing_secret.as_ref().map(|s| s.expose().as_str()),
        Some("secret123")
    );
}

#[test]
fn webhooks_rotate_secret_posts_to_rotate_path() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/webhooks/wh1/secret/rotate");
        then.status(200).json_body(serde_json::json!({
            "webhook_id": "wh1",
            "signing_secret": "new-secret-xyz",
            "rotated_at": 1700000000_i64,
            "rotation_count": 2
        }));
    });

    let resp = client(&server).webhooks().rotate_secret("wh1").unwrap();
    m.assert();
    assert_eq!(resp.webhook_id.as_deref(), Some("wh1"));
    assert_eq!(
        resp.signing_secret.as_ref().map(|s| s.expose().as_str()),
        Some("new-secret-xyz")
    );
    assert_eq!(resp.rotation_count, Some(2));
}

#[test]
fn webhooks_logs_list_fetches_logs_for_webhook() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/webhooks/wh1/logs");
        then.status(200).json_body(serde_json::json!({
            "logs": [
                { "event_id": "evt1", "response_status": 200 }
            ],
            "pagination": { "total": 1, "limit": 50, "offset": 0, "returned": 1, "has_more": false }
        }));
    });

    let resp = client(&server).webhooks().logs("wh1").list().unwrap();
    m.assert();
    assert_eq!(resp.logs.len(), 1);
    assert_eq!(resp.logs[0].event_id.as_deref(), Some("evt1"));
}

#[test]
fn webhooks_logs_list_all_iterator_fetches_successive_pages() {
    let server = MockServer::start();
    let full: Vec<_> = (0..50)
        .map(|i| serde_json::json!({ "event_id": format!("evt{i}"), "response_status": 200 }))
        .collect();

    let p1 = server.mock(|when, then| {
        when.method(GET)
            .path("/webhooks/wh1/logs")
            .query_param("offset", "0")
            .query_param("limit", "50");
        then.status(200).json_body(serde_json::json!({
            "logs": full,
            "pagination": { "limit": 50, "offset": 0, "total": null }
        }));
    });
    let p2 = server.mock(|when, then| {
        when.method(GET)
            .path("/webhooks/wh1/logs")
            .query_param("offset", "50")
            .query_param("limit", "50");
        then.status(200).json_body(serde_json::json!({
            "logs": [{ "event_id": "evt50", "response_status": 202 }],
            "pagination": { "limit": 50, "offset": 50, "total": null }
        }));
    });

    let c = client(&server);
    let mut total = 0usize;
    for page in c.webhooks().logs("wh1").list_all() {
        total += page.unwrap().len();
    }

    p1.assert();
    p2.assert();
    assert_eq!(total, 51);
}

// ---------------------------------------------------------------------------
// Location
// ---------------------------------------------------------------------------

#[test]
fn location_list_fetches_contacts() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/location/contacts");
        then.status(200).json_body(serde_json::json!({
            "friends": [
                { "handle": "+15550001111", "status": "sharing" }
            ]
        }));
    });

    let resp = client(&server).location().list().unwrap();
    m.assert();
    assert_eq!(resp.friends.len(), 1);
    assert_eq!(resp.friends[0].handle.as_deref(), Some("+15550001111"));
}

#[test]
fn location_refresh_posts_to_refresh_path() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/location/contacts/refresh");
        then.status(200).json_body(serde_json::json!({
            "success": true,
            "friends": []
        }));
    });

    let resp = client(&server).location().refresh().unwrap();
    m.assert();
    assert_eq!(resp.success, Some(true));
}

// ---------------------------------------------------------------------------
// Phone numbers
// ---------------------------------------------------------------------------

#[test]
fn phone_numbers_lookup_uses_query_param() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/phone-numbers/lookup")
            .query_param("number", "+15550001111");
        then.status(200).json_body(serde_json::json!({
            "input": "+15550001111",
            "valid": true,
            "e164": "+15550001111",
            "country": "US"
        }));
    });

    let resp = client(&server)
        .phone_numbers()
        .lookup("+15550001111")
        .unwrap();
    m.assert();
    assert_eq!(resp.input.as_deref(), Some("+15550001111"));
    assert_eq!(resp.valid, Some(true));
    assert_eq!(resp.country.as_deref(), Some("US"));
}

#[test]
fn phone_numbers_batch_posts_numbers_array() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/phone-numbers/batch")
            .header("content-type", "application/json")
            .json_body(serde_json::json!({
                "numbers": ["+15550001111", "+15550002222"]
            }));
        then.status(200).json_body(serde_json::json!({
            "results": [
                { "input": "+15550001111", "valid": true, "e164": "+15550001111" },
                { "input": "+15550002222", "valid": true, "e164": "+15550002222" }
            ]
        }));
    });

    let resp = client(&server)
        .phone_numbers()
        .batch(vec!["+15550001111".to_string(), "+15550002222".to_string()])
        .unwrap();
    m.assert();
    assert_eq!(resp.results.len(), 2);
    assert_eq!(resp.results[0].input.as_deref(), Some("+15550001111"));
    assert_eq!(resp.results[1].input.as_deref(), Some("+15550002222"));
}

#[test]
fn custom_user_agent_is_sent() {
    // Verifies ClientConfig::with_user_agent threads through to the executor.
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/me")
            .header("User-Agent", "my-app/9.9");
        then.status(200)
            .json_body(serde_json::json!({ "valid": true, "user_id": "u1" }));
    });

    let client = BlockingClient::from_config(
        ClientConfig::new("test-key")
            .with_base_url(server.base_url())
            .with_user_agent("my-app/9.9"),
    )
    .unwrap();
    client.account().get().unwrap();
    m.assert();
}

#[test]
fn send_with_meta_surfaces_rate_limit_headers() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/me");
        then.status(200)
            .header("x-ratelimit-limit", "100")
            .header("x-ratelimit-remaining", "7")
            .json_body(serde_json::json!({ "valid": true, "user_id": "u1" }));
    });

    let (me, meta) = client(&server)
        .send_with_meta(blooio::resources::account::GetMe)
        .unwrap();

    m.assert();
    assert_eq!(me.user_id.as_deref(), Some("u1"));
    let rate_limit = meta.rate_limit.expect("rate-limit headers present");
    assert_eq!(rate_limit.limit, Some(100));
    assert_eq!(rate_limit.remaining, Some(7));
}
