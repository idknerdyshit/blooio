//! Sensitive diagnostics integration tests.

#![cfg(all(
    feature = "sensitive-diagnostics",
    any(feature = "async", feature = "sync")
))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal
)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(feature = "sync")]
use blooio::BlockingClient;
#[cfg(feature = "async")]
use blooio::Client;
use blooio::resources::chats::SendMessage;
use blooio::{
    ClientConfig, RequestOptions, RetryPolicy, SensitiveDiagnosticEvent, SensitiveDiagnostics,
    SensitiveTransportErrorStage,
};

const API_KEY: &str = "test-key-sensitive-diagnostics";
const TRACE_LABEL: &str = "trace-sensitive-label";
const CHAT_ID: &str = "chat secret";
const MESSAGE_TEXT: &str = "body secret";
const QUERY_SECRET: &str = "query-secret";
const HEADER_SECRET: &str = "header-secret";
const IDEMPOTENCY_SECRET: &str = "idem-secret";
const RESPONSE_SECRET: &str = "response-secret";

type EventCapture = Arc<Mutex<Vec<SensitiveDiagnosticEvent>>>;

fn capture() -> (SensitiveDiagnostics, EventCapture) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let diagnostics = SensitiveDiagnostics::new(move |event: SensitiveDiagnosticEvent| {
        captured.lock().unwrap().push(event);
    });
    (diagnostics, events)
}

fn captured(events: &EventCapture) -> Vec<SensitiveDiagnosticEvent> {
    events.lock().unwrap().clone()
}

fn response(status: u16, headers: &[(&str, &str)], body: &str) -> String {
    let reason = match status {
        503 => "Service Unavailable",
        _ => "OK",
    };
    let mut out = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n",
        body.len()
    );
    for (key, value) in headers {
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
        out.push_str("\r\n");
    }
    out.push_str("\r\n");
    out.push_str(body);
    out
}

fn sequence_server(responses: Vec<String>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for response in responses {
            let Ok((mut stream, _addr)) = listener.accept() else {
                return;
            };
            let mut buf = [0_u8; 4096];
            if stream.read(&mut buf).is_err() {
                return;
            }
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    format!("http://{addr}")
}

fn unused_base_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{addr}")
}

fn config(base_url: impl Into<String>, diagnostics: SensitiveDiagnostics) -> ClientConfig {
    ClientConfig::new(API_KEY)
        .with_base_url(base_url)
        .with_retry(RetryPolicy::none())
        .with_sensitive_diagnostics(diagnostics)
}

fn retry_config(base_url: impl Into<String>, diagnostics: SensitiveDiagnostics) -> ClientConfig {
    ClientConfig::new(API_KEY)
        .with_base_url(base_url)
        .with_retry(
            RetryPolicy::default()
                .with_max_retries(1)
                .with_base_delay(Duration::from_millis(0))
                .with_jitter(false),
        )
        .with_sensitive_diagnostics(diagnostics)
}

fn send_message_options(diagnostics: SensitiveDiagnostics) -> RequestOptions {
    RequestOptions::new()
        .trace_label(TRACE_LABEL)
        .query("token", QUERY_SECRET)
        .header("x-sensitive", HEADER_SECRET)
        .sensitive_diagnostics(diagnostics)
}

fn send_message() -> SendMessage {
    SendMessage::new(CHAT_ID)
        .text(MESSAGE_TEXT)
        .idempotency_key(IDEMPOTENCY_SECRET)
}

fn assert_basic_request_response(events: &[SensitiveDiagnosticEvent], base_url: &str) {
    assert_eq!(events.len(), 2, "{events:#?}");
    let SensitiveDiagnosticEvent::Request(request) = &events[0] else {
        panic!("expected request event: {events:#?}");
    };
    assert_eq!(request.method, http::Method::GET);
    assert_eq!(request.url, format!("{base_url}/me"));
    assert_header(
        &request.headers,
        "authorization",
        &format!("Bearer {API_KEY}"),
    );

    let SensitiveDiagnosticEvent::Response(response) = &events[1] else {
        panic!("expected response event: {events:#?}");
    };
    assert_eq!(response.status, 200);
    assert_eq!(response.body.as_ref(), br#"{"valid":true}"#);
}

fn assert_send_message_snapshots(events: &[SensitiveDiagnosticEvent], base_url: &str) {
    assert_eq!(events.len(), 2, "{events:#?}");
    let SensitiveDiagnosticEvent::Request(request) = &events[0] else {
        panic!("expected request event: {events:#?}");
    };
    assert_eq!(request.method, http::Method::POST);
    assert_eq!(
        request.url,
        format!("{base_url}/chats/chat%20secret/messages?token={QUERY_SECRET}")
    );
    assert_eq!(request.trace_label.as_deref(), Some(TRACE_LABEL));
    assert_header(
        &request.headers,
        "authorization",
        &format!("Bearer {API_KEY}"),
    );
    assert_header(&request.headers, "idempotency-key", IDEMPOTENCY_SECRET);
    assert_header(&request.headers, "content-type", "application/json");
    assert_header(&request.headers, "x-sensitive", HEADER_SECRET);
    let body = request.body.as_ref().expect("request body");
    let body = std::str::from_utf8(body).unwrap();
    assert!(body.contains(MESSAGE_TEXT), "{body}");

    let SensitiveDiagnosticEvent::Response(response) = &events[1] else {
        panic!("expected response event: {events:#?}");
    };
    assert_eq!(response.method, http::Method::POST);
    assert_eq!(response.url, request.url);
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .headers
            .get("x-sensitive-response")
            .unwrap()
            .to_str()
            .unwrap(),
        RESPONSE_SECRET
    );
    assert_eq!(
        response.body.as_ref(),
        br#"{"message_id":"message-secret"}"#
    );
}

