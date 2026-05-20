//! Client configuration.

use std::time::Duration;

use crate::core::retry::RetryPolicy;
use crate::secret::Secret;

/// The production base URL for the Blooio API.
pub const DEFAULT_BASE_URL: &str = "https://backend.blooio.com/v2/api";

/// Shared configuration consumed by both the async and blocking clients.
///
/// The API key is wrapped in a [`Secret`] so it can never be logged or
/// serialized in cleartext. The derived `Debug` is safe: it prints
/// `api_key: [REDACTED]`.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    /// API base URL, without a trailing slash. Defaults to [`DEFAULT_BASE_URL`].
    pub base_url: String,
    /// Bearer API key.
    pub api_key: Secret<String>,
    /// Per-request timeout. Defaults to 30 seconds.
    pub timeout: Duration,
    /// `User-Agent` header value.
    pub user_agent: String,
    /// How transient failures are retried. Defaults to [`RetryPolicy::default`]
    /// (up to two retries with jittered exponential backoff).
    pub retry: RetryPolicy,
}

impl ClientConfig {
    /// Create a configuration from an API key, using production defaults.
    pub fn new(api_key: impl Into<Secret<String>>) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_owned(),
            api_key: api_key.into(),
            timeout: Duration::from_secs(30),
            user_agent: concat!("blooio-rs/", env!("CARGO_PKG_VERSION")).to_owned(),
            retry: RetryPolicy::default(),
        }
    }

    /// Override the base URL (trailing slashes are trimmed).
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        let mut url = base_url.into();
        while url.ends_with('/') {
            url.pop();
        }
        self.base_url = url;
        self
    }

    /// Override the per-request timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Override the `User-Agent` header.
    #[must_use]
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Override the retry policy. Pass [`RetryPolicy::none`] to disable retries.
    #[must_use]
    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Build the absolute URL for an operation path (which begins with `/`).
    pub(crate) fn url_for(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;

    #[test]
    fn new_uses_production_defaults() {
        let cfg = ClientConfig::new("k");
        assert_eq!(cfg.base_url, DEFAULT_BASE_URL);
        assert_eq!(cfg.timeout, Duration::from_secs(30));
        assert!(cfg.user_agent.starts_with("blooio-rs/"));
    }

    #[test]
    fn with_base_url_trims_trailing_slashes() {
        let one = ClientConfig::new("k").with_base_url("https://example.com/api/");
        assert_eq!(one.base_url, "https://example.com/api");
        let many = ClientConfig::new("k").with_base_url("https://example.com/api///");
        assert_eq!(many.base_url, "https://example.com/api");
        let none = ClientConfig::new("k").with_base_url("https://example.com/api");
        assert_eq!(none.base_url, "https://example.com/api");
    }

    #[test]
    fn with_timeout_and_user_agent_override() {
        let cfg = ClientConfig::new("k")
            .with_timeout(Duration::from_millis(500))
            .with_user_agent("my-app/1.0");
        assert_eq!(cfg.timeout, Duration::from_millis(500));
        assert_eq!(cfg.user_agent, "my-app/1.0");
    }

    #[test]
    fn url_for_concatenates_base_and_path() {
        let cfg = ClientConfig::new("k").with_base_url("https://example.com/api");
        assert_eq!(cfg.url_for("/me"), "https://example.com/api/me");
        assert_eq!(
            cfg.url_for("/chats/c1/messages"),
            "https://example.com/api/chats/c1/messages"
        );
    }

    #[test]
    fn debug_redacts_api_key() {
        let cfg = ClientConfig::new("super-secret-key");
        let dbg = format!("{cfg:?}");
        assert!(!dbg.contains("super-secret-key"), "api key leaked in Debug");
        assert!(dbg.contains("REDACTED"));
    }
}
