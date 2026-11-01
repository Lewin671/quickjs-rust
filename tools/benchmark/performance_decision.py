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

from .layout_identity import same_node, same_host
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


def _held_out_opportunity_ids() -> frozenset[str]:
    """The frozen held-out acceptance cases, which never select work.

    The queue and every plan are derived from the pinned performance suites.
    The held-out lane is the independent check on the generality of the
    result, so a plan that named one of its cases as a target or control -- or
    profiled one -- would be using the acceptance set to select work. These
    ids are therefore refused wherever a plan or queue may name a workload.
    """
    from .held_out import opportunity_ids
    return opportunity_ids()


def _reject_held_out(declared: set[str], where: str) -> None:
    forbidden = sorted(name for name in declared if name.startswith('held-out/'))
    if forbidden:
        raise PerformanceDecisionError(
            f"{where}: a held-out case is an acceptance check, not a source of work: {forbidden}"
        )


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


def _preview_entries(
    report: dict[str, Any], target_ratio: float, *, artifact_type: str, prefix: str
) -> list[dict[str, Any]]:
    """Rank one whole-process preview lane's cases above the campaign target.

    The external and development lanes share this analysis shape: per case a
    capability verdict and paired candidate/base and candidate/QuickJS-NG
    ratios. Only the artifact type and the id prefix differ.
    """
    if report.get("artifact_type") != artifact_type:
        raise PerformanceDecisionError("preview report: unsupported artifact type")
    if report.get("schema_version") != 3:
        raise PerformanceDecisionError("preview report: unsupported schema version")
    entries: list[dict[str, Any]] = []
    seen: set[str] = set()
    for suite_index, raw_suite in enumerate(_array(report.get("suites"), "preview report.suites")):
        suite = _object(raw_suite, f"preview report.suites[{suite_index}]")
        suite_id = _string(suite.get("id"), f"preview report.suites[{suite_index}].id")
        for case_index, raw_case in enumerate(
            _array(suite.get("cases"), f"preview report.suites[{suite_index}].cases")
        ):
            case = _object(raw_case, f"preview report.suites[{suite_index}].cases[{case_index}]")
            case_id = _string(case.get("id"), f"preview report case {suite_id}.id")
            opportunity_id = f"{prefix}/{suite_id}/{case_id}"
            if opportunity_id in seen:
                raise PerformanceDecisionError(f"preview report: duplicate {opportunity_id}")
            seen.add(opportunity_id)
            capability = _object(case.get("capability"), f"preview report case {opportunity_id}.capability")
            if any(capability.get(role) != "ok" for role in _ROLES):
                continue
            quickjs_ratio = _ratio(
                case.get("candidate_over_quickjs_ng"),
                f"preview report case {opportunity_id}.candidate_over_quickjs_ng",
            )
            base_value = case.get("candidate_over_base")
            base_ratio = None if base_value is None else _ratio(
                base_value, f"preview report case {opportunity_id}.candidate_over_base"
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


def _external_entries(report: dict[str, Any], target_ratio: float) -> list[dict[str, Any]]:
    return _preview_entries(
        report, target_ratio,
        artifact_type="quickjs-external-preview-report", prefix="external",
    )


def _development_entries(
    report: dict[str, Any], target_ratio: float, engines: dict[str, Any], host: Any
) -> list[dict[str, Any]]:
    """Rank the development lane, whose cases may select work.

    Development cases are real programs a plan may profile, so a queue keeps
    them apart from the acceptance suites: same run, same engines, the frozen
    corpus, ranked above the campaign target like every other selectable lane.
    """
    from .development import corpus_inventory
    if report.get("binary_sha256") != {role: engines[role]["binary_sha256"] for role in _ROLES}:
        raise PerformanceDecisionError("development: binary identities do not match summary")
    if not same_host(report.get("host"), host):
        raise PerformanceDecisionError("development: different measurement host")
    expected = corpus_inventory()
    suites = report.get("suites", [])
    ids = [suite.get("id") for suite in suites]
    if len(ids) != len(set(ids)) or set(ids) != set(expected):
        raise PerformanceDecisionError("development: wrong frozen suite inventory")
    for suite in suites:
        cases = [case.get("id") for case in suite.get("cases", [])]
        if set(cases) != expected[suite["id"]]:
            raise PerformanceDecisionError(
                f"development/{suite['id']}: incomplete frozen case inventory"
            )
    return _preview_entries(
        report, target_ratio,
        artifact_type="quickjs-development-preview-report", prefix="development",
    )


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
    sentinel_path: Path | None = None,
    development_path: Path | None = None,
) -> dict[str, Any]:
    """Rank the selectable lanes of one formal run.

    The development lane is the one whole-process lane whose cases may select
    work, so its report joins the queue as a channel of its own and a queue that
    carries it is schema 4. Held-out evidence is refused: this command derives
    work, and that lane never is work.
    """
    if target_ratio <= 0 or not math.isfinite(target_ratio):
        raise PerformanceDecisionError("target ratio must be a finite positive number")
    summary, summary_sha = _read_json(summary_path, "preview summary")
    broad, broad_sha = _read_json(broad_path, "broad report")
    external, external_sha = _read_json(external_path, "external report")
    candidate_sha, base_sha = _summary_revisions(summary)
    from .performance_evidence import validate_bundle
    sentinel, sentinel_sha = (None, None) if sentinel_path is None else _read_json(sentinel_path, "sentinel report")
    development, development_sha = (
        (None, None) if development_path is None
        else _read_json(development_path, "development report")
    )
    problems = validate_bundle(summary, broad, external, sentinel)
    if problems:
        raise PerformanceDecisionError("queue requires healthy complete evidence: " + "; ".join(problems))
    host = broad["run"]["host"]
    payload = {
        "schema_version": 4 if development is not None else 3,
        "host": host,
        "artifact_type": _QUEUE_TYPE,
        "engines": summary["engines"],
        "candidate_sha": candidate_sha,
        "comparison_base_sha": base_sha,
        "target_ratio": target_ratio,
        "evidence": {
            "preview_summary_sha256": summary_sha,
            "broad_report_sha256": broad_sha,
            "external_report_sha256": external_sha,
            "sentinel_report_sha256": sentinel_sha,
        },
        "external": _external_entries(external, target_ratio),
        "broad": _broad_entries(broad, target_ratio),
        "sentinel": [] if sentinel is None else [
            {**entry, "id": entry["id"].replace("broad/", "sentinel/", 1)}
            for entry in _broad_entries(sentinel, target_ratio)
        ],
    }
    if development is not None:
        payload["development"] = _development_entries(
            development, target_ratio, summary["engines"], host
        )
        payload["evidence"]["development_report_sha256"] = development_sha
    _reject_held_out(
        {entry["id"] for channel in ("external", "broad", "sentinel", "development")
         for entry in payload.get(channel, [])},
        "opportunity queue",
    )
    return payload


