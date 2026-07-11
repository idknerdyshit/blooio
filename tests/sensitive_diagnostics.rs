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

use std::collections::BTreeMap;
use std::fmt;
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
    BlooioCreds, ClientConfig, RequestOptions, RetryPolicy, SensitiveDiagnosticEvent,
    SensitiveDiagnostics, SensitiveTransportErrorStage,
};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;

const API_KEY: &str = "test-key-sensitive-diagnostics";
const TRACE_LABEL: &str = "trace-sensitive-label";
const CHAT_ID: &str = "chat secret";
const MESSAGE_TEXT: &str = "body secret";
const QUERY_SECRET: &str = "query-secret";
const HEADER_SECRET: &str = "header-secret";
const IDEMPOTENCY_SECRET: &str = "idem-secret";
const RESPONSE_SECRET: &str = "response-secret";
const SENSITIVE_TRACE_TARGET: &str = "blooio::sensitive";

type EventCapture = Arc<Mutex<Vec<SensitiveDiagnosticEvent>>>;

#[derive(Clone, Debug)]
struct CapturedTraceEvent {
    level: String,
    fields: BTreeMap<String, String>,
}

impl CapturedTraceEvent {
    fn field(&self, key: &str) -> &str {
        self.fields.get(key).map_or_else(
            || panic!("missing trace field {key}: {self:#?}"),
            String::as_str,
        )
    }
}

#[derive(Clone, Default)]
struct SensitiveTraceCapture {
    events: Arc<Mutex<Vec<CapturedTraceEvent>>>,
}

impl SensitiveTraceCapture {
    fn events_named(&self, name: &str) -> Vec<CapturedTraceEvent> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.field("event") == name)
            .cloned()
            .collect()
    }
}

#[derive(Default)]
struct FieldVisitor {
    fields: BTreeMap<String, String>,
}

impl Visit for FieldVisitor {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_owned(), value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_owned(), value.to_string());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .insert(field.name().to_owned(), value.to_owned());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.fields
            .insert(field.name().to_owned(), format!("{value:?}"));
    }
}

struct SensitiveTraceLayer {
    capture: SensitiveTraceCapture,
}

impl<S> Layer<S> for SensitiveTraceLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if event.metadata().target() != SENSITIVE_TRACE_TARGET {
            return;
        }

        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        self.capture
            .events
            .lock()
            .unwrap()
            .push(CapturedTraceEvent {
                level: event.metadata().level().to_string(),
                fields: visitor.fields,
            });
    }
}

fn capture_sensitive_traces() -> (SensitiveTraceCapture, impl Drop) {
    let capture = SensitiveTraceCapture::default();
    let subscriber = tracing_subscriber::registry().with(SensitiveTraceLayer {
        capture: capture.clone(),
    });
    let guard = tracing::subscriber::set_default(subscriber);
    (capture, guard)
}

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
    ClientConfig::new()
        .with_base_url(base_url)
        .with_retry(RetryPolicy::none())
        .with_sensitive_diagnostics(diagnostics)
}

fn retry_config(base_url: impl Into<String>, diagnostics: SensitiveDiagnostics) -> ClientConfig {
    ClientConfig::new()
        .with_base_url(base_url)
        .with_retry(
            RetryPolicy::default()
                .with_max_retries(1)
                .with_base_delay(Duration::from_millis(0))
                .with_jitter(false),
        )
        .with_sensitive_diagnostics(diagnostics)
}

fn tracing_config(base_url: impl Into<String>, enabled: bool) -> ClientConfig {
    let config = ClientConfig::new()
        .with_base_url(base_url)
        .with_retry(RetryPolicy::none());
    if enabled {
        config.with_sensitive_tracing()
    } else {
        config
    }
}

