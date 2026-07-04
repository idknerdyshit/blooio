//! Structured tracing contract tests for the HTTP client executors.

#![cfg(all(feature = "tracing", any(feature = "async", feature = "sync")))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal,
    clippy::unwrap_in_result
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
use blooio::{ClientConfig, RequestOptions, RetryPolicy};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Record};
use tracing::{Event, Id, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;

const TRACE_TARGET: &str = "blooio::trace";
const SECRET_KEY: &str = "sk-structured-tracing-secret";
const SENSITIVE_CHAT_ID: &str = "chat-secret-structured";
const SENSITIVE_BODY: &str = "structured secret body";
const SENSITIVE_HEADER_VALUE: &str = "structured-secret-header";
const SENSITIVE_QUERY_VALUE: &str = "structured-secret-query";
const SAFE_TRACE_LABEL: &str = "job-42";

#[derive(Clone, Debug)]
struct CapturedEvent {
    level: String,
    fields: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
struct CapturedSpan {
    name: String,
    fields: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default)]
struct TraceCapture {
    events: Arc<Mutex<Vec<CapturedEvent>>>,
    spans: Arc<Mutex<Vec<CapturedSpan>>>,
}

impl TraceCapture {
    fn events_named(&self, name: &str) -> Vec<CapturedEvent> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.field("event") == name)
            .cloned()
            .collect()
    }

    fn one_event(&self, name: &str) -> CapturedEvent {
        let events = self.events_named(name);
        assert_eq!(events.len(), 1, "expected exactly one {name}: {events:#?}");
        events.into_iter().next().unwrap()
    }

    fn spans_named(&self, name: &str) -> Vec<CapturedSpan> {
        self.spans
            .lock()
            .unwrap()
            .iter()
            .filter(|span| span.name == name)
            .cloned()
            .collect()
    }

    fn dump(&self) -> String {
        format!(
            "events={:#?}\nspans={:#?}",
            self.events.lock().unwrap(),
            self.spans.lock().unwrap()
        )
    }
}

impl CapturedEvent {
    fn field(&self, key: &str) -> &str {
        self.fields.get(key).map_or_else(
            || panic!("missing event field {key}: {self:#?}"),
            String::as_str,
        )
    }

    fn assert_level(&self, level: &str) {
        assert_eq!(self.level, level, "{self:#?}");
    }

    fn assert_elapsed(&self) {
        self.field("elapsed_ms").parse::<u64>().unwrap();
    }
}

impl CapturedSpan {
    fn field(&self, key: &str) -> &str {
        self.fields.get(key).map_or_else(
            || panic!("missing span field {key}: {self:#?}"),
            String::as_str,
        )
    }

    fn assert_elapsed(&self) {
        self.field("elapsed_ms").parse::<u64>().unwrap();
    }
}

struct CaptureLayer {
    capture: TraceCapture,
}

#[derive(Clone, Debug)]
struct SpanState {
    name: String,
    target: String,
    fields: BTreeMap<String, String>,
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

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.fields
            .insert(field.name().to_owned(), value.to_string());
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.fields
            .insert(field.name().to_owned(), value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
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

impl<S> Layer<S> for CaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if attrs.metadata().target() != TRACE_TARGET {
            return;
        }

        let mut visitor = FieldVisitor::default();
        attrs.record(&mut visitor);
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(SpanState {
                name: attrs.metadata().name().to_owned(),
                target: attrs.metadata().target().to_owned(),
                fields: visitor.fields,
            });
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else {
            return;
        };
        let mut extensions = span.extensions_mut();
        let Some(state) = extensions.get_mut::<SpanState>() else {
            return;
        };

        let mut visitor = FieldVisitor::default();
        values.record(&mut visitor);
        state.fields.extend(visitor.fields);
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if event.metadata().target() != TRACE_TARGET {
            return;
        }

        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        self.capture.events.lock().unwrap().push(CapturedEvent {
            level: event.metadata().level().to_string(),
            fields: visitor.fields,
        });
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else {
            return;
        };
        let extensions = span.extensions();
        let Some(state) = extensions.get::<SpanState>() else {
            return;
        };
        if state.target == TRACE_TARGET {
            self.capture.spans.lock().unwrap().push(CapturedSpan {
                name: state.name.clone(),
                fields: state.fields.clone(),
            });
        }
    }
}

fn capture_traces() -> (TraceCapture, impl Drop) {
    let capture = TraceCapture::default();
    let subscriber = tracing_subscriber::registry().with(CaptureLayer {
        capture: capture.clone(),
    });
    let guard = tracing::subscriber::set_default(subscriber);
    (capture, guard)
}

