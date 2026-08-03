"""Evidence-bound performance opportunity queues and promotion decisions.

The hosted Performance Preview is intentionally informational: it cannot turn
one noisy GitHub-hosted run into a performance claim.  It can, however, name
the external and broad cases that deserve profiling.  This module makes that
handoff explicit and fail-closed:

* ``queue`` derives a deterministic opportunity queue from an exact preview
  bundle;
* ``check-unit`` validates an immutable optimization-unit plan before code is
  written;
* ``validate-unit`` binds that plan to the queue from which it was selected;
* ``decide`` classifies a measured candidate as retained, rejected, or
  inconclusive without allowing target cases to be selected after timing.

Raw timing data remains in CI/local artifacts.  The small JSON outputs here
only bind a reviewable decision to those artifacts by SHA-256.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any

from .performance_schema import (
    _ROLES,
    PerformanceDecisionError,
    _array,
    _boolean,
    _fraction,
    _integer,
    _keys,
    _object,
    _opportunity_id,
    _ratio,
    _read_json,
    _revision,
    _sha256,
    _string,
    _unique_strings,
    _unit_id,
)

_QUEUE_TYPE = "quickjs-performance-opportunity-queue"
_UNIT_TYPE = "quickjs-performance-unit"
_DECISION_TYPE = "quickjs-performance-decision"


def _atomic_write(path: Path, payload: dict[str, Any]) -> None:
    resolved = path.expanduser().resolve()
    if resolved.exists():
        raise PerformanceDecisionError(f"refusing to overwrite existing output {resolved}")
    encoded = (json.dumps(payload, sort_keys=True, indent=2, allow_nan=False) + "\n").encode()
    resolved.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb", dir=resolved.parent, prefix=f".{resolved.name}.", delete=False
        ) as handle:
            temporary = Path(handle.name)
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary, resolved)
        except FileExistsError as error:
            raise PerformanceDecisionError(
                f"refusing to overwrite existing output {resolved}"
            ) from error
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def _summary_revisions(summary: dict[str, Any]) -> tuple[str, str]:
    if summary.get("state") != "success":
        raise PerformanceDecisionError("preview summary: requires a successful measurement state")
    engines = _object(summary.get("engines"), "preview summary.engines")
    candidate = _object(engines.get("candidate"), "preview summary.engines.candidate")
    base = _object(engines.get("base"), "preview summary.engines.base")
    return (
        _revision(candidate.get("source_revision"), "preview summary candidate revision"),
        _revision(base.get("source_revision"), "preview summary base revision"),
    )


def _external_entries(report: dict[str, Any], target_ratio: float) -> list[dict[str, Any]]:
    if report.get("artifact_type") != "quickjs-external-preview-report":
        raise PerformanceDecisionError("external report: unsupported artifact type")
    if report.get("schema_version") != 1:
        raise PerformanceDecisionError("external report: unsupported schema version")
    entries: list[dict[str, Any]] = []
    seen: set[str] = set()
    for suite_index, raw_suite in enumerate(_array(report.get("suites"), "external report.suites")):
        suite = _object(raw_suite, f"external report.suites[{suite_index}]")
        suite_id = _string(suite.get("id"), f"external report.suites[{suite_index}].id")
        for case_index, raw_case in enumerate(
            _array(suite.get("cases"), f"external report.suites[{suite_index}].cases")
        ):
            case = _object(raw_case, f"external report.suites[{suite_index}].cases[{case_index}]")
            case_id = _string(case.get("id"), f"external report case {suite_id}.id")
            opportunity_id = f"external/{suite_id}/{case_id}"
            if opportunity_id in seen:
                raise PerformanceDecisionError(f"external report: duplicate {opportunity_id}")
            seen.add(opportunity_id)
            capability = _object(case.get("capability"), f"external report case {opportunity_id}.capability")
            if any(capability.get(role) != "ok" for role in _ROLES):
                continue
            quickjs_ratio = _ratio(
                case.get("candidate_over_quickjs_ng"),
                f"external report case {opportunity_id}.candidate_over_quickjs_ng",
            )
            base_value = case.get("candidate_over_base")
            base_ratio = None if base_value is None else _ratio(
                base_value, f"external report case {opportunity_id}.candidate_over_base"
            )
            if quickjs_ratio > target_ratio:
                entries.append(
                    {
                        "id": opportunity_id,
                        "suite": suite_id,
                        "case": case_id,
                        "candidate_over_quickjs_ng": quickjs_ratio,
                        "candidate_over_base": base_ratio,
                    }
                )
    entries.sort(key=lambda item: (-item["candidate_over_quickjs_ng"], item["id"]))
    for rank, entry in enumerate(entries, start=1):
        entry["rank"] = rank
    return entries


def _broad_entries(report: dict[str, Any], target_ratio: float) -> list[dict[str, Any]]:
    comparisons = _object(report.get("comparisons"), "broad report.comparisons")
    quickjs = _object(
        comparisons.get("candidate_vs_quickjs_ng"),
        "broad report.comparisons.candidate_vs_quickjs_ng",
    )
    base = _object(
        comparisons.get("candidate_vs_base"),
        "broad report.comparisons.candidate_vs_base",
    )
    quickjs_cases = _object(quickjs.get("cases"), "broad quickjs comparison.cases")
    base_cases = _object(base.get("cases"), "broad base comparison.cases")
    entries: list[dict[str, Any]] = []
    for case_id in sorted(quickjs_cases):
        quickjs_case = _object(quickjs_cases[case_id], f"broad quickjs case {case_id}")
        ratio = _ratio(quickjs_case.get("ratio"), f"broad quickjs case {case_id}.ratio")
        if ratio <= target_ratio:
            continue
        base_case = _object(base_cases.get(case_id), f"broad base case {case_id}")
        entries.append(
            {
                "id": f"broad/{case_id}",
                "case": case_id,
                "family": _string(quickjs_case.get("family"), f"broad quickjs case {case_id}.family"),
                "candidate_over_quickjs_ng": ratio,
                "candidate_over_base": _ratio(
                    base_case.get("ratio"), f"broad base case {case_id}.ratio"
                ),
            }
        )
    entries.sort(key=lambda item: (-item["candidate_over_quickjs_ng"], item["id"]))
    for rank, entry in enumerate(entries, start=1):
        entry["rank"] = rank
    return entries


def build_queue(
    summary_path: Path,
    broad_path: Path,
    external_path: Path,
    target_ratio: float,
) -> dict[str, Any]:
    if target_ratio <= 0 or not math.isfinite(target_ratio):
        raise PerformanceDecisionError("target ratio must be a finite positive number")
    summary, summary_sha = _read_json(summary_path, "preview summary")
    broad, broad_sha = _read_json(broad_path, "broad report")
    external, external_sha = _read_json(external_path, "external report")
    candidate_sha, base_sha = _summary_revisions(summary)
    return {
        "schema_version": 1,
        "artifact_type": _QUEUE_TYPE,
        "candidate_sha": candidate_sha,
        "comparison_base_sha": base_sha,
        "target_ratio": target_ratio,
        "evidence": {
            "preview_summary_sha256": summary_sha,
            "broad_report_sha256": broad_sha,
            "external_report_sha256": external_sha,
        },
        "external": _external_entries(external, target_ratio),
        "broad": _broad_entries(broad, target_ratio),
    }


def load_queue(path: Path) -> tuple[dict[str, Any], str]:
    queue, digest = _read_json(path, "opportunity queue")
    _keys(
        queue,
        {
            "schema_version", "artifact_type", "candidate_sha", "comparison_base_sha",
            "target_ratio", "evidence", "external", "broad",
        },
        "opportunity queue",
    )
    if queue["schema_version"] != 1 or queue["artifact_type"] != _QUEUE_TYPE:
        raise PerformanceDecisionError("opportunity queue: unsupported schema")
    _revision(queue["candidate_sha"], "opportunity queue.candidate_sha")
    _revision(queue["comparison_base_sha"], "opportunity queue.comparison_base_sha")
    _ratio(queue["target_ratio"], "opportunity queue.target_ratio")
    evidence = _object(queue["evidence"], "opportunity queue.evidence")
    _keys(
        evidence,
        {"preview_summary_sha256", "broad_report_sha256", "external_report_sha256"},
        "opportunity queue.evidence",
    )
    for key, value in evidence.items():
        _sha256(value, f"opportunity queue.evidence.{key}")
    seen: set[str] = set()
    for channel in ("external", "broad"):
        entries = _array(queue[channel], f"opportunity queue.{channel}")
        expected_rank = 1
        for index, raw_entry in enumerate(entries):
            entry = _object(raw_entry, f"opportunity queue.{channel}[{index}]")
            opportunity_id = _opportunity_id(entry.get("id"), f"opportunity queue.{channel}[{index}].id")
            if opportunity_id in seen:
                raise PerformanceDecisionError(f"opportunity queue: duplicate {opportunity_id}")
            seen.add(opportunity_id)
            if _integer(entry.get("rank"), f"opportunity queue {opportunity_id}.rank", 1) != expected_rank:
                raise PerformanceDecisionError(f"opportunity queue {channel}: ranks must be contiguous")
            expected_rank += 1
            _ratio(
                entry.get("candidate_over_quickjs_ng"),
                f"opportunity queue {opportunity_id}.candidate_over_quickjs_ng",
            )
            base_ratio = entry.get("candidate_over_base")
            if base_ratio is not None:
                _ratio(base_ratio, f"opportunity queue {opportunity_id}.candidate_over_base")
    return queue, digest


def _load_migration(unit: dict[str, Any]) -> dict[str, Any]:
    """Validates a staged architectural migration's stage budget.

    A migration is judged at its end state, not after its first commit. Its
    intermediate stages exist to move real execution onto a new representation
    and are expected to be neutral or slightly negative; requiring each to pay
    for itself is what makes an architectural change unlandable. The stage
    budget therefore caps regression instead of requiring improvement, and the
    plan's `fast_gate` stays in force as the final payoff gate.
    """
    migration = _object(unit["migration"], "performance unit.migration")
    _keys(
        migration,
        {"stages", "current_stage", "cumulative_target_ids", "stage_max_candidate_over_base"},
        "performance unit.migration",
    )
    stages = _integer(migration["stages"], "performance unit.migration.stages", 2)
    if stages > 12:
        raise PerformanceDecisionError(
            "performance unit.migration.stages: a migration longer than 12 stages is not one "
            "reviewable program"
        )
    current = _integer(migration["current_stage"], "performance unit.migration.current_stage", 1)
    if current > stages:
        raise PerformanceDecisionError(
            "performance unit.migration.current_stage: cannot exceed the declared stage count"
        )
    # `_unique_strings` already rejects an empty array, which is the rule that
    # matters here: a migration must name what it is for before its first
    # stage lands.
    _unique_strings(
        migration["cumulative_target_ids"],
        "performance unit.migration.cumulative_target_ids",
        _opportunity_id,
    )
    budget = _ratio(
        migration["stage_max_candidate_over_base"],
        "performance unit.migration.stage_max_candidate_over_base",
    )
    if budget < 1:
        raise PerformanceDecisionError(
            "performance unit.migration.stage_max_candidate_over_base: a stage budget that "
            "requires improvement is a leaf gate, not a migration stage"
        )
    if budget > 1.25:
        raise PerformanceDecisionError(
            "performance unit.migration.stage_max_candidate_over_base: a stage may absorb "
            "scaffolding cost, not an unbounded regression"
        )
    return migration


def unit_kind(unit: dict[str, Any]) -> str:
    """Returns the plan's kind, defaulting schema 1 plans to `leaf`.

    Schema 1 predates staged migrations, so every existing plan is a leaf fast
    path. Defaulting rather than rewriting them keeps their frozen SHA-256
    bindings intact.
    """
    return unit.get("unit_kind", "leaf")


def load_unit(path: Path) -> tuple[dict[str, Any], str]:
    unit, digest = _read_json(path, "performance unit")
    version = unit.get("schema_version")
    if version not in {1, 2} or unit.get("artifact_type") != _UNIT_TYPE:
        raise PerformanceDecisionError("performance unit: unsupported schema")
    expected_keys = {
        "schema_version", "artifact_type", "unit_id", "base_sha", "queue",
        "priority", "mechanism", "profile_evidence", "fast_gate", "promotion_gate",
    }
    if version == 2:
        expected_keys.add("unit_kind")
        if unit.get("unit_kind") == "migration":
            expected_keys.add("migration")
    _keys(unit, expected_keys, "performance unit")
    kind = unit_kind(unit)
    if kind not in {"leaf", "migration"}:
        raise PerformanceDecisionError("performance unit.unit_kind: expected leaf or migration")
    if kind == "migration":
        _load_migration(unit)
    _unit_id(unit["unit_id"], "performance unit.unit_id")
    _revision(unit["base_sha"], "performance unit.base_sha")

    queue = _object(unit["queue"], "performance unit.queue")
    _keys(queue, {"candidate_sha", "sha256"}, "performance unit.queue")
    _revision(queue["candidate_sha"], "performance unit.queue.candidate_sha")
    _sha256(queue["sha256"], "performance unit.queue.sha256")

    priority = _object(unit["priority"], "performance unit.priority")
    _keys(priority, {"mode", "opportunity_ids", "rank_ceiling", "override_reason"}, "performance unit.priority")
    mode = _string(priority["mode"], "performance unit.priority.mode")
    if mode not in {"queue", "override"}:
        raise PerformanceDecisionError("performance unit.priority.mode: expected queue or override")
    _unique_strings(priority["opportunity_ids"], "performance unit.priority.opportunity_ids", _opportunity_id)
    _integer(priority["rank_ceiling"], "performance unit.priority.rank_ceiling", 1)
    override = priority["override_reason"]
    if mode == "queue" and override is not None:
        raise PerformanceDecisionError("performance unit.priority.override_reason: queue mode requires null")
    if mode == "override":
        _string(override, "performance unit.priority.override_reason")

    mechanism = _object(unit["mechanism"], "performance unit.mechanism")
    _keys(mechanism, {"summary", "generality", "semantic_risks"}, "performance unit.mechanism")
    _string(mechanism["summary"], "performance unit.mechanism.summary")
    _string(mechanism["generality"], "performance unit.mechanism.generality")
    _unique_strings(mechanism["semantic_risks"], "performance unit.mechanism.semantic_risks")

    profile_evidence = _array(unit["profile_evidence"], "performance unit.profile_evidence")
    if not profile_evidence:
        raise PerformanceDecisionError("performance unit.profile_evidence: expected at least one receipt")
    for index, raw_receipt in enumerate(profile_evidence):
        receipt = _object(raw_receipt, f"performance unit.profile_evidence[{index}]")
        _keys(
            receipt,
            {"source", "sha256", "base_sha", "opportunity_ids", "shared_cost", "inclusive_fraction"},
            f"performance unit.profile_evidence[{index}]",
        )
        _string(receipt["source"], f"performance unit.profile_evidence[{index}].source")
        _sha256(receipt["sha256"], f"performance unit.profile_evidence[{index}].sha256")
        _revision(receipt["base_sha"], f"performance unit.profile_evidence[{index}].base_sha")
        _unique_strings(
            receipt["opportunity_ids"],
            f"performance unit.profile_evidence[{index}].opportunity_ids",
            _opportunity_id,
        )
        _string(receipt["shared_cost"], f"performance unit.profile_evidence[{index}].shared_cost")
        _fraction(receipt["inclusive_fraction"], f"performance unit.profile_evidence[{index}].inclusive_fraction")

    fast_gate = _object(unit["fast_gate"], "performance unit.fast_gate")
    _keys(
        fast_gate,
        {
            "target_ids", "control_ids", "target_max_candidate_over_base",
            "control_max_candidate_over_base", "max_attempts",
        },
        "performance unit.fast_gate",
    )
    target_ids = _unique_strings(fast_gate["target_ids"], "performance unit.fast_gate.target_ids", _opportunity_id)
    _unique_strings(fast_gate["control_ids"], "performance unit.fast_gate.control_ids", _opportunity_id)
    if set(target_ids) != set(priority["opportunity_ids"]):
        raise PerformanceDecisionError(
            "performance unit.fast_gate.target_ids: must equal priority opportunity_ids"
        )
    target_max = _ratio(
        fast_gate["target_max_candidate_over_base"],
        "performance unit.fast_gate.target_max_candidate_over_base",
    )
    if target_max >= 1:
        raise PerformanceDecisionError("performance unit fast gate target must require an improvement")
    control_max = _ratio(
        fast_gate["control_max_candidate_over_base"],
        "performance unit.fast_gate.control_max_candidate_over_base",
    )
    if control_max < 1:
        raise PerformanceDecisionError("performance unit control gate cannot require every control to improve")
    _integer(fast_gate["max_attempts"], "performance unit.fast_gate.max_attempts", 1)

    promotion = _object(unit["promotion_gate"], "performance unit.promotion_gate")
    _keys(
        promotion,
        {"require_complete_broad", "require_complete_external", "require_test262_zero_gap"},
        "performance unit.promotion_gate",
    )
    for key, value in promotion.items():
        if not _boolean(value, f"performance unit.promotion_gate.{key}"):
            raise PerformanceDecisionError(f"performance unit.promotion_gate.{key}: must remain true")
    return unit, digest


def validate_unit_against_queue(unit: dict[str, Any], unit_sha: str, queue: dict[str, Any], queue_sha: str) -> dict[str, Any]:
    if unit["queue"]["sha256"] != queue_sha:
        raise PerformanceDecisionError("performance unit queue SHA-256 does not match supplied queue")
    if unit["queue"]["candidate_sha"] != queue["candidate_sha"]:
        raise PerformanceDecisionError("performance unit queue candidate SHA does not match supplied queue")
    if unit["base_sha"] != queue["candidate_sha"]:
        raise PerformanceDecisionError("performance unit base SHA must equal the queue candidate SHA")
    priority = unit["priority"]
    targets = set(priority["opportunity_ids"])
    profile_targets = {
        item
        for receipt in unit["profile_evidence"]
        for item in receipt["opportunity_ids"]
        if receipt["base_sha"] == unit["base_sha"]
    }
    if not targets <= profile_targets:
        raise PerformanceDecisionError("performance unit profile receipts do not cover every target")
    if priority["mode"] == "queue":
        ranks = {
            item["id"]: item["rank"]
            for channel in ("external", "broad")
            for item in queue[channel]
        }
        missing = sorted(targets - set(ranks))
        if missing:
            raise PerformanceDecisionError(
                f"performance unit queue targets are absent from current queue: {missing}"
            )
        if not any(ranks[target] <= priority["rank_ceiling"] for target in targets):
            raise PerformanceDecisionError(
                "performance unit queue mode requires a target inside its rank ceiling"
            )
    return {
        "schema_version": 1,
        "artifact_type": "quickjs-performance-unit-validation",
        "unit_id": unit["unit_id"],
        "unit_sha256": unit_sha,
        "base_sha": unit["base_sha"],
        "queue_sha256": queue_sha,
        "priority_mode": priority["mode"],
        "status": "valid",
    }


def _base_metric_index(broad: dict[str, Any], external: dict[str, Any]) -> tuple[dict[str, float], bool, bool]:
    metrics: dict[str, float] = {}
    external_complete = True
    if external.get("artifact_type") != "quickjs-external-preview-report":
        raise PerformanceDecisionError("external report: unsupported artifact type")
    for suite_index, raw_suite in enumerate(_array(external.get("suites"), "external report.suites")):
        suite = _object(raw_suite, f"external report.suites[{suite_index}]")
        suite_id = _string(suite.get("id"), f"external report.suites[{suite_index}].id")
        if suite.get("complete_base_comparison") is not True or suite.get("complete_comparison") is not True:
            external_complete = False
        for raw_case in _array(suite.get("cases"), f"external report {suite_id}.cases"):
            case = _object(raw_case, f"external report {suite_id}.case")
            case_id = _string(case.get("id"), f"external report {suite_id}.case.id")
            value = case.get("candidate_over_base")
            if value is not None:
                metrics[f"external/{suite_id}/{case_id}"] = _ratio(
                    value, f"external report {suite_id}/{case_id}.candidate_over_base"
                )
    comparisons = _object(broad.get("comparisons"), "broad report.comparisons")
    base = _object(
        comparisons.get("candidate_vs_base"), "broad report.comparisons.candidate_vs_base"
    )
    cases = _object(base.get("cases"), "broad report.candidate_vs_base.cases")
    broad_complete = len(cases) == 25
    for case_id, raw_case in cases.items():
        case = _object(raw_case, f"broad report case {case_id}")
        metrics[f"broad/{case_id}"] = _ratio(
            case.get("ratio"), f"broad report case {case_id}.ratio"
        )
    return metrics, broad_complete, external_complete


def _test262_zero_gap(report: dict[str, Any], candidate_sha: str) -> bool:
    if report.get("commit") != candidate_sha:
        raise PerformanceDecisionError("Test262 burndown commit does not match candidate SHA")
    rust = _object(report.get("rust"), "Test262 burndown.rust")
    comparison = _object(report.get("comparison"), "Test262 burndown.comparison")
    fields = (
        (rust, "fail"), (rust, "timeout"), (rust, "not_run"),
        (comparison, "actionable_gap"), (comparison, "ng_pass_rust_fail"),
        (comparison, "ng_pass_rust_timeout"), (comparison, "ng_pass_rust_not_run"),
    )
    return all(_integer(container.get(key), f"Test262 burndown.{key}") == 0 for container, key in fields)


def _stage_decision(
    unit: dict[str, Any],
    unit_sha: str,
    base_sha: str,
    candidate_sha: str,
    metrics: dict[str, float],
    validation: dict[str, Any],
    queue_sha: str,
    summary_sha: str,
    broad_sha: str,
    external_sha: str,
) -> dict[str, Any]:
    """Classifies one stage of a migration as advance, abort, or inconclusive.

    `retained` and `rejected` are deliberately unavailable here. A stage that
    does not regress past its budget has earned the right to continue, not a
    performance claim; a stage that does closes that one implementation, not
    the mechanism family it belongs to. Only the final stage's fast or
    promotion decision can retain or reject the migration itself.
    """
    migration = unit["migration"]
    budget = migration["stage_max_candidate_over_base"]
    watched = tuple(migration["cumulative_target_ids"]) + tuple(unit["fast_gate"]["control_ids"])
    reasons: list[str] = []
    missing = sorted(set(watched) - set(metrics))
    if missing:
        state = "inconclusive"
        reasons.append(f"missing candidate/base evidence for {missing}")
    else:
        regressed = [
            opportunity_id for opportunity_id in watched if metrics[opportunity_id] > budget
        ]
        if regressed:
            state = "abort"
            reasons.append(
                f"stage regression budget {budget} exceeded for {regressed}; this closes the "
                "stage's implementation shape, not its mechanism family"
            )
        else:
            state = "advance"
    return {
        "schema_version": 2,
        "artifact_type": _DECISION_TYPE,
        "unit_id": unit["unit_id"],
        "unit_sha256": unit_sha,
        "unit_kind": "migration",
        "base_sha": base_sha,
        "candidate_sha": candidate_sha,
        "mode": "stage",
        "stage": migration["current_stage"],
        "stages": migration["stages"],
        "decision": state,
        "reasons": reasons,
        "metrics": {
            opportunity_id: metrics[opportunity_id]
            for opportunity_id in watched
            if opportunity_id in metrics
        },
        "evidence": {
            "queue_sha256": queue_sha,
            "preview_summary_sha256": summary_sha,
            "broad_report_sha256": broad_sha,
            "external_report_sha256": external_sha,
            "test262_burndown_sha256": None,
        },
        "unit_validation": validation,
    }


def decide(
    unit: dict[str, Any],
    unit_sha: str,
    queue: dict[str, Any],
    queue_sha: str,
    summary: dict[str, Any],
    summary_sha: str,
    broad: dict[str, Any],
    broad_sha: str,
    external: dict[str, Any],
    external_sha: str,
    mode: str,
    test262: tuple[dict[str, Any], str] | None,
) -> dict[str, Any]:
    validation = validate_unit_against_queue(unit, unit_sha, queue, queue_sha)
    candidate_sha, base_sha = _summary_revisions(summary)
    # A migration keeps one base SHA across every stage, so this equality is
    # what makes each stage measurement cumulative against the migration base
    # rather than against the scaffolding commit before it.
    if base_sha != unit["base_sha"]:
        raise PerformanceDecisionError("preview summary base SHA does not match performance unit")
    kind = unit_kind(unit)
    if mode == "stage" and kind != "migration":
        raise PerformanceDecisionError("stage mode requires a migration unit")
    if kind == "migration" and mode != "stage":
        migration = unit["migration"]
        if migration["current_stage"] != migration["stages"]:
            raise PerformanceDecisionError(
                "a migration reaches fast or promotion mode only at its final stage; "
                "use --mode stage for an intermediate stage"
            )
    metrics, broad_complete, external_complete = _base_metric_index(broad, external)
    if mode == "stage":
        return _stage_decision(
            unit, unit_sha, base_sha, candidate_sha, metrics, validation,
            queue_sha, summary_sha, broad_sha, external_sha,
        )
    fast_gate = unit["fast_gate"]
    target_ids = tuple(fast_gate["target_ids"])
    control_ids = tuple(fast_gate["control_ids"])
    missing = sorted(set(target_ids + control_ids) - set(metrics))
    reasons: list[str] = []
    state = "retained"
    if missing:
        state = "inconclusive"
        reasons.append(f"missing candidate/base evidence for {missing}")
    else:
        failed_targets = [
            opportunity_id for opportunity_id in target_ids
            if metrics[opportunity_id] > fast_gate["target_max_candidate_over_base"]
        ]
        failed_controls = [
            opportunity_id for opportunity_id in control_ids
            if metrics[opportunity_id] > fast_gate["control_max_candidate_over_base"]
        ]
        if failed_targets:
            state = "rejected"
            reasons.append(f"target improvement gate failed for {failed_targets}")
        if failed_controls:
            state = "rejected"
            reasons.append(f"control regression gate failed for {failed_controls}")
    if mode == "promotion" and state == "retained":
        if not broad_complete:
            state = "inconclusive"
            reasons.append("broad report does not contain the complete 25-case base comparison")
        if not external_complete:
            state = "inconclusive"
            reasons.append("external report does not contain complete base and QuickJS-NG comparisons")
        if test262 is None:
            state = "inconclusive"
            reasons.append("promotion requires an exact Test262 burndown")
        elif not _test262_zero_gap(test262[0], candidate_sha):
            state = "rejected"
            reasons.append("Test262 parity gate is not zero")
    return {
        "schema_version": 1,
        "artifact_type": _DECISION_TYPE,
        "unit_id": unit["unit_id"],
        "unit_sha256": unit_sha,
        "base_sha": base_sha,
        "candidate_sha": candidate_sha,
        "mode": mode,
        "decision": state,
        "reasons": reasons,
        "metrics": {opportunity_id: metrics[opportunity_id] for opportunity_id in target_ids + control_ids if opportunity_id in metrics},
        "evidence": {
            "queue_sha256": queue_sha,
            "preview_summary_sha256": summary_sha,
            "broad_report_sha256": broad_sha,
            "external_report_sha256": external_sha,
            "test262_burndown_sha256": None if test262 is None else test262[1],
        },
        "unit_validation": validation,
    }


class _Parser(argparse.ArgumentParser):
    def error(self, message: str) -> None:
        raise PerformanceDecisionError(message)


def _parser() -> argparse.ArgumentParser:
    parser = _Parser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    queue = subparsers.add_parser("queue", help="derive a ranked opportunity queue")
    queue.add_argument("--summary", type=Path, required=True)
    queue.add_argument("--broad-report", type=Path, required=True)
    queue.add_argument("--external-report", type=Path, required=True)
    queue.add_argument("--target-ratio", type=float, default=0.5)
    queue.add_argument("--output", type=Path, required=True)

    check = subparsers.add_parser("check-unit", help="validate one plan without evidence files")
    check.add_argument("--unit", type=Path, required=True)

    validate = subparsers.add_parser("validate-unit", help="bind one plan to an opportunity queue")
    validate.add_argument("--unit", type=Path, required=True)
    validate.add_argument("--queue", type=Path, required=True)

    decision = subparsers.add_parser("decide", help="classify measured evidence for one plan")
    decision.add_argument("--unit", type=Path, required=True)
    decision.add_argument("--queue", type=Path, required=True)
    decision.add_argument("--summary", type=Path, required=True)
    decision.add_argument("--broad-report", type=Path, required=True)
    decision.add_argument("--external-report", type=Path, required=True)
    decision.add_argument("--test262-burndown", type=Path)
    decision.add_argument("--mode", choices=("stage", "fast", "promotion"), required=True)
    decision.add_argument("--require-retained", action="store_true")
    decision.add_argument("--output", type=Path, required=True)
    return parser


def main() -> int:
    try:
        args = _parser().parse_args()
        if args.command == "queue":
            payload = build_queue(
                args.summary, args.broad_report, args.external_report, args.target_ratio
            )
            _atomic_write(args.output, payload)
            print(json.dumps(payload, sort_keys=True))
            return 0
        unit, unit_sha = load_unit(args.unit)
        if args.command == "check-unit":
            print(json.dumps({"unit_id": unit["unit_id"], "unit_sha256": unit_sha, "status": "valid"}, sort_keys=True))
            return 0
        queue, queue_sha = load_queue(args.queue)
        if args.command == "validate-unit":
            print(json.dumps(validate_unit_against_queue(unit, unit_sha, queue, queue_sha), sort_keys=True))
            return 0
        summary, summary_sha = _read_json(args.summary, "preview summary")
        broad, broad_sha = _read_json(args.broad_report, "broad report")
        external, external_sha = _read_json(args.external_report, "external report")
        test262 = None
        if args.test262_burndown is not None:
            test262 = _read_json(args.test262_burndown, "Test262 burndown")
        payload = decide(
            unit, unit_sha, queue, queue_sha, summary, summary_sha, broad, broad_sha,
            external, external_sha, args.mode, test262,
        )
        _atomic_write(args.output, payload)
        print(json.dumps(payload, sort_keys=True))
        if not args.require_retained:
            return 0
        accepted = "advance" if args.mode == "stage" else "retained"
        return 0 if payload["decision"] == accepted else 1
    except PerformanceDecisionError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
