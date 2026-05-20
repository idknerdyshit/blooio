# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-05-19

Initial release.

### Added

- Typed Rust client for the Blooio API (iMessage / SMS automation) built on a
  single sans-IO core, exposing **both** an async (`Client`, reqwest) and a
  blocking (`BlockingClient`, ureq) surface as mirror images.
- Resource handles covering the API surface: `account`, `chats`/`chat`,
  `contacts`, `groups`, `contact_card`, `facetime`, `location`, `numbers`,
  `phone_numbers`, and `webhooks`.
- Fluent, `#[must_use]` builders for endpoints with many optional fields
  (e.g. message sending with effects, typing indicators, idempotency keys).
- Lazy offset/limit pagination via `*_all` paginators; the blocking paginator
  also implements `Iterator`.
- Public `Operation` trait as an escape hatch for sending any request directly.
- `Secret<T>` wrapper for the API key: zeroizes on drop and redacts in `Debug`;
  the key is never logged, traced, or serialized in cleartext.
- Stable, machine-readable error codes via `Error::code()`.
- Feature flags: `async`, `sync`, `rustls` (default), `native-tls`, `webhooks`,
  `tracing`. At least one of `async`/`sync` is enforced at compile time.
- Webhook support (`webhooks` feature): framework-agnostic typed payload parsing
  (`WebhookEvent::parse`) and constant-time HMAC signature verification.
- Secret-redacted `tracing` instrumentation (`tracing` feature): a
  `blooio.request` span per request carrying method, path, status, and elapsed.
- Dual-licensed under MIT OR Apache-2.0.

[Unreleased]: https://github.com/idknerdyshit/blooio/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/idknerdyshit/blooio/releases/tag/v0.1.0
