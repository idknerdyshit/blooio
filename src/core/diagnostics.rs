//! Explicitly sensitive request/response inspection hooks.
//!
//! This module is available only with the `sensitive-diagnostics` feature. It is
//! intended for local protocol debugging and can expose API keys, URLs, phone
//! numbers, message text, headers, and raw response bodies. Normal tracing,
//! public errors, and `Debug` output remain redacted. An explicit
//! sensitive-tracing opt-in can additionally emit these snapshots to a
//! dedicated tracing target without redaction.

use std::fmt;
use std::sync::Arc;

use bytes::Bytes;
use http::HeaderMap;
use http::header::{AUTHORIZATION, CONTENT_TYPE};

use crate::core::raw::RawResponse;
use crate::core::request::RequestSpec;

pub(crate) const SENSITIVE_TRACE_TARGET: &str = "blooio::sensitive";

pub(crate) fn emit_sensitive_trace(event: &SensitiveDiagnosticEvent) {
    match event {
        SensitiveDiagnosticEvent::Request(snapshot) => tracing::debug!(
            target: SENSITIVE_TRACE_TARGET,
            event = "blooio.sensitive.request",
            method = %snapshot.method,
            url = %snapshot.url,
            operation = snapshot.operation,
            attempt = snapshot.attempt,
            max_retries = snapshot.max_retries,
            trace_label = ?snapshot.trace_label,
            headers = ?snapshot.headers,
            body = ?snapshot.body,
        ),
        SensitiveDiagnosticEvent::Response(snapshot) => tracing::debug!(
            target: SENSITIVE_TRACE_TARGET,
            event = "blooio.sensitive.response",
            method = %snapshot.method,
            url = %snapshot.url,
            operation = snapshot.operation,
            attempt = snapshot.attempt,
            max_retries = snapshot.max_retries,
            trace_label = ?snapshot.trace_label,
            status = snapshot.status,
            headers = ?snapshot.headers,
            body = ?snapshot.body,
        ),
        SensitiveDiagnosticEvent::TransportError(snapshot) => {
            let request = &snapshot.request;
            tracing::warn!(
                target: SENSITIVE_TRACE_TARGET,
                event = "blooio.sensitive.transport_error",
                method = %request.method,
                url = %request.url,
                operation = request.operation,
                attempt = request.attempt,
                max_retries = request.max_retries,
                trace_label = ?request.trace_label,
                headers = ?request.headers,
                body = ?request.body,
                stage = ?snapshot.stage,
                error = %snapshot.error,
            );
        }
    }
}

/// A cloneable handle that dispatches sensitive diagnostic events to a caller
/// supplied sink.
///
/// Values of this type are redacted in [`Debug`](fmt::Debug). The events sent
/// to the sink are intentionally not redacted.
#[derive(Clone)]
pub struct SensitiveDiagnostics {
    sink: Arc<dyn SensitiveDiagnosticSink>,
}

impl SensitiveDiagnostics {
    /// Create diagnostics from a sink that receives every event.
    pub fn new(sink: impl SensitiveDiagnosticSink) -> Self {
        Self {
            sink: Arc::new(sink),
        }
    }

    /// Create a diagnostics handle that intentionally drops every event.
    ///
    /// Use this as a per-request override to disable a client-wide sensitive
    /// diagnostics sink for a single request.
    #[must_use]
    pub fn noop() -> Self {
        Self::new(NoopSink)
    }

    /// Start building diagnostics from separate event callbacks.
    #[must_use]
    pub fn builder() -> SensitiveDiagnosticsBuilder {
        SensitiveDiagnosticsBuilder::new()
    }

    pub(crate) fn record(&self, event: SensitiveDiagnosticEvent) {
        self.sink.record(event);
    }
}

impl fmt::Debug for SensitiveDiagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SensitiveDiagnostics")
            .field("sink", &format_args!("[REDACTED]"))
            .finish()
    }
}

/// A sink for sensitive diagnostic events.
///
/// Implement this trait for custom collectors, or pass a closure to
/// [`SensitiveDiagnostics::new`]. The callback is synchronous; async callers
/// should avoid blocking inside it.
pub trait SensitiveDiagnosticSink: Send + Sync + 'static {
    /// Record one sensitive diagnostic event.
    fn record(&self, event: SensitiveDiagnosticEvent);
}