def load_queue(path: Path) -> tuple[dict[str, Any], str]:
    queue, digest = _read_json(path, "opportunity queue")
    version = queue.get("schema_version")
    expected = {
        "schema_version", "artifact_type", "candidate_sha", "comparison_base_sha",
        "target_ratio", "evidence", "external", "broad", "sentinel", "engines",
    }
    if version in {3, 4}:
        expected.add("host")
        _object(queue.get("host"), "opportunity queue.host")
    if version == 4:
        expected.add("development")
    _keys(queue, expected, "opportunity queue")
    if version not in {2, 3, 4} or queue["artifact_type"] != _QUEUE_TYPE:
        raise PerformanceDecisionError("opportunity queue: unsupported schema")
    _revision(queue["candidate_sha"], "opportunity queue.candidate_sha")
    _revision(queue["comparison_base_sha"], "opportunity queue.comparison_base_sha")
    _ratio(queue["target_ratio"], "opportunity queue.target_ratio")
    from .performance_evidence import engine_identity
    engines = engine_identity(queue)
    if (engines["candidate"]["source_revision"] != queue["candidate_sha"]
            or engines["base"]["source_revision"] != queue["comparison_base_sha"]):
        raise PerformanceDecisionError("queue engine revisions disagree with queue identity")
    evidence = _object(queue["evidence"], "opportunity queue.evidence")
    evidence_keys = {
        "preview_summary_sha256", "broad_report_sha256", "external_report_sha256",
        "sentinel_report_sha256",
    }
    if version == 4:
        evidence_keys.add("development_report_sha256")
    _keys(evidence, evidence_keys, "opportunity queue.evidence")
    for key, value in evidence.items():
        if key in {"sentinel_report_sha256", "development_report_sha256"} and value is None:
            continue
        _sha256(value, f"opportunity queue.evidence.{key}")
    channels = ("external", "broad", "sentinel", "development") if version == 4 else (
        "external", "broad", "sentinel"
    )
    seen: set[str] = set()
    for channel in channels:
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
    _reject_held_out(seen, "opportunity queue")
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
    if version not in {1, 2, 3} or unit.get("artifact_type") != _UNIT_TYPE:
        raise PerformanceDecisionError("performance unit: unsupported schema")
    expected_keys = {
        "schema_version", "artifact_type", "unit_id", "base_sha", "queue",
        "priority", "mechanism", "profile_evidence", "fast_gate", "promotion_gate",
    }
    if version >= 2:
        expected_keys.add("unit_kind")
        if unit.get("unit_kind") == "migration":
            expected_keys.add("migration")
    if version == 3:
        expected_keys.add("layout_gate")
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
    fast_keys = {"target_ids", "target_max_candidate_over_base", "max_attempts"}
    if version < 3:
        fast_keys |= {"control_ids", "control_max_candidate_over_base"}
    _keys(fast_gate, fast_keys, "performance unit.fast_gate")
    target_ids = _unique_strings(fast_gate["target_ids"], "performance unit.fast_gate.target_ids", _opportunity_id)
    if version < 3:
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
    if version < 3:
        control_max = _ratio(
            fast_gate["control_max_candidate_over_base"],
            "performance unit.fast_gate.control_max_candidate_over_base",
        )
        if control_max < 1:
            raise PerformanceDecisionError("performance unit control gate cannot require every control to improve")
    if _integer(fast_gate["max_attempts"], "performance unit.fast_gate.max_attempts", 1) > 2:
        raise PerformanceDecisionError("performance unit max_attempts must not exceed two")

    promotion = _object(unit["promotion_gate"], "performance unit.promotion_gate")
    _keys(
        promotion,
        {"require_complete_broad", "require_complete_external", "require_test262_zero_gap"},
        "performance unit.promotion_gate",
    )
    for key, value in promotion.items():
        if not _boolean(value, f"performance unit.promotion_gate.{key}"):
            raise PerformanceDecisionError(f"performance unit.promotion_gate.{key}: must remain true")
    declared = set(priority["opportunity_ids"]) | set(target_ids)
    declared |= set(fast_gate.get("control_ids", []))
    for receipt in profile_evidence:
        declared |= set(receipt["opportunity_ids"])
    if kind == "migration":
        declared |= set(unit["migration"]["cumulative_target_ids"])
    _reject_held_out(declared, "performance unit")
    if version == 3:
        from .layout_plan import validate
        validate(unit)
    return unit, digest


