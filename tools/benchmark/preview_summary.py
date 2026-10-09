"""The hosted preview's published summary.

One document for all three lanes, answer first: an overview a reader can take
in without scrolling, then the per-case tables folded away, then method and
provenance. It renders only already-validated inputs -- the broad lane's
machine summary, the sentinel lane's machine summary, and the external
report -- so presentation can change without touching what a lane must prove
before it reports a ratio.

A lane that produced nothing is named with its reason rather than omitted:
a missing reading must never look like a clean one.

This module imports no other preview module, so every renderer and the
publisher can depend on it without forming a cycle.
"""

from __future__ import annotations

import html
import math
import re
from typing import Any

BROAD_SCOPE = (
    "Every broad case names its callee statically and holds its receiver "
    "fixed, so the specializing tiers can fold the measured operation away "
    "rather than accelerate it. At 100,000 nominal iterations "
    "`plain_function_call` performs 5 real calls and `property_read` 11 real "
    "property operations. Read these ratios as **specializer coverage**, not "
    "as ordinary-interpreter throughput; the interpreter sentinels answer "
    "the latter."
)
SENTINEL_SCOPE = (
    "Cases the specializing tiers cannot fold: real recursion, prototype "
    "dispatch over a rotating receiver, a rotating callee table, capturing "
    "closures, three storage shapes, and computed string-key churn. This is "
    "the ordinary interpreter's cost."
)
LARGEST_CHANGES = 5
_SAFE_ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.\-]*(/[A-Za-z0-9][A-Za-z0-9_.\-]*)?\Z")


def escape_markdown(value: str) -> str:
    """Escape dynamic text for a GitHub Markdown table cell."""
    escaped = html.escape(value, quote=False).replace("\\", "\\\\")
    for character in "`*_{}[]()#+-.!|>":
        escaped = escaped.replace(character, f"\\{character}")
    return escaped.replace("\r", " ").replace("\n", "<br>")


def _code(identifier: str) -> str:
    # A code span renders backslashes literally, so an identifier is shown in
    # one only when it needs no escaping; anything else is escaped as text.
    return f"`{identifier}`" if _SAFE_ID.fullmatch(identifier) else escape_markdown(identifier)


def _change(ratio: float) -> str:
    return f"{(ratio - 1) * 100:+.1f}%"


def _times(ratio: float) -> str:
    return f"{ratio:.3f}×"


def _interval(lower: float | None, upper: float | None, render) -> str:
    if lower is None or upper is None:
        return "—"
    return f"[{render(lower)}, {render(upper)}]"


def _short(revision: str) -> str:
    return revision[:8]


def _geomean(ratios: list[float]) -> float:
    return math.exp(sum(math.log(ratio) for ratio in ratios) / len(ratios))


def _broad_rows(broad: dict[str, Any] | None) -> list[dict[str, Any]]:
    """Per-case rows of the broad lane, or nothing when it reports no direction."""
    comparisons = (broad or {}).get("comparisons") or {}
    base = comparisons.get("candidate vs base")
    reference = comparisons.get("candidate vs QuickJS-NG")
    if not base or not reference:
        return []
    by_reference = {case["id"]: case for case in reference["cases"]}
    return [
        {"id": case["id"], "base": case, "reference": by_reference[case["id"]]}
        for case in base["cases"]
    ]


def _sentinel_rows(sentinel: dict[str, Any] | None) -> list[dict[str, Any]]:
    comparisons = (sentinel or {}).get("comparisons") or {}
    base = comparisons.get("candidate vs base")
    reference = comparisons.get("candidate vs QuickJS-NG")
    if not base or not reference:
        return []
    by_reference = {case["id"]: case for case in reference["cases"]}
    return [
        {"id": case["id"], "base": case, "reference": by_reference[case["id"]]}
        for case in base["cases"]
    ]


def _external_interval(case: dict[str, Any], role: str) -> tuple[float | None, float | None]:
    effect = (case.get("paired_comparisons") or {}).get(role)
    if not effect:
        return None, None
    interval = effect["confidence_interval"]
    return interval["lower"], interval["upper"]