fn assert_retry_events(events: &[SensitiveDiagnosticEvent], base_url: &str) {
    assert_eq!(events.len(), 4, "{events:#?}");
    let expected = [
        ("request", 1, None),
        ("response", 1, Some(503)),
        ("request", 2, None),
        ("response", 2, Some(200)),
    ];
    for (event, (kind, attempt, status)) in events.iter().zip(expected) {
        match (kind, event) {
            ("request", SensitiveDiagnosticEvent::Request(snapshot)) => {
                assert_eq!(snapshot.url, format!("{base_url}/me"));
                assert_eq!(snapshot.attempt, attempt);
                assert_eq!(snapshot.max_retries, 1);
            }
            ("response", SensitiveDiagnosticEvent::Response(snapshot)) => {
                assert_eq!(snapshot.url, format!("{base_url}/me"));
                assert_eq!(snapshot.attempt, attempt);
                assert_eq!(snapshot.max_retries, 1);
                assert_eq!(Some(snapshot.status), status);
            }
            _ => panic!("unexpected retry event: {event:#?}"),
        }
    }
}

fn assert_transport_error_events(events: &[SensitiveDiagnosticEvent], err: &blooio::Error) {
    assert_eq!(events.len(), 2, "{events:#?}");
    let SensitiveDiagnosticEvent::Request(request) = &events[0] else {
        panic!("expected request event: {events:#?}");
    };
    assert!(request.url.contains("127.0.0.1"), "{request:#?}");

    let SensitiveDiagnosticEvent::TransportError(error) = &events[1] else {
        panic!("expected transport error event: {events:#?}");
    };
    assert_eq!(error.stage, SensitiveTransportErrorStage::Send);
    assert_eq!(error.request.url, request.url);
    assert!(!error.error.is_empty(), "{error:#?}");
    assert!(!error.error.contains("[REDACTED URL]"), "{error:#?}");

    let message = err.to_string();
    assert!(!message.contains("127.0.0.1"), "{message}");
}

fn assert_build_error_events(events: &[SensitiveDiagnosticEvent], err: &blooio::Error) {
    assert_eq!(events.len(), 1, "{events:#?}");
    let SensitiveDiagnosticEvent::TransportError(error) = &events[0] else {
        panic!("expected transport error event: {events:#?}");
    };
    assert_eq!(error.stage, SensitiveTransportErrorStage::BuildRequest);
    assert!(error.request.url.ends_with("/me"), "{error:#?}");
    assert_header(&error.request.headers, "x-invalid", "bad-secret\nvalue");
    assert!(!error.error.is_empty(), "{error:#?}");

    let message = err.to_string();
    assert!(!message.contains("bad-secret"), "{message}");
}

