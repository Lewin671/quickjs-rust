"""The hosted preview's published summary.

Written for someone who opens the run to learn one thing: did this change
make the engine faster or slower. So it opens with that answer in a
sentence, says every number in words ("1.20× slower", "no clear change")
instead of leaving a ratio to be decoded, names the two sides by what they
are ("this commit", "the commit before it"), and lists a single test only
when it moved further than identical builds do. The per-test tables and the
provenance are folded away below.

It renders only already-validated inputs -- the broad lane's machine
summary, the sentinel lane's machine summary, and the external report -- so
presentation can change without touching what a lane must prove before it
reports a ratio. A lane that produced nothing is named with its reason
rather than omitted: a missing reading must never look like a clean one.

This module imports no other preview module, so every renderer and the
publisher can depend on it without forming a cycle.
"""

from __future__ import annotations

import html
import math
import re
from typing import Any

BROAD_SCOPE = (
    "Single operations in a tight loop: a call, a property read, an array "
    "write. Every case names its callee statically and holds its receiver "
    "fixed, so the specializing tiers can fold the measured operation away "
    "rather than accelerate it: at 100,000 nominal iterations "
    "`plain_function_call` performs 5 real calls and `property_read` 11 real "
    "property operations. Read this group as **specializer coverage** -- how "
    "much the optimizer recognizes -- not as interpreter speed; the "
    "interpreter basics answer that."
)
SENTINEL_SCOPE = (
    "Work the specializing tiers cannot fold: real recursion, prototype "
    "dispatch over a rotating receiver, a rotating callee table, capturing "
    "closures, three storage shapes, and computed string-key churn. This is "
    "the ordinary interpreter's cost."
)
# What identical builds show on the hosted runners, which is what "no clear
# change" has to mean. Between two unsharded runs of equivalent code (runs
# 37933477498 and 37950725509) every group moved by under 0.5% against the
# base and the furthest single test by 6.9%; the dated record is in
# tasks/T017. A group is called out beyond 2%, a single test beyond 7%.
GROUP_NOISE = 1.02
TEST_NOISE = 1.07
LISTED_TESTS = 5
# How the two builds are named, by what the run compared: the subject, the
# build it is compared with, and the short forms used in table headings.
SIDES = {
    "base_owned_harness": ("This pull request", "its base", "base", "This PR"),
    "main_push_head_owned_harness": (
        "This commit", "the commit before it", "previous commit", "This commit",
    ),
    "manual_main_head_owned_harness": (
        "The selected commit", "the selected base", "base", "Selected commit",
    ),
}
DEFAULT_SIDES = ("The candidate", "its base", "base", "Candidate")
CAPABILITY = {"ok": "finished", "timeout": "timed out", "not_run": "was not run"}
HOW_TO_READ = (
    "*How to read this.* Every cell compares running time, so **slower and "
    "faster mean what they say**: \"1.20× slower\" took 1.2 times as long. ⚪ "
    f"marks a difference under {(GROUP_NOISE - 1) * 100:.0f}%, which identical "
    "builds also show on these shared machines. *Interpreter basics* are calls, "
    "property access and recursion the engine cannot shortcut; *micro-operations* "
    "are single operations its optimizer may skip entirely, so that row shows how "
    "much the optimizer recognizes more than how fast the engine is. This is a "
    "quick check from three rounds, not a verdict to act on alone."
)
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


def _beyond(ratio: float, noise: float) -> bool:
    """Whether a time ratio differs from 1 by at least the floor, as displayed.

    The floor is applied to the same quantity the reader is shown -- the
    percentage change in time -- so a row is never listed as beyond 7% while
    reading "6.6% faster". The small tolerance keeps a ratio that is exactly
    at the floor on the listed side despite floating-point representation.
    """
    return abs(ratio - 1) >= (noise - 1) - 1e-12


