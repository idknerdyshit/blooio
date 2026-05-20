# CLAUDE.md

Guidance for Claude Code working in this repository.

## What this is

`blooio` is a typed Rust client for the Blooio API (iMessage/SMS automation). It
ships **both** an async and a blocking client from one **sans-IO core**, on
edition 2024.

## Architecture

The crate is split into a sans-IO description layer and two thin IO executors.

```
Operation (sans-IO)  ──►  RequestSpec  ──►  Client / BlockingClient  ──►  HTTP
  method/path/query/        built once          (reqwest)  (ureq)
  headers/body/Output
```

- **`src/core/operation.rs`** — the `Operation` trait. Each endpoint implements
  it: associated `Output` type, `const METHOD`, and `path`/`query`/`headers`/
  `body` methods. No IO happens here. `json_body(&value)` is the helper for
  serializing request bodies.
- **`src/core/request.rs`** — `RequestSpec`, the fully-resolved request built
  once from an `Operation` and consumed by either executor.
- **`src/core/response.rs`** — status → `Result` mapping and error decoding.
- **`src/core/pagination.rs`** — `Paginator` (offset/limit), the `Listing`
  trait, `Page`, `Pagination`. The blocking paginator also impls `Iterator`.
- **`src/client/asynchronous.rs`** / **`blocking.rs`** — the two executors. Both
  expose `new`, `from_config`, and `send<O: Operation>`. The auth header is
  injected here (`Authorization: Bearer …`), **not** stored in `RequestSpec`.
- **`src/resources/*.rs`** — hand-written ergonomic surface. Each module defines
  its public `Operation` types plus a resource handle (e.g. `Chats`, `Contacts`)
  whose methods build an operation and call `client.send(...)`.
- **`src/types/mod.rs`** — shared serde DTOs (responses and nested payloads).
- **`src/webhook/`** — framework-agnostic webhook payload parsing
  (`WebhookEvent::parse`) and constant-time HMAC verification (`signature::verify`).
- **`src/secret.rs`** — `Secret<T>`: zeroizes on drop, redacts in `Debug`.
- **`src/config.rs`** — `ClientConfig` builder.

The async and blocking surfaces are mirror images. **Any change to one almost
always needs the matching change to the other** (both `impl` blocks usually live
in the same resource file, gated by `#[cfg(feature = "async")]` /
`#[cfg(feature = "sync")]`).

## Adding an endpoint

1. Define an `Operation` struct in the relevant `src/resources/<resource>.rs`
   (or add a `path()`/`body()` to an existing one). Use a builder if it has many
   optional fields — builder setters take `self` and are `#[must_use]`.
2. Add the response/DTO types to `src/types/mod.rs` (or the resource module).
3. Add the convenience method to **both** the async and blocking `impl` blocks
   on the resource handle.
4. For list endpoints, implement `Listing` on the response and add a `*_all`
   paginator method.
5. Add a unit test in the module's `#[cfg(test)] mod tests` and/or a mock-server
   test under `tests/`.

## Conventions & invariants

- **Never put secrets in `RequestSpec` or anything that derives `Debug`.** The
  API key lives in `Secret<String>` and is injected into the request builder by
  the executor. The `redaction` test enforces that the key never appears in
  `Debug` or tracing output — keep it green.
- ID-style args are `impl Into<String>`; bodies are typed structs.
- Errors flow through `crate::Result<T>` / `Error`. Match on `Error::code()` for
  stable machine-readable codes.
- Mirror async/blocking. Mirror method names across them.

## Lints

A `[lints]` table in `Cargo.toml` enforces the bar (see the table for the full
set). Key points:

- `#![forbid(unsafe_code)]` — do not add `unsafe`.
- clippy `pedantic` + `cargo` groups are `warn`; restriction lints
  (`unwrap_used`, `expect_used`, `panic`, `dbg_macro`, `print_stdout`/`stderr`,
  `todo`, `unimplemented`) are on for **library** code.
- In **test** code these restriction lints are explicitly `#[allow]`ed (the four
  `tests/*.rs` files and each in-src `#[cfg(test)] mod tests`). When you add a
  new test module, copy that allow line.
- `missing_docs` is on. DTO structs/enums carry a single type-level
  `#[allow(missing_docs)]` instead of per-field docs; public types, functions,
  and the crate itself still require docs.
- Integer casts that can't be proven non-truncating use
  `T::try_from(...).unwrap_or(T::MAX)` rather than `as`.

Keep the tree clean across feature combos:

```sh
cargo clippy --all-features --all-targets
cargo clippy --no-default-features --features sync,webhooks --all-targets
```

## Build & test

```sh
cargo test --all-features          # full suite (mock-server + unit)
cargo test                         # async/default features
cargo test --no-default-features --features sync,webhooks
```

- `tests/integration_async.rs` / `integration_sync.rs` — mock-server coverage
  (wiremock / httpmock).
- `tests/redaction.rs` — secret never leaks. **Do not weaken.**
- `tests/live_smoke.rs` — `#[ignore]`d; hits the real API with a real key.

## Gotchas

- Reading `src/secret.rs` may trip a local "secrets file" hook on the filename —
  it contains no real secret, just the `Secret<T>` wrapper.
- `cargo clippy --lib` does **not** compile `#[cfg(test)]` modules; use
  `--all-targets` to catch warnings in test code.
- The `cargo` group's `multiple_crate_versions` is `allow`ed (transitive-dep
  noise we don't control); everything else in the group is `warn`.
