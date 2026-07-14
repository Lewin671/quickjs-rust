"""Fail-closed validation and plan replay for resource JSONL evidence."""

from __future__ import annotations

import hashlib
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from uuid import UUID

from .planning import measurement_plan
from .receipts import ReceiptError, canonical_receipt_sha256
from .records import parse_result
from .resource_process import ResourceProcessError, normalize_peak_rss
from .resource_schema import ResourceLane, ResourceManifest


class ResourceReportError(ValueError):
    """Resource evidence violates its frozen measurement contract."""


MAX_SIGNED_64 = (1 << 63) - 1


def _reject(value: str) -> None:
    raise ResourceReportError(f"resource evidence contains non-standard constant {value}")


def _unique(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ResourceReportError(f"resource evidence contains duplicate key {key!r}")
        result[key] = value
    return result


def _keys(value: Any, expected: set[str], where: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        actual = set(value) if isinstance(value, dict) else set()
        raise ResourceReportError(
            f"{where}: missing {sorted(expected - actual)}, unknown {sorted(actual - expected)}"
        )
    return value


def _integer(
    value: Any, where: str, minimum: int = 0, maximum: int = MAX_SIGNED_64
) -> int:
    if (
        isinstance(value, bool) or not isinstance(value, int)
        or value < minimum or value > maximum
    ):
        raise ResourceReportError(
            f"{where}: expected integer in [{minimum}, {maximum}]"
        )
    return value


def _signed_integer(value: Any, where: str) -> int:
    minimum = -(1 << 63)
    if (
        isinstance(value, bool) or not isinstance(value, int)
        or value < minimum or value > MAX_SIGNED_64
    ):
        raise ResourceReportError(
            f"{where}: expected integer in [{minimum}, {MAX_SIGNED_64}]"
        )
    return value


def _boolean(value: Any, where: str) -> bool:
    if not isinstance(value, bool):
        raise ResourceReportError(f"{where}: expected boolean")
    return value


def _string(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value:
        raise ResourceReportError(f"{where}: expected non-empty string")
    return value


def _sha(value: Any, where: str) -> str:
    if not isinstance(value, str) or len(value) != 64 or any(
        character not in "0123456789abcdef" for character in value
    ):
        raise ResourceReportError(f"{where}: invalid SHA-256")
    return value


ROLE_SET = {"candidate", "base", "quickjs-ng"}
ENGINE_FIELDS = {
    "role", "adapter_id", "binary_sha256", "binary_snapshot_path", "binary_source_path",
    "engine_identity", "provenance_status", "receipt", "receipt_sha256",
}
START_FIELDS = {
    "blocks", "build_recipes", "claim_eligible", "comparison_input_complete",
    "comparison_input_preconditions_met", "engines", "expected_samples", "host", "lane_id",
    "manifest_sha256", "profile", "protocol_files", "protocol_id", "protocol_sha256",
    "record_type", "run_id", "runner_repo", "schema_version", "seed", "series_id",
    "snapshot_root", "suite_id",
}
SAMPLE_FIELDS = {
    "adapter_id", "argv", "binary_sha256", "binary_snapshot_path", "binary_source_path",
    "block", "case_id", "checksum", "descendants_detected", "duration_ns", "engine_identity", "error", "exit_code",
    "host", "iterations", "lane_id", "manifest_sha256", "measurement_eligible", "metric",
    "monitor_error", "operations", "order", "phase", "profile_id", "protocol_id", "protocol_sha256",
    "provenance_status", "raw_rss", "raw_rss_unit", "receipt", "receipt_sha256",
    "record_type", "role", "run_id", "runner_repo", "sample_id", "schema_version",
    "series_id", "started_at", "status", "stderr", "stderr_truncated", "stdout",
    "stdout_truncated", "suite_id", "timed_out", "timer", "timer_phase_boundary", "unit",
    "value", "workload_sha256", "workload_snapshot_path", "workload_source_path",
}
END_FIELDS = {
    "claim_eligible", "comparison_input_complete", "coverage", "lane_id",
    "physical_plan_complete", "record_type", "run_id", "schema_version", "status",
}


@dataclass(frozen=True)
class ValidatedResourceRun:
    input_sha256: str
    input_bytes: int
    start: dict[str, Any]
    end: dict[str, Any]
    lane: ResourceLane
    engines: dict[str, dict[str, Any]]
    samples: tuple[dict[str, Any], ...]
    blocks: int
    valid_blocks: tuple[int, ...]
    invalid_blocks: tuple[dict[str, Any], ...]
    values: dict[tuple[str, int | None], int | float]
    comparison_input_complete: bool


def _validate_host(value: Any, where: str) -> None:
    fields = {"machine", "node", "platform", "processor", "python", "system"}
    value = _keys(value, fields, where)
    for field in fields:
        if not isinstance(value[field], str):
            raise ResourceReportError(f"{where}.{field}: expected string")


def _validate_repo(value: Any, where: str) -> None:
    value = _keys(value, {"commit", "dirty"}, where)
    if value["commit"] is not None and (
        not isinstance(value["commit"], str) or len(value["commit"]) != 40
        or any(character not in "0123456789abcdef" for character in value["commit"])
    ):
        raise ResourceReportError(f"{where}.commit: invalid")
    if value["dirty"] is not None and not isinstance(value["dirty"], bool):
        raise ResourceReportError(f"{where}.dirty: invalid")


def _expected_recipe(manifest: ResourceManifest, identity: str) -> dict[str, Any]:
    return {
        key: list(value) if isinstance(value, tuple) else value
        for key, value in manifest.build_recipes[identity].__dict__.items()
        if key != "engine_identity"
    }


def _validate_engine(
    engine: Any, manifest: ResourceManifest, where: str
) -> dict[str, Any]:
    engine = _keys(engine, ENGINE_FIELDS, where)
    role = _string(engine["role"], f"{where}.role")
    if role not in ROLE_SET:
        raise ResourceReportError(f"{where}.role: unknown role")
    identity = _string(engine["engine_identity"], f"{where}.engine_identity")
    if identity not in manifest.build_recipes:
        raise ResourceReportError(f"{where}.engine_identity: unknown identity")
    expected_identity = {
        "candidate": "qjs-rust", "base": "qjs-rust",
        "quickjs-ng": manifest.reference_identity,
    }[role]
    if identity != expected_identity:
        raise ResourceReportError(
            f"{where}.engine_identity: expected {expected_identity!r} for {role}"
        )
    adapter_id = _string(engine["adapter_id"], f"{where}.adapter_id")
    if adapter_id not in {"qjs-rust-raw", "qjs-file"}:
        raise ResourceReportError(f"{where}.adapter_id: invalid")
    for field in ("binary_source_path", "binary_snapshot_path"):
        _string(engine[field], f"{where}.{field}")
    _sha(engine["binary_sha256"], f"{where}.binary_sha256")
    status = _string(engine["provenance_status"], f"{where}.provenance_status")
    if status not in {"unverified", "dirty", "verified"}:
        raise ResourceReportError(f"{where}.provenance_status: invalid")
    receipt = engine["receipt"]
    receipt_sha = engine["receipt_sha256"]
    if receipt is None:
        if receipt_sha is not None or status != "unverified":
            raise ResourceReportError(f"{where}: missing receipt must be unverified")
        return engine
    if not isinstance(receipt, dict):
        raise ResourceReportError(f"{where}.receipt: expected object")
    _sha(receipt_sha, f"{where}.receipt_sha256")
    try:
        canonical = canonical_receipt_sha256(receipt)
    except ReceiptError as error:
        raise ResourceReportError(f"{where}.receipt: {error}") from error
    if canonical != receipt_sha:
        raise ResourceReportError(f"{where}.receipt_sha256: canonical digest mismatch")
    _keys(
        receipt,
        {"schema_version", "engine_identity", "source", "profile_id", "build", "binary_sha256"},
        f"{where}.receipt",
    )
    if isinstance(receipt["schema_version"], bool) or receipt["schema_version"] != 1:
        raise ResourceReportError(f"{where}.receipt.schema_version: invalid")
    if (
        receipt["engine_identity"] != identity
        or receipt["profile_id"] != manifest.profile.id
        or receipt["binary_sha256"] != engine["binary_sha256"]
        or receipt["build"] != _expected_recipe(manifest, identity)
    ):
        raise ResourceReportError(f"{where}.receipt: identity/recipe mismatch")
    source = _keys(receipt["source"], {"repo", "revision", "dirty"}, f"{where}.receipt.source")
    revision = source["revision"]
    if (
        not isinstance(source["repo"], str) or not source["repo"]
        or not isinstance(revision, str) or len(revision) != 40
        or any(character not in "0123456789abcdef" for character in revision)
        or not isinstance(source["dirty"], bool)
    ):
        raise ResourceReportError(f"{where}.receipt.source: invalid")
    expected_status = "dirty" if source["dirty"] else "verified"
    if status != expected_status:
        raise ResourceReportError(f"{where}.provenance_status: receipt mismatch")
    if role == "quickjs-ng" and (
        source["repo"] != manifest.reference_repo or revision != manifest.reference_revision
    ):
        raise ResourceReportError(f"{where}.receipt.source: reference pin mismatch")
    return engine


def _expected_argv(engine: dict[str, Any], row: dict[str, Any], lane: ResourceLane) -> list[str]:
    case = lane.case
    if case is None:
        return []
    argv = [engine["binary_snapshot_path"]]
    if engine["adapter_id"] == "qjs-rust-raw":
        argv.append("--raw")
    argv.extend([
        row["workload_snapshot_path"], case.id, str(case.iterations),
    ])
    return argv


def _validate_sample(
    row: Any,
    start: dict[str, Any],
    engine: dict[str, Any],
    manifest: ResourceManifest,
    lane: ResourceLane,
    expected_block: int | None,
    expected_order: int,
    where: str,
) -> None:
    row = _keys(row, SAMPLE_FIELDS, where)
    if _integer(row["schema_version"], f"{where}.schema_version", 1) != 1:
        raise ResourceReportError(f"{where}.schema_version: unsupported")
    _integer(row["order"], f"{where}.order")
    if expected_block is None:
        if row["block"] is not None:
            raise ResourceReportError(f"{where}.block: static lane requires null")
    else:
        _integer(row["block"], f"{where}.block")
    exact = {
        "record_type": "resource_sample", "schema_version": 1,
        "run_id": start["run_id"], "manifest_sha256": manifest.sha256,
        "series_id": manifest.series_id, "suite_id": manifest.suite_id,
        "lane_id": lane.id, "profile_id": manifest.profile.id,
        "protocol_id": manifest.protocol_id, "protocol_sha256": manifest.protocol_sha256,
        "host": start["host"], "runner_repo": start["runner_repo"], "phase": "measurement",
        "metric": lane.metric, "unit": lane.unit, "block": expected_block,
        "order": expected_order, "role": engine["role"],
    }
    for field, expected in exact.items():
        if row[field] != expected:
            raise ResourceReportError(f"{where}.{field}: identity/plan mismatch")
    for field in ENGINE_FIELDS - {"role"}:
        if row[field] != engine[field]:
            raise ResourceReportError(f"{where}.{field}: engine identity mismatch")
    try:
        UUID(_string(row["sample_id"], f"{where}.sample_id"))
    except ValueError as error:
        raise ResourceReportError(f"{where}.sample_id: invalid UUID") from error
    for field in (
        "descendants_detected", "measurement_eligible", "timed_out",
        "stdout_truncated", "stderr_truncated",
    ):
        _boolean(row[field], f"{where}.{field}")
    for field in ("stdout", "stderr"):
        if not isinstance(row[field], str) or len(row[field].encode("utf-8")) > 64 * 1024:
            raise ResourceReportError(f"{where}.{field}: invalid or unbounded")
    status = _string(row["status"], f"{where}.status")
    if status not in {"ok", "timeout", "failed", "invalid"}:
        raise ResourceReportError(f"{where}.status: invalid")
    if row["error"] is not None and not isinstance(row["error"], str):
        raise ResourceReportError(f"{where}.error: invalid")
    if row["monitor_error"] is not None and (
        not isinstance(row["monitor_error"], str) or not row["monitor_error"]
    ):
        raise ResourceReportError(f"{where}.monitor_error: invalid")
    if row["exit_code"] is not None:
        _signed_integer(row["exit_code"], f"{where}.exit_code")
    case = lane.case
    if case is None:
        null_fields = {
            "case_id", "checksum", "duration_ns", "exit_code", "iterations", "operations",
            "raw_rss", "raw_rss_unit", "started_at", "timer", "timer_phase_boundary",
            "workload_sha256", "workload_snapshot_path", "workload_source_path", "monitor_error",
        }
        if any(row[field] is not None for field in null_fields) or row["argv"] != []:
            raise ResourceReportError(f"{where}: binary-size process fields must be null")
        if row["stdout"] or row["stderr"] or row["timed_out"] or row["descendants_detected"]:
            raise ResourceReportError(f"{where}: binary-size streams must be empty")
        if status == "ok":
            _integer(row["value"], f"{where}.value", 1)
            if not row["measurement_eligible"] or row["error"] is not None:
                raise ResourceReportError(f"{where}: inconsistent successful size sample")
        elif (
            status != "failed" or row["value"] is not None
            or row["measurement_eligible"] or not row["error"]
        ):
            raise ResourceReportError(f"{where}: inconsistent failed size sample")
        return
    expected_case = {
        "case_id": case.id, "iterations": case.iterations,
        "workload_sha256": case.workload_sha256,
    }
    for field, expected in expected_case.items():
        if row[field] != expected:
            raise ResourceReportError(f"{where}.{field}: case identity mismatch")
    _integer(row["iterations"], f"{where}.iterations", 1)
    _string(row["workload_source_path"], f"{where}.workload_source_path")
    _string(row["workload_snapshot_path"], f"{where}.workload_snapshot_path")
    if row["argv"] != _expected_argv(engine, row, lane):
        raise ResourceReportError(f"{where}.argv: invocation mismatch")
    _integer(row["duration_ns"], f"{where}.duration_ns", 0)
    if not isinstance(row["started_at"], str) or not row["started_at"]:
        raise ResourceReportError(f"{where}.started_at: invalid")
    if lane.id == "fresh_process_latency/wall_ns_per_process":
        if (
            row["timer"] != "python.perf_counter_ns"
            or row["timer_phase_boundary"] != "before_process_spawn_to_wait_return"
            or row["raw_rss"] is not None or row["raw_rss_unit"] is not None
            or row["monitor_error"] is not None
        ):
            raise ResourceReportError(f"{where}: fresh-process timer/RSS mismatch")
    else:
        if row["timer"] is not None or row["timer_phase_boundary"] is not None:
            raise ResourceReportError(f"{where}: RSS sample must not claim wall timer")
        if row["raw_rss_unit"] != manifest.profile.rss_raw_unit:
            raise ResourceReportError(f"{where}.raw_rss_unit: profile unit mismatch")
        if row["raw_rss"] is not None:
            raw = _integer(row["raw_rss"], f"{where}.raw_rss")
            try:
                normalized, raw_unit = normalize_peak_rss(raw, manifest.profile.platform)
            except ResourceProcessError as error:
                raise ResourceReportError(f"{where}.raw_rss: {error}") from error
            if row["raw_rss_unit"] != raw_unit or (
                row["value"] is not None and row["value"] != normalized
            ):
                raise ResourceReportError(f"{where}: RSS unit conversion mismatch")
    expected_result = {
        "case_id": case.id, "iterations": case.iterations,
        "operations": case.operations, "checksum": case.expected_checksum(),
    }
    parsed = None
    expected_status: str
    expected_error: str | None
    if row["monitor_error"] is not None:
        expected_status = "failed"
        expected_error = f"resource monitor failed: {row['monitor_error']}"
    elif row["timed_out"]:
        expected_status = "timeout"
        expected_error = f"timed out after {case.timeout_seconds}s"
    elif row["exit_code"] is None:
        if not row["stderr"] or row["stdout"] or row["stdout_truncated"] or row["stderr_truncated"]:
            raise ResourceReportError(f"{where}: invalid spawn failure state")
        expected_status = "failed"
        expected_error = f"engine could not start: {row['stderr']}"
    elif row["exit_code"] != 0:
        expected_status = "failed"
        expected_error = f"engine exited with status {row['exit_code']}"
    elif row["stdout_truncated"] or row["stderr_truncated"]:
        expected_status = "invalid"
        expected_error = "process output exceeded validation limit"
    elif row["descendants_detected"]:
        expected_status = "invalid"
        expected_error = "engine spawned descendant processes"
    else:
        try:
            parsed = parse_result(row["stdout"])
        except (ValueError, json.JSONDecodeError) as error:
            expected_status = "invalid"
            expected_error = str(error)
        else:
            if parsed != expected_result:
                expected_status = "invalid"
                expected_error = f"result mismatch: expected {expected_result}, got {parsed}"
            else:
                expected_status = "ok"
                expected_error = None
    if status != expected_status or row["error"] != expected_error:
        raise ResourceReportError(f"{where}: status/error does not match recomputed process state")
    if status == "ok":
        if row["exit_code"] != 0 or row["timed_out"]:
            raise ResourceReportError(f"{where}: inconsistent successful process")
        _integer(row["operations"], f"{where}.operations", 1)
        _integer(row["checksum"], f"{where}.checksum")
        if row["operations"] != case.operations or row["checksum"] != expected_result["checksum"]:
            raise ResourceReportError(f"{where}: output/checksum fields mismatch")
        _integer(row["value"], f"{where}.value", 1)
        if not row["measurement_eligible"]:
            raise ResourceReportError(f"{where}: successful sample must be eligible")
        if lane.id == "peak_rss/bytes" and row["raw_rss"] is None:
            raise ResourceReportError(f"{where}: successful RSS sample needs wait4 rusage")
        if lane.id == "fresh_process_latency/wall_ns_per_process" and row["value"] != row["duration_ns"]:
            raise ResourceReportError(f"{where}: fresh-process value must equal wall duration")
    else:
        if row["measurement_eligible"] or row["value"] is not None:
            raise ResourceReportError(f"{where}: failed sample state mismatch")
        if row["operations"] is not None or row["checksum"] is not None:
            raise ResourceReportError(f"{where}: failed sample cannot carry validated output")


def validate_resource_run(
    path: Path, manifest: ResourceManifest
) -> ValidatedResourceRun:
    path = path.expanduser().resolve()
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise ResourceReportError(f"cannot read resource evidence {path}: {error}") from error
    rows = []
    for index, line in enumerate(raw.splitlines(), 1):
        if not line.strip():
            raise ResourceReportError(f"line {index}: blank lines are not allowed")
        try:
            row = json.loads(line, object_pairs_hook=_unique, parse_constant=_reject)
        except json.JSONDecodeError as error:
            raise ResourceReportError(f"line {index}: invalid JSON: {error}") from error
        if not isinstance(row, dict):
            raise ResourceReportError(f"line {index}: expected object")
        rows.append(row)
    if len(rows) < 3:
        raise ResourceReportError("resource evidence is incomplete")
    start = _keys(rows[0], START_FIELDS, "run_start")
    end = _keys(rows[-1], END_FIELDS, "run_end")
    for field in (
        "claim_eligible", "comparison_input_complete",
        "comparison_input_preconditions_met",
    ):
        _boolean(start[field], f"run_start.{field}")
    for field in (
        "claim_eligible", "comparison_input_complete", "physical_plan_complete",
    ):
        _boolean(end[field], f"run_end.{field}")
    if _integer(start["schema_version"], "run_start.schema_version", 1) != 1:
        raise ResourceReportError("run_start.schema_version: unsupported")
    if _integer(end["schema_version"], "run_end.schema_version", 1) != 1:
        raise ResourceReportError("run_end.schema_version: unsupported")
    if start["record_type"] != "resource_run_start" or end["record_type"] != "resource_run_end":
        raise ResourceReportError("resource evidence start/end order mismatch")
    run_id = _string(start["run_id"], "run_start.run_id")
    try:
        UUID(run_id)
    except ValueError as error:
        raise ResourceReportError("run_start.run_id: invalid UUID") from error
    lane_id = _string(start["lane_id"], "run_start.lane_id")
    if lane_id not in manifest.lanes:
        raise ResourceReportError("run_start.lane_id: unknown lane")
    lane = manifest.lanes[lane_id]
    blocks = _integer(start["blocks"], "run_start.blocks", 1)
    if blocks not in (
        {lane.initial_blocks, lane.max_blocks} if lane.kind == "dynamic" else {1}
    ):
        raise ResourceReportError("run_start.blocks: invalid cohort")
    _integer(start["expected_samples"], "run_start.expected_samples", 1)
    if lane.kind == "dynamic":
        _integer(start["seed"], "run_start.seed")
    elif start["seed"] is not None:
        raise ResourceReportError("run_start.seed: static lane requires null")
    exact_start = {
        "claim_eligible": False, "comparison_input_complete": False,
        "manifest_sha256": manifest.sha256, "profile": manifest.profile.__dict__,
        "protocol_files": list(manifest.protocol_file_ids), "protocol_id": manifest.protocol_id,
        "protocol_sha256": manifest.protocol_sha256, "series_id": manifest.series_id,
        "suite_id": manifest.suite_id,
        "seed": lane.seed if lane.kind == "dynamic" else None,
        "build_recipes": {
            identity: {
                key: list(value) if isinstance(value, tuple) else value
                for key, value in recipe.__dict__.items()
            }
            for identity, recipe in manifest.build_recipes.items()
        },
    }
    for field, expected in exact_start.items():
        if start[field] != expected:
            raise ResourceReportError(f"run_start.{field}: contract mismatch")
    _validate_host(start["host"], "run_start.host")
    if start["host"]["machine"] != manifest.profile.machine:
        raise ResourceReportError("run_start.host.machine: profile mismatch")
    _validate_repo(start["runner_repo"], "run_start.runner_repo")
    _string(start["snapshot_root"], "run_start.snapshot_root")
    if not isinstance(start["engines"], list) or not start["engines"]:
        raise ResourceReportError("run_start.engines: expected non-empty array")
    engines: dict[str, dict[str, Any]] = {}
    roles = []
    for index, item in enumerate(start["engines"]):
        engine = _validate_engine(item, manifest, f"run_start.engines[{index}]")
        role = engine["role"]
        if role in engines:
            raise ResourceReportError("run_start.engines: duplicate role")
        engines[role] = engine
        roles.append(role)
    canonical_roles = [role for role in ("candidate", "base", "quickjs-ng") if role in engines]
    if roles != canonical_roles:
        raise ResourceReportError("run_start.engines: roles must be canonical")
    preconditions = set(roles) == ROLE_SET and all(
        engine["provenance_status"] == "verified" for engine in engines.values()
    )
    if start["comparison_input_preconditions_met"] is not preconditions:
        raise ResourceReportError("run_start.comparison_input_preconditions_met: mismatch")
    if lane.kind == "dynamic":
        assert lane.case is not None
        plan = [
            (item.block, item.order, item.role)
            for item in measurement_plan(roles, [lane.case.id], blocks, lane.seed)
        ]
    else:
        plan = [(None, order, role) for order, role in enumerate(roles)]
    if start["expected_samples"] != len(plan) or len(rows) != len(plan) + 2:
        raise ResourceReportError("resource evidence physical plan length mismatch")
    samples = rows[1:-1]
    sample_ids = set()
    for index, (row, expected) in enumerate(zip(samples, plan), 1):
        block, order, role = expected
        _validate_sample(
            row, start, engines[role], manifest, lane, block, order, f"sample[{index}]"
        )
        if row["sample_id"] in sample_ids:
            raise ResourceReportError("resource evidence contains duplicate sample_id")
        sample_ids.add(row["sample_id"])
    valid_count = sum(row["status"] == "ok" for row in samples)
    valid_by_role = {
        role: sum(row["status"] == "ok" and row["role"] == role for row in samples)
        for role in roles
    }
    coverage = {
        "attempted": len(samples), "planned": len(plan), "valid": valid_count,
        "valid_by_role": valid_by_role,
    }
    end_coverage = _keys(end["coverage"], set(coverage), "run_end.coverage")
    for field in ("attempted", "planned", "valid"):
        _integer(end_coverage[field], f"run_end.coverage.{field}")
    valid_by_role_raw = _keys(
        end_coverage["valid_by_role"], set(roles), "run_end.coverage.valid_by_role"
    )
    for role in roles:
        _integer(valid_by_role_raw[role], f"run_end.coverage.valid_by_role.{role}")
    expected_complete = preconditions and valid_count == len(plan)
    exact_end = {
        "claim_eligible": False, "comparison_input_complete": expected_complete,
        "coverage": coverage, "lane_id": lane.id, "physical_plan_complete": True,
        "run_id": run_id, "schema_version": 1,
        "status": "complete" if valid_count == len(plan) else "failed",
    }
    for field, expected in exact_end.items():
        if end[field] != expected:
            raise ResourceReportError(f"run_end.{field}: recomputed value mismatch")
    values = {
        (row["role"], row["block"]): (
            float(row["value"]) if lane.kind == "dynamic" else int(row["value"])
        )
        for row in samples if row["status"] == "ok"
    }
    invalid_blocks = []
    valid_blocks = []
    if lane.kind == "dynamic":
        for block in range(blocks):
            triggers = [
                {"role": row["role"], "status": row["status"], "error": row["error"]}
                for row in samples if row["block"] == block and row["status"] != "ok"
            ]
            if triggers:
                invalid_blocks.append({"block": block, "triggers": triggers})
                for role in roles:
                    values.pop((role, block), None)
            else:
                valid_blocks.append(block)
    return ValidatedResourceRun(
        input_sha256=hashlib.sha256(raw).hexdigest(), input_bytes=len(raw), start=start,
        end=end, lane=lane, engines=engines, samples=tuple(samples), blocks=blocks,
        valid_blocks=tuple(valid_blocks), invalid_blocks=tuple(invalid_blocks), values=values,
        comparison_input_complete=expected_complete,
    )