def _percent(ratio: float) -> str:
    """A change against the base in words: `1.5% slower`, `2.0% faster`."""
    percent = (ratio - 1) * 100
    if abs(percent) < 0.05:
        return "the same"
    return f"{abs(percent):.1f}% {'slower' if percent > 0 else 'faster'}"


def _factor(ratio: float) -> str:
    """A comparison with the reference in words: `1.20× slower`."""
    if abs(ratio - 1) < 0.005:
        return "the same"
    return f"{ratio:.2f}× slower" if ratio > 1 else f"{1 / ratio:.2f}× faster"


def _group_change(ratio: float | None) -> str:
    if ratio is None:
        return "—"
    if not _beyond(ratio, GROUP_NOISE):
        words = _percent(ratio)
        return "⚪ no clear change" + ("" if words == "the same" else f" ({words})")
    return f"{'🔴' if ratio > 1 else '🟢'} {_percent(ratio)}"


def _group_reference(ratio: float | None) -> str:
    if ratio is None:
        return "—"
    if not _beyond(ratio, GROUP_NOISE):
        words = _factor(ratio)
        return "⚪ about the same" + ("" if words == "the same" else f" ({words})")
    return f"{'🔴' if ratio > 1 else '🟢'} {_factor(ratio)}"


def _range(lower: float | None, upper: float | None) -> str:
    """An interval against the base, each end in the same words as the estimate."""
    if lower is None or upper is None:
        return "—"
    return f"{_percent(lower)} to {_percent(upper)}"


def _short(revision: str) -> str:
    return revision[:8]


def _lane_rows(lane: dict[str, Any] | None) -> list[dict[str, Any]]:
    """Per-test rows of an internal lane, or nothing when it reports no direction."""
    comparisons = (lane or {}).get("comparisons") or {}
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


def _groups(
    broad: dict[str, Any] | None,
    external: dict[str, Any] | None,
    sentinel: dict[str, Any] | None,
    notes: dict[str, str],
) -> tuple[list[dict[str, Any]], list[str]]:
    """One entry per workload group, real programs first, and what is absent."""
    groups: list[dict[str, Any]] = []
    absent: list[str] = []

    def missing(name: str, reason: str) -> None:
        groups.append({"name": name, "tests": "—", "base": None, "reference": None})
        absent.append(f"**{name}:** {reason}")

    if external is not None:
        for suite in external["suites"]:
            total = suite["case_count"]
            versus_base = suite["base_comparable_case_count"]
            versus_reference = suite["comparable_case_count"]
            if versus_base == versus_reference == total:
                tests = f"{total} programs"
            elif versus_base == versus_reference:
                tests = f"{versus_base} of {total} programs"
            else:
                tests = (
                    f"{versus_base} of {total} programs against the base, "
                    f"{versus_reference} against QuickJS-NG"
                )
            groups.append({
                "name": escape_markdown(suite["name"]), "tests": tests,
                "base": suite["diagnostic_candidate_over_base_geomean_ratio"],
                "reference": suite["diagnostic_comparable_case_geomean_ratio"],
            })
    else:
        missing("Benchmark programs", notes.get("external", "no evidence was produced."))
    for name, lane, key, ratio_key in (
        ("Interpreter basics", sentinel, "sentinel", "geomean"),
        ("Micro-operations", broad, "broad", "ratio"),
    ):
        rows = _lane_rows(lane)
        if rows:
            comparisons = lane["comparisons"]
            groups.append({
                "name": name, "tests": f"{len(rows)} tests",
                "base": comparisons["candidate vs base"][ratio_key],
                "reference": comparisons["candidate vs QuickJS-NG"][ratio_key],
            })
        elif lane is not None:
            missing(name, "measured, but the linearity diagnostic failed, so no ratio is reported.")
        else:
            missing(name, notes.get(key, "no evidence was produced."))
    return groups, absent


def _join(names: list[str]) -> str:
    return names[0] if len(names) == 1 else ", ".join(names[:-1]) + " and " + names[-1]


