//! Blocking executor backed by [`ureq`]. Pulls no async runtime.

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

/// Blocking Blooio API client.
///
/// A thin wrapper over a [`ureq::Agent`]; cloning shares the connection pool.
///
/// Construct one `BlockingClient` per API key/base URL and reuse or clone it
/// across requests. Creating a fresh client for each request defeats connection
/// reuse.
/// Resource accessors mirror those on the async [`Client`](crate::Client).
#[derive(Clone, Debug)]
pub struct BlockingClient {
    config: ClientConfig,
    agent: ureq::Agent,
    // Precomputed `Bearer <key>` header value. Built once (the key never
    // changes after construction) and kept in `Secret` so it stays redacted.
    auth_header: Secret<String>,
}

impl BlockingClient {
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
        let builder = ureq::Agent::config_builder()
            // We read non-2xx bodies ourselves to map them to `Error::Api`.
            .http_status_as_error(false)
            .user_agent(&config.user_agent)
            .timeout_global(Some(config.timeout));

        // When the native-tls backend is selected (and rustls is not), point
        // ureq at the native-tls provider explicitly.
        #[cfg(all(feature = "native-tls", not(feature = "rustls")))]
        let builder = builder.tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .build(),
        );

        let agent: ureq::Agent = builder.build().into();
        Ok(Self::from_config_and_agent(config, agent))
    }

    /// Build a client from configuration and a caller-provided [`ureq::Agent`].
    ///
    /// This lets applications reuse an existing connection pool, proxy setup,
    /// DNS resolver, and transport policy. The supplied agent is used as-is;
    /// values such as [`ClientConfig::timeout`] and
    /// [`ClientConfig::user_agent`] are not applied to it by this constructor.
    pub fn from_config_and_agent(config: ClientConfig, agent: ureq::Agent) -> Self {
        let auth_header = Secret::new(format!("Bearer {}", config.api_key.expose()));
        BlockingClient {
            config,
            agent,
            auth_header,
        }
    }

    /// The configuration this client was built with.
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Execute an [`Operation`] and decode its response.
    ///
    /// The single blocking IO entry point; every resource method delegates
    /// here. Also the public escape hatch for uncovered operations.
    // Takes `op` by value to mirror the async `Client::send` signature; this
    // path only needs a borrow, but API symmetry across the two clients wins.
    #[allow(clippy::needless_pass_by_value)]
    pub fn send<O: Operation>(&self, op: O) -> Result<O::Output> {
        self.send_with_meta(op).map(|(out, _meta)| out)
    }

    /// Execute an [`Operation`] with request-scoped transport options.
    #[allow(clippy::needless_pass_by_value)]
    pub fn send_with_options<O: Operation>(
        &self,
        op: O,
        options: RequestOptions,
    ) -> Result<O::Output> {
        self.send_with_meta_with_options(op, options)
            .map(|(out, _meta)| out)
    }

    /// Execute an [`Operation`] and decode its response, also returning the
    /// [`ResponseMeta`] (rate-limit headers and `Retry-After`) from the HTTP
    /// response. Use this when you want to self-pace against the API's limits.
    #[allow(clippy::needless_pass_by_value)]
    pub fn send_with_meta<O: Operation>(&self, op: O) -> Result<(O::Output, ResponseMeta)> {
        self.send_with_meta_with_options(op, RequestOptions::new())
    }

    /// Execute an [`Operation`] with request-scoped transport options, returning
    /// the decoded output and parsed response metadata.
    #[allow(clippy::needless_pass_by_value)]
    pub fn send_with_meta_with_options<O: Operation>(
        &self,
        op: O,
        options: RequestOptions,
    ) -> Result<(O::Output, ResponseMeta)> {
        let response = self.send_with_response_with_options(op, options)?;
        Ok((response.output, response.meta))
    }

    /// Execute an [`Operation`] and return decoded output plus raw HTTP data.
    #[allow(clippy::needless_pass_by_value)]
    pub fn send_with_response<O: Operation>(&self, op: O) -> Result<ApiResponse<O::Output>> {
        self.send_with_response_with_options(op, RequestOptions::new())
    }

    /// Execute an [`Operation`] with request-scoped transport options and
    /// return decoded output plus raw HTTP data.
    #[allow(clippy::needless_pass_by_value)]
    pub fn send_with_response_with_options<O: Operation>(
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
            match self.send_raw_once(&spec, &url, &options, operation_type) {
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
                            std::thread::sleep(delay);
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
                    std::thread::sleep(delay);
                    retries_done += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// A single request attempt: build, send, and read the raw body.
    fn send_raw_once(
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
        let _enter = span.enter();
        #[cfg(feature = "tracing")]
        let start = std::time::Instant::now();

        let mut builder = http::Request::builder()
            .method(spec.method.clone())
            .uri(url);
        // Key exposed only to set the header; never logged. The User-Agent is
        // configured on the agent at build time, not per-request.
        builder = builder.header(AUTHORIZATION, self.auth_header.expose().as_str());
        for (k, v) in &spec.headers {
            if k.eq_ignore_ascii_case(AUTHORIZATION.as_str()) {
                continue;
            }
            builder = builder.header(k.as_str(), v.as_str());
        }

        let result = if let Some(body) = &spec.body {
            if !spec
                .headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case(CONTENT_TYPE.as_str()))
            {
                builder = builder.header(CONTENT_TYPE, "application/json");
            }
            let request = builder.body(body.as_ref()).map_err(Error::transport)?;
            self.run_request(request, options)
        } else {
            let request = builder.body(()).map_err(Error::transport)?;
            self.run_request(request, options)
        };

        let mut resp = result?;
        let status = resp.status().as_u16();
        let headers = resp.headers().clone();
        let bytes = resp.body_mut().read_to_vec().map_err(Error::transport)?;
        let raw: Result<RawResponse> = Ok(RawResponse::new(status, headers, bytes.into()));

        #[cfg(feature = "tracing")]
        {
            if let Ok(raw) = &raw {
                span.record("status", raw.status);
            }
            span.record(
                "elapsed_ms",
                u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
            );
            match &raw {
                Ok(_) => tracing::debug!("request completed"),
                Err(e) => tracing::warn!(code = ?e.code(), "request failed"),
            }
        }

        raw
    }

    fn run_request<S: ureq::AsSendBody>(
        &self,
        request: http::Request<S>,
        options: &RequestOptions,
    ) -> Result<http::Response<ureq::Body>> {
        let request = if let Some(timeout) = options.timeout {
            self.agent
                .configure_request(request)
                .timeout_global(Some(timeout))
                .build()
        } else {
            request
        };
        self.agent.run(request).map_err(Error::transport)
    }
}
