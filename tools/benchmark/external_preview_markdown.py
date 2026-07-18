"""Markdown rendering for non-claim external benchmark previews."""

from __future__ import annotations

from typing import Any


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        "## External Benchmark Preview", "",
        "> **Informational only.** These are pinned, neutral shell ports; no row is an official",
        "> JetStream, Kraken, or SunSpider score, and incomplete suites have no aggregate score.",
        "",
        "| Suite | Candidate/base comparable | Candidate/base | Candidate/base wins | QuickJS comparable | Candidate/QuickJS-NG | Candidate/QuickJS-NG wins |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for suite in report["suites"]:
        ratio = suite["diagnostic_comparable_case_geomean_ratio"]
        ratio_text = f"{ratio:.3f}x" if ratio is not None else "—"
        base_ratio = suite["diagnostic_candidate_over_base_geomean_ratio"]
        base_ratio_text = f"{base_ratio:.3f}x" if base_ratio is not None else "—"
        lines.append(
            f"| {suite['name']} | {suite['base_comparable_case_count']}/{suite['case_count']} | "
            f"{base_ratio_text} | {suite['base_wins']['candidate']}/{suite['base_wins']['base']} | "
            f"{suite['comparable_case_count']}/{suite['case_count']} | {ratio_text} | "
            f"{suite['wins']['candidate']}/{suite['wins']['quickjs-ng']} |"
        )
    lines.extend([
        "", "### External per-case performance", "",
        "Median wall time is the outer process duration per run. Lower ratios favor qjs-rust.", "",
        "| Suite / case | Candidate ms/run | Base ms/run | QuickJS-NG ms/run | Candidate/base | Candidate/QuickJS-NG |",
        "|---|---:|---:|---:|---:|---:|",
    ])
    for suite in report["suites"]:
        for case in suite["cases"]:
            candidate = case["median_duration_ns"]["candidate"]
            base = case["median_duration_ns"]["base"]
            quickjs = case["median_duration_ns"]["quickjs-ng"]
            base_ratio = case["candidate_over_base"]
            ratio = case["candidate_over_quickjs_ng"]
            candidate_text = "—" if candidate is None else f"{candidate / 1_000_000:.3f}"
            base_text = "—" if base is None else f"{base / 1_000_000:.3f}"
            quickjs_text = "—" if quickjs is None else f"{quickjs / 1_000_000:.3f}"
            base_ratio_text = "—" if base_ratio is None else f"{base_ratio:.3f}x"
            ratio_text = "—" if ratio is None else f"{ratio:.3f}x"
            lines.append(
                f"| `{suite['id']}/{case['id']}` | {candidate_text} | {base_text} | "
                f"{quickjs_text} | {base_ratio_text} | {ratio_text} |"
            )
    lines.extend([
        "", "Lower ratios are faster for qjs-rust. The ratio is a diagnostic geometric mean",
        "over explicitly reported comparable cases, never a substitute for a suite score.", "",
    ])
    return "\n".join(lines)