fn test_creds() -> BlooioCreds {
    BlooioCreds::new(API_KEY)
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
    assert!(matches!(err, blooio::Error::RequestBuild));
    assert!(!err.is_retryable());
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

fn assert_trace_request_response(events: &[CapturedTraceEvent], base_url: &str) {
    assert_eq!(events.len(), 2, "{events:#?}");
    let request = &events[0];
    assert_eq!(request.level, "DEBUG");
    assert_eq!(request.field("event"), "blooio.sensitive.request");
    assert_eq!(request.field("method"), "POST");
    assert_eq!(request.field("attempt"), "1");
    assert_eq!(request.field("max_retries"), "0");
    assert!(request.field("operation").contains("SendMessage"));
    assert!(request.field("url").contains(base_url));
    assert!(request.field("url").contains(QUERY_SECRET));
    assert!(request.field("headers").contains(API_KEY));
    assert!(request.field("headers").contains(HEADER_SECRET));
    assert!(request.field("body").contains(MESSAGE_TEXT));
    assert!(request.field("trace_label").contains(TRACE_LABEL));

    let response = &events[1];
    assert_eq!(response.level, "DEBUG");
    assert_eq!(response.field("event"), "blooio.sensitive.response");
    assert_eq!(response.field("status"), "200");
    assert!(response.field("url").contains(base_url));
    assert!(response.field("body").contains("message-secret"));
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_client_default_sink_captures_request_and_response() {
    let base_url = sequence_server(vec![response(200, &[], r#"{"valid":true}"#)]);
    let (diagnostics, events) = capture();
    let client = Client::from_config(config(base_url.clone(), diagnostics)).unwrap();
    let creds = test_creds();

    let _me = client.account(&creds).me().get().await.unwrap();

    assert_basic_request_response(&captured(&events), &base_url);
}

#[cfg(feature = "sync")]
#[test]
fn blocking_client_default_sink_captures_request_and_response() {
    let base_url = sequence_server(vec![response(200, &[], r#"{"valid":true}"#)]);
    let (diagnostics, events) = capture();
    let client = BlockingClient::from_config(config(base_url.clone(), diagnostics)).unwrap();
    let creds = test_creds();

    let _me = client.account(&creds).me().get().unwrap();

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
    let creds = test_creds();

    let _sent = client
        .account(&creds)
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
    let creds = test_creds();

    let _sent = client
        .account(&creds)
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
    let creds = test_creds();

    let _me = client
        .account(&creds)
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
    let creds = test_creds();

    let _me = client
        .account(&creds)
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
    let creds = test_creds();

    let err = client.account(&creds).me().get().await.unwrap_err();

    assert_transport_error_events(&captured(&events), &err);
}

#[cfg(feature = "sync")]
#[test]
fn blocking_transport_failure_emits_raw_sensitive_event_but_returns_scrubbed_error() {
    let base_url = unused_base_url();
    let (diagnostics, events) = capture();
    let client = BlockingClient::from_config(config(base_url, diagnostics)).unwrap();
    let creds = test_creds();

    let err = client.account(&creds).me().get().unwrap_err();

    assert_transport_error_events(&captured(&events), &err);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_build_failure_emits_build_request_without_request_event() {
    let base_url = unused_base_url();
    let (diagnostics, events) = capture();
    let client = Client::from_config(config(base_url, diagnostics)).unwrap();
    let creds = test_creds();

    let err = client
        .account(&creds)
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
    let creds = test_creds();

    let err = client
        .account(&creds)
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
    let creds = test_creds();

    let _me = client.account(&creds).me().get().await.unwrap();

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
    let creds = test_creds();

    let _me = client.account(&creds).me().get().unwrap();

    assert_retry_events(&captured(&events), &base_url);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_sensitive_tracing_respects_client_and_request_overrides() {
    let base_url = sequence_server(vec![
        response(
            200,
            &[("x-sensitive-response", RESPONSE_SECRET)],
            r#"{"message_id":"message-secret"}"#,
        ),
        response(
            200,
            &[("x-sensitive-response", RESPONSE_SECRET)],
            r#"{"message_id":"message-secret"}"#,
        ),
        response(
            200,
            &[("x-sensitive-response", RESPONSE_SECRET)],
            r#"{"message_id":"message-secret"}"#,
        ),
        response(
            200,
            &[("x-sensitive-response", RESPONSE_SECRET)],
            r#"{"message_id":"message-secret"}"#,
        ),
    ]);
    let enabled = Client::from_config(tracing_config(base_url.clone(), true)).unwrap();
    let disabled = Client::from_config(tracing_config(base_url.clone(), false)).unwrap();
    let creds = test_creds();
    let (capture, _guard) = capture_sensitive_traces();

    let _sent = disabled
        .account(&creds)
        .send_with_options(
            send_message(),
            send_message_options(SensitiveDiagnostics::noop()),
        )
        .await
        .unwrap();
    assert!(capture.events.lock().unwrap().is_empty());
    let _sent = enabled
        .account(&creds)
        .send_with_options(
            send_message(),
            send_message_options(SensitiveDiagnostics::noop()).without_sensitive_tracing(),
        )
        .await
        .unwrap();
    assert!(capture.events.lock().unwrap().is_empty());
    let _sent = enabled
        .account(&creds)
        .send_with_options(
            send_message(),
            send_message_options(SensitiveDiagnostics::noop()),
        )
        .await
        .unwrap();
    let _sent = disabled
        .account(&creds)
        .send_with_options(
            send_message(),
            send_message_options(SensitiveDiagnostics::noop()).sensitive_tracing(),
        )
        .await
        .unwrap();

    let events = capture.events.lock().unwrap().clone();
    assert_trace_request_response(&events[..2], &base_url);
    assert_trace_request_response(&events[2..], &base_url);
}

#[cfg(feature = "sync")]
#[test]
fn blocking_sensitive_tracing_respects_client_and_request_overrides() {
    let base_url = sequence_server(vec![
        response(
            200,
            &[("x-sensitive-response", RESPONSE_SECRET)],
            r#"{"message_id":"message-secret"}"#,
        ),
        response(
            200,
            &[("x-sensitive-response", RESPONSE_SECRET)],
            r#"{"message_id":"message-secret"}"#,
        ),
        response(
            200,
            &[("x-sensitive-response", RESPONSE_SECRET)],
            r#"{"message_id":"message-secret"}"#,
        ),
        response(
            200,
            &[("x-sensitive-response", RESPONSE_SECRET)],
            r#"{"message_id":"message-secret"}"#,
        ),
    ]);
    let enabled = BlockingClient::from_config(tracing_config(base_url.clone(), true)).unwrap();
    let disabled = BlockingClient::from_config(tracing_config(base_url.clone(), false)).unwrap();
    let creds = test_creds();
    let (capture, _guard) = capture_sensitive_traces();

    let _sent = disabled
        .account(&creds)
        .send_with_options(
            send_message(),
            send_message_options(SensitiveDiagnostics::noop()),
        )
        .unwrap();
    assert!(capture.events.lock().unwrap().is_empty());
    let _sent = enabled
        .account(&creds)
        .send_with_options(
            send_message(),
            send_message_options(SensitiveDiagnostics::noop()).without_sensitive_tracing(),
        )
        .unwrap();
    assert!(capture.events.lock().unwrap().is_empty());
    let _sent = enabled
        .account(&creds)
        .send_with_options(
            send_message(),
            send_message_options(SensitiveDiagnostics::noop()),
        )
        .unwrap();
    let _sent = disabled
        .account(&creds)
        .send_with_options(
            send_message(),
            send_message_options(SensitiveDiagnostics::noop()).sensitive_tracing(),
        )
        .unwrap();

    let events = capture.events.lock().unwrap().clone();
    assert_trace_request_response(&events[..2], &base_url);
    assert_trace_request_response(&events[2..], &base_url);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_sensitive_tracing_and_callback_diagnostics_are_independent() {
    let base_url = sequence_server(vec![response(200, &[], r#"{"valid":true}"#)]);
    let (diagnostics, callback_events) = capture();
    let client =
        Client::from_config(tracing_config(base_url, true).with_sensitive_diagnostics(diagnostics))
            .unwrap();
    let creds = test_creds();
    let (trace_events, _guard) = capture_sensitive_traces();

    let _me = client.account(&creds).me().get().await.unwrap();

    assert_eq!(captured(&callback_events).len(), 2);
    assert_eq!(
        trace_events.events_named("blooio.sensitive.request").len(),
        1
    );
    assert_eq!(
        trace_events.events_named("blooio.sensitive.response").len(),
        1
    );
}

#[cfg(feature = "sync")]
#[test]
fn blocking_sensitive_tracing_and_callback_diagnostics_are_independent() {
    let base_url = sequence_server(vec![response(200, &[], r#"{"valid":true}"#)]);
    let (diagnostics, callback_events) = capture();
    let client = BlockingClient::from_config(
        tracing_config(base_url, true).with_sensitive_diagnostics(diagnostics),
    )
    .unwrap();
    let creds = test_creds();
    let (trace_events, _guard) = capture_sensitive_traces();

    let _me = client.account(&creds).me().get().unwrap();

    assert_eq!(captured(&callback_events).len(), 2);
    assert_eq!(
        trace_events.events_named("blooio.sensitive.request").len(),
        1
    );
    assert_eq!(
        trace_events.events_named("blooio.sensitive.response").len(),
        1
    );
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_sensitive_tracing_captures_retries_and_transport_errors() {
    let base_url = sequence_server(vec![
        response(503, &[], r#"{"error":"retry response-secret"}"#),
        response(200, &[], r#"{"valid":true}"#),
    ]);
    let client = Client::from_config(
        ClientConfig::new()
            .with_base_url(base_url.clone())
            .with_retry(
                RetryPolicy::default()
                    .with_max_retries(1)
                    .with_base_delay(Duration::from_millis(0))
                    .with_jitter(false),
            )
            .with_sensitive_tracing(),
    )
    .unwrap();
    let creds = test_creds();
    let (capture, guard) = capture_sensitive_traces();

    let _me = client.account(&creds).me().get().await.unwrap();

    let responses = capture.events_named("blooio.sensitive.response");
    assert_eq!(responses.len(), 2, "{responses:#?}");
    assert_eq!(responses[0].field("status"), "503");
    assert_eq!(responses[1].field("status"), "200");
    assert!(responses[0].field("body").contains(RESPONSE_SECRET));

    drop(guard);
    let failed_base_url = unused_base_url();
    let failed_client = Client::from_config(tracing_config(failed_base_url.clone(), true)).unwrap();
    let (capture, _guard) = capture_sensitive_traces();
    let _err = failed_client.account(&creds).me().get().await.unwrap_err();

    let errors = capture.events_named("blooio.sensitive.transport_error");
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(errors[0].level, "WARN");
    assert!(errors[0].field("url").contains(&failed_base_url));
    assert!(errors[0].field("headers").contains(API_KEY));
    assert!(!errors[0].field("error").is_empty());
}
