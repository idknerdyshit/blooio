//! Async executor backed by [`reqwest`].

use http::header::{AUTHORIZATION, CONTENT_TYPE};

use crate::config::ClientConfig;
use crate::core::operation::Operation;
use crate::core::options::RequestOptions;
use crate::core::ratelimit::ResponseMeta;
use crate::core::raw::{ApiResponse, RawResponse};
use crate::core::request::{RequestSpec, url_with_query};
use crate::core::response::parse_with;
use crate::error::{Error, Result};
use crate::secret::Secret;

/// Asynchronous Blooio API client.
///
/// Cheap to clone (the underlying `reqwest::Client` is reference-counted and
/// maintains its own connection pool).
///
/// Construct one `Client` per API key/base URL and reuse or clone it across
/// requests. Creating a fresh client for each request defeats connection reuse.
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

    /// Build a client from environment variables.
    ///
    /// Reads `BLOOIO_API_KEY` (required) and `BLOOIO_BASE_URL` (optional).
    pub fn from_env() -> Result<Self> {
        Self::from_config(ClientConfig::from_env()?)
    }

    /// Build a client from a full [`ClientConfig`].
    pub fn from_config(config: ClientConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .user_agent(config.user_agent.clone())
            .build()
            .map_err(Error::transport)?;
        Ok(Self::from_config_and_http_client(config, http))
    }

    /// Build a client from configuration and a caller-provided [`reqwest::Client`].
    ///
    /// This lets applications reuse an existing connection pool, proxy setup,
    /// DNS resolver, and middleware-compatible timeout policy. The supplied
    /// client is used as-is; values such as [`ClientConfig::timeout`] and
    /// [`ClientConfig::user_agent`] are not applied to it by this constructor.
    pub fn from_config_and_http_client(config: ClientConfig, http: reqwest::Client) -> Self {
        let auth_header = Secret::new(format!("Bearer {}", config.api_key.expose()));
        Client {
            config,
            http,
            auth_header,
        }
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

    /// Execute an [`Operation`] with request-scoped transport options.
    pub async fn send_with_options<O: Operation>(
        &self,
        op: O,
        options: RequestOptions,
    ) -> Result<O::Output> {
        self.send_with_meta_with_options(op, options)
            .await
            .map(|(out, _meta)| out)
    }

    /// Execute an [`Operation`] and decode its response, also returning the
    /// [`ResponseMeta`] (rate-limit headers and `Retry-After`) from the HTTP
    /// response. Use this when you want to self-pace against the API's limits.
    pub async fn send_with_meta<O: Operation>(&self, op: O) -> Result<(O::Output, ResponseMeta)> {
        self.send_with_meta_with_options(op, RequestOptions::new())
            .await
    }

    /// Execute an [`Operation`] with request-scoped transport options, returning
    /// the decoded output and parsed response metadata.
    pub async fn send_with_meta_with_options<O: Operation>(
        &self,
        op: O,
        options: RequestOptions,
    ) -> Result<(O::Output, ResponseMeta)> {
        let response = self.send_with_response_with_options(op, options).await?;
        Ok((response.output, response.meta))
    }

    /// Execute an [`Operation`] and return decoded output plus raw HTTP data.
    pub async fn send_with_response<O: Operation>(&self, op: O) -> Result<ApiResponse<O::Output>> {
        self.send_with_response_with_options(op, RequestOptions::new())
            .await
    }

    /// Execute an [`Operation`] with request-scoped transport options and
    /// return decoded output plus raw HTTP data.
    pub async fn send_with_response_with_options<O: Operation>(
        &self,
        op: O,
        options: RequestOptions,
    ) -> Result<ApiResponse<O::Output>> {
        let mut spec = RequestSpec::build(&op)?;
        spec.apply_options(&options);
        let retry = options.retry_or(self.config.retry);
        // A retried mutating request must be idempotent.
        if retry.max_retries > 0 {
            spec.ensure_idempotency_key();
        }
        let url = url_with_query(&options.url_for(&self.config, &spec.path), &spec.query);

        let mut retries_done = 0u32;
        let operation_type = std::any::type_name::<O>();
        loop {
            match self
                .send_raw_once(&spec, &url, &options, operation_type)
                .await
            {
                Ok(raw) => {
                    let meta = ResponseMeta::from_headers(raw.status, &raw.headers);
                    match parse_with(raw.status, &raw.body, meta.retry_after) {
                        Ok(output) => return Ok(ApiResponse { output, meta, raw }),
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

    /// A single request attempt: build, send, and read the raw body.
    async fn send_raw_once(
        &self,
        spec: &RequestSpec,
        url: &str,
        options: &RequestOptions,
        operation_type: &'static str,
    ) -> Result<RawResponse> {
        #[cfg(not(feature = "tracing"))]
        let _ = operation_type;
        #[cfg(feature = "tracing")]
        let span = tracing::info_span!(
            "blooio.request",
            method = %spec.method,
            operation = %operation_type,
            status = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty,
        );
        #[cfg(feature = "tracing")]
        let start = std::time::Instant::now();

        let mut req = self.http.request(spec.method.clone(), url);
        // The key is exposed only here, to set the header. It is never logged.
        req = req.header(AUTHORIZATION, self.auth_header.expose().as_str());
        for (k, v) in &spec.headers {
            if k.eq_ignore_ascii_case(AUTHORIZATION.as_str()) {
                continue;
            }
            req = req.header(k.as_str(), v.as_str());
        }
        if let Some(body) = &spec.body {
            if !spec
                .headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case(CONTENT_TYPE.as_str()))
            {
                req = req.header(CONTENT_TYPE, "application/json");
            }
            req = req.body(body.clone());
        }
        if let Some(timeout) = options.timeout {
            req = req.timeout(timeout);
        }

        let result = match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let headers = resp.headers().clone();
                let bytes = resp.bytes().await.map_err(Error::transport)?;
                Ok(RawResponse::new(status, headers, bytes))
            }
            Err(e) => Err(Error::transport(e)),
        };

        #[cfg(feature = "tracing")]
        {
            if let Ok(raw) = &result {
                span.record("status", raw.status);
            }
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