impl<F> SensitiveDiagnosticSink for F
where
    F: Fn(SensitiveDiagnosticEvent) + Send + Sync + 'static,
{
    fn record(&self, event: SensitiveDiagnosticEvent) {
        self(event);
    }
}

#[derive(Debug)]
struct NoopSink;

impl SensitiveDiagnosticSink for NoopSink {
    fn record(&self, _event: SensitiveDiagnosticEvent) {}
}

/// Builder for a [`SensitiveDiagnostics`] sink from event-specific callbacks.
#[derive(Default)]
pub struct SensitiveDiagnosticsBuilder {
    event: Option<EventCallback>,
    request: Option<RequestCallback>,
    response: Option<ResponseCallback>,
    transport_error: Option<TransportErrorCallback>,
}

type EventCallback = Arc<dyn Fn(SensitiveDiagnosticEvent) + Send + Sync>;
type RequestCallback = Arc<dyn Fn(SensitiveRequestSnapshot) + Send + Sync>;
type ResponseCallback = Arc<dyn Fn(SensitiveResponseSnapshot) + Send + Sync>;
type TransportErrorCallback = Arc<dyn Fn(SensitiveTransportErrorSnapshot) + Send + Sync>;

impl SensitiveDiagnosticsBuilder {
    fn new() -> Self {
        Self::default()
    }

    /// Register a callback for every sensitive diagnostic event.
    #[must_use]
    pub fn on_event(
        mut self,
        callback: impl Fn(SensitiveDiagnosticEvent) + Send + Sync + 'static,
    ) -> Self {
        self.event = Some(Arc::new(callback));
        self
    }

    /// Register a callback for request snapshots.
    #[must_use]
    pub fn on_request(
        mut self,
        callback: impl Fn(SensitiveRequestSnapshot) + Send + Sync + 'static,
    ) -> Self {
        self.request = Some(Arc::new(callback));
        self
    }

    /// Register a callback for response snapshots.
    #[must_use]
    pub fn on_response(
        mut self,
        callback: impl Fn(SensitiveResponseSnapshot) + Send + Sync + 'static,
    ) -> Self {
        self.response = Some(Arc::new(callback));
        self
    }

    /// Register a callback for transport-error snapshots.
    #[must_use]
    pub fn on_transport_error(
        mut self,
        callback: impl Fn(SensitiveTransportErrorSnapshot) + Send + Sync + 'static,
    ) -> Self {
        self.transport_error = Some(Arc::new(callback));
        self
    }

    /// Build a [`SensitiveDiagnostics`] handle.
    #[must_use]
    pub fn build(self) -> SensitiveDiagnostics {
        SensitiveDiagnostics::new(BuilderSink {
            event: self.event,
            request: self.request,
            response: self.response,
            transport_error: self.transport_error,
        })
    }
}

impl fmt::Debug for SensitiveDiagnosticsBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SensitiveDiagnosticsBuilder")
            .field("on_event", &redacted_callback(self.event.is_some()))
            .field("on_request", &redacted_callback(self.request.is_some()))
            .field("on_response", &redacted_callback(self.response.is_some()))
            .field(
                "on_transport_error",
                &redacted_callback(self.transport_error.is_some()),
            )
            .finish()
    }
}

struct BuilderSink {
    event: Option<EventCallback>,
    request: Option<RequestCallback>,
    response: Option<ResponseCallback>,
    transport_error: Option<TransportErrorCallback>,
}

impl SensitiveDiagnosticSink for BuilderSink {
    fn record(&self, event: SensitiveDiagnosticEvent) {
        if let Some(callback) = &self.event {
            callback(event.clone());
        }

        match event {
            SensitiveDiagnosticEvent::Request(snapshot) => {
                if let Some(callback) = &self.request {
                    callback(snapshot);
                }
            }
            SensitiveDiagnosticEvent::Response(snapshot) => {
                if let Some(callback) = &self.response {
                    callback(snapshot);
                }
            }
            SensitiveDiagnosticEvent::TransportError(snapshot) => {
                if let Some(callback) = &self.transport_error {
                    callback(snapshot);
                }
            }
        }
    }
}

