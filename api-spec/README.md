# Blooio API snapshots

This directory contains the repository's pinned, offline API descriptions for
Blooio v2 and v4. The JSON files are exact snapshots of the provider's
machine-readable exports; tests and runtime code must not download them.

## Blooio v2

- Snapshot: [`blooio-v2.openapi.json`](blooio-v2.openapi.json)
- Version: `2.0.0` (see [`blooio-v2.version`](blooio-v2.version))
- Source: official Blooio v2 OpenAPI export, <https://api.blooio.com/v2/api/openapi.json>
- Provider: Blooio
- Retrieved: 2026-07-17
- Base URL: `https://api.blooio.com/v2/api`
- Snapshot SHA-256: `c569f4618cd58c50133ace51e821f98b92b39f94b74258cf22a7928558ae0a41`
- Paths/operations: 37 / 55

No v2 endpoints are deliberately excluded or manually corrected.

## Blooio v4

The official v4 export identifies the API as version `4.0.0-beta` and uses
`https://api.blooio.com/v4` as its base URL.

- Paths/operations: 59 / 87
- Implemented operations covered by Rust: 78
- Planned, deliberately non-callable operations: 9

## Provenance

- Source: official Blooio v4 OpenAPI export, <https://api.blooio.com/v4/openapi.json>
- Provider: Blooio
- Retrieved: 2026-07-25
- Snapshot version: `4.0.0-beta` (see `blooio-v4.version`)
- Snapshot SHA-256: `dc7d2fe46f12c6bfed785eb0bb25005a36de7784465bd14995625776faebc95d`

This is an exact byte-for-byte snapshot of the provider's JSON export. It is
available to Codex and maintainers without a network request. Do not generate
public Rust modules from it while the v4 schema remains beta.

## Corrections and ambiguities

- The comparison tool considers HTTP method/path pairs and excludes operations
  marked `x-blooio-status: planned` from callable coverage.
- Planned operations comprise three chat-participant, two attachment, and four
  template operations. The checker rejects accidental public exposure of them.
- `GET /contacts` intentionally has separate cursor-list and offset-search Rust
  operations over the same method/path pair.

## Updating

1. Download the relevant official export manually; never add a network
   download to tests or runtime code.
2. Replace the JSON and matching version file together.
3. Update the retrieval date, provenance, exclusions, corrections, and SHA-256
   entry in this document.
4. Run `python3 tools/check_v4_operations.py` for v4 and review every reported
   drift.
5. Review the complete JSON diff. Keep Rust resource modules hand-written.

The comparison command is an offline CI gate for the feature-gated v4 resource
tree. It never fetches provider documentation.
