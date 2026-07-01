//! Error types for the crate.

use std::{fmt, time::Duration};

#[cfg(any(feature = "async", feature = "sync"))]
use serde::Deserialize;
use serde_json::{Map, Value};

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type returned by all fallible operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The API returned a non-2xx response. Stable `code`/`error` fields are
    /// decoded from the Blooio `Error` schema where possible.
    ///
    /// Match on [`ApiError::code`] for stable, machine-readable error handling,
    /// or use helpers such as [`ApiError::is_quota_error`].
    #[error("{0}")]
    Api(ApiError),

    /// A transport-level failure: connection, DNS, TLS, timeout, etc.
    #[error("transport error: {0}")]
    Transport(String),

    /// The request body could not be serialized to JSON.
    #[error("failed to encode request body: {0}")]
    Encode(String),

    /// A 2xx response body could not be deserialized into the expected type.
    #[error("failed to decode response body: {0}")]
    Decode(String),

    /// Client configuration was missing or invalid.
    #[error("configuration error: {0}")]
    Config(String),

    /// Webhook signature verification failed.
    #[cfg(feature = "webhooks")]
    #[error("webhook verification failed: {0}")]
    Webhook(#[from] crate::webhook::VerifyError),
}

impl Error {
    #[cfg(any(feature = "async", feature = "sync"))]
    pub(crate) fn transport(e: impl std::fmt::Display) -> Self {
        Error::Transport(e.to_string())
    }

    #[cfg(any(feature = "async", feature = "sync"))]
    pub(crate) fn encode(e: impl std::fmt::Display) -> Self {
        Error::Encode(e.to_string())
    }

    pub(crate) fn decode(_e: impl std::fmt::Display) -> Self {
        Error::Decode("failed to decode JSON body".to_owned())
    }

    #[cfg(any(feature = "async", feature = "sync"))]
    pub(crate) fn config(e: impl std::fmt::Display) -> Self {
        Error::Config(e.to_string())
    }

    /// The machine-readable API error code, if this is an [`Error::Api`].
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        match self {
            Error::Api(err) => err.code(),
            _ => None,
        }
    }

    /// The HTTP status code, if this is an [`Error::Api`].
    #[must_use]
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Api(err) => Some(err.status()),
            _ => None,
        }
    }

    /// The server-advised delay before retrying, if the response carried a
    /// `Retry-After` header expressed in delta-seconds.
    #[must_use]
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Api(err) => err.retry_after(),
            _ => None,
        }
    }

    /// Whether this is a documented quota/cap API error.
    #[must_use]
    pub fn is_quota_error(&self) -> bool {
        matches!(self, Error::Api(err) if err.is_quota_error())
    }

    /// Whether this is a documented threaded-reply target API error.
    #[must_use]
    pub fn is_reply_target_error(&self) -> bool {
        matches!(self, Error::Api(err) if err.is_reply_target_error())
    }

    /// Whether this is the documented inbound-only prior-inbound API error.
    #[must_use]
    pub fn is_inbound_only_error(&self) -> bool {
        matches!(self, Error::Api(err) if err.is_inbound_only_error())
    }

    /// Whether retrying this request may succeed.
    ///
    /// `true` for transport failures (connection/DNS/TLS/timeout) and for the
    /// transient API statuses `408`, `425`, unknown/no-code `429`, and `5xx`.
    /// Documented quota/cap `429` errors are not retried by default because
    /// another immediate attempt cannot clear the account or plan cap. Encoding,
    /// decoding, and webhook-verification errors — and 4xx other than the
    /// listed transient ones — are not retryable.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Transport(_) => true,
            Error::Api(err) => err.is_retryable(),
            _ => false,
        }
    }
}

/// A non-2xx response returned by the Blooio API.
///
/// Server-provided prose and structured detail values are stored for explicit
/// inspection through accessors, but they are redacted from [`Debug`] and never
/// included in [`Display`](fmt::Display). Prefer matching on [`Self::code`] or
/// the helper predicates for programmatic handling.
pub struct ApiError {
    status: u16,
    code: Option<String>,
    error: Option<String>,
    server_message: Option<String>,
    details: ApiErrorDetails,
    retry_after: Option<Duration>,
    schema_body: bool,
}

impl ApiError {
    pub(crate) fn from_schema(
        status: u16,
        code: Option<String>,
        error: Option<String>,
        server_message: Option<String>,
        details: ApiErrorDetails,
        retry_after: Option<Duration>,
    ) -> Self {
        Self {
            status,
            code,
            error,
            server_message,
            details,
            retry_after,
            schema_body: true,
        }
    }

    pub(crate) fn from_non_schema(status: u16, retry_after: Option<Duration>) -> Self {
        Self {
            status,
            code: None,
            error: None,
            server_message: None,
            details: ApiErrorDetails::default(),
            retry_after,
            schema_body: false,
        }
    }