def _verdict(
    groups: list[dict[str, Any]], subject: str, base: str, identical: bool,
) -> list[str]:
    measured = [group for group in groups if group["base"] is not None]
    if not measured:
        # No comparison with the base at all; the reference still has an answer.
        reference = _reference_verdict(groups)
        return [" ".join(reference), ""] if reference else []
    slower = [g["name"] for g in measured if g["base"] > 1 and _beyond(g["base"], GROUP_NOISE)]
    faster = [g["name"] for g in measured if g["base"] < 1 and _beyond(g["base"], GROUP_NOISE)]
    if identical:
        # Nothing that reaches the engine changed, so there is no effect to
        # look for and the reader should not go looking for one.
        headline = (
            f"**{subject} did not change the engine:** its build is byte-identical "
            f"to {base}, so every difference below is measurement noise."
        )
    elif slower and faster:
        headline = (
            f"**{subject} is mixed against {base}:** slower on {_join(slower)}, "
            f"faster on {_join(faster)}."
        )
    elif slower:
        headline = f"**{subject} looks slower than {base}** on {_join(slower)}."
    elif faster:
        headline = f"**{subject} looks faster than {base}** on {_join(faster)}."
    else:
        headline = f"**{subject}: no clear change in speed** compared with {base}."
    return [" ".join([headline, *_reference_verdict(groups)]), ""]


def _reference_verdict(groups: list[dict[str, Any]]) -> list[str]:
    """How the groups stand against the reference, whatever their base coverage."""
    compared = [group for group in groups if group["reference"] is not None]
    if not compared:
        return []
    behind = sum(g["reference"] > 1 and _beyond(g["reference"], GROUP_NOISE) for g in compared)
    ahead = sum(g["reference"] < 1 and _beyond(g["reference"], GROUP_NOISE) for g in compared)
    return [
        f"Against QuickJS-NG it is slower on {behind} and faster on {ahead} of "
        f"{len(compared)} workload groups."
    ]


def _overview(groups: list[dict[str, Any]], absent: list[str], short_base: str) -> list[str]:
    lines = [
        f"| Workload | Size | Against the {short_base} | Against QuickJS-NG |",
        "| --- | --- | --- | --- |",
    ]
    for group in groups:
        lines.append(
            f"| {group['name']} | {group['tests']} | {_group_change(group['base'])} | "
            f"{_group_reference(group['reference'])} |"
        )
    lines.append("")
    if absent:
        lines.append("**No performance direction is reported for:**")
        lines.append("")
        lines.extend(f"- {entry}" for entry in absent)
        lines.append("")
    return lines


def _moved_tests(
    broad: dict[str, Any] | None,
    external: dict[str, Any] | None,
    sentinel: dict[str, Any] | None,
    short_base: str,
    identical: bool = False,
) -> list[str]:
    observed: list[tuple[str, str, float, float | None, float | None]] = []
    for suite in (external or {}).get("suites", ()):
        for case in suite["cases"]:
            if case["candidate_over_base"] is None:
                continue
            lower, upper = _external_interval(case, "base")
            observed.append((
                escape_markdown(suite["name"]), f"{suite['id']}/{case['id']}",
                case["candidate_over_base"], lower, upper,
            ))
    for name, lane in (("Interpreter basics", sentinel), ("Micro-operations", broad)):
        for row in _lane_rows(lane):
            case = row["base"]
            observed.append((
                name, row["id"], case["ratio"], case.get("ci_lower"), case.get("ci_upper"),
            ))
    if not observed:
        return []
    limit = f"{(TEST_NOISE - 1) * 100:.0f}%"
    moved = [item for item in observed if _beyond(item[2], TEST_NOISE)]
    if not moved:
        return [
            f"**No single test moved by more than {limit}** against the {short_base}, "
            f"out of {len(observed)}. Identical builds differ by that much on these "
            "shared machines, so smaller movements are not listed.",
            "",
        ]
    moved.sort(key=lambda item: (-abs(math.log(item[2])), item[0], item[1]))
    if identical:
        # The builds are the same bytes, so these are not candidates for a
        # second look; they measure how noisy this particular run was.
        _, identifier, ratio, _, _ = moved[0]
        return [
            f"**{len(moved)} of {len(observed)} tests still differed by more than "
            f"{limit}** between the two identical builds (the furthest: "
            f"{_code(identifier)}, {_percent(ratio)}). That is this run's own noise, "
            "and more of it than these machines usually show.",
            "",
        ]
    lines = [
        f"**Tests that moved by more than {limit} against the {short_base}** "
        f"({len(moved)} of {len(observed)})",
        "",
        f"| Test | Workload | Against the {short_base} | Likely range |",
        "| --- | --- | --- | --- |",
    ]
    for group, identifier, ratio, lower, upper in moved[:LISTED_TESTS]:
        lines.append(
            f"| {_code(identifier)} | {group} | {_percent(ratio)} | "
            f"{_range(lower, upper)} |"
        )
    lines.extend([
        "",
        f"Identical builds differ by up to about {limit} per test on these shared "
        "machines, so smaller movements are not listed. A test here is worth a "
        "second look, not a confirmed change.",
        "",
    ])
    return lines


