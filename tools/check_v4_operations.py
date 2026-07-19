#!/usr/bin/env python3
"""Compare hand-written Rust operation method/path pairs with the v4 snapshot."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path


METHODS = {"get", "post", "put", "patch", "delete", "head", "options", "trace"}
OPERATION_RE = re.compile(
    r"impl\s+(?:crate::)?Operation\s+for\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\{"
    r"(?P<body>.*?)(?=\nimpl\s+(?:crate::)?Operation\s+for\s+|\Z)",
    re.DOTALL,
)
METHOD_RE = re.compile(r"const\s+METHOD\s*:\s*Method\s*=\s*Method::(?P<method>[A-Z]+)")
PATH_RE = re.compile(r"fn\s+path\s*\([^)]*\)\s*->\s*String\s*\{(?P<body>.*?)\n\s*\}", re.DOTALL)
STRING_RE = re.compile(r'"((?:\\.|[^"\\])*)"')


def rust_pairs(source_dir: Path) -> list[tuple[str, str, str]]:
    pairs = []
    for source in sorted(source_dir.glob("*.rs")):
        text = source.read_text(encoding="utf-8")
        for match in OPERATION_RE.finditer(text):
            method_match = METHOD_RE.search(match.group("body"))
            path_match = PATH_RE.search(match.group("body"))
            if not method_match or not path_match:
                continue
            strings = STRING_RE.findall(path_match.group("body"))
            path = next((value for value in strings if value.startswith("/")), None)
            if path is None:
                continue
            path = re.sub(r"\{\s*\}", "{parameter}", path)
            pairs.append((method_match.group("method").lower(), path, match.group("name")))
    return pairs


def spec_pairs(spec_path: Path) -> tuple[set[tuple[str, str]], set[tuple[str, str]]]:
    document = json.loads(spec_path.read_text(encoding="utf-8"))
    implemented = {
        (method, re.sub(r"\{[^{}]+\}", "{parameter}", path))
        for path, item in document["paths"].items()
        for method, operation in item.items()
        if method in METHODS
        if operation.get("x-blooio-status") != "planned"
    }
    planned = {
        (method, re.sub(r"\{[^{}]+\}", "{parameter}", path))
        for path, item in document["paths"].items()
        for method, operation in item.items()
        if method in METHODS
        if operation.get("x-blooio-status") == "planned"
    }
    return implemented, planned


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spec", type=Path, default=Path("api-spec/blooio-v4.openapi.json"))
    parser.add_argument("--source-dir", type=Path, default=Path("src/v4/resources"))
    args = parser.parse_args()

    expected, planned = spec_pairs(args.spec)
    actual_rows = rust_pairs(args.source_dir)
    actual = {(method, path) for method, path, _ in actual_rows}
    multiplicities = Counter((method, path) for method, path, _ in actual_rows)
    expected_multiplicities = {("get", "/contacts"): 2}
    unexpected_multiplicities = sorted(
        (pair, count, expected_multiplicities.get(pair, 1))
        for pair, count in multiplicities.items()
        if count != expected_multiplicities.get(pair, 1)
    )
    missing = sorted(expected - actual)
    unexpected = sorted(actual - expected)
    exposed_planned = sorted(actual & planned)

    for method, path in missing:
        print(f"missing from Rust: {method.upper():7} {path}")
    for method, path in unexpected:
        print(f"unexpected in Rust: {method.upper():7} {path}")
    for method, path in exposed_planned:
        print(f"planned endpoint exposed in Rust: {method.upper():7} {path}")
    for (method, path), count, expected_count in unexpected_multiplicities:
        print(
            f"unexpected Rust operation count: {method.upper():7} {path} "
            f"(found {count}, expected {expected_count})"
        )

    print(f"implemented snapshot operations: {len(expected)}")
    print(f"planned snapshot operations:     {len(planned)}")
    print(f"Rust operations:     {len(actual_rows)}")
    return 1 if missing or unexpected or unexpected_multiplicities or exposed_planned else 0


if __name__ == "__main__":
    sys.exit(main())
