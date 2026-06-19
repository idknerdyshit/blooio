# blooio

[![Crates.io](https://img.shields.io/crates/v/blooio.svg)](https://crates.io/crates/blooio)
[![Docs.rs](https://docs.rs/blooio/badge.svg)](https://docs.rs/blooio)
[![License](https://img.shields.io/crates/l/blooio.svg)](#license)

Typed, low-overhead Rust bindings for the [Blooio](https://blooio.com) API
(iMessage / SMS automation), exposing **both** an async and a blocking surface
from a single sans-IO core. Sync users pull no async runtime.

## Features

| Feature      | Default | Description                                               |
| ------------ | :-----: | --------------------------------------------------------- |
| `async`      |   ✅    | The async [`Client`] executor (reqwest).                  |
| `sync`       |         | The blocking `BlockingClient` executor (ureq), no tokio.  |
| `rustls`     |   ✅    | TLS via rustls.                                           |
| `native-tls` |         | TLS via the system's native stack.                        |
| `webhooks`   |   ✅    | Typed webhook payloads + HMAC signature verification.     |
| `axum`       |         | Verified axum webhook extractor; implies `webhooks`.      |
| `actix`      |         | Verified actix-web webhook extractor; implies `webhooks`. |
| `tracing`    |   ✅    | Secret-redacted request instrumentation.                  |

At least one of `async` / `sync` must be enabled (enforced at compile time).

## Install

```toml
[dependencies]
blooio = "0.2"
```

Blocking client only, no async runtime:

```toml
[dependencies]
blooio = { version = "0.2", default-features = false, features = ["sync", "rustls", "webhooks"] }
```

## Quick start (async)

```rust,no_run
use blooio::Client;

#[tokio::main]
async fn main() -> blooio::Result<()> {
    let client = Client::new(std::env::var("BLOOIO_API_KEY").unwrap())?;

    // Who am I?
    let me = client.account().get().await?;

    // Send a message.
    let chat = client.chat("chat-id");
    chat.send_text("hello from rust").await?;

    Ok(())
}
```

## Quick start (blocking)

```rust,no_run
use blooio::BlockingClient;

fn main() -> blooio::Result<()> {
    let client = BlockingClient::new(std::env::var("BLOOIO_API_KEY").unwrap())?;
    client.chat("chat-id").send_text("hello from rust")?;
    Ok(())
}
```

The async and blocking surfaces are mirror images: the same resource handles and
method names, differing only by `.await`.

## Resources

Resource handles hang off the client and group the endpoints:

| Handle               | Highlights                                                            |
| -------------------- | --------------------------------------------------------------------- |
| `account()`          | `get`                                                                 |
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
# async fn demo(client: blooio::Client) -> blooio::Result<()> {
let message = client
    .chat("chat-id")
    .message()
    .text("hi")
    .effect("slam")
    .use_typing_indicator(true)
    .idempotency_key("abc-123");
client.chat("chat-id").send(message).await?;
# Ok(()) }
```

### Pagination

List endpoints expose a `*_all` paginator that fetches successive pages lazily:

```rust,no_run
# async fn demo(client: blooio::Client) -> blooio::Result<()> {
let mut pages = client.chats().list_all();
while let Some(page) = pages.next_page().await {
    for chat in page? {
        // ...
    }
}
// or drain everything:
let all = client.contacts().list_all().collect_all().await?;
# Ok(()) }
```

In the blocking client, the paginator also implements `Iterator`.

With the async client, a paginator can also be converted into a `Stream`:

```rust,no_run
# async fn demo(client: blooio::Client) -> blooio::Result<()> {
use futures::TryStreamExt;

let chats = client.chats().list_all().stream().try_collect::<Vec<_>>().await?;
# Ok(()) }
```

### Escape hatch

Every endpoint is described once as a public [`Operation`]. Anything not covered
by a convenience method can be sent directly:

```rust,no_run
# async fn demo(client: blooio::Client, op: impl blooio::Operation<Output = ()>) -> blooio::Result<()> {
let out = client.send(op).await?;
# Ok(()) }
```

## Configuration

`Client::new(key)` uses production defaults. For more control, build a
`ClientConfig`:

```rust,no_run
use blooio::{Client, ClientConfig};
use std::time::Duration;

# fn demo() -> blooio::Result<()> {
let config = ClientConfig::new("my-api-key")
    .with_base_url("https://backend.blooio.com/v2/api")
    .with_timeout(Duration::from_secs(10))
    .with_user_agent("my-app/1.0");
let client = Client::from_config(config)?;
# Ok(()) }
```

Applications that already own an HTTP client can reuse it:

```rust,no_run
use blooio::{Client, ClientConfig};

# fn demo(http: reqwest::Client) -> blooio::Result<()> {
let config = ClientConfig::new("my-api-key");
let client = Client::from_config_and_http_client(config, http);
# Ok(()) }
```

The API key is wrapped in a `Secret`, which zeroizes on drop and redacts itself
in `Debug` output (`api_key: [REDACTED]`) — it is never logged or serialized in
cleartext.

### Retries and rate limits

Transient failures are retried by default with jittered backoff. Customize that
behavior with `ClientConfig::with_retry`, or pass `RetryPolicy::none()` to
disable it.

Use `send_with_meta` to inspect response metadata such as rate-limit headers and
`Retry-After`:

```rust,no_run
# async fn demo(client: blooio::Client) -> blooio::Result<()> {
let (_account, meta) = client.send_with_meta(blooio::resources::account::GetMe).await?;
if let Some(limit) = meta.rate_limit {
    let remaining = limit.remaining;
}
# Ok(()) }
```

## Errors

All fallible calls return `blooio::Result<T>`. The `Error` enum distinguishes
`Api` (non-2xx, with a machine-readable `code`), `Transport`, `Encode`,
`Decode`, and (with `webhooks`) `Webhook`. Match on the stable code for
programmatic handling:

```rust,no_run
# fn handle(err: blooio::Error) {
if err.code() == Some("outbound_limit_reached") {
    // back off and retry later
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
// Verify the signature (constant-time, with replay protection).
webhook::verify_default(secret, sig_header, raw_body)?;

// Parse the typed payload.
let event = WebhookEvent::parse(raw_body)?;
if let Some(kind) = event.kind() {
    // dispatch on the message event kind
}
# Ok(()) }
```

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

With the `tracing` feature, each request emits a `blooio.request` span carrying
the method, path, status, and elapsed time. The API key is never recorded.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

[`Client`]: https://docs.rs/blooio/latest/blooio/struct.Client.html
[`Operation`]: https://docs.rs/blooio/latest/blooio/trait.Operation.html