def _not_compared(external: dict[str, Any] | None, sides: tuple[str, ...]) -> list[str]:
    entries = []
    for suite in (external or {}).get("suites", ()):
        for case in suite["cases"]:
            if case["candidate_over_base"] is not None and case["candidate_over_quickjs_ng"] is not None:
                continue
            capability = case.get("capability") or {}
            states = ", ".join(
                f"{label} {escape_markdown(CAPABILITY.get(str(state), str(state)))}"
                for label, state in (
                    (sides[0].lower(), capability.get("candidate", "not_run")),
                    (sides[1], capability.get("base", "not_run")),
                    ("QuickJS-NG", capability.get("quickjs-ng", "not_run")),
                )
            )
            suite_name = escape_markdown(suite["name"])
            if case["candidate_over_base"] is not None:
                excluded = f"the {suite_name} comparison with QuickJS-NG"
            elif case["candidate_over_quickjs_ng"] is not None:
                excluded = f"the {suite_name} comparison with {sides[1]}"
            else:
                excluded = f"both {suite_name} comparisons"
            entries.append(
                f"- {_code(suite['id'] + '/' + case['id'])} — {states}. "
                f"It is left out of {excluded}."
            )
    if not entries:
        return []
    return ["**Programs that could not be compared**", "", *entries, ""]


def _milliseconds(value: int | None) -> str:
    return "—" if value is None else f"{value / 1_000_000:,.1f}"


def _external_details(
    external: dict[str, Any] | None, short_base: str, column: str,
) -> list[str]:
    if external is None:
        return []
    total = sum(suite["case_count"] for suite in external["suites"])
    lines = [
        "<details>",
        f"<summary>Benchmark programs — all {total}</summary>",
        "",
        "Whole programs from the public suites, run once per round as their own "
        "process. Times are wall-clock milliseconds including startup. These are "
        "pinned, neutral ports: no row is an official JetStream, Kraken, or "
        "SunSpider score, and a suite's number is the geometric mean over the "
        "programs that could be compared.",
        "",
    ]
    for suite in external["suites"]:
        lines.extend([
            f"**{escape_markdown(suite['name'])}**",
            "",
            f"| Program | {column} ms | {short_base.capitalize()} ms | QuickJS-NG ms | "
            f"Against the {short_base} | Likely range | Against QuickJS-NG |",
            "| --- | ---: | ---: | ---: | --- | --- | --- |",
        ])
        for case in suite["cases"]:
            medians = case["median_duration_ns"]
            base_ratio = case["candidate_over_base"]
            reference_ratio = case["candidate_over_quickjs_ng"]
            lines.append(
                f"| {_code(case['id'])} | {_milliseconds(medians['candidate'])} | "
                f"{_milliseconds(medians['base'])} | {_milliseconds(medians['quickjs-ng'])} | "
                f"{'—' if base_ratio is None else _percent(base_ratio)} | "
                f"{_range(*_external_interval(case, 'base'))} | "
                f"{'—' if reference_ratio is None else _factor(reference_ratio)} |"
            )
        lines.append("")
    lines.extend(["</details>", ""])
    return lines