/// A sensitive event emitted by a client executor.
#[derive(Clone)]
pub enum SensitiveDiagnosticEvent {
    /// A request attempt is about to be handed to the transport.
    Request(SensitiveRequestSnapshot),
    /// A response was received and fully read.
    Response(SensitiveResponseSnapshot),
    /// A transport error prevented a usable response from being produced.
    TransportError(SensitiveTransportErrorSnapshot),
}

impl fmt::Debug for SensitiveDiagnosticEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(snapshot) => f.debug_tuple("Request").field(snapshot).finish(),
            Self::Response(snapshot) => f.debug_tuple("Response").field(snapshot).finish(),
            Self::TransportError(snapshot) => {
                f.debug_tuple("TransportError").field(snapshot).finish()
            }
        }
    }
}

/// Request data known to the Blooio executor for one HTTP attempt.
#[derive(Clone)]
pub struct SensitiveRequestSnapshot {
    /// HTTP method.
    pub method: http::Method,
    /// Full URL after base URL and query parameters are resolved.
    pub url: String,
    /// Rust operation type name.
    pub operation: &'static str,
    /// 1-based request attempt number.
    pub attempt: u32,
    /// Maximum configured retries for this operation.
    pub max_retries: u32,
    /// Caller-provided trace label, when present.
    pub trace_label: Option<String>,
    /// Request headers known to this crate, including injected authorization.
    pub headers: Vec<(String, String)>,
    /// Request body bytes, when the operation has a body.
    pub body: Option<Bytes>,
}

impl SensitiveRequestSnapshot {
    pub(crate) fn from_spec(
        spec: &RequestSpec,
        url: &str,
        auth_header: &str,
        operation: &'static str,
        attempt: u32,
        max_retries: u32,
        trace_label: Option<&str>,
    ) -> Self {
        let mut headers = Vec::with_capacity(spec.headers.len() + 2);
        headers.push((AUTHORIZATION.as_str().to_owned(), auth_header.to_owned()));
        for (key, value) in &spec.headers {
            if !key.eq_ignore_ascii_case(AUTHORIZATION.as_str()) {
                headers.push((key.clone(), value.clone()));
            }
        }
        if spec.body.is_some()
            && !spec
                .headers
                .iter()
                .any(|(key, _)| key.eq_ignore_ascii_case(CONTENT_TYPE.as_str()))
        {
            headers.push((
                CONTENT_TYPE.as_str().to_owned(),
                "application/json".to_owned(),
            ));
        }

        Self {
            method: spec.method.clone(),
            url: url.to_owned(),
            operation,
            attempt,
            max_retries,
            trace_label: trace_label.map(ToOwned::to_owned),
            headers,
            body: spec.body.clone(),
        }
    }
}

impl fmt::Debug for SensitiveRequestSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let body = self.body.as_ref().map_or_else(
            || "None".to_owned(),
            |body| format!("Some([REDACTED; {} bytes])", body.len()),
        );
        f.debug_struct("SensitiveRequestSnapshot")
            .field("method", &self.method)
            .field("url", &format_args!("[REDACTED; {} chars]", self.url.len()))
            .field("operation", &self.operation)
            .field("attempt", &self.attempt)
            .field("max_retries", &self.max_retries)
            .field(
                "trace_label",
                &format_args!(
                    "{}",
                    if self.trace_label.is_some() {
                        "Some([REDACTED])"
                    } else {
                        "None"
                    }
                ),
            )
            .field(
                "headers",
                &format_args!("[REDACTED; {}]", self.headers.len()),
            )
            .field("body", &format_args!("{body}"))
            .finish()
    }
}

