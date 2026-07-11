//! Async executor backed by [`reqwest`].

use http::header::{AUTHORIZATION, CONTENT_TYPE};
#[cfg(feature = "tracing")]
use tracing::Instrument as _;

use crate::client::AttemptContext;
#[cfg(feature = "sensitive-diagnostics")]
use crate::client::sensitive::SensitiveAttempt;
#[cfg(feature = "tracing")]
use crate::client::trace::{self, OperationTrace};
use crate::config::{ClientConfig, DEFAULT_MAX_RESPONSE_BODY_BYTES};
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

/// Asynchronous Blooio API client.
///
/// Cheap to clone (the underlying `reqwest::Client` is reference-counted and
/// maintains its own connection pool).
///
/// Construct one `Client` per base URL/transport configuration and reuse it
/// across account-scoped API handles. Creating a fresh client for each request
/// defeats connection reuse.
#[derive(Clone, Debug)]
pub struct Client {
    config: ClientConfig,
    http: reqwest::Client,
    max_response_body_bytes: usize,
}

/// Account-scoped asynchronous Blooio API handle.
#[derive(Clone, Copy, Debug)]
pub struct BlooioAccount<'a> {
    pub(crate) client: &'a Client,
    pub(crate) creds: &'a BlooioCreds,
}

impl Client {
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
        config.validate()?;
        let builder = reqwest::Client::builder()
            .timeout(config.timeout)
            .user_agent(config.user_agent.clone())
            .redirect(reqwest::redirect::Policy::none());
        #[cfg(feature = "native-tls")]
        let builder = builder.tls_backend_native();
        #[cfg(all(feature = "rustls", not(feature = "native-tls")))]
        let builder = builder.tls_backend_rustls();
        let http = builder.build().map_err(Error::reqwest_transport)?;
        Self::try_from_config_and_http_client(config, http)
    }

    /// Build a client from configuration and a caller-provided [`reqwest::Client`].
    ///
    /// This lets applications reuse an existing connection pool, proxy setup,
    /// DNS resolver, and middleware-compatible timeout policy. The supplied
    /// client is used as-is; values such as [`ClientConfig::timeout`] and
    /// [`ClientConfig::user_agent`] are not applied to it by this constructor.
    /// Use [`Self::try_from_config_and_http_client`] when configuration should
    /// be validated before construction.
    #[must_use]
    pub fn from_config_and_http_client(config: ClientConfig, http: reqwest::Client) -> Self {
        Client {
            config,
            http,
            max_response_body_bytes: DEFAULT_MAX_RESPONSE_BODY_BYTES,
        }
    }

    /// Build a client from configuration and a caller-provided transport,
    /// validating the configuration first.
    pub fn try_from_config_and_http_client(
        config: ClientConfig,
        http: reqwest::Client,
    ) -> Result<Self> {
        config.validate()?;
        Ok(Self::from_config_and_http_client(config, http))
    }

    /// Override the maximum response body size retained in memory.
    #[must_use]
    pub fn with_max_response_body_bytes(mut self, max_response_body_bytes: usize) -> Self {
        self.max_response_body_bytes = max_response_body_bytes;
        self
    }

    /// The configuration this client was built with.
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Create an account-scoped API handle. Credentials are borrowed and are
    /// not retained by the root client.
    #[must_use]
    pub fn account<'a>(&'a self, creds: &'a BlooioCreds) -> BlooioAccount<'a> {
        BlooioAccount {
            client: self,
            creds,
        }
    }
}

