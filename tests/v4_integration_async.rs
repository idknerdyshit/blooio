//! Async mock-server coverage for the opt-in v4 client.

#![cfg(all(feature = "api-v4", feature = "async"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]

use blooio::v4::resources::channels::{
    ListAvailableBlooioNumbers, PurchaseBlooioNumbers, RemoveBlooioNumber,
};
use blooio::v4::resources::chats::{CreateChat, SendPoll, VotePoll};
use blooio::v4::resources::contacts::UpdateContact;
use blooio::v4::resources::groups::UpdateGroup;
use blooio::v4::resources::messages::SendMessage;
use blooio::v4::types::{
    BlooioNumberType, BlooioPurchaseStatus, MessageContentFields, MessageSendResult, Recipient,
};
use blooio::v4::{Client, DEFAULT_BASE_URL};
use blooio::{BlooioCreds, ClientConfig};
use wiremock::matchers::{body_json, header, header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn new_uses_v4_production_url() {
    let client = Client::new().unwrap();
    assert_eq!(client.config().base_url, DEFAULT_BASE_URL);
    assert_ne!(client.config().base_url, blooio::DEFAULT_BASE_URL);
}

#[tokio::test]
async fn account_get_uses_shared_transport_and_v4_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "id": "acct_1", "organization_id": "org_1" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(ClientConfig::new().with_base_url(server.uri())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let response = client.account(&creds).me().get().await.unwrap();
    assert_eq!(response.data.id.as_deref(), Some("acct_1"));
}

#[tokio::test]
async fn cursor_paginator_uses_opaque_next_cursor() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/events"))
        .and(query_param("limit", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"id":"evt_1","type":"message.sent"}],
            "has_more": true,
            "next_cursor": "cursor-secret"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/events"))
        .and(query_param("limit", "50"))
        .and(query_param("cursor", "cursor-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"id":"evt_2","type":"message.delivered"}],
            "has_more": false,
            "next_cursor": null
        })))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(ClientConfig::new().with_base_url(server.uri())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let events = client
        .account(&creds)
        .events()
        .list_all()
        .collect_all()
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn available_number_paginator_uses_next_cursor_and_preserves_filters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/channels/blooio/available"))
        .and(query_param("type", "dedicated"))
        .and(query_param("area_code", "415"))
        .and(query_param("limit", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"phone_number": "+14155550100"}],
            "has_more": true,
            "next_cursor": "available-next"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/channels/blooio/available"))
        .and(query_param("type", "dedicated"))
        .and(query_param("area_code", "415"))
        .and(query_param("limit", "50"))
        .and(query_param("cursor", "available-next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"phone_number": "+14155550101"}],
            "has_more": false,
            "next_cursor": null
        })))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(ClientConfig::new().with_base_url(server.uri())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let numbers = client
        .account(&creds)
        .channels()
        .available_all(ListAvailableBlooioNumbers {
            number_type: Some(BlooioNumberType::Dedicated),
            area_codes: vec!["415".into()],
            ..Default::default()
        })
        .collect_all()
        .await
        .unwrap();

    assert_eq!(numbers.len(), 2);
}

#[tokio::test]
async fn cursor_paginator_rejects_missing_cursor_without_reflection() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/events"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [],
            "has_more": true,
            "next_cursor": null
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(ClientConfig::new().with_base_url(server.uri())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let error = client
        .account(&creds)
        .events()
        .list_all()
        .collect_all()
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "failed to decode JSON body: invalid cursor pagination metadata: missing next cursor"
    );
}

#[tokio::test]
async fn cursor_paginator_rejects_repeated_cursor_without_reflection() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/events"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [],
            "has_more": true,
            "next_cursor": "private-cursor-value"
        })))
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/events"))
        .and(query_param("cursor", "private-cursor-value"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [],
            "has_more": true,
            "next_cursor": "private-cursor-value"
        })))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(ClientConfig::new().with_base_url(server.uri())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let error = client
        .account(&creds)
        .events()
        .list_all()
        .collect_all()
        .await
        .unwrap_err();
    let rendered = format!("{error:?} {error}");
    assert!(rendered.contains("repeated next cursor"));
    assert!(!rendered.contains("private-cursor-value"));
}

