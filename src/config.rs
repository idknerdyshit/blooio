//! Client configuration.

use std::time::Duration;

use crate::core::retry::RetryPolicy;
use crate::error::{Error, Result};
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

    /// Create a configuration from environment variables.
    ///
    /// Reads `BLOOIO_API_KEY` (required) and `BLOOIO_BASE_URL` (optional).
    /// Empty values are treated as missing, and the API key is never reflected
    /// in error messages.
    pub fn from_env() -> Result<Self> {
        Self::from_env_values(env_var("BLOOIO_API_KEY")?, env_var("BLOOIO_BASE_URL")?)
    }

    pub(crate) fn from_env_values(
        api_key: Option<String>,
        base_url: Option<String>,
    ) -> Result<Self> {
        let api_key = api_key
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| Error::config("BLOOIO_API_KEY is not set"))?;

        let mut config = Self::new(api_key);
        if let Some(base_url) = base_url.filter(|value| !value.trim().is_empty()) {
            validate_base_url(&base_url, "BLOOIO_BASE_URL")?;
            config = config.with_base_url(base_url);
        }
        Ok(config)
    }

    /// Override the base URL (trailing slashes are trimmed).
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = normalize_base_url(base_url.into());
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

fn env_var(name: &'static str) -> Result<Option<String>> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(Error::config(format!("{name} is not valid Unicode")))
        }
    }
}

pub(crate) fn normalize_base_url(mut base_url: String) -> String {
    while base_url.ends_with('/') {
        base_url.pop();
    }
    base_url
}

pub(crate) fn validate_base_url(base_url: &str, name: &'static str) -> Result<()> {
    if base_url.trim() != base_url {
        return Err(Error::config(format!("{name} is not a valid base URL")));
    }

    let uri: http::Uri = base_url
        .parse()
        .map_err(|_| Error::config(format!("{name} is not a valid base URL")))?;
    match uri.scheme_str() {
        Some("http" | "https") => {}
        _ => {
            return Err(Error::config(format!(
                "{name} must start with http:// or https://"
            )));
        }
    }
    if uri.authority().is_none()
        || uri
            .path_and_query()
            .is_some_and(|parts| parts.query().is_some())
    {
        return Err(Error::config(format!("{name} is not a valid base URL")));
    }

    Ok(())
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
    fn url_for_concatenates_base_and_path() {
        let cfg = ClientConfig::new("k").with_base_url("https://example.com/api");
        assert_eq!(cfg.url_for("/me"), "https://example.com/api/me");
        assert_eq!(
            cfg.url_for("/chats/c1/messages"),
            "https://example.com/api/chats/c1/messages"
        );
    }

    #[test]
    fn from_env_values_requires_api_key() {
        let err = ClientConfig::from_env_values(None, None).unwrap_err();
        assert!(matches!(err, Error::Config(_)));
        assert!(!err.to_string().contains("secret"));
    }

    #[test]
    fn from_env_values_uses_base_url_override() {
        let cfg = ClientConfig::from_env_values(
            Some("secret-key".into()),
            Some("https://example.com/api/".into()),
        )
        .unwrap();
        assert_eq!(cfg.base_url, "https://example.com/api");
        assert_eq!(cfg.api_key.expose(), "secret-key");
    }

    #[test]
    fn from_env_values_rejects_invalid_base_url() {
        let err = ClientConfig::from_env_values(Some("secret-key".into()), Some("nope".into()))
            .unwrap_err();
        assert!(matches!(err, Error::Config(_)));
        assert!(!err.to_string().contains("secret-key"));
    }

    #[test]
    fn from_env_values_rejects_base_url_query() {
        let err = ClientConfig::from_env_values(
            Some("secret-key".into()),
            Some("https://example.com/api?token=secret-key".into()),
        )
        .unwrap_err();
        assert!(matches!(err, Error::Config(_)));
        assert!(!err.to_string().contains("secret-key"));
    }
}