impl BlooioAccount<'_> {
    /// Execute an [`Operation`] and decode its response.
    ///
    /// This is the single async IO entry point for an authenticated account
    /// handle; every resource method delegates here. It is also the public
    /// escape hatch for operations not covered by a convenience method.
    pub async fn send<O: Operation>(self, op: O) -> Result<O::Output> {
        self.send_with_meta(op).await.map(|(out, _meta)| out)
    }

    /// Execute an [`Operation`] with request-scoped transport options.
    pub async fn send_with_options<O: Operation>(
        self,
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
    pub async fn send_with_meta<O: Operation>(self, op: O) -> Result<(O::Output, ResponseMeta)> {
        self.send_with_meta_with_options(op, RequestOptions::new())
            .await
    }

    /// Execute an [`Operation`] with request-scoped transport options, returning
    /// the decoded output and parsed response metadata.
    pub async fn send_with_meta_with_options<O: Operation>(
        self,
        op: O,
        options: RequestOptions,
    ) -> Result<(O::Output, ResponseMeta)> {
        let response = self.send_with_response_with_options(op, options).await?;
        Ok((response.output, response.meta))
    }

    /// Execute an [`Operation`] and return decoded output plus raw HTTP data.
    pub async fn send_with_response<O: Operation>(self, op: O) -> Result<ApiResponse<O::Output>> {
        self.send_with_response_with_options(op, RequestOptions::new())
            .await
    }

    /// Execute an [`Operation`] with request-scoped transport options and
    /// return decoded output plus raw HTTP data.
    pub async fn send_with_response_with_options<O: Operation>(
        self,
        op: O,
        options: RequestOptions,
    ) -> Result<ApiResponse<O::Output>> {
        options.validate_base_url()?;
        let operation_retry_safe = matches!(
            O::METHOD,
            http::Method::GET | http::Method::HEAD | http::Method::OPTIONS
        ) || O::RETRY_SAFE;
        let retry = if operation_retry_safe {
            options.retry_or(self.client.config.retry)
        } else {
            crate::RetryPolicy::none()
        };
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
        if max_retries > 0 && O::RETRY_SAFE {
            spec.ensure_idempotency_key();
        }
        let url = url_with_query(
            &options.url_for(&self.client.config, &spec.path),
            &spec.query,
        );

        let mut retries_done = 0u32;
        loop {
            let attempt = retries_done + 1;
            match self
                .client
                .send_raw_once(AttemptContext {
                    creds: self.creds,
                    spec: &spec,
                    url: &url,
                    options: &options,
                    operation_type,
                    attempt,
                    max_retries,
                })
                .await
            {
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
                            tokio::time::sleep(delay).await;
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
                    tokio::time::sleep(delay).await;
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

impl Client {
    /// A single request attempt: build, send, and read the raw body.
    async fn send_raw_once(&self, ctx: AttemptContext<'_>) -> Result<RawResponse> {
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
        let sensitive = SensitiveAttempt::new(crate::client::sensitive::SensitiveAttemptParts {
            config: &self.config,
            options: ctx.options,
            spec: ctx.spec,
            url: ctx.url,
            auth_header: auth_header.expose(),
            operation: ctx.operation_type,
            attempt: ctx.attempt,
            max_retries: ctx.max_retries,
        });

        let mut req = self.http.request(ctx.spec.method.clone(), ctx.url);
        // The key is exposed only here, to set the header. It is never logged.
        req = req.header(AUTHORIZATION, auth_header.expose().as_str());
        for (k, v) in &ctx.spec.headers {
            if k.eq_ignore_ascii_case(AUTHORIZATION.as_str()) {
                continue;
            }
            req = req.header(k.as_str(), v.as_str());
        }
        if let Some(body) = &ctx.spec.body {
            if !ctx
                .spec
                .headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case(CONTENT_TYPE.as_str()))
            {
                req = req.header(CONTENT_TYPE, "application/json");
            }
            req = req.body(body.clone());
        }
        if let Some(timeout) = ctx.options.timeout {
            req = req.timeout(timeout);
        }

        let send = Self::execute_request(
            &self.http,
            req,
            ctx.options
                .max_response_body_bytes
                .unwrap_or(self.max_response_body_bytes),
            #[cfg(feature = "sensitive-diagnostics")]
            &sensitive,
        );
        #[cfg(feature = "tracing")]
        let result = send.instrument(span.clone()).await;
        #[cfg(not(feature = "tracing"))]
        let result = send.await;

        #[cfg(feature = "tracing")]
        match &result {
            Ok(resp) => {
                trace::attempt_response(&span, &attempt_trace, resp.status, start.elapsed());
            }
            Err(e) => {
                trace::attempt_error(&span, &attempt_trace, start.elapsed(), e);
            }
        }

        result
    }

    async fn execute_request(
        http: &reqwest::Client,
        req: reqwest::RequestBuilder,
        max_response_body_bytes: usize,
        #[cfg(feature = "sensitive-diagnostics")] sensitive: &SensitiveAttempt<'_>,
    ) -> Result<RawResponse> {
        let req = match req.build() {
            Ok(req) => req,
            Err(e) => {
                #[cfg(feature = "sensitive-diagnostics")]
                sensitive
                    .transport_error(SensitiveTransportErrorStage::BuildRequest, e.to_string());
                #[cfg(not(feature = "sensitive-diagnostics"))]
                let _ = e;
                return Err(Error::request_build());
            }
        };

        #[cfg(feature = "sensitive-diagnostics")]
        sensitive.request();
        match http.execute(req).await {
            Ok(mut resp) => {
                let status = resp.status().as_u16();
                let headers = resp.headers().clone();
                let mut bytes = bytes::BytesMut::new();
                loop {
                    match resp.chunk().await {
                        Ok(Some(chunk)) => {
                            if bytes.len().saturating_add(chunk.len()) > max_response_body_bytes {
                                return Err(Error::ResponseBodyTooLarge {
                                    limit: max_response_body_bytes,
                                });
                            }
                            bytes.extend_from_slice(&chunk);
                        }
                        Ok(None) => {
                            let raw = RawResponse::new(status, headers, bytes.freeze());
                            #[cfg(feature = "sensitive-diagnostics")]
                            sensitive.response(&raw);
                            return Ok(raw);
                        }
                        Err(e) => {
                            #[cfg(feature = "sensitive-diagnostics")]
                            sensitive.transport_error(
                                SensitiveTransportErrorStage::ReadBody,
                                e.to_string(),
                            );
                            return Err(Error::reqwest_transport(e));
                        }
                    }
                }
            }
            Err(e) => {
                #[cfg(feature = "sensitive-diagnostics")]
                sensitive.transport_error(SensitiveTransportErrorStage::Send, e.to_string());
                Err(Error::reqwest_transport(e))
            }
        }
    }
}
