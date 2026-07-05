//! Blocking executor backed by [`ureq`]. Pulls no async runtime.

use http::header::{AUTHORIZATION, CONTENT_TYPE};

use crate::client::AttemptContext;
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
use crate::credentials::BlooioCreds;
use crate::error::{Error, Result};

/// Blocking Blooio API client.
///
/// A thin wrapper over a [`ureq::Agent`]; cloning shares the connection pool.
///
/// Construct one `BlockingClient` per base URL/transport configuration and
/// reuse or clone it across account-scoped API handles. Creating a fresh client
/// for each request defeats connection reuse.
#[derive(Clone, Debug)]
pub struct BlockingClient {
    config: ClientConfig,
    agent: ureq::Agent,
}

/// Account-scoped blocking Blooio API handle.
#[derive(Clone, Copy, Debug)]
pub struct BlockingBlooioAccount<'a> {
    pub(crate) client: &'a BlockingClient,
    pub(crate) creds: &'a BlooioCreds,
}

impl BlockingClient {
    /// Build a client using production defaults.
    pub fn new() -> Result<Self> {
        Self::from_config(ClientConfig::new())
    }

    /// Build a client from environment variables.
    ///
    /// Reads transport configuration such as `BLOOIO_BASE_URL`. Credentials
    /// are intentionally separate; use [`BlooioCreds::from_env`].
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
        BlockingClient { config, agent }
    }

    /// The configuration this client was built with.
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Create an account-scoped API handle. Credentials are borrowed and are
    /// not retained by the root client.
    #[must_use]
    pub fn account<'a>(&'a self, creds: &'a BlooioCreds) -> BlockingBlooioAccount<'a> {
        BlockingBlooioAccount {
            client: self,
            creds,
        }
    }
}

impl BlockingBlooioAccount<'_> {
    /// Execute an [`Operation`] and decode its response.
    ///
    /// The single blocking IO entry point for an authenticated account handle;
    /// every resource method delegates here. Also the public escape hatch for
    /// uncovered operations.
    // Takes `op` by value to mirror the async `Client::send` signature; this
    // path only needs a borrow, but API symmetry across the two clients wins.
    #[allow(clippy::needless_pass_by_value)]
    pub fn send<O: Operation>(self, op: O) -> Result<O::Output> {
        self.send_with_meta(op).map(|(out, _meta)| out)
    }

    /// Execute an [`Operation`] with request-scoped transport options.
    #[allow(clippy::needless_pass_by_value)]
    pub fn send_with_options<O: Operation>(
        self,
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
    pub fn send_with_meta<O: Operation>(self, op: O) -> Result<(O::Output, ResponseMeta)> {
        self.send_with_meta_with_options(op, RequestOptions::new())
    }

    /// Execute an [`Operation`] with request-scoped transport options, returning
    /// the decoded output and parsed response metadata.
    #[allow(clippy::needless_pass_by_value)]
    pub fn send_with_meta_with_options<O: Operation>(
        self,
        op: O,
        options: RequestOptions,
    ) -> Result<(O::Output, ResponseMeta)> {
        let response = self.send_with_response_with_options(op, options)?;
        Ok((response.output, response.meta))
    }

    /// Execute an [`Operation`] and return decoded output plus raw HTTP data.
    #[allow(clippy::needless_pass_by_value)]
    pub fn send_with_response<O: Operation>(self, op: O) -> Result<ApiResponse<O::Output>> {
        self.send_with_response_with_options(op, RequestOptions::new())
    }

    /// Execute an [`Operation`] with request-scoped transport options and
    /// return decoded output plus raw HTTP data.
    #[allow(clippy::needless_pass_by_value)]
    pub fn send_with_response_with_options<O: Operation>(
        self,
        op: O,
        options: RequestOptions,
    ) -> Result<ApiResponse<O::Output>> {
        let retry = options.retry_or(self.client.config.retry);
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
        let url = url_with_query(
            &options.url_for(&self.client.config, &spec.path),
            &spec.query,
        );

        let mut retries_done = 0u32;
        loop {
            let attempt = retries_done + 1;
            match self.client.send_raw_once(AttemptContext {
                creds: self.creds,
                spec: &spec,
                url: &url,
                options: &options,
                operation_type,
                attempt,
                max_retries,
            }) {
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
}

impl BlockingClient {
    /// A single request attempt: build, send, and read the raw body.
    fn send_raw_once(&self, ctx: AttemptContext<'_>) -> Result<RawResponse> {
        #[cfg(not(any(feature = "tracing", feature = "sensitive-diagnostics")))]
        let _ = (ctx.operation_type, ctx.attempt, ctx.max_retries);
        #[cfg(feature = "tracing")]
        let attempt_trace = trace::AttemptTrace::new(
            &ctx.spec.method,
            ctx.operation_type,
            ctx.attempt,
            ctx.max_retries,
            ctx.options.trace_label.as_deref(),
        );
        #[cfg(feature = "tracing")]
        let span = trace::request_span(&attempt_trace);
        #[cfg(feature = "tracing")]
        let start = std::time::Instant::now();
        let auth_header = ctx.creds.bearer_header();
        #[cfg(feature = "sensitive-diagnostics")]
        let sensitive = SensitiveAttempt::new(SensitiveAttemptParts {
            config: &self.config,
            options: ctx.options,
            spec: ctx.spec,
            url: ctx.url,
            auth_header: auth_header.expose(),
            operation: ctx.operation_type,
            attempt: ctx.attempt,
            max_retries: ctx.max_retries,
        });

        #[cfg(feature = "tracing")]
        let _enter = span.enter();
        let mut builder = http::Request::builder()
            .method(ctx.spec.method.clone())
            .uri(ctx.url);
        // Key exposed only to set the header; never logged. The User-Agent is
        // configured on the agent at build time, not per-request.
        builder = builder.header(AUTHORIZATION, auth_header.expose().as_str());
        for (k, v) in &ctx.spec.headers {
            if k.eq_ignore_ascii_case(AUTHORIZATION.as_str()) {
                continue;
            }
            builder = builder.header(k.as_str(), v.as_str());
        }

        let response = if let Some(body) = &ctx.spec.body {
            if !ctx
                .spec
                .headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case(CONTENT_TYPE.as_str()))
            {
                builder = builder.header(CONTENT_TYPE, "application/json");
            }
            self.run_built_request(
                builder.body(body.as_ref()),
                ctx.options,
                #[cfg(feature = "sensitive-diagnostics")]
                &sensitive,
            )
        } else {
            self.run_built_request(
                builder.body(()),
                ctx.options,
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