    /// The HTTP response status code.
    #[must_use]
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The machine-readable API error code, if the body carried one.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// The short API error label from the `error` field, if present.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The raw server `message` field, if present.
    ///
    /// This may contain server-provided prose or reflected request context, so
    /// it is exposed only through this explicit accessor and is redacted from
    /// [`Debug`] and [`Display`](fmt::Display).
    #[must_use]
    pub fn server_message(&self) -> Option<&str> {
        self.server_message.as_deref()
    }

    /// The server-advised delay before retrying, if the response carried a
    /// `Retry-After` header expressed in delta-seconds.
    #[must_use]
    pub fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }

    /// Additional structured fields returned by the API error body.
    ///
    /// These are intentionally loose JSON because Blooio owns the shape of
    /// endpoint-specific context fields.
    #[must_use]
    pub fn details(&self) -> &ApiErrorDetails {
        &self.details
    }

    /// Whether this error is one of the documented quota/cap API errors.
    #[must_use]
    pub fn is_quota_error(&self) -> bool {
        self.code.as_deref().is_some_and(codes::is_quota_error)
    }

    /// Whether this error is one of the documented threaded-reply target API
    /// errors.
    #[must_use]
    pub fn is_reply_target_error(&self) -> bool {
        self.code
            .as_deref()
            .is_some_and(codes::is_reply_target_error)
    }

    /// Whether this error is the documented inbound-only prior-inbound API
    /// error.
    #[must_use]
    pub fn is_inbound_only_error(&self) -> bool {
        self.code
            .as_deref()
            .is_some_and(codes::is_inbound_only_error)
    }

    fn is_retryable(&self) -> bool {
        match self.status {
            408 | 425 => true,
            429 => !self.is_quota_error(),
            status => (500..600).contains(&status),
        }
    }

    fn safe_summary(&self) -> String {
        if let Some(code) = self.code() {
            format!("HTTP {} ({code})", self.status)
        } else if let Some(error) = self.error() {
            format!("HTTP {} ({error})", self.status)
        } else if self.schema_body {
            format!("HTTP {}", self.status)
        } else {
            format!("HTTP {} (non-schema error body omitted)", self.status)
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let summary = self.safe_summary();
        if let Some(code) = self.code() {
            write!(
                f,
                "blooio api error (status {}, code {}): {}",
                self.status, code, summary
            )
        } else {
            write!(f, "blooio api error (status {}): {}", self.status, summary)
        }
    }
}

impl fmt::Debug for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let server_message = self.server_message.as_ref().map(|_| "[REDACTED]");
        let summary = self.safe_summary();
        f.debug_struct("ApiError")
            .field("status", &self.status)
            .field("code", &self.code)
            .field("error", &self.error)
            .field("server_message", &server_message)
            .field("details", &self.details)
            .field("retry_after", &self.retry_after)
            .field("schema_body", &self.schema_body)
            .field("summary", &summary)
            .finish()
    }
}

/// Additional structured JSON fields returned by an API error response.
///
/// The standard Blooio error fields (`error`, `status`, `code`, and `message`)
/// are exposed on [`ApiError`]. Remaining endpoint-specific fields are kept
/// here as intentionally loose JSON and redacted from [`Debug`].
#[derive(Clone, Default)]
pub struct ApiErrorDetails {
    fields: Map<String, Value>,
}

impl ApiErrorDetails {
    pub(crate) fn new(fields: Map<String, Value>) -> Self {
        Self { fields }
    }

    /// Return a detail value by field name.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }

    /// Whether the API error body included any endpoint-specific fields.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Borrow all endpoint-specific detail fields as a JSON object map.
    #[must_use]
    pub fn as_object(&self) -> &Map<String, Value> {
        &self.fields
    }
}

impl fmt::Debug for ApiErrorDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let keys = self.fields.keys().map(String::as_str).collect::<Vec<_>>();
        f.debug_struct("ApiErrorDetails")
            .field("len", &self.fields.len())
            .field("keys", &keys)
            .field("values", &"[REDACTED]")
            .finish()
    }
}

