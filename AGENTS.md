# AGENTS.md

Canonical guidance for coding agents working in this repository.

## Project Overview

`blooio` is a typed Rust client for the Blooio API (iMessage/SMS automation). It
ships both an async and a blocking client from one sans-IO core, on edition
2024.

## Repository Structure

The crate is split into a sans-IO description layer and two thin IO executors.

```text
Operation (sans-IO)  ->  RequestSpec  ->  Client / BlockingClient  ->  HTTP
  method/path/query/       built once       (reqwest)  (ureq)
  headers/body/Output
```

- `src/core/operation.rs` - the `Operation` trait. Each endpoint implements it:
  associated `Output` type, `const METHOD`, and `path`/`query`/`headers`/`body`
  methods. No IO happens here. `json_body(&value)` is the helper for
  serializing request bodies.
- `src/core/request.rs` - `RequestSpec`, the fully-resolved request built once
  from an `Operation` and consumed by either executor.
- `src/core/response.rs` - status-to-`Result` mapping and error decoding.
- `src/core/pagination.rs` - `Paginator` (offset/limit), the `Listing` trait,
  `Page`, and `Pagination`. The blocking paginator also implements `Iterator`.
- `src/client/asynchronous.rs` and `src/client/blocking.rs` - the two
  executors. Both expose `new`, `from_config`, and `send<O: Operation>`. The
  auth header is injected here (`Authorization: Bearer ...`), not stored in
  `RequestSpec`.
- `src/resources/*.rs` - hand-written ergonomic surface. Each module defines its
  public `Operation` types plus a resource handle, such as `Chats` or
  `Contacts`, whose methods build an operation and call `client.send(...)`.
- `src/types/mod.rs` - shared serde DTOs for responses and nested payloads.
- `src/webhook/` - framework-agnostic webhook payload parsing
  (`WebhookEvent::parse`) and constant-time HMAC verification
  (`signature::verify`).
- `src/secret.rs` - `Secret<T>`: zeroizes on drop and redacts in `Debug`.
- `src/config.rs` - `ClientConfig` builder.
- `tests/` - integration, redaction, and ignored live smoke tests.

The async and blocking surfaces are mirror images. Any change to one almost
always needs the matching change to the other. Both `impl` blocks usually live
in the same resource file, gated by `#[cfg(feature = "async")]` and
`#[cfg(feature = "sync")]`.

## Development Commands

Run formatting before handing off changes:

```sh
cargo fmt --all --check
```

Run clippy across the feature combinations covered by CI:

```sh
cargo clippy --all-features --all-targets -- -D warnings
cargo clippy --no-default-features --features sync,webhooks --all-targets -- -D warnings
```

Run documentation checks when public API or docs change:

```sh
RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo doc --all-features --no-deps
```

## Testing Instructions

Run the main test matrix:

```sh
cargo test --all-features
cargo test
cargo test --no-default-features --features sync,webhooks
```

- `tests/integration_async.rs` and `tests/integration_sync.rs` provide
  mock-server coverage with wiremock/httpmock.
- `tests/redaction.rs` verifies that secrets never leak. Do not weaken this
  coverage.
- `tests/live_smoke.rs` is ignored and hits the real API with a real key.

## Coding Guidelines

- Never put secrets in `RequestSpec` or anything that derives `Debug`. The API
  key lives in `Secret<String>` and is injected into the request builder by the
  executor.
- ID-style arguments are `impl Into<String>`; request bodies are typed structs.
- Errors flow through `crate::Result<T>` and `Error`. Match on `Error::code()`
  for stable machine-readable codes.
- Mirror async and blocking APIs, including method names.
- A `[lints]` table in `Cargo.toml` sets the lint policy. Keep the code clean
  under the configured lint levels.
- `#![forbid(unsafe_code)]` is enforced. Do not add `unsafe`.
- Clippy `pedantic` and `cargo` groups are `warn`. Restriction lints such as
  `unwrap_used`, `expect_used`, `panic`, `dbg_macro`, `print_stdout`,
  `print_stderr`, `todo`, and `unimplemented` are enabled for library code.
- Test code explicitly allows those restriction lints in the four `tests/*.rs`
  files and in each in-source `#[cfg(test)] mod tests`. When adding a new test
  module, copy the local allow line.
- `missing_docs` is enabled. DTO structs/enums carry a single type-level
  `#[allow(missing_docs)]` instead of per-field docs; public types, functions,
  and the crate itself still require docs.
- Integer casts that cannot be proven non-truncating use
  `T::try_from(...).unwrap_or(T::MAX)` rather than `as`.

## Adding an Endpoint

1. Define an `Operation` struct in the relevant `src/resources/<resource>.rs`
   file, or add a `path()`/`body()` to an existing operation. Use a builder if
   the endpoint has many optional fields. Builder setters take `self` and are
   `#[must_use]`.
2. Add response/DTO types to `src/types/mod.rs` or the resource module.
3. Add the convenience method to both the async and blocking `impl` blocks on
   the resource handle.
4. For list endpoints, implement `Listing` on the response and add a `*_all`
   paginator method.
5. Add a unit test in the module's `#[cfg(test)] mod tests` and/or a
   mock-server test under `tests/`.

## Agent Workflow Notes

- `AGENTS.md` is the canonical agent instruction file for this repository.
- This repository has one root crate and no nested workspaces, apps, or crates
  requiring their own local `AGENTS.md` file.
- Keep the tree clean across feature combinations, especially async-only,
  sync-plus-webhooks, and all-features builds.
- `cargo clippy --lib` does not compile `#[cfg(test)]` modules. Use
  `--all-targets` to catch warnings in test code.
- The `cargo` group's `multiple_crate_versions` lint is allowed because it is
  transitive dependency noise; everything else in the group is warned.

## Security / Safety Notes

- Do not log, serialize, or expose API keys or webhook secrets.
- Keep secret redaction tests green.
- Reading `src/secret.rs` may trip a local "secrets file" hook because of the
  filename. It contains no real secret, only the `Secret<T>` wrapper.
