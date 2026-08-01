"""The generic-path sentinel lane's Markdown section.

It lives beside `preview` rather than inside it because that module carries
the whole hosted orchestration -- manifest preparation, receipts, the broad
summary, status writing, and the CLI -- and this is a self-contained
renderer for the second lane.

The import direction is one-way: this module reads `preview`'s shared
vocabulary, and `preview` reaches this renderer only from its argument parser,
where the deferred import keeps the two from forming a cycle.
"""

from __future__ import annotations

import argparse
import math

from .preview import (
    HOSTED_SENTINEL_CASES,
    PreviewError,
    _assert_lane_health,
    _read_object,
    _SAFE_CASE_ID,
    _write_replace,
    escape_markdown,
)


def sentinel_summary(args: argparse.Namespace) -> None:
    """Renders the generic-path sentinel lane as its own summary section.

    It deliberately does not reuse `summarize`, whose validation is bound to
    the broad portfolio's exact case inventory. The two lanes answer different
    questions and must not be conflated: broad reports how much the
    specializing tiers recognize, and these report what the ordinary
    interpreter costs when they cannot.
    """
    report = _read_object(args.report, "sentinel report")
    # The same invariants the broad lane enforces, against this lane's frozen
    # inventory. A degraded run must produce no ratios at all rather than a
    # number that reads as ordinary-interpreter cost.
    status, linearity_status, valid_blocks = _assert_lane_health(
        report, len(HOSTED_SENTINEL_CASES)
    )
    if status == "invalid":
        _write_replace(args.markdown, ("\n".join([
            "", "### Generic-path sentinels", "",
            "> **No ordinary-interpreter reading is reported.**",
            "> The sentinel measurement completed but failed its linearity "
            "diagnostic; raw evidence is preserved for audit.",
            "",
            f"- Health: invalid; linearity: {escape_markdown(str(linearity_status))}",
            f"- Valid blocks: `{valid_blocks}/3`",
            "",
        ]) + "\n").encode("utf-8"))
        return
    comparisons = report.get("comparisons")
    if not isinstance(comparisons, dict):
        raise PreviewError("sentinel report is missing comparisons")
    lines = [
        "", "### Generic-path sentinels", "",
        "> Cases the specializing tiers cannot fold: real recursion, prototype "
        "dispatch over a rotating receiver, a rotating callee table, capturing "
        "closures, three storage shapes, and computed string-key churn. This is "
        "the ordinary interpreter's cost.",
        "",
    ]
    rows = []
    for key, label in (
        ("candidate_vs_base", "candidate vs base"),
        ("candidate_vs_quickjs_ng", "candidate vs QuickJS-NG"),
    ):
        section = comparisons.get(key)
        if not isinstance(section, dict):
            continue
        cases = section.get("cases")
        if not isinstance(cases, dict) or not cases:
            continue
        ratios = []
        for case_id in sorted(cases):
            case = cases[case_id]
            if isinstance(case, dict) and isinstance(case.get("ratio"), (int, float)):
                ratios.append((case_id, float(case["ratio"])))
        if not ratios:
            continue
        geomean = math.exp(sum(math.log(r) for _, r in ratios) / len(ratios))
        rows.append((label, geomean, ratios))
    if not rows:
        lines.append("> No complete sentinel comparison was produced.")
        _write_replace(args.markdown, ("\n".join(lines) + "\n").encode("utf-8"))
        return
    lines += ["| Comparison | Geometric mean | Cases |", "| --- | ---: | ---: |"]
    for label, geomean, ratios in rows:
        lines.append(f"| {label} | {geomean:.4f}× | {len(ratios)} |")
    lines += ["", "| Case | " + " | ".join(label for label, _, _ in rows) + " |",
              "| --- | " + " | ".join("---:" for _ in rows) + " |"]
    case_ids = sorted({case_id for _, _, ratios in rows for case_id, _ in ratios})
    for case_id in case_ids:
        # A code span renders backslashes literally, so the identifier is
        # validated rather than escaped. Manifest case IDs are already
        # constrained to this shape; anything else is a malformed report.
        if not _SAFE_CASE_ID.fullmatch(case_id):
            raise PreviewError("sentinel case id contains unsafe Markdown characters")
        cells = []
        for _, _, ratios in rows:
            value = dict(ratios).get(case_id)
            cells.append(f"{value:.4f}×" if value is not None else "—")
        lines.append(f"| `{case_id}` | " + " | ".join(cells) + " |")
    lines.append("")
    _write_replace(args.markdown, ("\n".join(lines) + "\n").encode("utf-8"))
