//! Version-specific client wrappers that delegate to the shared executors.

use crate::v4::Operation;
use crate::{ApiResponse, BlooioCreds, ClientConfig, RequestOptions, ResponseMeta, Result};

#[cfg(feature = "async")]
/// Asynchronous Blooio v4 client.
#[derive(Clone, Debug)]
pub struct Client {
    inner: crate::Client,
}

#[cfg(feature = "async")]
/// Account-scoped asynchronous Blooio v4 handle.
#[derive(Clone, Copy, Debug)]
pub struct BlooioAccount<'a> {
    pub(crate) inner: crate::BlooioAccount<'a>,
}

#[cfg(feature = "async")]
impl Client {
    /// Build a client using the v4 production base URL.
    pub fn new() -> Result<Self> {
        Self::from_config(ClientConfig::new().with_base_url(super::DEFAULT_BASE_URL))
    }

    /// Build a client from environment configuration, defaulting to v4.
    pub fn from_env() -> Result<Self> {
        Self::from_config(ClientConfig::from_env_with_default(
            super::DEFAULT_BASE_URL,
        )?)
    }

    /// Build a client from explicit transport configuration.
    pub fn from_config(config: ClientConfig) -> Result<Self> {
        crate::Client::from_config(config).map(|inner| Self { inner })
    }

    /// Build from configuration and a caller-provided reqwest client.
    #[must_use]
    pub fn from_config_and_http_client(config: ClientConfig, http: reqwest::Client) -> Self {
        Self {
            inner: crate::Client::from_config_and_http_client(config, http),
        }
    }

    /// Build from configuration and a caller-provided reqwest client, validating first.
    pub fn try_from_config_and_http_client(
        config: ClientConfig,
        http: reqwest::Client,
    ) -> Result<Self> {
        crate::Client::try_from_config_and_http_client(config, http).map(|inner| Self { inner })
    }

    /// Override the maximum retained response body size.
    #[must_use]
    pub fn with_max_response_body_bytes(mut self, limit: usize) -> Self {
        self.inner = self.inner.with_max_response_body_bytes(limit);
        self
    }

    /// Return the underlying transport configuration.
    pub fn config(&self) -> &ClientConfig {
        self.inner.config()
    }

    /// Create an account-scoped v4 handle.
    #[must_use]
    pub fn account<'a>(&'a self, creds: &'a BlooioCreds) -> BlooioAccount<'a> {
        BlooioAccount {
            inner: self.inner.account(creds),
        }
    }
}

#[cfg(feature = "async")]
impl BlooioAccount<'_> {
    /// Execute a v4 operation.
    pub async fn send<O: Operation>(self, op: O) -> Result<O::Output> {
        self.inner.send(op).await
    }
    /// Execute a v4 operation with request options.
    pub async fn send_with_options<O: Operation>(
        self,
        op: O,
        options: RequestOptions,
    ) -> Result<O::Output> {
        self.inner.send_with_options(op, options).await
    }
    /// Execute a v4 operation and return response metadata.
    pub async fn send_with_meta<O: Operation>(self, op: O) -> Result<(O::Output, ResponseMeta)> {
        self.inner.send_with_meta(op).await
    }
    /// Execute a v4 operation with options and response metadata.
    pub async fn send_with_meta_with_options<O: Operation>(
        self,
        op: O,
        options: RequestOptions,
    ) -> Result<(O::Output, ResponseMeta)> {
        self.inner.send_with_meta_with_options(op, options).await
    }
    /// Execute a v4 operation and retain the raw response.
    pub async fn send_with_response<O: Operation>(self, op: O) -> Result<ApiResponse<O::Output>> {
        self.inner.send_with_response(op).await
    }
    /// Execute a v4 operation with options and retain the raw response.
    pub async fn send_with_response_with_options<O: Operation>(
        self,
        op: O,
        options: RequestOptions,
    ) -> Result<ApiResponse<O::Output>> {
        self.inner
            .send_with_response_with_options(op, options)
            .await
    }
}