def validate_unit_against_queue(unit: dict[str, Any], unit_sha: str, queue: dict[str, Any], queue_sha: str) -> dict[str, Any]:
    if unit["queue"]["sha256"] != queue_sha:
        raise PerformanceDecisionError("performance unit queue SHA-256 does not match supplied queue")
    if unit["queue"]["candidate_sha"] != queue["candidate_sha"]:
        raise PerformanceDecisionError("performance unit queue candidate SHA does not match supplied queue")
    if unit["base_sha"] != queue["candidate_sha"]:
        raise PerformanceDecisionError("performance unit base SHA must equal the queue candidate SHA")
    if queue.get('schema_version') in {3, 4} and unit.get('schema_version') != 3:
        raise PerformanceDecisionError('new opportunity queues require a schema-3 layout plan')
    if unit.get('schema_version') == 3:
        if queue.get('schema_version') not in {3, 4}:
            raise PerformanceDecisionError('layout plans require a host-bound schema-3 queue')
        from .layout_policy import load_policy
        from .performance_evidence import ROOT, inventories
        contract = unit['layout_gate']
        policy = load_policy(ROOT / contract['policy_path'], contract['policy_sha256'], live=False)
        if not same_node(queue.get('host', {}).get('node'), policy['host_node']):
            raise PerformanceDecisionError('layout policy and opportunity queue hosts differ')
        expected = {f'{lane}/{case}' if lane in ('broad', 'sentinel') else f'external/{lane}/{case}'
                    for lane, cases in inventories().items() for case in cases}
        if 'development' in policy['lanes']:
            from .development import opportunity_ids
            expected |= opportunity_ids()
        elif any(n.startswith('development/') for n in (
                unit['fast_gate']['target_ids']
                + unit.get('migration', {}).get('cumulative_target_ids', [])
                + [n for receipt in unit['profile_evidence'] for n in receipt['opportunity_ids']])):
            raise PerformanceDecisionError('development references require a qualified development policy')
        if set(policy['cases']) != expected or set(policy['held_out']) != _held_out_opportunity_ids():
            raise PerformanceDecisionError('layout policy inventory differs from current corpora')
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
            for channel in ("external", "broad", "sentinel", "development")
            for item in queue.get(channel) or []
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


