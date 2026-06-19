//! # blooio
//!
//! Typed, low-overhead Rust bindings for the [Blooio](https://blooio.com) API
//! (iMessage / SMS automation), exposing **both** an async and a blocking
//! surface from a single sans-IO core.
//!
//! ## Design
//!
//! Every endpoint is described once as an [`Operation`] (its method, path,
//! query, headers, body, and output type). Two thin executors —
//! [`Client`] (async, [`reqwest`]) and [`BlockingClient`] (blocking, `ureq`) —
//! perform the actual IO. Sync users pull no async runtime.
//!
//! Hand-written resource handles provide the ergonomic surface:
//!
//! ```no_run
//! # #[cfg(feature = "async")]
//! # async fn demo() -> blooio::Result<()> {
//! use blooio::Client;
//!
//! let client = Client::new("my-api-key")?;
//! let me = client.account().get().await?;
//! let chat = client.chat("chat-id");
//! chat.send_text("hello from rust").await?;
//! # Ok(()) }
//! ```
//!
//! The [`Operation`] types are public, so anything not covered by a convenience
//! method can be sent directly: `client.send(op).await`.
//!
//! ## Features
//!
//! - `async` *(default)* — the [`Client`] executor (reqwest).
//! - `sync` — the [`BlockingClient`] executor (ureq), no tokio.
//! - `rustls` *(default)* / `native-tls` — TLS backend selection.
//! - `webhooks` *(default)* — typed payloads + HMAC signature verification.
//! - `axum` / `actix` — webhook extractors for those frameworks (each implies
//!   `webhooks`).
//! - `tracing` *(default)* — secret-redacted request instrumentation.
//!
//! At least one of `async` / `sync` must be enabled.
//!
//! ## Resilience
//!
//! Both executors retry transient failures (transport errors and the
//! `408`/`425`/`429`/`5xx` statuses) with jittered exponential backoff,
//! honoring any `Retry-After` header. Tune or disable this via
//! [`ClientConfig::with_retry`] and [`RetryPolicy`]. Mutating requests that are
//! retried automatically carry an `Idempotency-Key`.
//!
//! Use `send_with_meta` (on either client) to receive [`ResponseMeta`] —
//! rate-limit headers and `Retry-After` — alongside the decoded response, so
//! you can pace requests against the API's limits.

#![forbid(unsafe_code)]

#[cfg(not(any(feature = "async", feature = "sync")))]
compile_error!("blooio: enable at least one of the `async` or `sync` features");

pub mod config;
pub mod core;
pub mod error;
pub mod resources;
pub mod secret;
pub mod types;

#[cfg(feature = "webhooks")]
pub mod webhook;

mod client;

pub use config::{ClientConfig, DEFAULT_BASE_URL};
pub use core::{
    Listing, Operation, Page, Pagination, Paginator, RateLimit, ResponseMeta, RetryPolicy,
};
pub use error::{Error, Result};
pub use secret::Secret;
pub use types::*;

#[cfg(feature = "sync")]
pub use client::BlockingClient;
#[cfg(feature = "async")]
pub use client::Client;
