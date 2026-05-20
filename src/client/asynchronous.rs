//! Async executor backed by [`reqwest`].

use http::header::{AUTHORIZATION, CONTENT_TYPE};

use crate::config::ClientConfig;
use crate::core::operation::Operation;
use crate::core::ratelimit::ResponseMeta;
use crate::core::request::RequestSpec;
use crate::core::response::parse_with;
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
        self.send_with_meta(op).await.map(|(out, _meta)| out)
    }

    /// Execute an [`Operation`] and decode its response, also returning the
    /// [`ResponseMeta`] (rate-limit headers and `Retry-After`) from the HTTP
    /// response. Use this when you want to self-pace against the API's limits.
    pub async fn send_with_meta<O: Operation>(&self, op: O) -> Result<(O::Output, ResponseMeta)> {
        let mut spec = RequestSpec::build(&op)?;
        let retry = self.config.retry;
        // A retried mutating request must be idempotent.
        if retry.max_retries > 0 {
            spec.ensure_idempotency_key();
        }
        let url = self.config.url_for(&spec.path);

        let mut retries_done = 0u32;
        loop {
            match self.send_once::<O>(&spec, &url).await {
                Ok(v) => return Ok(v),
                Err(e) if retry.should_retry(retries_done, &e) => {
                    let delay = retry.delay_for(retries_done, &e);
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        attempt = retries_done + 1,
                        delay_ms = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX),
                        code = ?e.code(),
                        "retrying request after transient failure"
                    );
                    tokio::time::sleep(delay).await;
                    retries_done += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// A single request attempt: build, send, and decode.
    async fn send_once<O: Operation>(
        &self,
        spec: &RequestSpec,
        url: &str,
    ) -> Result<(O::Output, ResponseMeta)> {
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

        let mut req = self.http.request(spec.method.clone(), url);
        if !spec.query.is_empty() {
            req = req.query(&spec.query);
        }
        // The key is exposed only here, to set the header. It is never logged.
        req = req.header(AUTHORIZATION, self.auth_header.expose().as_str());
        for (k, v) in &spec.headers {
            req = req.header(*k, v);
        }
        if let Some(body) = &spec.body {
            req = req
                .header(CONTENT_TYPE, "application/json")
                .body(body.clone());
        }

        let resp = req.send().await.map_err(Error::transport)?;
        let status = resp.status().as_u16();
        let meta = ResponseMeta::from_headers(status, resp.headers());
        let bytes = resp.bytes().await.map_err(Error::transport)?;

        let result = parse_with(status, &bytes, meta.retry_after).map(|out| (out, meta));

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