/// Response data known to the Blooio executor after the body is fully read.
#[derive(Clone)]
pub struct SensitiveResponseSnapshot {
    /// HTTP method for the originating request.
    pub method: http::Method,
    /// Full URL for the originating request.
    pub url: String,
    /// Rust operation type name.
    pub operation: &'static str,
    /// 1-based request attempt number.
    pub attempt: u32,
    /// Maximum configured retries for this operation.
    pub max_retries: u32,
    /// Caller-provided trace label, when present.
    pub trace_label: Option<String>,
    /// HTTP response status code.
    pub status: u16,
    /// Raw response headers.
    pub headers: HeaderMap,
    /// Raw response body bytes.
    pub body: Bytes,
}

impl SensitiveResponseSnapshot {
    pub(crate) fn from_raw(request: &SensitiveRequestSnapshot, raw: &RawResponse) -> Self {
        Self {
            method: request.method.clone(),
            url: request.url.clone(),
            operation: request.operation,
            attempt: request.attempt,
            max_retries: request.max_retries,
            trace_label: request.trace_label.clone(),
            status: raw.status,
            headers: raw.headers.clone(),
            body: raw.body.clone(),
        }
    }
}

impl fmt::Debug for SensitiveResponseSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SensitiveResponseSnapshot")
            .field("method", &self.method)
            .field("url", &format_args!("[REDACTED; {} chars]", self.url.len()))
            .field("operation", &self.operation)
            .field("attempt", &self.attempt)
            .field("max_retries", &self.max_retries)
            .field(
                "trace_label",
                &format_args!(
                    "{}",
                    if self.trace_label.is_some() {
                        "Some([REDACTED])"
                    } else {
                        "None"
                    }
                ),
            )
            .field("status", &self.status)
            .field(
                "headers",
                &format_args!("[REDACTED; {}]", self.headers.len()),
            )
            .field(
                "body",
                &format_args!("[REDACTED; {} bytes]", self.body.len()),
            )
            .finish()
    }
}

/// A transport error plus the request attempt that produced it.
#[derive(Clone)]
pub struct SensitiveTransportErrorSnapshot {
    /// Request attempt associated with the transport failure.
    pub request: SensitiveRequestSnapshot,
    /// Where the failure happened.
    pub stage: SensitiveTransportErrorStage,
    /// Raw unsanitized display string from the transport error.
    pub error: String,
}

impl SensitiveTransportErrorSnapshot {
    pub(crate) fn new(
        request: SensitiveRequestSnapshot,
        stage: SensitiveTransportErrorStage,
        error: String,
    ) -> Self {
        Self {
            request,
            stage,
            error,
        }
    }
}

impl fmt::Debug for SensitiveTransportErrorSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SensitiveTransportErrorSnapshot")
            .field("request", &self.request)
            .field("stage", &self.stage)
            .field(
                "error",
                &format_args!("[REDACTED; {} chars]", self.error.len()),
            )
            .finish()
    }
}

/// Stage of an HTTP attempt where a transport failure occurred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SensitiveTransportErrorStage {
    /// The executor could not build the transport request.
    BuildRequest,
    /// Sending the request failed before a response could be read.
    Send,
    /// A response existed, but reading the body failed.
    ReadBody,
}

fn redacted_callback(is_some: bool) -> impl fmt::Debug {
    struct RedactedCallback(bool);
    impl fmt::Debug for RedactedCallback {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if self.0 {
                f.write_str("Some([REDACTED])")
            } else {
                f.write_str("None")
            }
        }
    }
    RedactedCallback(is_some)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]
mod tests {
    use super::*;

    #[test]
    fn event_sink_receives_event_enum() {
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = events.clone();
        let diagnostics = SensitiveDiagnostics::new(move |event| {
            captured.lock().unwrap().push(event);
        });

        diagnostics.record(SensitiveDiagnosticEvent::Request(request_snapshot()));

        let events = events.lock().unwrap();
        assert!(matches!(
            events.as_slice(),
            [SensitiveDiagnosticEvent::Request(_)]
        ));
    }

