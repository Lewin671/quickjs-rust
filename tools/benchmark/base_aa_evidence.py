"""Replay same-base A/A evidence bound to a sealed whole-process case.

No supplied interval or instruction floor is trusted. Both are reconstructed
from complete paired samples; identities and launch protocol match the bundle.
"""
from __future__ import annotations

import json
from pathlib import Path

from .external_preview import _EXPECTED_STDOUT_BY_ROLE
from .external_report import paired_effect
from .performance_evidence import MAX_WIDTH, MIN_BLOCKS, interval
from .performance_schema import PerformanceDecisionError, _integer, _keys, _read_json

CONTEXT_KEYS = ("host", "manifest_sha256", "bundle_sha256", "argv", "metric",
                "timer", "timer_phase_boundary", "timeout_seconds", "source_revision",
                "source_files", "preview_id", "schema_version", "record_type", "suite_id", "case_id")


def case_context(external_path: Path, case_id: str):
    """Read launch identity and instruction pairs from authenticated raw data."""
    parts = case_id.split("/")
    if len(parts) != 3 or parts[0] != "external":
        raise PerformanceDecisionError("base A/A requires an audited whole-process external case")
    records = [json.loads(line) for line in
               (external_path.parent / "external-raw.jsonl").read_text().splitlines()]
    records = [r for r in records if r.get("phase") == "measurement"
               and r.get("suite_id") == parts[1] and r.get("case_id") == parts[2]]
    base = [r for r in records if r.get("role") == "base"]
    candidate = [r for r in records if r.get("role") == "candidate"]
    if not base or len(base) != len(candidate):
        raise PerformanceDecisionError(f"{case_id}: missing complete raw instruction pairs")
    context = {key: base[0][key] for key in CONTEXT_KEYS}
    if any({key: r[key] for key in CONTEXT_KEYS} != context for r in base):
        raise PerformanceDecisionError(f"{case_id}: inconsistent base launch protocol")
    count = len(base)
    instructions = paired_effect(candidate, base, count, "instructions")
    if instructions is None:
        raise PerformanceDecisionError(f"{case_id}: missing candidate/base instruction evidence")
    return context, count, instructions


def replay_aa(payload, summary, summary_sha, context, required_blocks, metric):
    """Refuse short/mixed evidence and derive A/A noise without discarding rows."""
    _keys(payload, {"schema_version", "artifact_type", "case_id", "base_engine",
                    "summary_sha256", "context", "blocks", "samples"}, "base A/A evidence")
    if (payload["schema_version"] != 1
            or payload["artifact_type"] != "quickjs-base-case-aa-evidence"):
        raise PerformanceDecisionError("unsupported base A/A evidence schema")
    if payload["base_engine"] != summary["engines"]["base"]:
        raise PerformanceDecisionError("base A/A executable/revision mismatch")
    if payload["summary_sha256"] != summary_sha or payload["context"] != context:
        raise PerformanceDecisionError("base A/A bundle or launch protocol mismatch")
    if metric not in {"cycles", "wall_time"}:
        raise PerformanceDecisionError("base A/A unknown judged metric")
    if payload["case_id"] != f"external/{context["suite_id"]}/{context["case_id"]}":
        raise PerformanceDecisionError("base A/A case mismatch")
    blocks = _integer(payload["blocks"], "base A/A blocks", MIN_BLOCKS)
    if blocks < required_blocks:
        raise PerformanceDecisionError("base A/A has fewer blocks than the bundle")
    samples = payload["samples"]
    if not isinstance(samples, list) or len(samples) != 2 * blocks:
        raise PerformanceDecisionError("base A/A missing paired samples")
    roles = {"candidate": [], "base": []}
    seen = set()
    for row in samples:
        if not isinstance(row, dict):
            raise PerformanceDecisionError("base A/A sample must be an object")
        role, block = row.get("role"), row.get("block")
        if (role not in roles or type(block) is not int or not 0 <= block < blocks
                or (role, block) in seen):
            raise PerformanceDecisionError("base A/A duplicate or missing role/block")
        seen.add((role, block))
        if (row.get("binary_sha256") != payload["base_engine"]["binary_sha256"]
                or {key: row.get(key) for key in CONTEXT_KEYS} != context
                or row.get("measurement_blocks") != blocks
                or row.get("phase") != "measurement"
                or row.get("status") != "ok" or row.get("exit_code") != 0
                or row.get("timed_out") is not False or row.get("error") is not None
                or row.get("stdout_truncated") is not False
                or row.get("stderr_truncated") is not False
                or row.get("stdout") != _EXPECTED_STDOUT_BY_ROLE[role]):
            raise PerformanceDecisionError("base A/A sample identity/protocol/output mismatch")
        for name in ("cycles", "instructions", "duration_ns"):
            _integer(row.get(name), f"base A/A {name}", 1)
        start = _integer(row.get("timer_started_ns"), "base A/A timer start")
        finish = _integer(row.get("timer_finished_ns"), "base A/A timer finish")
        if finish - start != row["duration_ns"]:
            raise PerformanceDecisionError("base A/A duration does not match timer boundaries")
        roles[role].append(row)
    cycles = paired_effect(roles["candidate"], roles["base"], blocks, "cycles")
    wall = paired_effect(roles["candidate"], roles["base"], blocks, "duration_ns")
    instructions = paired_effect(roles["candidate"], roles["base"], blocks, "instructions")
    judged = cycles if metric == "cycles" else wall
    if judged is None or interval(judged, "base A/A")[2]:
        raise PerformanceDecisionError("base A/A interval does not exceed the precision limit")
    left = {r["block"]: r["instructions"] for r in roles["candidate"]}
    right = {r["block"]: r["instructions"] for r in roles["base"]}
    floor = max(max(left[b] / right[b], right[b] / left[b]) for b in left)
    return {"cycles": cycles, "wall_time": wall, "instructions": instructions,
            "instruction_floor": {"lower": 1 / floor, "upper": floor,
                                  "method": "symmetric-full-paired-spread"}}


