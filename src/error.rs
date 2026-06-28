//! Error types for the crate.

use std::time::Duration;

#[cfg(any(feature = "async", feature = "sync"))]
use serde::Deserialize;

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type returned by all fallible operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The API returned a non-2xx response. The body was decoded from the
    /// Blooio `Error` schema where possible.
    ///
    /// Match on [`code`](Error::Api::code) for stable, machine-readable error
    /// handling (e.g. `outbound_limit_reached`).
    #[error("blooio api error (status {status}{}): {message}", code.as_deref().map(|c| format!(", code {c}")).unwrap_or_default())]
    Api {
        /// HTTP status code.
        status: u16,
        /// Machine-readable error code, if the body carried one.
        code: Option<String>,
        /// Human-readable message.
        message: String,
        /// The short error label (the `error` field), if present.
        error: Option<String>,
        /// The `Retry-After` hint (delta-seconds) if the response carried one.
        /// Populated on throttling (`429`) and `503` responses; see
        /// [`retry_after`](Error::retry_after).
        retry_after: Option<Duration>,
    },

    /// A transport-level failure: connection, DNS, TLS, timeout, etc.
    #[error("transport error: {0}")]
    Transport(String),

    /// The request body could not be serialized to JSON.
    #[error("failed to encode request body: {0}")]
    Encode(String),

    /// A 2xx response body could not be deserialized into the expected type.
    #[error("failed to decode response body: {0}")]
    Decode(String),

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

    pub(crate) fn decode(e: impl std::fmt::Display) -> Self {
        Error::Decode(e.to_string())
    }

    /// The machine-readable API error code, if this is an [`Error::Api`].
    pub fn code(&self) -> Option<&str> {
        match self {
            Error::Api { code, .. } => code.as_deref(),
            _ => None,
        }
    }

    /// The HTTP status code, if this is an [`Error::Api`].
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Api { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// The server-advised delay before retrying, if the response carried a
    /// `Retry-After` header expressed in delta-seconds.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Api { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Whether retrying this request may succeed.
    ///
    /// `true` for transport failures (connection/DNS/TLS/timeout) and for the
    /// transient API statuses `408`, `425`, `429`, and `5xx`. Encoding,
    /// decoding, and webhook-verification errors — and 4xx other than the
    /// listed transient ones — are not retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Transport(_) => true,
            Error::Api { status, .. } => {
                matches!(status, 408 | 425 | 429) || (500..600).contains(status)
            }
            _ => false,
        }
    }
}

/// Wire shape of the Blooio `Error` schema: `{ error, message, status, code }`.
#[cfg(any(feature = "async", feature = "sync"))]
#[derive(Debug, Default, Deserialize)]
pub(crate) struct ApiErrorBody {
    pub error: Option<String>,
    pub message: Option<String>,
    #[allow(dead_code)]
    pub status: Option<u16>,
    pub code: Option<String>,
}