def _sentinel_details(sentinel: dict[str, Any] | None, short_base: str) -> list[str]:
    rows = _lane_rows(sentinel)
    if not rows:
        return []
    lines = [
        "<details>",
        f"<summary>Interpreter basics — all {len(rows)} tests</summary>",
        "",
        SENTINEL_SCOPE,
        "",
        f"| Test | Against the {short_base} | Likely range | Against QuickJS-NG |",
        "| --- | --- | --- | --- |",
    ]
    for row in rows:
        base, reference = row["base"], row["reference"]
        lines.append(
            f"| {_code(row['id'])} | {_percent(base['ratio'])} | "
            f"{_range(base.get('ci_lower'), base.get('ci_upper'))} | "
            f"{_factor(reference['ratio'])} |"
        )
    lines.extend(["", "</details>", ""])
    return lines


def _broad_details(broad: dict[str, Any] | None, short_base: str, column: str) -> list[str]:
    rows = _lane_rows(broad)
    if not rows:
        return []
    lines = [
        "<details>",
        f"<summary>Micro-operations — all {len(rows)} tests</summary>",
        "",
        BROAD_SCOPE,
        "",
        "Times are nanoseconds per operation.",
        "",
        f"| Test | {column} ns | {short_base.capitalize()} ns | QuickJS-NG ns | "
        f"Against the {short_base} | Likely range | Against QuickJS-NG |",
        "| --- | ---: | ---: | ---: | --- | --- | --- |",
    ]
    for row in rows:
        base, reference = row["base"], row["reference"]
        lines.append(
            f"| {_code(row['id'])} | {base['candidate_median_ns_per_op']:,.2f} | "
            f"{base['comparator_median_ns_per_op']:,.2f} | "
            f"{reference['comparator_median_ns_per_op']:,.2f} | {_percent(base['ratio'])} | "
            f"{_range(base.get('ci_lower'), base.get('ci_upper'))} | "
            f"{_factor(reference['ratio'])} |"
        )
    lines.extend(["", "</details>", ""])
    return lines


def _exact_cell(ratio: float | None, lower: float | None, upper: float | None) -> str:
    if ratio is None:
        return "—"
    interval = "" if lower is None or upper is None else f" [{lower:.4f}, {upper:.4f}]"
    return f"{ratio:.4f}{interval}"


def _exact(
    broad: dict[str, Any] | None,
    external: dict[str, Any] | None,
    sentinel: dict[str, Any] | None,
) -> list[str]:
    """Every ratio the summary words, as the number and interval it came from."""
    rows: list[str] = []
    for suite in (external or {}).get("suites", ()):
        for case in suite["cases"]:
            rows.append(
                f"| {_code(suite['id'] + '/' + case['id'])} | "
                f"{_exact_cell(case['candidate_over_base'], *_external_interval(case, 'base'))} | "
                f"{_exact_cell(case['candidate_over_quickjs_ng'], *_external_interval(case, 'quickjs-ng'))} |"
            )
    for lane in (sentinel, broad):
        for row in _lane_rows(lane):
            base, reference = row["base"], row["reference"]
            rows.append(
                f"| {_code(row['id'])} | "
                f"{_exact_cell(base['ratio'], base.get('ci_lower'), base.get('ci_upper'))} | "
                f"{_exact_cell(reference['ratio'], reference.get('ci_lower'), reference.get('ci_upper'))} |"
            )
    if not rows:
        return []
    return [
        "<details>",
        f"<summary>Exact ratios and intervals — all {len(rows)} tests</summary>",
        "",
        "Candidate time ÷ comparator time, with the 95% interval where the lane "
        "computes one. Below 1 the candidate took less time.",
        "",
        "| Test | Candidate ÷ base | Candidate ÷ QuickJS-NG |",
        "| --- | --- | --- |",
        *rows,
        "",
        "</details>",
        "",
    ]


