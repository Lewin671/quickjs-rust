"""The generic-path sentinel lane's validated machine summary.

It lives beside `preview` rather than inside it because that module carries
the whole hosted orchestration -- manifest preparation, receipts, the broad
summary, status writing, and the CLI -- and this is the second lane's
self-contained validation. The published Markdown is `preview_summary`'s;
this module decides only whether the lane may report a ratio at all.

The import direction is one-way: this module reads `preview`'s shared
vocabulary, and `preview` reaches it only from its argument parser, where
the deferred import keeps the two from forming a cycle.
"""

from __future__ import annotations

import argparse
import math
from typing import Any

from .preview import (
    HOSTED_SENTINEL_CASES,
    PreviewError,
    _assert_lane_health,
    _json_bytes,
    _read_object,
    _SAFE_CASE_ID,
    _write_replace,
)


def _interval(case: dict[str, Any], ratio: float, where: str) -> dict[str, float]:
    """A case's confidence interval, when the report carries one."""
    interval = case.get("confidence_interval")
    if interval is None:
        return {}
    if not isinstance(interval, dict):
        raise PreviewError(f"sentinel report {where} has a malformed confidence interval")
    lower, upper = interval.get("lower"), interval.get("upper")
    if not all(
        isinstance(bound, (int, float)) and not isinstance(bound, bool)
        and math.isfinite(bound) and bound > 0
        for bound in (lower, upper)
    ) or not lower <= ratio <= upper:
        raise PreviewError(f"sentinel report {where} has a malformed confidence interval")
    return {"ci_lower": float(lower), "ci_upper": float(upper)}


def sentinel_machine(report: dict[str, Any]) -> dict[str, Any]:
    """Validates a sentinel report and reduces it to what may be published.

    It deliberately does not reuse `summarize`, whose validation is bound to
    the broad portfolio's exact case inventory. The two lanes answer different
    questions and must not be conflated: broad reports how much the
    specializing tiers recognize, and these report what the ordinary
    interpreter costs when they cannot.
    """
    # The same invariants the broad lane enforces, against this lane's frozen
    # inventory. A degraded run must produce no ratios at all rather than a
    # number that reads as ordinary-interpreter cost.
    status, linearity_status, valid_blocks = _assert_lane_health(
        report, len(HOSTED_SENTINEL_CASES)
    )
    machine: dict[str, Any] = {
        "schema_version": 1,
        "state": "success",
        "health": status,
        "linearity": linearity_status,
        "valid_blocks": valid_blocks,
        "comparisons": {},
    }
    if status == "invalid":
        # Measured, but the linearity diagnostic failed: no ratio is reported
        # and the raw evidence stays available for audit.
        return machine
    comparisons = report.get("comparisons")
    if not isinstance(comparisons, dict):
        raise PreviewError("sentinel report is missing comparisons")
    # `coverage` reports how many cases were measured; it does not prove the
    # comparison maps carry all of them. A report that claims complete coverage
    # but omits a comparison, or carries a subset of cases, would otherwise
    # publish a geometric mean over whatever survived -- the same class of
    # mistake as reading a folded benchmark as throughput. Both comparisons
    # must be present and cover exactly the frozen inventory.
    expected = set(HOSTED_SENTINEL_CASES)
    for key, label in (
        ("candidate_vs_base", "candidate vs base"),
        ("candidate_vs_quickjs_ng", "candidate vs QuickJS-NG"),
    ):
        section = comparisons.get(key)
        if not isinstance(section, dict):
            raise PreviewError(f"sentinel report is missing the {key} comparison")
        cases = section.get("cases")
        if not isinstance(cases, dict):
            raise PreviewError(f"sentinel report {key} is missing its cases")
        rows = []
        for case_id in sorted(cases):
            case = cases[case_id]
            ratio = case.get("ratio") if isinstance(case, dict) else None
            if (
                isinstance(ratio, bool) or not isinstance(ratio, (int, float))
                or not math.isfinite(ratio) or ratio <= 0
            ):
                raise PreviewError(f"sentinel report {key}/{case_id} is missing its ratio")
            # Identifiers are published in code spans, which render
            # backslashes literally, so they are validated rather than
            # escaped. Manifest case IDs are already constrained to this
            # shape; anything else is a malformed report.
            if not _SAFE_CASE_ID.fullmatch(case_id):
                raise PreviewError("sentinel case id contains unsafe Markdown characters")
            rows.append({
                "id": case_id, "ratio": float(ratio),
                **_interval(case, float(ratio), f"{key}/{case_id}"),
            })
        if {row["id"] for row in rows} != expected:
            raise PreviewError(
                f"sentinel report {key} does not cover the complete frozen inventory"
            )
        machine["comparisons"][label] = {
            "geomean": math.exp(sum(math.log(row["ratio"]) for row in rows) / len(rows)),
            "cases": rows,
        }
    return machine


def sentinel_summary(args: argparse.Namespace) -> None:
    """Writes the sentinel lane's machine summary for the composed preview."""
    machine = sentinel_machine(_read_object(args.report, "sentinel report"))
    _write_replace(args.json_output, _json_bytes(machine))
