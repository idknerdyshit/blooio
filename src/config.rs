//! Client configuration.

use std::fmt;
use std::time::Duration;

use http::HeaderValue;

#[cfg(feature = "sensitive-diagnostics")]
use crate::core::diagnostics::SensitiveDiagnostics;
use crate::core::retry::RetryPolicy;
use crate::error::{Error, Result};

/// The production base URL for the Blooio API.
pub const DEFAULT_BASE_URL: &str = "https://backend.blooio.com/v2/api";

/// Default maximum response body retained in memory by either executor (64 MiB).
pub const DEFAULT_MAX_RESPONSE_BODY_BYTES: usize = 64 * 1024 * 1024;

/// Shared transport configuration consumed by both the async and blocking
/// clients.
#[derive(Clone)]
pub struct ClientConfig {
    /// API base URL, without a trailing slash. Defaults to [`DEFAULT_BASE_URL`].
    pub base_url: String,
    /// Per-request timeout. Defaults to 30 seconds.
    pub timeout: Duration,
    /// `User-Agent` header value.
    pub user_agent: String,
    /// How transient failures are retried. Defaults to [`RetryPolicy::default`]
    /// (up to two retries with jittered exponential backoff).
    pub retry: RetryPolicy,
    /// Explicitly sensitive request/response diagnostics sink.
    ///
    /// This field exists only with the `sensitive-diagnostics` feature. When
    /// set, it receives raw request/response material for every request made by
    /// clients built from this config, unless a request-level diagnostics
    /// override is supplied. The sink itself is redacted from [`Debug`].
    #[cfg(feature = "sensitive-diagnostics")]
    pub sensitive_diagnostics: Option<SensitiveDiagnostics>,
    /// Enable intentionally unredacted tracing for every request made by
    /// clients built from this configuration.
    #[cfg(feature = "sensitive-diagnostics")]
    pub sensitive_tracing: bool,
}

impl fmt::Debug for ClientConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("ClientConfig");
        debug
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field("user_agent", &self.user_agent)
            .field("retry", &self.retry);
        #[cfg(feature = "sensitive-diagnostics")]
        debug
            .field(
                "sensitive_diagnostics",
                &format_args!(
                    "{}",
                    if self.sensitive_diagnostics.is_some() {
                        "Some([REDACTED])"
                    } else {
                        "None"
                    }
                ),
            )
            .field(
                "sensitive_tracing",
                &format_args!(
                    "{}",
                    if self.sensitive_tracing {
                        "[REDACTED]"
                    } else {
                        "false"
                    }
                ),
            );
        debug.finish()
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_owned(),
            timeout: Duration::from_secs(30),
            user_agent: concat!("blooio-rs/", env!("CARGO_PKG_VERSION")).to_owned(),
            retry: RetryPolicy::default(),
            #[cfg(feature = "sensitive-diagnostics")]
            sensitive_diagnostics: None,
            #[cfg(feature = "sensitive-diagnostics")]
            sensitive_tracing: false,
        }
    }
}

impl ClientConfig {
    /// Create a configuration using production defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration from environment variables.
    ///
    /// Reads `BLOOIO_BASE_URL` (optional). Credentials are intentionally kept
    /// separate; use [`crate::BlooioCreds::from_env`] for `BLOOIO_API_KEY`.
    pub fn from_env() -> Result<Self> {
        Self::from_env_values(env_var("BLOOIO_BASE_URL")?)
    }

    #[cfg(feature = "api-v4")]
    pub(crate) fn from_env_with_default(default_base_url: &str) -> Result<Self> {
        Self::from_env_values_with_default(env_var("BLOOIO_BASE_URL")?, default_base_url)
    }

    pub(crate) fn from_env_values(base_url: Option<String>) -> Result<Self> {
        Self::from_env_values_with_default(base_url, DEFAULT_BASE_URL)
    }

