//! Client configuration.

use std::time::Duration;

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
}

impl ClientConfig {
    /// Create a configuration from an API key, using production defaults.
    pub fn new(api_key: impl Into<Secret<String>>) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_owned(),
            api_key: api_key.into(),
            timeout: Duration::from_secs(30),
            user_agent: concat!("blooio-rs/", env!("CARGO_PKG_VERSION")).to_owned(),
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

    /// Build the absolute URL for an operation path (which begins with `/`).
    pub(crate) fn url_for(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}
