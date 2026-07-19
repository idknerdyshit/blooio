//! Blocking mock-server coverage for the opt-in v4 client.

#![cfg(all(feature = "api-v4", feature = "sync"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]

use blooio::v4::{BlockingClient, DEFAULT_BASE_URL};
use blooio::{BlooioCreds, ClientConfig};
use httpmock::prelude::*;

#[test]
fn new_uses_v4_production_url() {
    let client = BlockingClient::new().unwrap();
    assert_eq!(client.config().base_url, DEFAULT_BASE_URL);
}

#[test]
fn account_get_uses_shared_blocking_transport() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/me")
            .header("authorization", "Bearer test-key");
        then.status(200)
            .json_body(serde_json::json!({"data":{"id":"acct_1"}}));
    });
    let client =
        BlockingClient::from_config(ClientConfig::new().with_base_url(server.base_url())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let response = client.account(&creds).me().get().unwrap();
    mock.assert();
    assert_eq!(response.data.id.as_deref(), Some("acct_1"));
}

#[test]
fn cursor_paginator_is_a_blocking_iterator() {
    let server = MockServer::start();
    let first = server.mock(|when, then| {
        when.method(GET)
            .path("/webhooks")
            .query_param("limit", "50");
        then.status(200).json_body(
            serde_json::json!({"data":[{"id":"wh_1"}],"has_more":false,"next_cursor":null}),
        );
    });
    let client =
        BlockingClient::from_config(ClientConfig::new().with_base_url(server.base_url())).unwrap();
    let creds = BlooioCreds::new("test-key");
    let pages: Vec<_> = client.account(&creds).webhooks().list_all().collect();
    first.assert();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].as_ref().unwrap().len(), 1);
}