from .performance_evaluation import decide


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
    queue.add_argument("--sentinel-report", type=Path)
    queue.add_argument("--development-report", type=Path)
    queue.add_argument("--output", type=Path, required=True)

    check = subparsers.add_parser("check-unit", help="validate one plan without evidence files")
    check.add_argument("--unit", type=Path, required=True)

    validate = subparsers.add_parser("validate-unit", help="bind one plan to an opportunity queue")
    validate.add_argument("--unit", type=Path, required=True)
    validate.add_argument("--queue", type=Path, required=True)
    validate.add_argument("--profile-root", type=Path, required=True)

    decision = subparsers.add_parser("decide", help="classify measured evidence for one plan")
    decision.add_argument("--unit", type=Path, required=True)
    decision.add_argument("--queue", type=Path, required=True)
    decision.add_argument("--summary", type=Path, required=True)
    decision.add_argument("--broad-report", type=Path, required=True)
    decision.add_argument("--external-report", type=Path, required=True)
    decision.add_argument("--sentinel-report", type=Path, required=True)
    decision.add_argument("--profile-root", type=Path, required=True)
    decision.add_argument("--test262-burndown", type=Path)
    decision.add_argument("--unstable-base-evidence", type=Path, action="append", default=[])
    decision.add_argument("--resource-report", type=Path)
    decision.add_argument("--resource-raw", type=Path)
    decision.add_argument("--layout-panel", type=Path)
    decision.add_argument("--held-out-panel", type=Path)
    decision.add_argument("--formal-continuation", type=Path, action="append", default=[],
                          help="latest candidate/base precision continuation per formal case")
    decision.add_argument("--mode", choices=("stage", "fast", "promotion"), required=True)
    decision.add_argument("--require-retained", action="store_true")
    decision.add_argument("--output", type=Path, required=True)
    return parser


def main() -> int:
    try:
        args = _parser().parse_args()
        if args.command == "queue":
            from .performance_evidence import replay_reports
            from .bundle import verify
            verify(args.summary, args.broad_report, args.external_report, args.sentinel_report)
            from .bundle import development_report_path
            args.development_report = development_report_path(args.summary, args.development_report)
            replay_reports(
                args.broad_report, args.external_report, args.sentinel_report,
                args.development_report,
            )
            payload = build_queue(
                args.summary, args.broad_report, args.external_report, args.target_ratio,
                args.sentinel_report, args.development_report,
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
            from .profile import verify_profiles
            verify_profiles(unit, queue, args.profile_root)
            print(json.dumps(validate_unit_against_queue(unit, unit_sha, queue, queue_sha), sort_keys=True))
            return 0
        from .performance_evidence import replay_reports
        from .bundle import verify
        verify(args.summary, args.broad_report, args.external_report, args.sentinel_report)
        from .bundle import development_report_path
        replay_reports(args.broad_report, args.external_report, args.sentinel_report,
                       development_report_path(args.summary))
        sentinel, sentinel_sha = _read_json(args.sentinel_report, "sentinel report")
        summary, summary_sha = _read_json(args.summary, "preview summary")
        broad, broad_sha = _read_json(args.broad_report, "broad report")
        external, external_sha = _read_json(args.external_report, "external report")
        test262 = None
        if args.test262_burndown is not None:
            test262 = _read_json(args.test262_burndown, "Test262 burndown")
        from .base_aa_evidence import load_checks
        base_checks = load_checks(args.unstable_base_evidence, args.summary, args.external_report, summary)
        payload = decide(
            unit, unit_sha, queue, queue_sha, summary, summary_sha, broad, broad_sha,
            external, external_sha, args.mode, test262, sentinel=sentinel, sentinel_sha=sentinel_sha,
            profile_root=args.profile_root, unstable_base_checks=base_checks,
            layout_panel_path=args.layout_panel, held_out_path=args.held_out_panel,
            formal_bundle=args.summary.parent, resource_report=args.resource_report, resource_raw=args.resource_raw,
            formal_continuations=args.formal_continuation,
        )
        _atomic_write(args.output, payload)
        print(json.dumps(payload, sort_keys=True))
        if not args.require_retained:
            return 0
        accepted = "advance" if args.mode == "stage" else "retained"
        return 0 if payload["decision"] == accepted else 1
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