fn assert_header(headers: &[(String, String)], name: &str, expected: &str) {
    let Some((_, value)) = headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
    else {
        panic!("missing header {name}: {headers:#?}");
    };
    assert_eq!(value, expected);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_client_default_sink_captures_request_and_response() {
    let base_url = sequence_server(vec![response(200, &[], r#"{"valid":true}"#)]);
    let (diagnostics, events) = capture();
    let client = Client::from_config(config(base_url.clone(), diagnostics)).unwrap();

    let _me = client.account().get().await.unwrap();

    assert_basic_request_response(&captured(&events), &base_url);
}

#[cfg(feature = "sync")]
#[test]
fn blocking_client_default_sink_captures_request_and_response() {
    let base_url = sequence_server(vec![response(200, &[], r#"{"valid":true}"#)]);
    let (diagnostics, events) = capture();
    let client = BlockingClient::from_config(config(base_url.clone(), diagnostics)).unwrap();

    let _me = client.account().get().unwrap();

    assert_basic_request_response(&captured(&events), &base_url);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_per_request_sink_overrides_client_default_and_captures_sensitive_data() {
    let base_url = sequence_server(vec![response(
        200,
        &[("x-sensitive-response", RESPONSE_SECRET)],
        r#"{"message_id":"message-secret"}"#,
    )]);
    let (client_diagnostics, client_events) = capture();
    let (request_diagnostics, request_events) = capture();
    let client = Client::from_config(config(base_url.clone(), client_diagnostics)).unwrap();

    let _sent = client
        .send_with_options(send_message(), send_message_options(request_diagnostics))
        .await
        .unwrap();

    assert!(captured(&client_events).is_empty());
    assert_send_message_snapshots(&captured(&request_events), &base_url);
}

#[cfg(feature = "sync")]
#[test]
fn blocking_per_request_sink_overrides_client_default_and_captures_sensitive_data() {
    let base_url = sequence_server(vec![response(
        200,
        &[("x-sensitive-response", RESPONSE_SECRET)],
        r#"{"message_id":"message-secret"}"#,
    )]);
    let (client_diagnostics, client_events) = capture();
    let (request_diagnostics, request_events) = capture();
    let client = BlockingClient::from_config(config(base_url.clone(), client_diagnostics)).unwrap();

    let _sent = client
        .send_with_options(send_message(), send_message_options(request_diagnostics))
        .unwrap();

    assert!(captured(&client_events).is_empty());
    assert_send_message_snapshots(&captured(&request_events), &base_url);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_noop_request_override_disables_client_default() {
    let base_url = sequence_server(vec![response(200, &[], r#"{"valid":true}"#)]);
    let (diagnostics, events) = capture();
    let client = Client::from_config(config(base_url, diagnostics)).unwrap();

    let _me = client
        .send_with_options(
            blooio::resources::account::GetMe,
            RequestOptions::new().sensitive_diagnostics(SensitiveDiagnostics::noop()),
        )
        .await
        .unwrap();

    assert!(captured(&events).is_empty());
}

#[cfg(feature = "sync")]
#[test]
fn blocking_noop_request_override_disables_client_default() {
    let base_url = sequence_server(vec![response(200, &[], r#"{"valid":true}"#)]);
    let (diagnostics, events) = capture();
    let client = BlockingClient::from_config(config(base_url, diagnostics)).unwrap();

    let _me = client
        .send_with_options(
            blooio::resources::account::GetMe,
            RequestOptions::new().sensitive_diagnostics(SensitiveDiagnostics::noop()),
        )
        .unwrap();

    assert!(captured(&events).is_empty());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_transport_failure_emits_raw_sensitive_event_but_returns_scrubbed_error() {
    let base_url = unused_base_url();
    let (diagnostics, events) = capture();
    let client = Client::from_config(config(base_url, diagnostics)).unwrap();

    let err = client.account().get().await.unwrap_err();

    assert_transport_error_events(&captured(&events), &err);
}

#[cfg(feature = "sync")]
#[test]
fn blocking_transport_failure_emits_raw_sensitive_event_but_returns_scrubbed_error() {
    let base_url = unused_base_url();
    let (diagnostics, events) = capture();
    let client = BlockingClient::from_config(config(base_url, diagnostics)).unwrap();

    let err = client.account().get().unwrap_err();

    assert_transport_error_events(&captured(&events), &err);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_build_failure_emits_build_request_without_request_event() {
    let base_url = unused_base_url();
    let (diagnostics, events) = capture();
    let client = Client::from_config(config(base_url, diagnostics)).unwrap();

    let err = client
        .send_with_options(
            blooio::resources::account::GetMe,
            RequestOptions::new().header("x-invalid", "bad-secret\nvalue"),
        )
        .await
        .unwrap_err();

    assert_build_error_events(&captured(&events), &err);
}

#[cfg(feature = "sync")]
#[test]
fn blocking_build_failure_emits_build_request_without_request_event() {
    let base_url = unused_base_url();
    let (diagnostics, events) = capture();
    let client = BlockingClient::from_config(config(base_url, diagnostics)).unwrap();

    let err = client
        .send_with_options(
            blooio::resources::account::GetMe,
            RequestOptions::new().header("x-invalid", "bad-secret\nvalue"),
        )
        .unwrap_err();

    assert_build_error_events(&captured(&events), &err);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_retries_emit_request_and_response_for_each_attempt() {
    let base_url = sequence_server(vec![
        response(
            503,
            &[("retry-after", "0")],
            r#"{"error":"unavailable","code":"temporarily_unavailable"}"#,
        ),
        response(200, &[], r#"{"valid":true}"#),
    ]);
    let (diagnostics, events) = capture();
    let client = Client::from_config(retry_config(base_url.clone(), diagnostics)).unwrap();

    let _me = client.account().get().await.unwrap();

    assert_retry_events(&captured(&events), &base_url);
}

#[cfg(feature = "sync")]
#[test]
fn blocking_retries_emit_request_and_response_for_each_attempt() {
    let base_url = sequence_server(vec![
        response(
            503,
            &[("retry-after", "0")],
            r#"{"error":"unavailable","code":"temporarily_unavailable"}"#,
        ),
        response(200, &[], r#"{"valid":true}"#),
    ]);
    let (diagnostics, events) = capture();
    let client = BlockingClient::from_config(retry_config(base_url.clone(), diagnostics)).unwrap();

    let _me = client.account().get().unwrap();

    assert_retry_events(&captured(&events), &base_url);
}
