# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.2] - 2026-06-30

### Added

- Added the numbers call-forwarding request endpoint.

### Fixed

- Send chat background uploads as `multipart/form-data` raw image bytes instead
  of JSON.

## [0.3.1] - 2026-06-30

### Added

- Added configurable webhook extractor body limits and enforced timestamp
  freshness for dynamic webhook resolvers.

### Changed

- Use shared async/blocking query serialization and avoid raw request paths in
  tracing spans.
- Sanitize API/decode error display text so reflected response data is not
  stored in default error messages.
- Reuse request body buffers across retries instead of cloning body bytes per
  attempt.

### Fixed

- Support arbitrary HTTP methods through `BlockingClient::send`.
- Make `SendMessage` idempotency headers deterministic for each operation
  value.
- Reject oversized webhook requests before buffering the full body.

## [0.3.0] - 2026-06-28

### Added

- Added webhooks-only builds, public parsed signature primitives, preparsed
  verification, webhook body peeking, received-SMS conversion, and dynamic
  Axum/Actix webhook verification resolvers.
- Added support for both `Blooio-Signature` and `x-blooio-signature` in the
  default framework extractors.

## [0.2.1] - 2026-06-28

### Changed

- Expand tracing redaction coverage. (7a6cd4d)
- Set MSRV to Rust 1.88. (b640ecd)

## [0.2.0] - 2026-06-19

### Added

- Added retry support with configurable `RetryPolicy`, jittered backoff,
  `Retry-After` handling, and idempotency keys for retried mutating requests.
- Added `ResponseMeta`, `RateLimit`, and `send_with_meta` so callers can inspect
  rate-limit and retry headers.
- Added async paginator `Stream` support for users who prefer stream adapters
  over `next_page` or `collect_all`.
- Added optional `axum` and `actix` webhook extractors built around
  `WebhookVerifier` and `VerifiedWebhook`.
- Added usage examples for quick start, messaging, pagination, configuration,
  error handling, blocking calls, and webhooks.
- Added constructors for using caller-provided HTTP clients:
  `Client::from_config_and_http_client` and
  `BlockingClient::from_config_and_agent`.

### Changed

- Bumped the crate version to `0.2.0`.
- Replaced the packaged agent metadata exclusion with `AGENTS.md`.

### Fixed

- Percent-encode interpolated path segments such as chat IDs, contact handles,
  group IDs, location IDs, and webhook IDs.
- Redact webhook signing secrets returned by create and rotate-secret responses.
- Allow the documented stable rustdoc command to build with `--cfg docsrs`.
- Fix paginator lifetime capture on Rust 2024.

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

[Unreleased]: https://github.com/idknerdyshit/blooio/compare/v0.3.2...HEAD
[0.3.2]: https://github.com/idknerdyshit/blooio/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/idknerdyshit/blooio/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/idknerdyshit/blooio/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/idknerdyshit/blooio/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/idknerdyshit/blooio/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/idknerdyshit/blooio/releases/tag/v0.1.0