def _overview(
    broad: dict[str, Any] | None,
    external: dict[str, Any] | None,
    sentinel: dict[str, Any] | None,
    notes: dict[str, str],
) -> list[str]:
    lines = [
        "| Workload group | Cases compared | Change vs base | Time vs QuickJS-NG |",
        "| --- | ---: | ---: | ---: |",
    ]

    def absent(label: str, reason: str) -> None:
        lines.append(f"| {label} | — | — | — |")
        missing.append(f"**{label}:** {reason}")

    missing: list[str] = []
    sentinel_rows = _sentinel_rows(sentinel)
    if sentinel_rows:
        comparisons = sentinel["comparisons"]
        lines.append(
            f"| Interpreter sentinels | {len(sentinel_rows)}/{len(sentinel_rows)} | "
            f"{_change(comparisons['candidate vs base']['geomean'])} | "
            f"{_times(comparisons['candidate vs QuickJS-NG']['geomean'])} |"
        )
    elif sentinel is not None:
        absent(
            "Interpreter sentinels",
            "measured, but the linearity diagnostic failed, so no ratio is reported.",
        )
    else:
        absent("Interpreter sentinels", notes.get("sentinel", "no evidence was produced."))
    if external is not None:
        for suite in external["suites"]:
            total = suite["case_count"]
            versus_base = suite["base_comparable_case_count"]
            versus_reference = suite["comparable_case_count"]
            compared = (
                f"{versus_base}/{total}" if versus_base == versus_reference
                else f"base {versus_base}/{total} · QuickJS-NG {versus_reference}/{total}"
            )
            base_ratio = suite["diagnostic_candidate_over_base_geomean_ratio"]
            reference_ratio = suite["diagnostic_comparable_case_geomean_ratio"]
            lines.append(
                f"| {escape_markdown(suite['name'])} | {compared} | "
                f"{'—' if base_ratio is None else _change(base_ratio)} | "
                f"{'—' if reference_ratio is None else _times(reference_ratio)} |"
            )
    else:
        absent("External benchmarks", notes.get("external", "no evidence was produced."))
    broad_rows = _broad_rows(broad)
    if broad_rows:
        comparisons = broad["comparisons"]
        lines.append(
            f"| Broad microbenchmarks (specializer coverage) | "
            f"{len(broad_rows)}/{len(broad_rows)} | "
            f"{_change(comparisons['candidate vs base']['ratio'])} | "
            f"{_times(comparisons['candidate vs QuickJS-NG']['ratio'])} |"
        )
    elif broad is not None:
        absent(
            "Broad microbenchmarks (specializer coverage)",
            "measured, but the linearity diagnostic failed, so no ratio is reported.",
        )
    else:
        absent(
            "Broad microbenchmarks (specializer coverage)",
            notes.get("broad", "no evidence was produced."),
        )
    lines.append("")
    if missing:
        lines.append("**No performance direction is reported for:**")
        lines.append("")
        lines.extend(f"- {entry}" for entry in missing)
        lines.append("")
    return lines


def _largest_changes(
    broad: dict[str, Any] | None,
    external: dict[str, Any] | None,
    sentinel: dict[str, Any] | None,
) -> list[str]:
    observed: list[tuple[str, str, float, float | None, float | None]] = []
    for row in _sentinel_rows(sentinel):
        case = row["base"]
        observed.append((
            "Interpreter sentinels", row["id"], case["ratio"],
            case.get("ci_lower"), case.get("ci_upper"),
        ))
    for suite in (external or {}).get("suites", ()):
        for case in suite["cases"]:
            if case["candidate_over_base"] is None:
                continue
            lower, upper = _external_interval(case, "base")
            observed.append((
                suite["name"], f"{suite['id']}/{case['id']}",
                case["candidate_over_base"], lower, upper,
            ))
    for row in _broad_rows(broad):
        case = row["base"]
        observed.append((
            "Broad microbenchmarks", row["id"], case["ratio"],
            case["ci_lower"], case["ci_upper"],
        ))
    if not observed:
        return []
    observed.sort(key=lambda item: (-abs(math.log(item[2])), item[0], item[1]))
    lines = [
        "**Largest observed changes vs base**",
        "",
        "| Case | Workload group | Change vs base | 95% interval |",
        "| --- | --- | ---: | ---: |",
    ]
    for group, identifier, ratio, lower, upper in observed[:LARGEST_CHANGES]:
        lines.append(
            f"| {_code(identifier)} | {escape_markdown(group)} | {_change(ratio)} | "
            f"{_interval(lower, upper, _change)} |"
        )
    lines.extend([
        "",
        f"Selected by observed magnitude from {len(observed)} cases, so some "
        "always appear even between identical builds; intervals are per case "
        "and not adjusted for that selection.",
        "",
    ])
    return lines