def _method(
    broad: dict[str, Any] | None,
    external: dict[str, Any] | None,
    sentinel: dict[str, Any] | None,
) -> list[str]:
    lines = [
        "<details>",
        "<summary>How this was measured, and exact revisions</summary>",
        "",
        "- Each workload ran with all three engines on one GitHub-hosted runner, "
        "three rounds each, in rotating order. Every comparison is between engines "
        "on the same machine; different workloads ran on different machines.",
        "- Classification: informational only — non-gating — not a fixed-hardware "
        "claim. Missing or malformed evidence fails the run; a slower result never "
        "does.",
        "- A group's number is the geometric mean of its tests' time ratios "
        "(candidate time ÷ comparator time). \"Likely range\" is the 95% interval of "
        "one test's ratio from three rounds, which is narrow evidence: use "
        "`./scripts/perf-compare.sh` before relying on a difference.",
        f"- \"No clear change\" means under {(GROUP_NOISE - 1) * 100:.0f}% for a group "
        f"and under {(TEST_NOISE - 1) * 100:.0f}% for one test, which is what "
        "identical builds show here.",
    ]
    if broad is not None:
        shards = broad.get("shards")
        lines.append(
            f"- Micro-operations: health {broad['health']}; block health non_claim; "
            f"linearity: {broad['linearity']}; valid blocks "
            f"`{broad['valid_blocks']}/{broad['requested_blocks']}`"
            + (
                f"; measured as {shards} shards on separate runners, so the group "
                "number has no interval of its own" if shards else ""
            )
        )
        comparisons = broad.get("comparisons") or {}
        for label in ("candidate vs base", "candidate vs QuickJS-NG"):
            result = comparisons.get(label)
            if result:
                interval = (
                    f" [{result['ci_lower']:.4f}×, {result['ci_upper']:.4f}×]"
                    if "ci_lower" in result else ""
                )
                lines.append(f"- Micro-operations, {label}: {result['ratio']:.4f}×{interval}")
    if sentinel is not None:
        lines.append(
            f"- Interpreter basics: health {sentinel['health']}; linearity: "
            f"{sentinel['linearity']}; valid blocks `{sentinel['valid_blocks']}/3`"
        )
    if external is not None:
        lines.append(
            f"- Benchmark programs: {external['blocks']} rounds; a per-program interval "
            f"is treated as conclusive only from {external['analysis']['min_blocks']} rounds"
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
    sides = SIDES.get(((broad or {}).get("harness") or {}).get("mode"), DEFAULT_SIDES)
    subject, base, short_base, column = sides
    groups, absent = _groups(broad, external, sentinel, notes)
    engines = (broad or {}).get("engines") or {}
    identical = bool(engines) and (
        engines["candidate"]["binary_sha256"] == engines["base"]["binary_sha256"]
    )
    lines = ["## Performance Preview", ""]
    if banner:
        lines.extend([f"> {banner}", ""])
    lines.extend(_verdict(groups, subject, base, identical))
    if broad is not None:
        lines.extend([
            f"{subject} is `{_short(engines['candidate']['source_revision'])}`; "
            f"{base} is `{_short(engines['base']['source_revision'])}`. "
            "QuickJS-NG is the reference engine this project measures itself against.",
            "",
        ])
    lines.extend(_overview(groups, absent, short_base))
    lines.extend([HOW_TO_READ, ""])
    lines.extend(_moved_tests(broad, external, sentinel, short_base, identical))
    lines.extend(_not_compared(external, sides))
    lines.extend(_external_details(external, short_base, column))
    lines.extend(_sentinel_details(sentinel, short_base))
    lines.extend(_broad_details(broad, short_base, column))
    lines.extend(_exact(broad, external, sentinel))
    lines.extend(_method(broad, external, sentinel))
    return "\n".join(lines)
