//! Blocking executor backed by [`ureq`]. Pulls no async runtime.

use http::Method;
use http::header::AUTHORIZATION;

use crate::config::ClientConfig;
use crate::core::operation::Operation;
use crate::core::request::RequestSpec;
use crate::core::response::parse;
use crate::error::{Error, Result};
use crate::secret::Secret;

/// Blocking Blooio API client.
///
/// A thin wrapper over a [`ureq::Agent`]; cloning shares the connection pool.
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
        let auth_header = Secret::new(format!("Bearer {}", config.api_key.expose()));
        Ok(BlockingClient {
            config,
            agent,
            auth_header,
        })
    }

    /// The configuration this client was built with.
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    fn apply<B>(
        &self,
        mut rb: ureq::RequestBuilder<B>,
        spec: &RequestSpec,
    ) -> ureq::RequestBuilder<B> {
        for (k, v) in &spec.query {
            rb = rb.query(*k, v);
        }
        // Key exposed only to set the header; never logged. The User-Agent is
        // configured on the agent at build time, not per-request.
        rb = rb.header(AUTHORIZATION.as_str(), self.auth_header.expose().as_str());
        for (k, v) in &spec.headers {
            rb = rb.header(*k, v);
        }
        rb
    }

    /// Execute an [`Operation`] and decode its response.
    ///
    /// The single blocking IO entry point; every resource method delegates
    /// here. Also the public escape hatch for uncovered operations.
    // Takes `op` by value to mirror the async `Client::send` signature; this
    // path only needs a borrow, but API symmetry across the two clients wins.
    #[allow(clippy::needless_pass_by_value)]
    pub fn send<O: Operation>(&self, op: O) -> Result<O::Output> {
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
        let _enter = span.enter();
        #[cfg(feature = "tracing")]
        let start = std::time::Instant::now();

        let result = match spec.method {
            Method::GET => self.apply(self.agent.get(&url), &spec).call(),
            Method::DELETE => self.apply(self.agent.delete(&url), &spec).call(),
            Method::POST => self.send_with_body(self.agent.post(&url), &spec),
            Method::PUT => self.send_with_body(self.agent.put(&url), &spec),
            Method::PATCH => self.send_with_body(self.agent.patch(&url), &spec),
            ref other => {
                return Err(Error::transport(format!("unsupported HTTP method {other}")));
            }
        };

        let mut resp = result.map_err(Error::transport)?;
        let status = resp.status().as_u16();
        let bytes = resp.body_mut().read_to_vec().map_err(Error::transport)?;

        let parsed = parse(status, &bytes);

        #[cfg(feature = "tracing")]
        {
            span.record("status", status);
            span.record(
                "elapsed_ms",
                u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
            );
            match &parsed {
                Ok(_) => tracing::debug!("request completed"),
                Err(e) => tracing::warn!(code = ?e.code(), "request failed"),
            }
        }

        parsed
    }

    fn send_with_body(
        &self,
        rb: ureq::RequestBuilder<ureq::typestate::WithBody>,
        spec: &RequestSpec,
    ) -> std::result::Result<http::Response<ureq::Body>, ureq::Error> {
        let rb = self.apply(rb, spec);
        match &spec.body {
            Some(body) => rb.content_type("application/json").send(body.as_slice()),
            None => rb.send_empty(),
        }
    }
}
