//! # blooio
//!
//! Typed, low-overhead Rust bindings for the [Blooio](https://blooio.com) API
//! (iMessage / SMS automation), exposing **both** an async and a blocking
//! surface from a single sans-IO core.
//!
//! ## Design
//!
//! Every endpoint is described once as an `Operation` (its method, path,
//! query, headers, body, and output type). Two thin executors —
//! `Client` (async, reqwest) and `BlockingClient` (blocking, ureq) —
//! perform the actual IO. Sync users pull no async runtime.
//!
//! Hand-written resource handles provide the ergonomic surface:
//!
//! ```no_run
//! # #[cfg(feature = "async")]
//! # async fn demo() -> blooio::Result<()> {
//! use blooio::{BlooioCreds, Client};
//!
//! let client = Client::new()?;
//! let creds = BlooioCreds::new("my-api-key");
//! let account = client.account(&creds);
//! let me = account.me().get().await?;
//! let chat = account.chat("chat-id");
//! chat.send_text("hello from rust").await?;
//! # Ok(()) }
//! ```
//!
//! The `Operation` types are public, so anything not covered by a convenience
//! method can be sent directly: `account.send(op).await`.
//!
//! ## Client reuse
//!
//! Construct one client per base URL/transport configuration and reuse it
//! across account-scoped API handles. The async `Client` wraps a pooled
//! `reqwest::Client`, and the blocking `BlockingClient` wraps a pooled
//! `ureq::Agent`; cloning a Blooio client is cheap and shares the underlying
//! transport state.
//!
//! Avoid creating a new client for each request in hot paths, because that
//! defeats connection reuse. Applications that already own a configured
//! transport can inject it with `Client::from_config_and_http_client` or
//! `BlockingClient::from_config_and_agent`; fallible `try_` variants validate
//! the accompanying `ClientConfig` first.
//!
//! ## Features
//!
//! - `async` *(default)* — the `Client` executor (reqwest).
//! - `sync` — the `BlockingClient` executor (ureq), no tokio.
//! - `rustls` *(default)* / `native-tls` — TLS backend selection. If both are
//!   enabled, `native-tls` takes precedence for both executors.
//! - `webhooks` *(default)* — typed payloads + HMAC signature verification.
//! - `axum` / `actix` — webhook extractors for those frameworks (each implies
//!   `webhooks`).
//! - `tracing` *(default)* — secret-redacted request instrumentation.
//! - `sensitive-diagnostics` — explicit raw request/response inspection hooks
//!   for local debugging. This feature is dangerous by design and is not enabled
//!   by default.
//!
//! At least one of `async` / `sync` / `webhooks` must be enabled. A
//! webhooks-only build does not compile either HTTP client executor.
//!
//! ## Resilience
//!
//! Both executors retry transient failures (transport errors plus `408`, `425`,
//! unknown/no-code `429`, and `5xx` API errors) with jittered exponential
//! backoff, honoring any `Retry-After` header. Documented quota/cap `429`
//! errors are not retried by default. Tune or disable retrying via
//! `ClientConfig::with_retry` and `RetryPolicy`. Safe read operations are
//! retried automatically; mutating operations must explicitly opt in through
//! their `Operation` implementation.
//!
//! Use `send_with_meta` (on either client) to receive `ResponseMeta` —
//! rate-limit headers and `Retry-After` — alongside the decoded response, so
//! you can pace requests against the API's limits.
//!
//! Request-scoped transport controls are available through `RequestOptions`
//! and `send_with_options`. They can override retry policy, set a per-attempt
//! timeout or response-body limit, override the base URL, append query
//! parameters, and add extra headers. They can also attach a caller-provided
//! safe trace label for correlation in this crate's structured tracing.
//! `Authorization` is still injected by the executor from the redacted
//! account-scoped credentials.
//!
//! Use `send_with_response` when you need `ApiResponse`, which combines the
//! decoded output, `ResponseMeta`, and a `RawResponse` containing status,
//! headers, and body bytes. Raw response debug output redacts header values and
//! body bytes.
//!
//! ## Sensitive protocol tracing
//!
//! The non-default `sensitive-diagnostics` feature exposes an explicit,
//! intentionally dangerous tracing escape hatch. Call
//! `ClientConfig::with_sensitive_tracing` to emit complete request and response
//! snapshots plus raw transport errors to the `blooio::sensitive` tracing
//! target, or use `RequestOptions::sensitive_tracing` for one request. The
//! emitted values include credentials, URLs, headers, and bodies, so use this
//! only with a local development subscriber. It is disabled by default, cannot
//! be enabled through environment variables, and does not change the redaction
//! applied to normal tracing, public errors, or `Debug` output.

#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, doc(auto_cfg))]

#[cfg(not(any(feature = "async", feature = "sync", feature = "webhooks")))]
compile_error!("blooio: enable at least one of the `async`, `sync`, or `webhooks` features");

pub mod error;
pub mod secret;
pub mod types;

#[cfg(feature = "api-v4")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-v4")))]
pub mod v4;

#[cfg(any(feature = "async", feature = "sync"))]
pub mod config;
#[cfg(any(feature = "async", feature = "sync"))]
pub mod core;
#[cfg(any(feature = "async", feature = "sync"))]
mod credentials;
#[cfg(any(feature = "async", feature = "sync"))]
pub mod resources;

#[cfg(feature = "webhooks")]
pub mod webhook;

#[cfg(any(feature = "async", feature = "sync"))]
mod client;

#[cfg(any(feature = "async", feature = "sync"))]
pub use config::{ClientConfig, DEFAULT_BASE_URL, DEFAULT_MAX_RESPONSE_BODY_BYTES};
#[cfg(any(feature = "async", feature = "sync"))]
pub use core::{
    ApiResponse, Listing, Operation, Page, Pagination, Paginator, RateLimit, RawResponse,
    RequestOptions, ResponseMeta, RetryPolicy,
};
#[cfg(all(
    feature = "sensitive-diagnostics",
    any(feature = "async", feature = "sync")
))]
pub use core::{
    SensitiveDiagnosticEvent, SensitiveDiagnosticSink, SensitiveDiagnostics,
    SensitiveDiagnosticsBuilder, SensitiveRequestSnapshot, SensitiveResponseSnapshot,
    SensitiveTransportErrorSnapshot, SensitiveTransportErrorStage,
};
#[cfg(any(feature = "async", feature = "sync"))]
pub use credentials::BlooioCreds;
pub use error::{ApiError, ApiErrorDetails, Error, Result};
pub use secret::Secret;
pub use types::*;

#[cfg(feature = "sync")]
pub use client::{BlockingBlooioAccount, BlockingClient};
#[cfg(feature = "async")]
pub use client::{BlooioAccount, Client};