/// Machine-readable Blooio API error codes documented by the v2 API.
pub mod codes {
    /// Sending would exceed the organization's new-contact outbound cap.
    pub const OUTBOUND_LIMIT_REACHED: &str = "outbound_limit_reached";
    /// Sending would exceed the shared plan's daily new-conversation cap.
    pub const NEW_CONVERSATION_LIMIT_REACHED: &str = "new_conversation_limit_reached";
    /// An inbound-only allocation has no prior inbound conversation.
    pub const INBOUND_ONLY_NO_PRIOR_INBOUND: &str = "inbound_only_no_prior_inbound";
    /// The reply target is invalid.
    pub const REPLY_TARGET_INVALID: &str = "reply_target_invalid";
    /// The reply target belongs to a different chat.
    pub const REPLY_TARGET_CHAT_MISMATCH: &str = "reply_target_chat_mismatch";
    /// The reply target belongs to a different device.
    pub const REPLY_TARGET_DEVICE_MISMATCH: &str = "reply_target_device_mismatch";
    /// The reply target has expired.
    pub const REPLY_TARGET_EXPIRED: &str = "reply_target_expired";
    /// The reply target is unsupported.
    pub const REPLY_TARGET_NOT_SUPPORTED: &str = "reply_target_not_supported";
    /// The reply target could not be found.
    pub const REPLY_TARGET_NOT_FOUND: &str = "reply_target_not_found";

    /// Whether `code` is a documented quota/cap error code.
    #[must_use]
    pub fn is_quota_error(code: &str) -> bool {
        matches!(
            code,
            OUTBOUND_LIMIT_REACHED | NEW_CONVERSATION_LIMIT_REACHED
        )
    }

    /// Whether `code` is a documented threaded-reply target error code.
    #[must_use]
    pub fn is_reply_target_error(code: &str) -> bool {
        matches!(
            code,
            REPLY_TARGET_INVALID
                | REPLY_TARGET_CHAT_MISMATCH
                | REPLY_TARGET_DEVICE_MISMATCH
                | REPLY_TARGET_EXPIRED
                | REPLY_TARGET_NOT_SUPPORTED
                | REPLY_TARGET_NOT_FOUND
        )
    }

    /// Whether `code` is the documented inbound-only prior-inbound error code.
    #[must_use]
    pub fn is_inbound_only_error(code: &str) -> bool {
        code == INBOUND_ONLY_NO_PRIOR_INBOUND
    }
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
    fn decode_error_does_not_store_source_message() {
        let err = Error::decode("bad response contained sk-secret-123");
        assert!(!err.to_string().contains("sk-secret-123"));
    }

    #[test]
    fn api_error_accessors_and_predicates_work() {
        let mut fields = Map::new();
        fields.insert("limit".to_owned(), Value::from(10));
        let api = ApiError::from_schema(
            429,
            Some(codes::OUTBOUND_LIMIT_REACHED.to_owned()),
            Some("rate_limited".to_owned()),
            Some("slow down".to_owned()),
            ApiErrorDetails::new(fields),
            Some(Duration::from_secs(3)),
        );
        let err = Error::Api(api);

        assert_eq!(err.status(), Some(429));
        assert_eq!(err.code(), Some(codes::OUTBOUND_LIMIT_REACHED));
        assert_eq!(err.retry_after(), Some(Duration::from_secs(3)));
        assert!(err.is_quota_error());
        assert!(!err.is_retryable());
        let Error::Api(api) = err else {
            panic!("expected api error");
        };
        assert_eq!(api.error(), Some("rate_limited"));
        assert_eq!(api.server_message(), Some("slow down"));
        assert_eq!(api.details().get("limit"), Some(&Value::from(10)));
    }

    #[test]
    fn api_error_debug_redacts_server_message_and_detail_values() {
        let mut fields = Map::new();
        fields.insert("external_id".to_owned(), Value::from("secret-ish-context"));
        let err = Error::Api(ApiError::from_schema(
            403,
            Some(codes::INBOUND_ONLY_NO_PRIOR_INBOUND.to_owned()),
            Some("forbidden".to_owned()),
            Some("contains sk-secret-123".to_owned()),
            ApiErrorDetails::new(fields),
            None,
        ));

        let debug = format!("{err:?}");
        let display = err.to_string();
        assert!(!debug.contains("sk-secret-123"));
        assert!(!debug.contains("secret-ish-context"));
        assert!(!display.contains("sk-secret-123"));
        assert!(!display.contains("secret-ish-context"));
    }

    #[test]
    fn code_helpers_classify_documented_codes() {
        assert!(codes::is_quota_error(codes::NEW_CONVERSATION_LIMIT_REACHED));
        assert!(codes::is_reply_target_error(
            codes::REPLY_TARGET_DEVICE_MISMATCH
        ));
        assert!(codes::is_inbound_only_error(
            codes::INBOUND_ONLY_NO_PRIOR_INBOUND
        ));
        assert!(!codes::is_quota_error(codes::REPLY_TARGET_NOT_FOUND));
    }
}

/// Safe subset of the Blooio `Error` schema.
#[cfg(any(feature = "async", feature = "sync"))]
#[derive(Default, Deserialize)]
pub(crate) struct ApiErrorBody {
    pub error: Option<String>,
    pub message: Option<String>,
    #[allow(dead_code)]
    pub status: Option<u16>,
    pub code: Option<String>,
    #[serde(flatten)]
    pub details: Map<String, Value>,
}
