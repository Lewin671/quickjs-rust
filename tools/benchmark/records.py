"""Shared strict contracts for benchmark workload result records."""

from __future__ import annotations

import json
from typing import Any

RESULT_PREFIX = "QJS_BENCH_RESULT "


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"workload result contains duplicate key {key!r}")
        result[key] = value
    return result


def _reject_constant(value: str) -> None:
    raise ValueError(f"workload result contains non-standard numeric constant {value}")


def parse_result(stdout: str) -> dict[str, Any]:
    matches = [
        line[len(RESULT_PREFIX):]
        for line in stdout.splitlines()
        if line.startswith(RESULT_PREFIX)
    ]
    if len(matches) != 1:
        raise ValueError(f"expected exactly one {RESULT_PREFIX.strip()} line, got {len(matches)}")
    value = json.loads(
        matches[0], object_pairs_hook=_unique_object, parse_constant=_reject_constant
    )
    if not isinstance(value, dict) or set(value) != {
        "case_id", "iterations", "operations", "checksum",
    }:
        raise ValueError("workload result has an invalid field set")
    return value
