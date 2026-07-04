//! Blocking executor backed by [`ureq`]. Pulls no async runtime.

use http::header::{AUTHORIZATION, CONTENT_TYPE};

#[cfg(feature = "sensitive-diagnostics")]
use crate::client::sensitive::{SensitiveAttempt, SensitiveAttemptParts};
#[cfg(feature = "tracing")]
use crate::client::trace::{self, OperationTrace};
use crate::config::ClientConfig;
#[cfg(feature = "sensitive-diagnostics")]
use crate::core::diagnostics::SensitiveTransportErrorStage;
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
        let retry = options.retry_or(self.config.retry);
        let max_retries = retry.max_retries;
        let operation_type = std::any::type_name::<O>();
        #[cfg(feature = "tracing")]
        let operation_trace =
            OperationTrace::new(operation_type, max_retries, options.trace_label.as_deref());

        let mut spec = match RequestSpec::build(&op) {
            Ok(spec) => spec,
            Err(e) => {
                #[cfg(feature = "tracing")]
                operation_trace.failure(&O::METHOD, 0, None, &e);
                return Err(e);
            }
        };
        spec.apply_options(&options);
        // A retried mutating request must be idempotent.
        if max_retries > 0 {
            spec.ensure_idempotency_key();
        }
        let url = url_with_query(&options.url_for(&self.config, &spec.path), &spec.query);

        let mut retries_done = 0u32;
        loop {
            let attempt = retries_done + 1;
            match self.send_raw_once(&spec, &url, &options, operation_type, attempt, max_retries) {
                Ok(raw) => {
                    let meta = ResponseMeta::from_headers(raw.status, &raw.headers);
                    match parse_with(raw.status, &raw.body, meta.retry_after) {
                        Ok(output) => {
                            #[cfg(feature = "tracing")]
                            operation_trace.success(&spec.method, attempt, raw.status);
                            return Ok(ApiResponse { output, meta, raw });
                        }
                        Err(e) if retry.should_retry(retries_done, &e) => {
                            let delay = retry.delay_for(retries_done, &e);
                            #[cfg(feature = "tracing")]
                            operation_trace.retry(&spec.method, attempt, attempt + 1, delay, &e);
                            std::thread::sleep(delay);
                            retries_done += 1;
                        }
                        Err(e) => {
                            #[cfg(feature = "tracing")]
                            operation_trace.failure(&spec.method, attempt, Some(raw.status), &e);
                            return Err(e);
                        }
                    }
                }
                Err(e) if retry.should_retry(retries_done, &e) => {
                    let delay = retry.delay_for(retries_done, &e);
                    #[cfg(feature = "tracing")]
                    operation_trace.retry(&spec.method, attempt, attempt + 1, delay, &e);
                    std::thread::sleep(delay);
                    retries_done += 1;
                }
                Err(e) => {
                    #[cfg(feature = "tracing")]
                    operation_trace.failure(&spec.method, attempt, None, &e);
                    return Err(e);
                }
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
        attempt: u32,
        max_retries: u32,
    ) -> Result<RawResponse> {
        #[cfg(not(any(feature = "tracing", feature = "sensitive-diagnostics")))]
        let _ = (operation_type, attempt, max_retries);
        #[cfg(feature = "tracing")]
        let attempt_trace = trace::AttemptTrace::new(
            &spec.method,
            operation_type,
            attempt,
            max_retries,
            options.trace_label.as_deref(),
        );
        #[cfg(feature = "tracing")]
        let span = trace::request_span(&attempt_trace);
        #[cfg(feature = "tracing")]
        let start = std::time::Instant::now();
        #[cfg(feature = "sensitive-diagnostics")]
        let sensitive = SensitiveAttempt::new(SensitiveAttemptParts {
            config: &self.config,
            options,
            spec,
            url,
            auth_header: self.auth_header.expose(),
            operation: operation_type,
            attempt,
            max_retries,
        });

        #[cfg(feature = "tracing")]
        let _enter = span.enter();
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

        let response = if let Some(body) = &spec.body {
            if !spec
                .headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case(CONTENT_TYPE.as_str()))
            {
                builder = builder.header(CONTENT_TYPE, "application/json");
            }
            self.run_built_request(
                builder.body(body.as_ref()),
                options,
                #[cfg(feature = "sensitive-diagnostics")]
                &sensitive,
            )
        } else {
            self.run_built_request(
                builder.body(()),
                options,
                #[cfg(feature = "sensitive-diagnostics")]
                &sensitive,
            )
        };
        let raw = response.and_then(|resp| {
            Self::read_raw_response(
                resp,
                #[cfg(feature = "sensitive-diagnostics")]
                &sensitive,
            )
        });
        #[cfg(feature = "tracing")]
        match &raw {
            Ok(resp) => {
                trace::attempt_response(&span, &attempt_trace, resp.status, start.elapsed());
            }
            Err(e) => {
                trace::attempt_error(&span, &attempt_trace, start.elapsed(), e);
            }
        }

        raw
    }

    fn run_built_request<S: ureq::AsSendBody>(
        &self,
        request: std::result::Result<http::Request<S>, http::Error>,
        options: &RequestOptions,
        #[cfg(feature = "sensitive-diagnostics")] sensitive: &SensitiveAttempt<'_>,
    ) -> Result<http::Response<ureq::Body>> {
        let request = match request {
            Ok(request) => request,
            Err(e) => {
                let raw_error = e.to_string();
                #[cfg(feature = "sensitive-diagnostics")]
                sensitive.transport_error(
                    SensitiveTransportErrorStage::BuildRequest,
                    raw_error.clone(),
                );
                return Err(Error::transport(&raw_error));
            }
        };

        #[cfg(feature = "sensitive-diagnostics")]
        sensitive.request();
        match self.run_request(request, options) {
            Ok(response) => Ok(response),
            Err(e) => {
                let raw_error = e.to_string();
                #[cfg(feature = "sensitive-diagnostics")]
                sensitive.transport_error(SensitiveTransportErrorStage::Send, raw_error.clone());
                Err(Error::transport(&raw_error))
            }
        }
    }

    fn read_raw_response(
        mut resp: http::Response<ureq::Body>,
        #[cfg(feature = "sensitive-diagnostics")] sensitive: &SensitiveAttempt<'_>,
    ) -> Result<RawResponse> {
        let status = resp.status().as_u16();
        let headers = resp.headers().clone();
        match resp.body_mut().read_to_vec() {
            Ok(bytes) => {
                let raw = RawResponse::new(status, headers, bytes.into());
                #[cfg(feature = "sensitive-diagnostics")]
                sensitive.response(&raw);
                Ok(raw)
            }
            Err(e) => {
                let raw_error = e.to_string();
                #[cfg(feature = "sensitive-diagnostics")]
                sensitive
                    .transport_error(SensitiveTransportErrorStage::ReadBody, raw_error.clone());
                Err(Error::transport(&raw_error))
            }
        }
    }

    fn run_request<S: ureq::AsSendBody>(
        &self,
        request: http::Request<S>,
        options: &RequestOptions,
    ) -> std::result::Result<http::Response<ureq::Body>, ureq::Error> {
        let request = if let Some(timeout) = options.timeout {
            self.agent
                .configure_request(request)
                .timeout_global(Some(timeout))
                .build()
        } else {
            request
        };
        self.agent.run(request)
    }
}