#[tokio::test]
async fn global_send_uses_typed_body_and_idempotency_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(header_exists("idempotency-key"))
        .and(body_json(serde_json::json!({
            "to": "+15551234567",
            "text": "hello"
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "msg_1", "chat_id": "chat_1", "status": "queued"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(ClientConfig::new().with_base_url(server.uri())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let sent = client
        .account(&creds)
        .messages()
        .send(SendMessage::new(
            Recipient::identifier("+15551234567"),
            MessageContentFields::text("hello"),
        ))
        .await
        .unwrap();
    let MessageSendResult::Message(sent) = sent else {
        panic!("expected a single-message response");
    };
    assert_eq!(sent.id.as_deref(), Some("msg_1"));
}

#[tokio::test]
async fn blooio_number_lifecycle_uses_typed_mirrored_operations() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/channels/blooio/available"))
        .and(query_param("type", "dedicated"))
        .and(query_param("area_code", "415"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"phone_number": "+14155550100"}],
            "matched_count": 1,
            "custom_order_count": 0,
            "has_more": false,
            "next_cursor": null
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/channels/blooio/purchases"))
        .and(header("idempotency-key", "purchase-test-1"))
        .and(body_json(serde_json::json!({
            "plan": "dedicated",
            "quantity": 1,
            "area_codes": ["415"]
        })))
        .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({
            "data": {"purchase_id": "purchase_1", "status": "provisioning"}
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/channels/blooio/purchases/purchase_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "purchase_id": "purchase_1",
                "status": "completed",
                "allocations": [{"phone_number": "+14155550100", "channel_id": "ch_1"}]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/channels/%2B14155550100"))
        .and(body_json(
            serde_json::json!({"reasons": ["no_longer_needed"]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "phone_number": "+14155550100",
                "channel_id": "ch_1",
                "reasons": ["no_longer_needed"]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(ClientConfig::new().with_base_url(server.uri())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let channels = client.account(&creds).channels();

    let available = channels
        .available(ListAvailableBlooioNumbers {
            number_type: Some(BlooioNumberType::Dedicated),
            area_codes: vec!["415".into()],
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(available.matched_count, Some(1));

    let mut purchase = PurchaseBlooioNumbers::new("dedicated", "purchase-test-1");
    purchase.quantity = Some(1);
    purchase.area_codes = vec!["415".into()];
    let accepted = channels.purchase(purchase).await.unwrap();
    assert_eq!(
        accepted.data.status,
        Some(BlooioPurchaseStatus::Provisioning)
    );

    let completed = channels.purchase_status("purchase_1").await.unwrap();
    assert_eq!(completed.data.status, Some(BlooioPurchaseStatus::Completed));
    assert_eq!(completed.data.allocations.len(), 1);

    let removed = channels
        .remove(RemoveBlooioNumber::new(
            "+14155550100",
            ["no_longer_needed"],
        ))
        .await
        .unwrap();
    assert_eq!(removed.data.channel_id.as_deref(), Some("ch_1"));
}

#[tokio::test]
async fn default_priority_accepts_null_data() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/priority"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": null})))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(ClientConfig::new().with_base_url(server.uri())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let response = client.account(&creds).me().priority().await.unwrap();
    assert!(response.data.is_none());
}

#[tokio::test]
async fn create_chat_decodes_unwrapped_chat_created() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chats"))
        .and(body_json(serde_json::json!({
            "channel_id": "ch_1",
            "to": "+15551234567"
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "chat_1",
            "channel_id": "ch_1",
            "state": "open",
            "created": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(ClientConfig::new().with_base_url(server.uri())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let created = client
        .account(&creds)
        .chats()
        .create(CreateChat::new("ch_1", "+15551234567"))
        .await
        .unwrap();
    assert_eq!(created.id.as_deref(), Some("chat_1"));
    assert_eq!(created.created, Some(true));
}

#[tokio::test]
async fn poll_operations_use_the_documented_request_and_response_shapes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chats/chat_1/polls"))
        .and(body_json(serde_json::json!({
            "title": "Lunch?",
            "options": ["Yes", "No"]
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {
                "id": "msg_1",
                "chat_id": "chat_1",
                "type": "poll",
                "poll": {"title": "Lunch?", "options": ["Yes", "No"]}
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chats/chat_1/polls/msg_1/vote"))
        .and(body_json(serde_json::json!({"option_index": 1})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"id": "vote_1", "poll_id": "msg_1", "voted_option": "No"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(ClientConfig::new().with_base_url(server.uri())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let account = client.account(&creds);
    let created = account
        .send(SendPoll::new("chat_1", "Lunch?", ["Yes", "No"]))
        .await
        .unwrap();
    assert_eq!(created.data.id.as_deref(), Some("msg_1"));
    assert_eq!(
        created
            .data
            .poll
            .as_ref()
            .and_then(|poll| poll.title.as_deref()),
        Some("Lunch?")
    );
    account
        .send(VotePoll::by_index("chat_1", "msg_1", 1))
        .await
        .unwrap();
}

#[tokio::test]
async fn multi_recipient_send_decodes_fan_out_result() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(207).set_body_json(serde_json::json!({
            "data": [
                {"id": "msg_1", "status": "queued", "to": "+15550000001"},
                {"id": "msg_2", "status": "failed", "to": "+15550000002"}
            ],
            "fan_out": true,
            "sent": 1,
            "failed": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(ClientConfig::new().with_base_url(server.uri())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let result = client
        .account(&creds)
        .messages()
        .send(SendMessage::new(
            Recipient::identifiers(["+15550000001", "+15550000002"]),
            MessageContentFields::text("hello"),
        ))
        .await
        .unwrap();
    let MessageSendResult::FanOut(result) = result else {
        panic!("expected a fan-out response");
    };
    assert_eq!(result.data.len(), 2);
    assert_eq!(result.sent, Some(1));
    assert_eq!(result.failed, Some(1));
}

#[tokio::test]
async fn contact_and_group_updates_require_string_names() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/contacts/ct_1"))
        .and(body_json(serde_json::json!({"name": "Ada"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"id": "ct_1", "name": "Ada"}
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/groups/grp_1"))
        .and(body_json(serde_json::json!({"name": "Friends"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"id": "grp_1", "name": "Friends"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::from_config(ClientConfig::new().with_base_url(server.uri())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let account = client.account(&creds);
    account
        .send(UpdateContact {
            contact_id: "ct_1".into(),
            name: "Ada".into(),
        })
        .await
        .unwrap();
    account
        .send(UpdateGroup {
            group_id: "grp_1".into(),
            name: "Friends".into(),
        })
        .await
        .unwrap();
}
