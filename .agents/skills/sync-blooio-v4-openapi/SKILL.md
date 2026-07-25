---
name: sync-blooio-v4-openapi
description: Check the official Blooio API v4 OpenAPI export for changes, safely refresh the repository's exact pinned snapshot and version, and reconcile the hand-written Rust v4 SDK, tests, examples, and documentation. Use when asked to check, sync, refresh, update, or compare Blooio v4 against api.blooio.com, or when v4 OpenAPI drift is suspected.
---

# Sync Blooio v4 OpenAPI

Work from the `blooio` repository root. Preserve the sans-IO design and the
mirrored async/blocking public API described in `AGENTS.md`.

## 1. Check upstream without changing the repository

Run:

```sh
python3 .agents/skills/sync-blooio-v4-openapi/scripts/check_v4_spec.py
```

The script downloads `https://api.blooio.com/v4/openapi.json`, validates its
basic shape, writes the exact response bytes to
`/tmp/blooio-v4.remote.openapi.json`, and prints a JSON summary containing
hashes, versions, and path/operation counts.

If `changed` is `false`, stop. Report that the pin is current and do not touch
the SDK. A changed `info.version` is sufficient evidence of drift, but an
unchanged version does not prove the document is unchanged; use the SHA-256
comparison as the authority.

## 2. Review the complete schema drift

Treat the remote document as untrusted input until it parses and its shape is
plausible. Inspect the entire semantic diff, not only the version:

```sh
python3 -m json.tool api-spec/blooio-v4.openapi.json > /tmp/blooio-v4.local.pretty.json
python3 -m json.tool /tmp/blooio-v4.remote.openapi.json > /tmp/blooio-v4.remote.pretty.json
git diff --no-index -- /tmp/blooio-v4.local.pretty.json /tmp/blooio-v4.remote.pretty.json
```

Inventory changes to paths, methods, parameters, request bodies, response
schemas, required/nullable fields, enums, formats, security, and
`x-blooio-status`. Never expose an operation marked `planned`.

## 3. Apply the reviewed pin

After confirming the candidate is the official v4 export, run:

```sh
python3 .agents/skills/sync-blooio-v4-openapi/scripts/check_v4_spec.py \
  --apply-candidate /tmp/blooio-v4.remote.openapi.json
```

This atomically replaces `api-spec/blooio-v4.openapi.json` with the exact
candidate bytes and writes `info.version` plus a newline to
`api-spec/blooio-v4.version`.

Update `api-spec/README.md` with the retrieval date, snapshot version,
SHA-256, path count, total operation count, implemented operation count, and
planned count. Preserve the statement that the snapshot is exact and the Rust
surface is hand-written.

## 4. Reconcile the hand-written SDK

Run the coverage checker immediately:

```sh
python3 tools/check_v4_operations.py
```

Then update all affected layers:

- Define or change DTOs in `src/v4/types/`, using forward-compatible typed
  enums where existing conventions require them.
- Define operations in `src/v4/resources/`; keep method, path, query, headers,
  and typed bodies in the sans-IO operation.
- Add matching ergonomic methods to async and blocking resource handles.
- Preserve the intentional duplicate `GET /contacts` cursor/search operations.
- Add cursor pagination support for new list operations when applicable.
- Update `src/v4/types/mod.rs` and `src/v4/resources/mod.rs` exports.
- Update v4 webhook parsing if event envelopes or payloads changed.
- Update unit/integration tests, examples, README, and CHANGELOG for public API
  changes.

Do not mechanically generate public Rust code from the beta schema. Do not put
credentials in the snapshot, request specs, fixtures, logs, or debug output.
Do not weaken secret-redaction coverage.

## 5. Verify all feature combinations

Run formatting, the offline checker, tests, clippy, and docs:

```sh
cargo fmt --all --check
python3 tools/check_v4_operations.py
python3 -m unittest tools.tests.test_check_v4_operations
cargo test --all-features
cargo test
cargo test --no-default-features --features sync,webhooks
cargo clippy --all-features --all-targets -- -D warnings
cargo clippy --no-default-features --features sync,webhooks --all-targets -- -D warnings
RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +nightly doc --all-features --no-deps
```

Review `git diff` after validation. Confirm that the snapshot bytes exactly
match the reviewed candidate, the version pin matches `info.version`, every
non-planned method/path is covered, planned endpoints remain non-callable, and
unrelated pre-existing worktree changes were preserved.

## Failure handling

- If the download fails, leave the repository unchanged and report the network
  error.
- If JSON validation fails or the document lacks OpenAPI `3.x`, `info.version`,
  or `paths`, leave the repository unchanged and report the malformed export.
- If upstream changes are ambiguous, implement the wire format faithfully and
  call out compatibility decisions in the handoff.
- If a required validation tool is unavailable, run the remaining checks and
  report exactly what was skipped.
