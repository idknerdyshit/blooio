#!/usr/bin/env python3
"""Check or apply the official Blooio v4 OpenAPI snapshot."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import tempfile
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


DEFAULT_URL = "https://api.blooio.com/v4/openapi.json"
DEFAULT_PIN = Path("api-spec/blooio-v4.openapi.json")
DEFAULT_VERSION = Path("api-spec/blooio-v4.version")
DEFAULT_CANDIDATE = Path("/tmp/blooio-v4.remote.openapi.json")
HTTP_METHODS = {"get", "post", "put", "patch", "delete", "head", "options", "trace"}


def sha256(data: bytes) -> str:
    """Return the lowercase SHA-256 digest for bytes."""
    return hashlib.sha256(data).hexdigest()


def parse_spec(data: bytes, source: str) -> dict[str, Any]:
    """Parse and minimally validate an OpenAPI v3 document."""
    try:
        document = json.loads(data)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError(f"{source} is not valid UTF-8 JSON: {error}") from error
    if not isinstance(document, dict):
        raise ValueError(f"{source} is not a JSON object")
    openapi = document.get("openapi")
    info = document.get("info")
    paths = document.get("paths")
    if not isinstance(openapi, str) or not openapi.startswith("3."):
        raise ValueError(f"{source} does not declare OpenAPI 3.x")
    if not isinstance(info, dict) or not isinstance(info.get("version"), str):
        raise ValueError(f"{source} lacks info.version")
    if not isinstance(paths, dict):
        raise ValueError(f"{source} lacks a paths object")
    return document


def counts(document: dict[str, Any]) -> tuple[int, int, int]:
    """Return path, callable-operation, and planned-operation counts."""
    callable_operations = 0
    planned_operations = 0
    paths = document["paths"]
    for item in paths.values():
        if not isinstance(item, dict):
            continue
        for method, operation in item.items():
            if method not in HTTP_METHODS or not isinstance(operation, dict):
                continue
            if operation.get("x-blooio-status") == "planned":
                planned_operations += 1
            else:
                callable_operations += 1
    return len(paths), callable_operations, planned_operations


def write_atomic(path: Path, data: bytes) -> None:
    """Atomically write bytes beside the destination."""
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary_path = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary_path, path)
    finally:
        temporary_path.unlink(missing_ok=True)


def summary(
    local_data: bytes,
    local_document: dict[str, Any],
    remote_data: bytes,
    remote_document: dict[str, Any],
    candidate: Path,
) -> dict[str, Any]:
    """Build the machine-readable comparison summary."""
    path_count, callable_count, planned_count = counts(remote_document)
    return {
        "changed": local_data != remote_data,
        "local_sha256": sha256(local_data),
        "remote_sha256": sha256(remote_data),
        "local_version": local_document["info"]["version"],
        "remote_version": remote_document["info"]["version"],
        "remote_paths": path_count,
        "remote_callable_operations": callable_count,
        "remote_planned_operations": planned_count,
        "remote_total_operations": callable_count + planned_count,
        "candidate": str(candidate),
    }


def main() -> int:
    """Run the command-line checker."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default=DEFAULT_URL)
    parser.add_argument("--pin", type=Path, default=DEFAULT_PIN)
    parser.add_argument("--version-file", type=Path, default=DEFAULT_VERSION)
    parser.add_argument("--candidate", type=Path, default=DEFAULT_CANDIDATE)
    parser.add_argument(
        "--apply-candidate",
        type=Path,
        help="apply a previously reviewed candidate instead of downloading",
    )
    args = parser.parse_args()

    try:
        local_data = args.pin.read_bytes()
        local_document = parse_spec(local_data, str(args.pin))

        if args.apply_candidate is not None:
            candidate_data = args.apply_candidate.read_bytes()
            candidate_document = parse_spec(candidate_data, str(args.apply_candidate))
            write_atomic(args.pin, candidate_data)
            version = candidate_document["info"]["version"]
            write_atomic(args.version_file, f"{version}\n".encode())
            result = summary(
                local_data,
                local_document,
                candidate_data,
                candidate_document,
                args.apply_candidate,
            )
            result["applied"] = True
        else:
            request = urllib.request.Request(
                args.url,
                headers={"Accept": "application/json", "User-Agent": "blooio-openapi-sync"},
            )
            with urllib.request.urlopen(request, timeout=30) as response:
                candidate_data = response.read()
            candidate_document = parse_spec(candidate_data, args.url)
            write_atomic(args.candidate, candidate_data)
            result = summary(
                local_data,
                local_document,
                candidate_data,
                candidate_document,
                args.candidate,
            )
            result["applied"] = False

        print(json.dumps(result, indent=2, sort_keys=True))
        return 0
    except (OSError, ValueError, urllib.error.URLError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
