//! Per-request executor options.

use std::fmt;
use std::time::Duration;

use http::header::{HeaderName, HeaderValue};

use crate::config::{ClientConfig, normalize_base_url, validate_base_url};
use crate::core::retry::RetryPolicy;
use crate::error::{Error, Result};

/// Request-scoped overrides applied by the executors after an
/// [`Operation`](crate::Operation) is resolved into a concrete request.
///
/// These options are intentionally transport-level concerns. Endpoint
/// [`Operation`](crate::Operation) types stay focused on the API's method,
/// path, query, headers, and body.
#[derive(Clone, Default)]
pub struct RequestOptions {
    pub(crate) base_url: Option<String>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) retry: Option<RetryPolicy>,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) query: Vec<(String, String)>,
}

impl RequestOptions {
    /// Create empty request options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the API base URL for this request only.
    ///
    /// The value is concatenated with the operation path after trailing slashes
    /// are trimmed. Include the API prefix you want: v2 operations usually need
    /// a base like `https://backend.blooio.com/v2/api`, while legacy v1
    /// experiments likely need `https://backend.blooio.com` plus an operation
    /// path beginning with `/v1/api`.
    ///
    /// Use [`try_base_url`](Self::try_base_url) for user-supplied or otherwise
    /// dynamic URLs that should be validated immediately.
    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(normalize_base_url(base_url.into()));
        self
    }

    /// Override the API base URL for this request only, validating that the
    /// value is an absolute `http` or `https` URL without a query string.
    pub fn try_base_url(mut self, base_url: impl AsRef<str>) -> Result<Self> {
        let base_url = base_url.as_ref();
        validate_base_url(base_url, "request base URL")?;
        self.base_url = Some(normalize_base_url(base_url.to_owned()));
        Ok(self)
    }

    /// Override the per-attempt timeout for this request.
    ///
    /// Async callers can cancel the whole operation by dropping the returned
    /// future or wrapping it in `tokio::time::timeout`; this timeout controls
    /// each individual HTTP attempt, including retries.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Override the retry policy for this request.
    #[must_use]
    pub fn retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = Some(retry);
        self
    }

    /// Disable retries for this request.
    #[must_use]
    pub fn no_retry(self) -> Self {
        self.retry(RetryPolicy::none())
    }

    /// Add or override a trusted static header.
    ///
    /// Use [`try_header`](Self::try_header) for user-supplied or otherwise
    /// dynamic header names and values that should be validated immediately.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Add or override a header, validating that the name and value are valid
    /// HTTP header components.
    pub fn try_header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Result<Self> {
        let name = name.as_ref();
        let value = value.as_ref();
        HeaderName::from_bytes(name.as_bytes()).map_err(Error::config)?;
        HeaderValue::from_str(value).map_err(Error::config)?;
        self.headers.push((name.to_owned(), value.to_owned()));
        Ok(self)
    }

    /// Append an extra query parameter to this request.
    ///
    /// Extra query parameters are appended after operation-provided parameters.
    /// If the same key appears multiple times, all values are sent.
    #[must_use]
    pub fn query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((key.into(), value.into()));
        self
    }

    pub(crate) fn retry_or(&self, fallback: RetryPolicy) -> RetryPolicy {
        self.retry.unwrap_or(fallback)
    }

    pub(crate) fn url_for(&self, fallback: &ClientConfig, path: &str) -> String {
        if let Some(base_url) = &self.base_url {
            format!("{base_url}{path}")
        } else {
            fallback.url_for(path)
        }
    }
}

impl fmt::Debug for RequestOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestOptions")
            .field(
                "base_url",
                &format_args!(
                    "{}",
                    if self.base_url.is_some() {
                        "Some([REDACTED])"
                    } else {
                        "None"
                    }
                ),
            )
            .field("timeout", &self.timeout)
            .field("retry", &self.retry)
            .field(
                "headers",
                &format_args!("[REDACTED; {}]", self.headers.len()),
            )
            .field("query", &format_args!("[REDACTED; {}]", self.query.len()))
            .finish()
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
    fn base_url_trims_trailing_slashes() {
        let options = RequestOptions::new().base_url("https://example.com/v2/api///");
        assert_eq!(
            options.url_for(&ClientConfig::new("k"), "/me"),
            "https://example.com/v2/api/me"
        );
    }

    #[test]
    fn try_base_url_rejects_invalid_url() {
        let err = RequestOptions::new()
            .try_base_url("https://example.com/api?token=secret")
            .unwrap_err();
        assert!(matches!(err, Error::Config(_)));
        assert!(!err.to_string().contains("secret"));
    }

    #[test]
    fn try_header_rejects_invalid_name() {
        let err = RequestOptions::new()
            .try_header("bad header", "value")
            .unwrap_err();
        assert!(matches!(err, Error::Config(_)));
    }

    #[test]
    fn debug_redacts_header_and_query_values() {
        let options = RequestOptions::new()
            .base_url("https://secret.example/v2/api")
            .header("x-api-key", "secret")
            .query("token", "also-secret");
        let dbg = format!("{options:?}");
        assert!(!dbg.contains("secret"));
        assert!(!dbg.contains("secret.example"));
        assert!(dbg.contains("REDACTED"));
    }
}