fn response(status: u16, headers: &[(&str, &str)], body: &str) -> String {
    let reason = match status {
        400 => "Bad Request",
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

fn no_retry_config(base_url: impl Into<String>) -> ClientConfig {
    ClientConfig::new(SECRET_KEY)
        .with_base_url(base_url)
        .with_retry(RetryPolicy::none())
}

fn one_retry_config(base_url: impl Into<String>) -> ClientConfig {
    ClientConfig::new(SECRET_KEY)
        .with_base_url(base_url)
        .with_retry(
            RetryPolicy::default()
                .with_max_retries(1)
                .with_base_delay(Duration::from_millis(0))
                .with_jitter(false),
        )
}

fn assert_common_attempt(event: &CapturedEvent, method: &str, attempt: &str, max_retries: &str) {
    assert_eq!(event.field("method"), method);
    assert_eq!(event.field("attempt"), attempt);
    assert_eq!(event.field("max_retries"), max_retries);
    assert!(event.field("operation").starts_with("blooio::resources::"));
    event.assert_elapsed();
}

fn assert_success_capture(capture: &TraceCapture, method: &str) {
    let attempt = capture.one_event("blooio.request.attempt.response");
    attempt.assert_level("DEBUG");
    assert_common_attempt(&attempt, method, "1", "0");
    assert_eq!(attempt.field("status"), "200");
    assert!(!attempt.fields.contains_key("trace_label"));

    let success = capture.one_event("blooio.operation.success");
    success.assert_level("DEBUG");
    assert_eq!(success.field("method"), method);
    assert_eq!(success.field("attempts"), "1");
    assert_eq!(success.field("max_retries"), "0");
    assert_eq!(success.field("status"), "200");
    assert!(
        success
            .field("operation")
            .starts_with("blooio::resources::")
    );
    success.assert_elapsed();
    assert!(!success.fields.contains_key("trace_label"));

    let spans = capture.spans_named("blooio.request");
    assert_eq!(spans.len(), 1, "{spans:#?}");
    let span = &spans[0];
    assert_eq!(span.field("method"), method);
    assert_eq!(span.field("attempt"), "1");
    assert_eq!(span.field("max_retries"), "0");
    assert_eq!(span.field("status"), "200");
    span.assert_elapsed();
    assert!(!span.fields.contains_key("trace_label"));
}

fn assert_trace_label_capture(capture: &TraceCapture, method: &str) {
    let attempt = capture.one_event("blooio.request.attempt.response");
    attempt.assert_level("DEBUG");
    assert_common_attempt(&attempt, method, "1", "0");
    assert_eq!(attempt.field("trace_label"), SAFE_TRACE_LABEL);

    let success = capture.one_event("blooio.operation.success");
    success.assert_level("DEBUG");
    assert_eq!(success.field("method"), method);
    assert_eq!(success.field("trace_label"), SAFE_TRACE_LABEL);

    let spans = capture.spans_named("blooio.request");
    assert_eq!(spans.len(), 1, "{spans:#?}");
    assert_eq!(spans[0].field("trace_label"), SAFE_TRACE_LABEL);
}

fn assert_api_failure_capture(capture: &TraceCapture) {
    let attempt = capture.one_event("blooio.request.attempt.response");
    attempt.assert_level("DEBUG");
    assert_common_attempt(&attempt, "GET", "1", "0");
    assert_eq!(attempt.field("status"), "400");

    let failure = capture.one_event("blooio.operation.failure");
    failure.assert_level("WARN");
    assert_eq!(failure.field("method"), "GET");
    assert_eq!(failure.field("attempts"), "1");
    assert_eq!(failure.field("max_retries"), "0");
    assert_eq!(failure.field("status"), "400");
    assert_eq!(failure.field("code"), "invalid_request");
    assert_eq!(failure.field("error_kind"), "api");
    failure.assert_elapsed();
}

fn assert_retry_success_capture(capture: &TraceCapture) {
    let responses = capture.events_named("blooio.request.attempt.response");
    assert_eq!(responses.len(), 2, "{responses:#?}");
    assert_common_attempt(&responses[0], "GET", "1", "1");
    assert_eq!(responses[0].field("status"), "503");
    assert_common_attempt(&responses[1], "GET", "2", "1");
    assert_eq!(responses[1].field("status"), "200");

    let retry = capture.one_event("blooio.request.retry");
    retry.assert_level("WARN");
    assert_eq!(retry.field("method"), "GET");
    assert_eq!(retry.field("attempt"), "1");
    assert_eq!(retry.field("next_attempt"), "2");
    assert_eq!(retry.field("max_retries"), "1");
    assert_eq!(retry.field("delay_ms"), "0");
    assert_eq!(retry.field("delay_source"), "retry_after");
    assert_eq!(retry.field("error_kind"), "api");
    assert_eq!(retry.field("status"), "503");
    assert_eq!(retry.field("code"), "temporarily_unavailable");
    assert_eq!(retry.field("retry_after_ms"), "0");

    let success = capture.one_event("blooio.operation.success");
    assert_eq!(success.field("method"), "GET");
    assert_eq!(success.field("attempts"), "2");
    assert_eq!(success.field("max_retries"), "1");
    assert_eq!(success.field("status"), "200");
    success.assert_elapsed();
}

fn assert_transport_failure_capture(capture: &TraceCapture, forbidden_base_url: &str) {
    let attempts = capture.events_named("blooio.request.attempt.error");
    assert_eq!(attempts.len(), 2, "{attempts:#?}");
    for (i, event) in attempts.iter().enumerate() {
        event.assert_level("WARN");
        assert_common_attempt(event, "POST", &(i + 1).to_string(), "1");
        assert_eq!(event.field("error_kind"), "transport");
        assert_eq!(event.field("trace_label"), SAFE_TRACE_LABEL);
    }

    let retry = capture.one_event("blooio.request.retry");
    assert_eq!(retry.field("delay_source"), "backoff");
    assert_eq!(retry.field("error_kind"), "transport");
    assert_eq!(retry.field("attempt"), "1");
    assert_eq!(retry.field("next_attempt"), "2");
    assert!(!retry.fields.contains_key("status"));
    assert!(!retry.fields.contains_key("code"));
    assert_eq!(retry.field("trace_label"), SAFE_TRACE_LABEL);

    let failure = capture.one_event("blooio.operation.failure");
    failure.assert_level("WARN");
    assert_eq!(failure.field("method"), "POST");
    assert_eq!(failure.field("attempts"), "2");
    assert_eq!(failure.field("max_retries"), "1");
    assert_eq!(failure.field("error_kind"), "transport");
    assert!(!failure.fields.contains_key("status"));
    assert!(!failure.fields.contains_key("code"));
    assert_eq!(failure.field("trace_label"), SAFE_TRACE_LABEL);
    failure.assert_elapsed();

    let spans = capture.spans_named("blooio.request");
    assert_eq!(spans.len(), 2, "{spans:#?}");
    for span in spans {
        assert_eq!(span.field("trace_label"), SAFE_TRACE_LABEL);
    }

    assert_redacted(capture, &[forbidden_base_url]);
}

fn assert_redacted(capture: &TraceCapture, extra_forbidden: &[&str]) {
    let dump = capture.dump();
    for forbidden in [
        SECRET_KEY,
        "Bearer",
        "authorization",
        "Idempotency-Key",
        "idempotency-key",
        SENSITIVE_CHAT_ID,
        SENSITIVE_BODY,
        SENSITIVE_HEADER_VALUE,
        SENSITIVE_QUERY_VALUE,
    ] {
        assert!(
            !dump.contains(forbidden),
            "trace leaked {forbidden:?}:\n{dump}"
        );
    }
    for forbidden in extra_forbidden {
        assert!(
            !dump.contains(forbidden),
            "trace leaked {forbidden:?}:\n{dump}"
        );
    }
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_success_emits_attempt_and_operation_success() {
    let base_url = sequence_server(vec![response(200, &[], r#"{"valid":true}"#)]);
    let (capture, _guard) = capture_traces();

    let client = Client::from_config(no_retry_config(base_url.clone())).unwrap();
    let _me = client.account().get().await.unwrap();

    assert_success_capture(&capture, "GET");
    assert_redacted(&capture, &[base_url.as_str()]);
}

#[cfg(feature = "sync")]
#[test]
fn blocking_success_emits_attempt_and_operation_success() {
    let base_url = sequence_server(vec![response(200, &[], r#"{"valid":true}"#)]);
    let (capture, _guard) = capture_traces();

    let client = BlockingClient::from_config(no_retry_config(base_url.clone())).unwrap();
    let _me = client.account().get().unwrap();

    assert_success_capture(&capture, "GET");
    assert_redacted(&capture, &[base_url.as_str()]);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_trace_label_is_emitted_when_set() {
    let base_url = sequence_server(vec![response(200, &[], r#"{"valid":true}"#)]);
    let (capture, _guard) = capture_traces();

    let client = Client::from_config(no_retry_config(base_url.clone())).unwrap();
    let _me = client
        .send_with_options(
            blooio::resources::account::GetMe,
            RequestOptions::new().trace_label(SAFE_TRACE_LABEL),
        )
        .await
        .unwrap();

    assert_trace_label_capture(&capture, "GET");
    assert_redacted(&capture, &[base_url.as_str()]);
}

#[cfg(feature = "sync")]
#[test]
fn blocking_trace_label_is_emitted_when_set() {
    let base_url = sequence_server(vec![response(200, &[], r#"{"valid":true}"#)]);
    let (capture, _guard) = capture_traces();

    let client = BlockingClient::from_config(no_retry_config(base_url.clone())).unwrap();
    let _me = client
        .send_with_options(
            blooio::resources::account::GetMe,
            RequestOptions::new().trace_label(SAFE_TRACE_LABEL),
        )
        .unwrap();

    assert_trace_label_capture(&capture, "GET");
    assert_redacted(&capture, &[base_url.as_str()]);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_api_failure_emits_attempt_response_and_operation_failure() {
    let base_url = sequence_server(vec![response(
        400,
        &[],
        r#"{"error":"bad_request","code":"invalid_request","message":"contains sk-structured-tracing-secret"}"#,
    )]);
    let (capture, _guard) = capture_traces();

    let client = Client::from_config(no_retry_config(base_url.clone())).unwrap();
    let _err = client.account().get().await.unwrap_err();

    assert_api_failure_capture(&capture);
    assert_redacted(&capture, &[base_url.as_str(), "contains sk-structured"]);
}

#[cfg(feature = "sync")]
#[test]
fn blocking_api_failure_emits_attempt_response_and_operation_failure() {
    let base_url = sequence_server(vec![response(
        400,
        &[],
        r#"{"error":"bad_request","code":"invalid_request","message":"contains sk-structured-tracing-secret"}"#,
    )]);
    let (capture, _guard) = capture_traces();

    let client = BlockingClient::from_config(no_retry_config(base_url.clone())).unwrap();
    let _err = client.account().get().unwrap_err();

    assert_api_failure_capture(&capture);
    assert_redacted(&capture, &[base_url.as_str(), "contains sk-structured"]);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_retry_event_links_transient_failure_to_success() {
    let base_url = sequence_server(vec![
        response(
            503,
            &[("retry-after", "0")],
            r#"{"error":"unavailable","code":"temporarily_unavailable"}"#,
        ),
        response(200, &[], r#"{"valid":true}"#),
    ]);
    let (capture, _guard) = capture_traces();

    let client = Client::from_config(one_retry_config(base_url.clone())).unwrap();
    let _me = client.account().get().await.unwrap();

    assert_retry_success_capture(&capture);
    assert_redacted(&capture, &[base_url.as_str()]);
}

#[cfg(feature = "sync")]
#[test]
fn blocking_retry_event_links_transient_failure_to_success() {
    let base_url = sequence_server(vec![
        response(
            503,
            &[("retry-after", "0")],
            r#"{"error":"unavailable","code":"temporarily_unavailable"}"#,
        ),
        response(200, &[], r#"{"valid":true}"#),
    ]);
    let (capture, _guard) = capture_traces();

    let client = BlockingClient::from_config(one_retry_config(base_url.clone())).unwrap();
    let _me = client.account().get().unwrap();

    assert_retry_success_capture(&capture);
    assert_redacted(&capture, &[base_url.as_str()]);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_transport_failure_is_redacted_and_structured() {
    let base_url = unused_base_url();
    let (capture, _guard) = capture_traces();

    let client = Client::from_config(one_retry_config(base_url.clone())).unwrap();
    let _err = client
        .send_with_options(
            blooio::resources::chats::SendMessage::new(SENSITIVE_CHAT_ID).text(SENSITIVE_BODY),
            blooio::RequestOptions::new()
                .trace_label(SAFE_TRACE_LABEL)
                .header("x-sensitive", SENSITIVE_HEADER_VALUE)
                .query("token", SENSITIVE_QUERY_VALUE),
        )
        .await
        .unwrap_err();

    assert_transport_failure_capture(&capture, base_url.as_str());
}

#[cfg(feature = "sync")]
#[test]
fn blocking_transport_failure_is_redacted_and_structured() {
    let base_url = unused_base_url();
    let (capture, _guard) = capture_traces();

    let client = BlockingClient::from_config(one_retry_config(base_url.clone())).unwrap();
    let _err = client
        .send_with_options(
            blooio::resources::chats::SendMessage::new(SENSITIVE_CHAT_ID).text(SENSITIVE_BODY),
            blooio::RequestOptions::new()
                .trace_label(SAFE_TRACE_LABEL)
                .header("x-sensitive", SENSITIVE_HEADER_VALUE)
                .query("token", SENSITIVE_QUERY_VALUE),
        )
        .unwrap_err();

    assert_transport_failure_capture(&capture, base_url.as_str());
}
