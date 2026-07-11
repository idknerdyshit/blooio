# blooio

[![Crates.io](https://img.shields.io/crates/v/blooio.svg)](https://crates.io/crates/blooio)
[![Docs.rs](https://docs.rs/blooio/badge.svg)](https://docs.rs/blooio)
[![License](https://img.shields.io/crates/l/blooio.svg)](#license)

Typed, low-overhead Rust bindings for the [Blooio](https://blooio.com) API
(iMessage / SMS automation), exposing **both** an async and a blocking surface
from a single sans-IO core. Sync users pull no async runtime.

## Features

| Feature                   | Default | Description                                               |
| ------------------------- | :-----: | --------------------------------------------------------- |
| `async`                   |   ✅    | The async [`Client`] executor (reqwest).                  |
| `sync`                    |         | The blocking `BlockingClient` executor (ureq), no tokio.  |
| `rustls`                  |   ✅    | TLS via rustls.                                           |
| `native-tls`              |         | TLS via the system's native stack; takes precedence if both TLS features are enabled. |
| `webhooks`                |   ✅    | Typed webhook payloads + HMAC signature verification, usable without HTTP clients. |
| `axum`                    |         | Verified axum webhook extractor; implies `webhooks`.      |
| `actix`                   |         | Verified actix-web webhook extractor; implies `webhooks`. |
| `tracing`                 |   ✅    | Secret-redacted request instrumentation.                  |
| `sensitive-diagnostics`   |         | Explicit raw request/response inspection and tracing for local debugging. |

At least one of `async` / `sync` / `webhooks` must be enabled (enforced at
compile time).

## Install

```toml
[dependencies]
blooio = "1"
```

Blocking client only, no async runtime:

```toml
[dependencies]
blooio = { version = "1", default-features = false, features = ["sync", "rustls", "webhooks"] }
```

Webhook parsing and verification only, no Blooio API HTTP client:

```toml
[dependencies]
blooio = { version = "1", default-features = false, features = ["webhooks"] }
```

## Quick start (async)

```rust,no_run
use blooio::{BlooioCreds, Client};

#[tokio::main]
async fn main() -> blooio::Result<()> {
    let client = Client::from_env()?;
    let creds = BlooioCreds::from_env()?;
    let account = client.account(&creds);

    // Who am I?
    let me = account.me().get().await?;

    // Send a message.
    let chat = account.chat("chat-id");
    chat.send_text("hello from rust").await?;

    Ok(())
}
```

## Quick start (blocking)

```rust,no_run
use blooio::{BlockingClient, BlooioCreds};

fn main() -> blooio::Result<()> {
    let client = BlockingClient::from_env()?;
    let creds = BlooioCreds::from_env()?;
    let account = client.account(&creds);
    account.chat("chat-id").send_text("hello from rust")?;
    Ok(())
}
```

The async and blocking surfaces are mirror images: the same resource handles and
method names, differing only by `.await`.

## Client reuse

Create one client per base URL/transport configuration and reuse it across
account-scoped credential handles. The async `Client` wraps a pooled
`reqwest::Client`; the blocking `BlockingClient` wraps a pooled `ureq::Agent`.
Cloning either Blooio client is cheap and shares the underlying transport state.

Avoid constructing a fresh client inside hot request loops, because that defeats
connection reuse. If your application already owns a configured HTTP transport,
inject it with `Client::from_config_and_http_client` or
`BlockingClient::from_config_and_agent`.

## Resources

Resource handles hang off an account-scoped handle and group the endpoints:

| Handle               | Highlights                                                            |
| -------------------- | --------------------------------------------------------------------- |
| `me()`               | `get`                                                                 |
| `chats()` / `chat(id)` | `list`, `send`/`send_text`, messages, reactions, polls, typing, read receipts, backgrounds |
| `contacts()`         | `list`, `create`, `get`, `update`, `delete`, `capabilities`, tags     |
| `groups()`           | `list`, `create`, `get`, `update`, `delete`, icons, `members(id)`     |
| `contact_card()`     | `get`, `update`                                                       |
| `facetime()`         | `call`                                                                |
| `location()`         | `list`, `get`, `refresh`                                              |
| `numbers()`          | `list`                                                                |
| `phone_numbers()`    | `lookup`, `lookup_post`, `batch`                                      |
| `webhooks()`         | `list`, `create`, `get`, `update`, `delete`, `rotate_secret`, `logs(id)` |

### Builders

Endpoints with many optional fields use a fluent builder. For example, sending
a message:

```rust,no_run
# async fn demo(account: blooio::BlooioAccount<'_>) -> blooio::Result<()> {
let chat = account.chat("chat-id");
let message = chat.message()
    .text("hi")
    .effect("slam")
    .use_typing_indicator(true)
    .idempotency_key("abc-123");
chat.send(message).await?;
# Ok(()) }
```

### Pagination

List endpoints expose a `*_all` paginator that fetches successive pages lazily:

```rust,no_run
# async fn demo(account: blooio::BlooioAccount<'_>) -> blooio::Result<()> {
let mut pages = account.chats().list_all();
while let Some(page) = pages.next_page().await {
    for chat in page? {
        // ...
    }
}
// or drain everything:
let all = account.contacts().list_all().collect_all().await?;
# Ok(()) }
```

In the blocking client, the paginator also implements `Iterator`.

With the async client, a paginator can also be converted into a `Stream`:

```rust,no_run
# async fn demo(account: blooio::BlooioAccount<'_>) -> blooio::Result<()> {
use futures::TryStreamExt;

let chats = account.chats().list_all().stream().try_collect::<Vec<_>>().await?;
# Ok(()) }
```

Paginator helpers request 50 items per page. They stop after an empty page,
when `pagination.has_more` is `false`, when `pagination.total` has been reached,
after a short page without contrary metadata, or on the first error. Server
pagination metadata takes precedence over the short-page heuristic. Use
`list_with`/`list_messages_with` style methods when you need explicit
`limit`/`offset` control instead of the default walk.

### Escape hatch

Every endpoint is described once as a public [`Operation`]. Anything not covered
by a convenience method can be sent directly:

```rust,no_run
# async fn demo(account: blooio::BlooioAccount<'_>) -> blooio::Result<()> {
use blooio::resources::chats::ListChats;

let first_page = account
    .send(ListChats {
        limit: Some(25),
        offset: Some(0),
        q: Some("support".into()),
        sort: None,
    })
    .await?;
# Ok(()) }
```

This is also useful for request-scoped transport options:

```rust,no_run
# async fn demo(account: blooio::BlooioAccount<'_>) -> blooio::Result<()> {
use blooio::{RequestOptions, RetryPolicy};
use blooio::resources::account::GetMe;

let me = account
    .send_with_options(GetMe, RequestOptions::new().retry(RetryPolicy::none()))
    .await?;
# Ok(()) }
```

### Forward-compatible fields

Some response fields use `blooio::Json` for server-owned nested objects whose
shape is intentionally allowed to evolve. String values such as effects,
reactions, directions, webhook types, sort expressions, and message statuses
are passed through to Blooio unchanged rather than constrained to SDK enums.

## Configuration

`Client::from_env()` and `BlockingClient::from_env()` read transport settings
such as `BLOOIO_BASE_URL` (optional). `BlooioCreds::from_env()` reads
`BLOOIO_API_KEY`. `Client::new()` uses production transport defaults. For more
control, build a `ClientConfig`:

```rust,no_run
use blooio::{BlooioCreds, Client, ClientConfig};
use std::time::Duration;

# fn demo() -> blooio::Result<()> {
let config = ClientConfig::new()
    .with_base_url("https://backend.blooio.com/v2/api")
    .with_timeout(Duration::from_secs(10))
    .with_user_agent("my-app/1.0");
let client = Client::from_config(config)?
    .with_max_response_body_bytes(8 * 1024 * 1024);
let creds = BlooioCreds::new("my-api-key");
let account = client.account(&creds);
# Ok(()) }
```

`ClientConfig::from_env()` returns transport configuration without constructing
a client or reading credentials.

Applications that already own an HTTP client can reuse it:

```rust,no_run
use blooio::{Client, ClientConfig};

# fn demo(http: reqwest::Client) -> blooio::Result<()> {
let config = ClientConfig::new();
let client = Client::try_from_config_and_http_client(config, http)?;
# Ok(()) }
```

`BlooioCreds` wraps the API key in a `Secret`, which zeroizes on drop and
redacts itself in `Debug` output — it is never logged or serialized in
cleartext.

### Retries and rate limits

Transient failures from safe read operations are retried by default with
jittered backoff. Mutating operations retry only when their `Operation`
implementation explicitly declares that doing so is safe. Unknown or
no-code `429` responses are treated as transient, but documented quota/cap
`429` errors are not retried by default. Customize retry behavior with
`ClientConfig::with_retry`, or pass `RetryPolicy::none()` to disable it.

Per-request transport options are available at the executor layer. Extra
headers override operation headers except `Authorization`, which is always
injected from account-scoped credentials. Extra query parameters are appended
after the operation's query parameters. A per-request base URL is concatenated
with the operation path, so include the API prefix you need, such as
`https://backend.blooio.com/v2/api` for v2 operations.

```rust,no_run
# async fn demo(account: blooio::BlooioAccount<'_>) -> blooio::Result<()> {
use blooio::{RequestOptions, RetryPolicy};
use std::time::Duration;

let me = account
    .send_with_options(
        blooio::resources::account::GetMe,
        RequestOptions::new()
            .base_url("https://backend.blooio.com/v2/api")
            .timeout(Duration::from_secs(5))
            .retry(RetryPolicy::none())
            .header("x-request-id", "req-123")
            .query("trace", "1"),
    )
    .await?;
# Ok(()) }
```

Per-request timeout applies to each HTTP attempt, including retries. Async
callers can cancel the whole operation by dropping the future or wrapping it in
`tokio::time::timeout`; blocking callers get timeout control but not external
cancellation. Both executors enforce the same configurable in-memory response
body limit (64 MiB by default); override it on the client or for one request
with `RequestOptions::max_response_body_bytes`.

Use `send_with_meta` to inspect response metadata such as rate-limit headers and
`Retry-After`:

```rust,no_run
# async fn demo(account: blooio::BlooioAccount<'_>) -> blooio::Result<()> {
let (_me, meta) = account.send_with_meta(blooio::resources::account::GetMe).await?;
if let Some(limit) = meta.rate_limit {
    let remaining = limit.remaining;
}
# Ok(()) }
```

Use `send_with_response` when you need the decoded output and the raw HTTP
response from the same request:

```rust,no_run
# async fn demo(account: blooio::BlooioAccount<'_>) -> blooio::Result<()> {
let response = account
    .send_with_response(blooio::resources::account::GetMe)
    .await?;
let status = response.raw.status;
let body = response.raw.body;
# Ok(()) }
```

`RawResponse` contains status, headers, and body bytes. Its `Debug`
implementation redacts header values and body bytes to avoid accidental leaks.

With the non-default `sensitive-diagnostics` feature, `ClientConfig` and
`RequestOptions` can attach a caller-provided sink that receives raw
request/response snapshots and raw transport error strings. This is intentionally
dangerous: snapshots can contain API keys, URLs, phone numbers, message text,
headers, request bodies, and response bodies. The feature never enables itself
from environment variables, and its own `Debug` output remains redacted.

### Sensitive protocol tracing

With the non-default `sensitive-diagnostics` feature, callers may explicitly
emit complete request/response snapshots and raw transport errors through the
`blooio::sensitive` tracing target. This includes authorization credentials,
URLs and query strings, headers, bodies, phone numbers, and message text. It is
for local protocol debugging only; do not enable it with a production log sink.

It is disabled by default and cannot be enabled from environment variables:

```rust,no_run
use blooio::{Client, ClientConfig, RequestOptions};

# async fn example() -> blooio::Result<()> {
let client = Client::from_config(ClientConfig::new().with_sensitive_tracing())?;

// Enable it for just one request instead of the whole client.
let options = RequestOptions::new().sensitive_tracing();

// Suppress a client-wide setting for one request.
let safe_options = RequestOptions::new().without_sensitive_tracing();
# let _ = (client, options, safe_options);
# Ok(())
# }
```

Request-level settings override the client default. Sensitive tracing is
independent from the callback-based `SensitiveDiagnostics` sink; either or both
may be enabled. Normal `blooio::trace` events, public errors, and `Debug` output
remain redacted.

## Errors

All fallible calls return `blooio::Result<T>`. The `Error` enum distinguishes
`Api` (non-2xx, with a machine-readable `code`), `Transport`, `Encode`,
`Decode`, `Config`, and (with `webhooks`) `Webhook`. Match on the stable code
for programmatic handling:

```rust,no_run
# fn handle(err: blooio::Error) {
use blooio::error::codes;

if err.is_quota_error() {
    // A documented account/plan cap was reached; do not blindly retry.
} else if err.code() == Some(codes::REPLY_TARGET_NOT_FOUND) {
    // The threaded-reply target no longer exists.
}
# }
```

## Webhooks

With the `webhooks` feature, verify and parse incoming events. The module is
framework-agnostic, and the optional `axum` and `actix` features add verified
server extractors:

```rust,no_run
use blooio::webhook::{self, WebhookEvent};

# fn handle(secret: &[u8], sig_header: &str, raw_body: &[u8]) -> blooio::Result<()> {
// Reject an oversized HTTP body before buffering it. The crate's recommended
// default is webhook::DEFAULT_MAX_WEBHOOK_BODY_BYTES (256 KiB).
// Then verify the HMAC in constant time and enforce timestamp freshness.
webhook::verify_default(secret, sig_header, raw_body)?;

// Parse the typed payload.
let event = WebhookEvent::parse(raw_body)?;
if let Some(kind) = event.kind() {
    // dispatch on the message event kind
}
# Ok(()) }
```

Timestamp freshness is not one-time replay protection: the same authentic
delivery can be submitted repeatedly within the tolerance window. Deduplicate
stable event/message IDs or make handlers idempotent when duplicates are
harmful. The framework extractors enforce the recommended body limit while
streaming; framework-agnostic callers must cap the HTTP body before buffering
and calling `verify` or `WebhookEvent::parse`.

If the webhook secret depends on fields inside the payload, parse the signature
first, check timestamp freshness, peek only the untrusted routing fields, then
verify with the resolved secret:

```rust,no_run
use blooio::webhook::{
    self, DEFAULT_TOLERANCE_SECS, SignatureHeader, WebhookEvent,
};

# fn now() -> i64 { 1_700_000_000 }
# fn lookup_secret(_internal_id: &str) -> Option<Vec<u8>> { Some(b"whsec".to_vec()) }
# fn handle(sig_header: &str, raw_body: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
let sig = SignatureHeader::parse(sig_header)?;
sig.check_tolerance(now(), DEFAULT_TOLERANCE_SECS)?;

let peek = webhook::peek(raw_body)?;
let resolved = peek.internal_id.as_deref().and_then(lookup_secret);
let known_identifier = resolved.is_some();
let secret = resolved.unwrap_or_else(|| b"non-production-dummy-secret".to_vec());

// Always perform HMAC verification, even for an unknown routing identifier,
// and return the same unauthorized response for unknown IDs and bad HMACs.
let verified = webhook::verify_preparsed(&secret, &sig, raw_body);
if !known_identifier || verified.is_err() {
    return Err(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "invalid webhook signature",
    ).into());
}
let sms = WebhookEvent::parse(raw_body)?.try_into_received_sms()?;
let _sender = sms.sender;
# Ok(()) }
```

The built-in extractors accept both `x-blooio-signature` and the legacy
`Blooio-Signature` by default. Use `WebhookVerifier::with_header_name` to
replace that default lookup with a custom header.

For an axum app, put a `WebhookVerifier` in router state and accept
`VerifiedWebhook` in the handler:

```rust,no_run
# #[cfg(feature = "axum")]
# async fn demo() {
use axum::{routing::post, Router};
use blooio::webhook::{VerifiedWebhook, WebhookVerifier};

async fn on_event(VerifiedWebhook(event): VerifiedWebhook) {
    let _kind = event.kind();
}

let app = Router::new()
    .route("/webhooks/blooio", post(on_event))
    .with_state(WebhookVerifier::new("whsec_..."));
# }
```

## Tracing

With the `tracing` feature, each HTTP attempt emits a `blooio.request` span and
structured events carrying the method, operation type, attempt number, retry
budget, HTTP status when available, and elapsed time. Retries and final logical
operation success/failure are also emitted as structured events. URLs, paths,
query parameters, headers, bodies, and the API key are never recorded.

## Contributing

Endpoint implementations live in `src/resources/*.rs`. Each public endpoint is
an `Operation` plus matching async and blocking resource methods, so changes to
one surface usually need the mirror change in the other. Keep request-specific
details documented on the public operation fields; response DTOs may stay
shape-oriented when they directly mirror Blooio's JSON schema.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

[`Client`]: https://docs.rs/blooio/latest/blooio/struct.Client.html
[`Operation`]: https://docs.rs/blooio/latest/blooio/trait.Operation.html
