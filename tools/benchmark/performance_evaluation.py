"""Conservative optimization decisions over verified, paired comparisons."""
from __future__ import annotations

import subprocess
from typing import Any

from .performance_evidence import ROOT, comparison_index, interval, inventories, validate_bundle
from .performance_schema import PerformanceDecisionError, _integer, _object
from .profile import verify_profiles


def pinned_test262_count() -> int:
    """Count the pinned Git tree, not a caller-supplied subset or worktree scan."""
    try:
        pin = subprocess.run(["git", "ls-tree", "HEAD", "third_party/test262"], cwd=ROOT,
                             capture_output=True, text=True, check=True, timeout=10).stdout.split()[2]
        files = subprocess.run(["git", "ls-tree", "-r", "--name-only", pin, "--", "test"],
                               cwd=ROOT / "third_party/test262", capture_output=True,
                               text=True, check=True, timeout=10).stdout.splitlines()
    except (OSError, subprocess.SubprocessError, IndexError) as error:
        raise PerformanceDecisionError(f"cannot verify pinned Test262 inventory: {error}") from error
    count = sum(name.endswith(".js") for name in files)
    if count == 0:
        raise PerformanceDecisionError("pinned Test262 inventory is empty")
    return count


def _test262_zero_gap(report: dict[str, Any], candidate_sha: str) -> bool:
    if report.get("commit") != candidate_sha:
        raise PerformanceDecisionError("Test262 burndown commit does not match candidate SHA")
    if report.get("schema") != 1:
        raise PerformanceDecisionError("Test262 burndown requires schema 1")
    total = _integer(report.get("total"), "Test262 total", 1)
    configured = _integer(report.get("configured"), "Test262 configured", 1)
    skipped = _integer(report.get("ng_config_skipped"), "Test262 skipped")
    if total != pinned_test262_count() or total != configured + skipped:
        raise PerformanceDecisionError("Test262 burndown does not cover the complete pinned inventory")
    rust = _object(report.get("rust"), "Test262 burndown.rust")
    ng = _object(report.get("ng"), "Test262 burndown.ng")
    for role, counts, keys in (("rust", rust, ("pass", "fail", "timeout", "not_run")),
                               ("ng", ng, ("pass", "fail", "timeout"))):
        if sum(_integer(counts.get(key), f"Test262 {role}.{key}") for key in keys) != configured:
            raise PerformanceDecisionError(f"Test262 {role} counts do not account for configured inventory")
    comparison = _object(report.get("comparison"), "Test262 burndown.comparison")
    fields = ((rust, "fail"), (rust, "timeout"), (rust, "not_run"),
              (comparison, "actionable_gap"), (comparison, "ng_pass_rust_fail"),
              (comparison, "ng_pass_rust_timeout"), (comparison, "ng_pass_rust_not_run"))
    return all(_integer(container.get(key), f"Test262 burndown.{key}") == 0
               for container, key in fields)


def decide(unit, unit_sha, queue, queue_sha, summary, summary_sha, broad, broad_sha,
           external, external_sha, mode, test262, *, sentinel=None, sentinel_sha=None,
           profile_root=None) -> dict[str, Any]:
    from .performance_decision import validate_unit_against_queue, _summary_revisions, unit_kind
    validation = validate_unit_against_queue(unit, unit_sha, queue, queue_sha)
    candidate_sha, base_sha = _summary_revisions(summary)
    if base_sha != unit["base_sha"]:
        raise PerformanceDecisionError("preview summary base SHA does not match performance unit")
    kind = unit_kind(unit)
    if mode not in {"stage", "fast", "promotion"}:
        raise PerformanceDecisionError("unsupported decision mode")
    if mode == "stage" and kind != "migration":
        raise PerformanceDecisionError("stage mode requires a migration unit")
    if kind == "migration" and mode != "stage" and unit["migration"]["current_stage"] != unit["migration"]["stages"]:
        raise PerformanceDecisionError("a migration reaches fast or promotion mode only at its final stage")
    reasons = validate_bundle(summary, broad, external, sentinel)
    if summary["engines"]["base"] != queue.get("engines", {}).get("candidate"):
        raise PerformanceDecisionError("decision base executable differs from the profiled queue candidate")
    profiles = verify_profiles(unit, queue, profile_root)
    if sentinel is None:
        reasons.append("missing generic-path sentinel evidence")
    metrics = comparison_index(broad, external, sentinel)
    gate = unit["fast_gate"]
    controls = set(gate["control_ids"])
    controls.update(f"sentinel/{case}" for case in inventories()["sentinel"])
    targets = set(gate["target_ids"])
    if mode == "stage":
        targets = set(unit["migration"]["cumulative_target_ids"])
        target_limit = control_limit = unit["migration"]["stage_max_candidate_over_base"]
    else:
        target_limit = gate["target_max_candidate_over_base"]
        control_limit = gate["control_max_candidate_over_base"]
    if mode == "promotion":
        if any(suite.get("complete_comparison") is not True or suite.get("complete_base_comparison") is not True
               for suite in external["suites"]):
            reasons.append("external report does not contain complete base and QuickJS-NG comparisons")
        for lane, cases in inventories().items():
            controls.update(f"{lane}/{case}" if lane in {"broad", "sentinel"}
                            else f"external/{lane}/{case}" for case in cases)
    limits = {case: control_limit for case in controls}
    limits.update({case: target_limit for case in targets})
    failed = []
    missing = sorted(set(limits) - set(metrics))
    if missing:
        reasons.append(f"missing candidate/base evidence for {missing}")
    for case, limit in sorted(limits.items()):
        if case not in metrics:
            continue
        lower, upper, precise = interval(metrics[case], case)
        if not precise:
            reasons.append(f"{case}: insufficient blocks or wide confidence interval")
        elif lower > limit:
            failed.append(case)
        elif upper > limit:
            reasons.append(f"{case}: confidence interval crosses acceptance threshold {limit}")
    state = "inconclusive" if reasons else ("advance" if mode == "stage" else "retained")
    if failed:
        # A clear regression is actionable even if another lane is noisy.
        state = "abort" if mode == "stage" else "rejected"
        if mode == "stage":
            reasons.insert(0, f"stage regression budget exceeded for {failed}; this closes the implementation, not its mechanism family")
        else:
            reasons.insert(0, f"target improvement or control regression gate failed for {failed}")
    if mode == "promotion":
        if test262 is None:
            if state == "retained":
                state = "inconclusive"
            reasons.append("promotion requires an exact Test262 burndown")
        elif not _test262_zero_gap(test262[0], candidate_sha):
            state = "rejected"
            reasons.append("Test262 parity gate is not zero")
    result = {
        "schema_version": 2,
        "artifact_type": "quickjs-performance-decision",
        "claim_eligible": False,
        "unit_id": unit["unit_id"], "unit_sha256": unit_sha, "unit_kind": kind,
        "base_sha": base_sha, "candidate_sha": candidate_sha, "mode": mode,
        "decision": state, "reasons": reasons,
        "metrics": {key: metrics[key] for key in sorted(limits) if key in metrics},
        "evidence": {"queue_sha256": queue_sha, "preview_summary_sha256": summary_sha,
                     "broad_report_sha256": broad_sha, "external_report_sha256": external_sha,
                     "sentinel_report_sha256": sentinel_sha, "profiles": profiles,
                     "test262_burndown_sha256": None if test262 is None else test262[1]},
        "unit_validation": validation,
    }
    if mode == "stage":
        result.update(stage=unit["migration"]["current_stage"], stages=unit["migration"]["stages"])
    return result