def _incomplete(external: dict[str, Any] | None) -> list[str]:
    entries = []
    for suite in (external or {}).get("suites", ()):
        for case in suite["cases"]:
            if case["candidate_over_base"] is not None and case["candidate_over_quickjs_ng"] is not None:
                continue
            capability = case.get("capability") or {}
            states = ", ".join(
                f"{label} {escape_markdown(str(capability.get(role, 'not run')))}"
                for role, label in (
                    ("candidate", "candidate"), ("base", "base"), ("quickjs-ng", "QuickJS-NG"),
                )
            )
            entries.append(
                f"- {_code(suite['id'] + '/' + case['id'])} — {states}. "
                "It is excluded from the affected suite ratios."
            )
    if not entries:
        return []
    return ["**Cases without a complete comparison**", "", *entries, ""]


def _sentinel_details(sentinel: dict[str, Any] | None) -> list[str]:
    rows = _sentinel_rows(sentinel)
    if not rows:
        return []
    lines = [
        "<details>",
        f"<summary>Interpreter sentinels — all {len(rows)} cases</summary>",
        "",
        SENTINEL_SCOPE,
        "",
        "| Case | Time vs base | 95% interval | Time vs QuickJS-NG | 95% interval |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]
    for row in rows:
        base, reference = row["base"], row["reference"]
        lines.append(
            f"| {_code(row['id'])} | {_times(base['ratio'])} | "
            f"{_interval(base.get('ci_lower'), base.get('ci_upper'), _times)} | "
            f"{_times(reference['ratio'])} | "
            f"{_interval(reference.get('ci_lower'), reference.get('ci_upper'), _times)} |"
        )
    lines.extend(["", "</details>", ""])
    return lines


def _milliseconds(value: int | None) -> str:
    return "—" if value is None else f"{value / 1_000_000:,.1f}"


def _external_details(external: dict[str, Any] | None) -> list[str]:
    if external is None:
        return []
    total = sum(suite["case_count"] for suite in external["suites"])
    lines = [
        "<details>",
        f"<summary>External benchmarks — all {total} cases</summary>",
        "",
        "Pinned, neutral shell ports: no row is an official JetStream, Kraken, "
        "or SunSpider score, and a suite ratio is a geometric mean over the "
        "cases that could be compared. Times are whole-process wall time in "
        "milliseconds, including startup, parsing, execution and shutdown; "
        "ratios use paired blocks.",
        "",
    ]
    for suite in external["suites"]:
        lines.extend([
            f"**{escape_markdown(suite['name'])}**",
            "",
            "| Case | Candidate ms | Base ms | QuickJS-NG ms | Time vs base | 95% interval | Time vs QuickJS-NG | 95% interval |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ])
        for case in suite["cases"]:
            medians = case["median_duration_ns"]
            base_ratio = case["candidate_over_base"]
            reference_ratio = case["candidate_over_quickjs_ng"]
            lines.append(
                f"| {_code(case['id'])} | {_milliseconds(medians['candidate'])} | "
                f"{_milliseconds(medians['base'])} | {_milliseconds(medians['quickjs-ng'])} | "
                f"{'—' if base_ratio is None else _times(base_ratio)} | "
                f"{_interval(*_external_interval(case, 'base'), _times)} | "
                f"{'—' if reference_ratio is None else _times(reference_ratio)} | "
                f"{_interval(*_external_interval(case, 'quickjs-ng'), _times)} |"
            )
        lines.append("")
    lines.extend(["</details>", ""])
    return lines


def _broad_details(broad: dict[str, Any] | None) -> list[str]:
    rows = _broad_rows(broad)
    if not rows:
        return []
    comparisons = broad["comparisons"]
    lines = [
        "<details>",
        f"<summary>Broad microbenchmarks — all {len(rows)} cases</summary>",
        "",
        BROAD_SCOPE,
        "",
        "| Comparison | Overall ratio | 95% interval |",
        "| --- | ---: | ---: |",
    ]
    for label in ("candidate vs base", "candidate vs QuickJS-NG"):
        result = comparisons[label]
        lines.append(
            f"| {label} | {result['ratio']:.4f}× | "
            f"[{result['ci_lower']:.4f}×, {result['ci_upper']:.4f}×] |"
        )
    lines.extend([
        "",
        "Medians are wall ns per operation.",
        "",
        "| Case | Candidate ns/op | Base ns/op | Time vs base | QuickJS-NG ns/op | Time vs QuickJS-NG |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ])
    for row in rows:
        base, reference = row["base"], row["reference"]
        lines.append(
            f"| {_code(row['id'])} | {base['candidate_median_ns_per_op']:,.2f} | "
            f"{base['comparator_median_ns_per_op']:,.2f} | {_times(base['ratio'])} | "
            f"{reference['comparator_median_ns_per_op']:,.2f} | {_times(reference['ratio'])} |"
        )
    lines.extend(["", "</details>", ""])
    return lines


def _method(
    broad: dict[str, Any] | None,
    external: dict[str, Any] | None,
    sentinel: dict[str, Any] | None,
) -> list[str]:
    lines = [
        "<details>",
        "<summary>Method, health and provenance</summary>",
        "",
        "- Classification: informational only — non-gating — not a fixed-hardware claim. "
        "GitHub-hosted runners are variable; missing or malformed evidence fails, "
        "and a completed noisy measurement is inconclusive.",
        "- Ratio = candidate wall time ÷ comparator wall time, per operation in the "
        "internal lanes and per process in the external lane.",
    ]
    if broad is not None:
        lines.extend([
            f"- Broad lane health: {broad['health']}; block health: non_claim; "
            f"linearity: {broad['linearity']}",
            f"- Valid blocks: `{broad['valid_blocks']}/{broad['requested_blocks']}`",
        ])
    if sentinel is not None:
        lines.append(
            f"- Sentinel lane health: {sentinel['health']}; linearity: "
            f"{sentinel['linearity']}; valid blocks: `{sentinel['valid_blocks']}/3`"
        )
    if external is not None:
        lines.append(
            f"- External lane: {external['blocks']} blocks; every per-case interval is "
            f"inconclusive below {external['analysis']['min_blocks']} blocks"
        )
    if broad is not None:
        engines = broad["engines"]
        lines.extend([
            f"- Harness ownership mode: `{broad['harness']['mode']}` at "
            f"`{broad['harness']['revision']}`",
            f"- Integrity scope: `{broad['integrity_scope']}`",
            "- Security boundary: candidate build/execution is not sandboxed; "
            "artifacts do not resist a malicious candidate",
            f"- Profile: `{broad['profile_id']}`",
            f"- Candidate: `{engines['candidate']['source_revision']}` / "
            f"`{engines['candidate']['binary_sha256']}`",
            f"- Base: `{engines['base']['source_revision']}` / "
            f"`{engines['base']['binary_sha256']}`",
            f"- QuickJS-NG: `{engines['quickjs-ng']['source_revision']}` / "
            f"`{engines['quickjs-ng']['binary_sha256']}`",
        ])
    lines.extend(["", "</details>", ""])
    return lines


def render_preview(
    broad: dict[str, Any] | None,
    external: dict[str, Any] | None = None,
    sentinel: dict[str, Any] | None = None,
    notes: dict[str, str] | None = None,
    *,
    banner: str | None = None,
) -> str:
    """Render the published summary from whichever lanes produced evidence.

    `broad` and `sentinel` are the lanes' validated machine summaries and
    `external` is the external report. A lane passed as `None` is reported as
    absent with its entry in `notes`, which must already be safe Markdown.
    `banner` leads the document when the preview as a whole is incomplete.
    """
    notes = notes or {}
    lines = ["## Performance Preview", ""]
    if banner:
        lines.extend([f"> {banner}", ""])
    if broad is not None:
        engines = broad["engines"]
        lines.extend([
            f"**Candidate `{_short(engines['candidate']['source_revision'])}` vs base "
            f"`{_short(engines['base']['source_revision'])}`** · reference QuickJS-NG "
            f"`{_short(engines['quickjs-ng']['source_revision'])}`",
            "",
        ])
    lines.extend([
        "> Informational and non-gating. Three blocks on a shared GitHub-hosted "
        "runner show a direction; they cannot confirm a regression or accept an "
        "optimization.",
        "",
        "Every number compares candidate time with comparator time, so a "
        "**negative change or a ratio below 1× means the candidate took less "
        "time**. Changes are observed estimates, not confirmed effects.",
        "",
    ])
    lines.extend(_overview(broad, external, sentinel, notes))
    lines.extend(_largest_changes(broad, external, sentinel))
    lines.extend(_incomplete(external))
    lines.extend(_sentinel_details(sentinel))
    lines.extend(_external_details(external))
    lines.extend(_broad_details(broad))
    lines.extend(_method(broad, external, sentinel))
    return "\n".join(lines)
