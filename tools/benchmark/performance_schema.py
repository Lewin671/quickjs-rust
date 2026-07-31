"""Fail-closed input vocabulary shared by performance queues, plans, and decisions.

Every performance artifact in this campaign is a small JSON file that binds a
reviewable decision to large raw evidence by SHA-256. That only works if the
readers refuse malformed input rather than coercing it: a queue that silently
accepts ``NaN``, a plan that accepts a duplicate JSON key, or a decision that
accepts an out-of-range ratio would each let an unreviewable claim through.

These primitives are the one place those rules live, so the queue builder, the
plan loader, and the decision classifier cannot drift apart on what counts as
valid evidence.
"""

from __future__ import annotations

import hashlib
import json
import math
import re
from pathlib import Path
from typing import Any


class PerformanceDecisionError(ValueError):
    """A priority queue, plan, or decision input is malformed."""


_SHA256 = re.compile(r"[0-9a-f]{64}\Z")
_REVISION = re.compile(r"[0-9a-f]{40}\Z")
_UNIT_ID = re.compile(r"[a-z0-9][a-z0-9-]*\Z")
_OPPORTUNITY_ID = re.compile(r"(?:external|broad)/[a-z0-9][a-z0-9._/-]*\Z")
_ROLES = ("candidate", "base", "quickjs-ng")
_QUEUE_TYPE = "quickjs-performance-opportunity-queue"
_UNIT_TYPE = "quickjs-performance-unit"
_DECISION_TYPE = "quickjs-performance-decision"


def _reject_constant(value: str) -> None:
    raise PerformanceDecisionError(
        f"JSON contains non-standard numeric constant {value}"
    )


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise PerformanceDecisionError(f"JSON contains duplicate key {key!r}")
        result[key] = value
    return result


def _read_json(path: Path, where: str) -> tuple[dict[str, Any], str]:
    resolved = path.expanduser().resolve()
    try:
        raw = resolved.read_bytes()
        value = json.loads(
            raw, object_pairs_hook=_unique_object, parse_constant=_reject_constant
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise PerformanceDecisionError(f"cannot read {where} {resolved}: {error}") from error
    if not isinstance(value, dict):
        raise PerformanceDecisionError(f"{where}: expected an object")
    return value, hashlib.sha256(raw).hexdigest()


def _object(value: Any, where: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise PerformanceDecisionError(f"{where}: expected an object")
    return value


def _array(value: Any, where: str) -> list[Any]:
    if not isinstance(value, list):
        raise PerformanceDecisionError(f"{where}: expected an array")
    return value


def _keys(value: dict[str, Any], expected: set[str], where: str) -> None:
    actual = set(value)
    if actual != expected:
        missing = sorted(expected - actual)
        unknown = sorted(actual - expected)
        details = []
        if missing:
            details.append(f"missing {missing}")
        if unknown:
            details.append(f"unknown {unknown}")
        raise PerformanceDecisionError(f"{where}: {', '.join(details)}")


def _string(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value or value.strip() != value:
        raise PerformanceDecisionError(f"{where}: expected a non-empty trimmed string")
    return value


def _sha256(value: Any, where: str) -> str:
    text = _string(value, where)
    if not _SHA256.fullmatch(text) or text == "0" * 64:
        raise PerformanceDecisionError(f"{where}: expected a non-zero lowercase SHA-256")
    return text


def _revision(value: Any, where: str) -> str:
    text = _string(value, where)
    if not _REVISION.fullmatch(text) or text == "0" * 40:
        raise PerformanceDecisionError(f"{where}: expected a full lowercase git SHA")
    return text


def _ratio(value: Any, where: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise PerformanceDecisionError(f"{where}: expected a finite positive ratio")
    result = float(value)
    if not math.isfinite(result) or result <= 0:
        raise PerformanceDecisionError(f"{where}: expected a finite positive ratio")
    return result


def _fraction(value: Any, where: str) -> float:
    result = _ratio(value, where)
    if result > 1:
        raise PerformanceDecisionError(f"{where}: expected a fraction in (0, 1]")
    return result


def _integer(value: Any, where: str, minimum: int = 0) -> int:
    if type(value) is not int or value < minimum:
        raise PerformanceDecisionError(f"{where}: expected an integer >= {minimum}")
    return value


def _boolean(value: Any, where: str) -> bool:
    if not isinstance(value, bool):
        raise PerformanceDecisionError(f"{where}: expected a boolean")
    return value


def _opportunity_id(value: Any, where: str) -> str:
    text = _string(value, where)
    if not _OPPORTUNITY_ID.fullmatch(text) or "//" in text or ".." in text:
        raise PerformanceDecisionError(f"{where}: invalid opportunity id")
    return text


def _unit_id(value: Any, where: str) -> str:
    text = _string(value, where)
    if not _UNIT_ID.fullmatch(text):
        raise PerformanceDecisionError(f"{where}: invalid stable unit id")
    return text


def _unique_strings(values: Any, where: str, parser: Any = _string) -> tuple[str, ...]:
    result = tuple(parser(value, f"{where}[]") for value in _array(values, where))
    if not result:
        raise PerformanceDecisionError(f"{where}: expected a non-empty array")
    if len(result) != len(set(result)):
        raise PerformanceDecisionError(f"{where}: values must be unique")
    return result