def load_checks(paths, summary_path, external_path, summary):
    """Called only after seal verification and report replay by the decision CLI."""
    if not paths:
        return {}
    _, summary_sha = _read_json(summary_path, "comparison summary")
    result = {}
    for path in paths:
        payload, evidence_sha = _read_json(path, "base A/A evidence")
        case_id = payload.get("case_id")
        if not isinstance(case_id, str) or case_id in result:
            raise PerformanceDecisionError("base A/A invalid or duplicate case id")
        context, blocks, instructions = case_context(external_path, case_id)
        # Select the same metric as the bundle, not a caller-chosen looser one.
        from .performance_evidence import promotion_metric
        broad, _ = _read_json(summary_path.parent / "report.json", "broad report")
        external, _ = _read_json(external_path, "external report")
        sentinel, _ = _read_json(summary_path.parent / "sentinel-report.json", "sentinel report")
        metric = promotion_metric(broad, external, sentinel)
        aa = replay_aa(payload, summary, summary_sha, context, blocks, metric)
        result[case_id] = {"aa": aa, "sha256": evidence_sha,
                           "candidate_over_base_instructions": instructions}
    return result


def classify(checks, limits, targets, metrics):
    """Separate eligible exclusions, changed work, and never-passing targets."""
    rows, excluded, reasons, failed = {}, set(), [], []
    for case, check in checks.items():
        if case not in limits or case not in metrics:
            raise PerformanceDecisionError(f"{case}: base A/A case is not watched by this decision")
        instructions = check["candidate_over_base_instructions"]
        lower, upper, precise = interval(instructions, f"{case} instructions")
        floor = check["aa"]["instruction_floor"]
        eligible = precise and lower >= floor["lower"] and upper <= floor["upper"]
        rows[case] = {**check, "classification": "unstable_on_base" if eligible else "changed_or_uncertain_work",
                      "instruction_gate": "pass" if eligible else "not_passed",
                      "precision_excluded": eligible, "declared_target": case in targets,
                      "candidate_over_base_timing": metrics[case]}
        if case in targets:
            reasons.append(f"{case}: unstable base A/A cannot count as a passed target")
            if eligible:
                excluded.add(case)
        elif eligible:
            excluded.add(case)
        elif lower > floor["upper"]:
            failed.append(case)
            reasons.append(f"{case}: instructions exceed base A/A instruction floor")
        else:
            reasons.append(f"{case}: instruction interval is outside base A/A floor")
    return rows, excluded, reasons, failed