    fn from_env_values_with_default(
        base_url: Option<String>,
        default_base_url: &str,
    ) -> Result<Self> {
        let mut config = Self::new().with_base_url(default_base_url);
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

    /// Validate values interpreted as URL or HTTP header components.
    pub fn validate(&self) -> Result<()> {
        validate_base_url(&self.base_url, "client base URL")?;
        HeaderValue::from_str(&self.user_agent)
            .map_err(|_| Error::config("user agent is not a valid HTTP header value"))?;
        Ok(())
    }

    /// Attach a client-wide sensitive diagnostics sink.
    ///
    /// This is available only with the `sensitive-diagnostics` feature and can
    /// expose API keys, URLs, headers, request bodies, response bodies, and raw
    /// transport errors to the supplied sink. It is intended for local/protocol
    /// debugging, not production logging.
    #[cfg(feature = "sensitive-diagnostics")]
    #[must_use]
    pub fn with_sensitive_diagnostics(mut self, diagnostics: SensitiveDiagnostics) -> Self {
        self.sensitive_diagnostics = Some(diagnostics);
        self
    }

    /// Remove any client-wide sensitive diagnostics sink.
    #[cfg(feature = "sensitive-diagnostics")]
    #[must_use]
    pub fn without_sensitive_diagnostics(mut self) -> Self {
        self.sensitive_diagnostics = None;
        self
    }

    /// Enable intentionally unredacted tracing for all client requests.
    ///
    /// This is available only with the `sensitive-diagnostics` feature. It
    /// emits complete request/response snapshots and raw transport errors to
    /// the `blooio::sensitive` tracing target, including credentials, URLs,
    /// headers, and bodies. It is intended only for local protocol debugging;
    /// do not enable it with production log sinks.
    #[cfg(feature = "sensitive-diagnostics")]
    #[must_use]
    pub fn with_sensitive_tracing(mut self) -> Self {
        self.sensitive_tracing = true;
        self
    }

    /// Disable intentionally unredacted tracing for clients built from this
    /// configuration.
    #[cfg(feature = "sensitive-diagnostics")]
    #[must_use]
    pub fn without_sensitive_tracing(mut self) -> Self {
        self.sensitive_tracing = false;
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
        let cfg = ClientConfig::new();
        assert_eq!(cfg.base_url, DEFAULT_BASE_URL);
        assert_eq!(cfg.timeout, Duration::from_secs(30));
        assert!(cfg.user_agent.starts_with("blooio-rs/"));
    }

    #[test]
    fn with_base_url_trims_trailing_slashes() {
        let one = ClientConfig::new().with_base_url("https://example.com/api/");
        assert_eq!(one.base_url, "https://example.com/api");
        let many = ClientConfig::new().with_base_url("https://example.com/api///");
        assert_eq!(many.base_url, "https://example.com/api");
        let none = ClientConfig::new().with_base_url("https://example.com/api");
        assert_eq!(none.base_url, "https://example.com/api");
    }

    #[test]
    fn url_for_concatenates_base_and_path() {
        let cfg = ClientConfig::new().with_base_url("https://example.com/api");
        assert_eq!(cfg.url_for("/me"), "https://example.com/api/me");
        assert_eq!(
            cfg.url_for("/chats/c1/messages"),
            "https://example.com/api/chats/c1/messages"
        );
    }

    #[cfg(feature = "sensitive-diagnostics")]
    #[test]
    fn sensitive_diagnostics_can_be_set_and_cleared() {
        let cfg = ClientConfig::new()
            .with_sensitive_diagnostics(SensitiveDiagnostics::noop())
            .with_sensitive_tracing();
        assert!(cfg.sensitive_diagnostics.is_some());
        assert!(cfg.sensitive_tracing);
        let dbg = format!("{cfg:?}");
        assert!(dbg.contains("sensitive_diagnostics"));
        assert!(dbg.contains("sensitive_tracing"));
        assert!(dbg.contains("REDACTED"));

        let cfg = cfg
            .without_sensitive_diagnostics()
            .without_sensitive_tracing();
        assert!(cfg.sensitive_diagnostics.is_none());
        assert!(!cfg.sensitive_tracing);
    }

    #[test]
    fn from_env_values_uses_defaults_without_base_url() {
        let cfg = ClientConfig::from_env_values(None).unwrap();
        assert_eq!(cfg.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn from_env_values_uses_base_url_override() {
        let cfg = ClientConfig::from_env_values(Some("https://example.com/api/".into())).unwrap();
        assert_eq!(cfg.base_url, "https://example.com/api");
    }

    #[test]
    fn from_env_values_rejects_invalid_base_url() {
        let err = ClientConfig::from_env_values(Some("nope".into())).unwrap_err();
        assert!(matches!(err, Error::Config(_)));
    }

    #[test]
    fn from_env_values_rejects_base_url_query() {
        let err =
            ClientConfig::from_env_values(Some("https://example.com/api?token=secret".into()))
                .unwrap_err();
        assert!(matches!(err, Error::Config(_)));
    }

    #[test]
    fn validate_rejects_programmatic_base_url_query() {
        let err = ClientConfig::new()
            .with_base_url("https://example.com/api?token=secret")
            .validate()
            .unwrap_err();
        assert!(matches!(err, Error::Config(_)));
        assert!(!err.to_string().contains("secret"));
    }

    #[test]
    fn validate_rejects_invalid_user_agent_without_reflecting_it() {
        let err = ClientConfig::new()
            .with_user_agent("bad\nsecret")
            .validate()
            .unwrap_err();
        assert!(matches!(err, Error::Config(_)));
        assert!(!err.to_string().contains("secret"));
    }
}
