//! Async executor backed by [`reqwest`].

use http::header::{AUTHORIZATION, CONTENT_TYPE};

use crate::config::ClientConfig;
use crate::core::operation::Operation;
use crate::core::request::RequestSpec;
use crate::core::response::parse;
use crate::error::{Error, Result};
use crate::secret::Secret;

/// Asynchronous Blooio API client.
///
/// Cheap to clone (the underlying `reqwest::Client` is reference-counted).
/// Resource accessors (`client.contacts()`, `client.chat(id)`, …) are defined
/// across the [`crate::resources`] modules.
#[derive(Clone, Debug)]
pub struct Client {
    config: ClientConfig,
    http: reqwest::Client,
    // Precomputed `Bearer <key>` header value. Built once (the key never
    // changes after construction) and kept in `Secret` so it stays redacted.
    auth_header: Secret<String>,
}

impl Client {
    /// Build a client from an API key using production defaults.
    pub fn new(api_key: impl Into<Secret<String>>) -> Result<Self> {
        Self::from_config(ClientConfig::new(api_key))
    }

    /// Build a client from a full [`ClientConfig`].
    pub fn from_config(config: ClientConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .user_agent(config.user_agent.clone())
            .build()
            .map_err(Error::transport)?;
        let auth_header = Secret::new(format!("Bearer {}", config.api_key.expose()));
        Ok(Client {
            config,
            http,
            auth_header,
        })
    }

    /// The configuration this client was built with.
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Execute an [`Operation`] and decode its response.
    ///
    /// This is the single async IO entry point; every resource method delegates
    /// here. It is also the public escape hatch for operations not covered by a
    /// convenience method.
    pub async fn send<O: Operation>(&self, op: O) -> Result<O::Output> {
        let spec = RequestSpec::build(&op)?;
        let url = self.config.url_for(&spec.path);

        #[cfg(feature = "tracing")]
        let span = tracing::info_span!(
            "blooio.request",
            method = %spec.method,
            path = %spec.path,
            status = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty,
        );
        #[cfg(feature = "tracing")]
        let start = std::time::Instant::now();

        let mut req = self.http.request(spec.method, &url);
        if !spec.query.is_empty() {
            req = req.query(&spec.query);
        }
        // The key is exposed only here, to set the header. It is never logged.
        req = req.header(AUTHORIZATION, self.auth_header.expose().as_str());
        for (k, v) in &spec.headers {
            req = req.header(*k, v);
        }
        if let Some(body) = spec.body {
            req = req.header(CONTENT_TYPE, "application/json").body(body);
        }

        let resp = req.send().await.map_err(Error::transport)?;
        let status = resp.status().as_u16();
        let bytes = resp.bytes().await.map_err(Error::transport)?;

        let result = parse(status, &bytes);

        #[cfg(feature = "tracing")]
        {
            span.record("status", status);
            span.record(
                "elapsed_ms",
                u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
            );
            let _e = span.enter();
            match &result {
                Ok(_) => tracing::debug!("request completed"),
                Err(e) => tracing::warn!(code = ?e.code(), "request failed"),
            }
        }

        result
    }
}