    #[test]
    fn builder_callbacks_receive_matching_events() {
        let all = std::sync::Arc::new(std::sync::Mutex::new(0_u32));
        let requests = std::sync::Arc::new(std::sync::Mutex::new(0_u32));
        let responses = std::sync::Arc::new(std::sync::Mutex::new(0_u32));
        let errors = std::sync::Arc::new(std::sync::Mutex::new(0_u32));

        let diagnostics = SensitiveDiagnostics::builder()
            .on_event({
                let all = all.clone();
                move |_| *all.lock().unwrap() += 1
            })
            .on_request({
                let requests = requests.clone();
                move |_| *requests.lock().unwrap() += 1
            })
            .on_response({
                let responses = responses.clone();
                move |_| *responses.lock().unwrap() += 1
            })
            .on_transport_error({
                let errors = errors.clone();
                move |_| *errors.lock().unwrap() += 1
            })
            .build();

        let request = request_snapshot();
        diagnostics.record(SensitiveDiagnosticEvent::Request(request.clone()));
        diagnostics.record(SensitiveDiagnosticEvent::Response(response_snapshot(
            &request,
        )));
        diagnostics.record(SensitiveDiagnosticEvent::TransportError(
            SensitiveTransportErrorSnapshot::new(
                request,
                SensitiveTransportErrorStage::Send,
                "raw transport secret".to_owned(),
            ),
        ));

        assert_eq!(*all.lock().unwrap(), 3);
        assert_eq!(*requests.lock().unwrap(), 1);
        assert_eq!(*responses.lock().unwrap(), 1);
        assert_eq!(*errors.lock().unwrap(), 1);
    }

    #[test]
    fn custom_sink_trait_path_works() {
        #[derive(Debug)]
        struct CountSink(std::sync::Arc<std::sync::Mutex<u32>>);

        impl SensitiveDiagnosticSink for CountSink {
            fn record(&self, _event: SensitiveDiagnosticEvent) {
                *self.0.lock().unwrap() += 1;
            }
        }

        let count = std::sync::Arc::new(std::sync::Mutex::new(0_u32));
        let diagnostics = SensitiveDiagnostics::new(CountSink(count.clone()));
        diagnostics.record(SensitiveDiagnosticEvent::Request(request_snapshot()));
        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[test]
    fn debug_output_redacts_sensitive_material() {
        let request = request_snapshot();
        let response = response_snapshot(&request);
        let error = SensitiveTransportErrorSnapshot::new(
            request.clone(),
            SensitiveTransportErrorStage::Send,
            "raw error https://secret.example/path?token=sk-secret".to_owned(),
        );

        for debug in [
            format!("{:?}", SensitiveDiagnostics::noop()),
            format!("{request:?}"),
            format!("{response:?}"),
            format!("{error:?}"),
            format!("{:?}", SensitiveDiagnosticEvent::TransportError(error)),
        ] {
            for forbidden in [
                "https://secret.example",
                "Bearer sk-secret",
                "secret-body",
                "secret-response-header",
                "secret-response-body",
                "token=sk-secret",
                "trace-secret",
            ] {
                assert!(
                    !debug.contains(forbidden),
                    "sensitive diagnostics Debug leaked {forbidden:?}: {debug}"
                );
            }
            assert!(debug.contains("REDACTED"));
        }
    }

    fn request_snapshot() -> SensitiveRequestSnapshot {
        SensitiveRequestSnapshot {
            method: http::Method::POST,
            url: "https://secret.example/chats/chat-secret/messages?token=sk-secret".to_owned(),
            operation: "blooio::test::SecretOperation",
            attempt: 1,
            max_retries: 2,
            trace_label: Some("trace-secret".to_owned()),
            headers: vec![
                ("authorization".to_owned(), "Bearer sk-secret".to_owned()),
                ("idempotency-key".to_owned(), "idem-secret".to_owned()),
            ],
            body: Some(Bytes::from_static(b"secret-body")),
        }
    }

    fn response_snapshot(request: &SensitiveRequestSnapshot) -> SensitiveResponseSnapshot {
        let mut headers = HeaderMap::new();
        headers.insert("x-secret", "secret-response-header".parse().unwrap());
        SensitiveResponseSnapshot {
            method: request.method.clone(),
            url: request.url.clone(),
            operation: request.operation,
            attempt: request.attempt,
            max_retries: request.max_retries,
            trace_label: request.trace_label.clone(),
            status: 200,
            headers,
            body: Bytes::from_static(b"secret-response-body"),
        }
    }
}