#[cfg(feature = "sync")]
/// Blocking Blooio v4 client.
#[derive(Clone, Debug)]
pub struct BlockingClient {
    inner: crate::BlockingClient,
}

#[cfg(feature = "sync")]
/// Account-scoped blocking Blooio v4 handle.
#[derive(Clone, Copy, Debug)]
pub struct BlockingBlooioAccount<'a> {
    pub(crate) inner: crate::BlockingBlooioAccount<'a>,
}

#[cfg(feature = "sync")]
impl BlockingClient {
    /// Build a client using the v4 production base URL.
    pub fn new() -> Result<Self> {
        Self::from_config(ClientConfig::new().with_base_url(super::DEFAULT_BASE_URL))
    }
    /// Build a client from environment configuration, defaulting to v4.
    pub fn from_env() -> Result<Self> {
        Self::from_config(ClientConfig::from_env_with_default(
            super::DEFAULT_BASE_URL,
        )?)
    }
    /// Build a client from explicit transport configuration.
    pub fn from_config(config: ClientConfig) -> Result<Self> {
        crate::BlockingClient::from_config(config).map(|inner| Self { inner })
    }
    /// Build from configuration and a caller-provided ureq agent.
    #[must_use]
    pub fn from_config_and_agent(config: ClientConfig, agent: ureq::Agent) -> Self {
        Self {
            inner: crate::BlockingClient::from_config_and_agent(config, agent),
        }
    }
    /// Build from configuration and a caller-provided ureq agent, validating first.
    pub fn try_from_config_and_agent(config: ClientConfig, agent: ureq::Agent) -> Result<Self> {
        crate::BlockingClient::try_from_config_and_agent(config, agent).map(|inner| Self { inner })
    }
    /// Override the maximum retained response body size.
    #[must_use]
    pub fn with_max_response_body_bytes(mut self, limit: usize) -> Self {
        self.inner = self.inner.with_max_response_body_bytes(limit);
        self
    }
    /// Return the underlying transport configuration.
    pub fn config(&self) -> &ClientConfig {
        self.inner.config()
    }
    /// Create an account-scoped v4 handle.
    #[must_use]
    pub fn account<'a>(&'a self, creds: &'a BlooioCreds) -> BlockingBlooioAccount<'a> {
        BlockingBlooioAccount {
            inner: self.inner.account(creds),
        }
    }
}

#[cfg(feature = "sync")]
impl BlockingBlooioAccount<'_> {
    /// Execute a v4 operation.
    pub fn send<O: Operation>(self, op: O) -> Result<O::Output> {
        self.inner.send(op)
    }
    /// Execute a v4 operation with request options.
    pub fn send_with_options<O: Operation>(
        self,
        op: O,
        options: RequestOptions,
    ) -> Result<O::Output> {
        self.inner.send_with_options(op, options)
    }
    /// Execute a v4 operation and return response metadata.
    pub fn send_with_meta<O: Operation>(self, op: O) -> Result<(O::Output, ResponseMeta)> {
        self.inner.send_with_meta(op)
    }
    /// Execute a v4 operation with options and response metadata.
    pub fn send_with_meta_with_options<O: Operation>(
        self,
        op: O,
        options: RequestOptions,
    ) -> Result<(O::Output, ResponseMeta)> {
        self.inner.send_with_meta_with_options(op, options)
    }
    /// Execute a v4 operation and retain the raw response.
    pub fn send_with_response<O: Operation>(self, op: O) -> Result<ApiResponse<O::Output>> {
        self.inner.send_with_response(op)
    }
    /// Execute a v4 operation with options and retain the raw response.
    pub fn send_with_response_with_options<O: Operation>(
        self,
        op: O,
        options: RequestOptions,
    ) -> Result<ApiResponse<O::Output>> {
        self.inner.send_with_response_with_options(op, options)
    }
}
