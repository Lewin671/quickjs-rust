"""Strict field, provenance, and sample-state validation for raw evidence."""

from __future__ import annotations

import json
import math
from typing import Any

from .records import parse_result
from .receipts import ReceiptError, canonical_receipt_sha256
from .schema import Manifest


class ReportError(ValueError):
    """Raw evidence violates its measurement contract."""


def reject_constant(value: str) -> None:
    raise ReportError(f"raw JSON contains non-standard numeric constant {value}")


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ReportError(f"raw JSON contains duplicate key {key!r}")
        result[key] = value
    return result


def keys(value: dict[str, Any], expected: set[str], where: str) -> None:
    if set(value) != expected:
        raise ReportError(
            f"{where}: missing {sorted(expected - set(value))}, "
            f"unknown {sorted(set(value) - expected)}"
        )


def integer(value: Any, where: str, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        raise ReportError(f"{where}: expected integer >= {minimum}")
    return value


def number(value: Any, where: str, *, positive: bool = False) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ReportError(f"{where}: expected number")
    result = float(value)
    if not math.isfinite(result) or (positive and result <= 0):
        qualifier = "positive " if positive else ""
        raise ReportError(f"{where}: expected {qualifier}finite number")
    return result


def boolean(value: Any, where: str) -> bool:
    if not isinstance(value, bool):
        raise ReportError(f"{where}: expected boolean")
    return value


def nonempty_string(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value:
        raise ReportError(f"{where}: expected non-empty string")
    return value


def sha256(value: Any, where: str, *, nullable: bool = False) -> str | None:
    if nullable and value is None:
        return None
    if not isinstance(value, str) or len(value) != 64 or any(
        character not in "0123456789abcdef" for character in value
    ):
        raise ReportError(f"{where}: invalid SHA-256")
    return value


ROLE_SET = {"candidate", "base", "quickjs-ng"}
HOST_FIELDS = {"machine", "node", "platform", "processor", "python", "system"}
RUNNER_REPO_FIELDS = {"commit", "dirty"}
COVERAGE_FIELDS = {"common", "manifest_total", "measured_by_role", "selected_total"}
ENGINE_FIELDS = {
    "role", "binary_source_path", "binary_snapshot_path", "binary_sha256",
    "binary_version", "binary_version_probe_snapshot_path",
    "binary_version_probe_pre_sha256", "binary_version_probe_post_sha256",
    "adapter_id", "engine_identity", "provenance_status", "receipt", "receipt_sha256",
}
RUN_START_FIELDS = {
    "blocks", "build_recipes", "claim_eligible", "comparison_input_complete",
    "comparison_input_preconditions_met", "coverage", "engines", "host", "manifest",
    "manifest_cases", "manifest_sha256", "portfolio_complete", "profile",
    "protocol_files", "protocol_id", "protocol_sha256", "provenance_status",
    "lane_id", "record_type", "run_id", "runner_repo", "snapshot_root",
    "schema_version", "seed", "selected_cases", "series_id", "suite_id",
}
SAMPLE_FIELDS = {
    "adapter_id", "argv", "binary_sha256", "binary_snapshot_path", "binary_source_path",
    "binary_version", "binary_version_probe_post_sha256", "binary_version_probe_pre_sha256",
    "binary_version_probe_snapshot_path", "block", "case_id", "checksum",
    "diagnostic_point", "duration_ns", "engine_identity", "error", "exit_code", "family",
    "host", "iterations", "lane_id", "manifest_sha256", "measurement_eligible", "metric",
    "operations", "order", "phase", "profile_id", "protocol_id", "protocol_sha256",
    "provenance_status", "quality", "receipt", "receipt_sha256", "record_type", "role",
    "run_id", "runner_repo", "schema_version", "series_id", "started_at", "status",
    "stderr", "stderr_truncated", "stdout", "stdout_truncated", "suite_id", "timed_out",
    "timer", "timer_phase_boundary", "workload_sha256", "workload_snapshot_path",
    "workload_source_path",
}
RUN_END_FIELDS = {
    "claim_eligible", "comparison_input_complete", "coverage", "lane_id",
    "portfolio_complete", "provenance_status", "record_type", "run_id",
    "schema_version", "status",
}


def validate_host(value: Any, where: str) -> None:
    if not isinstance(value, dict):
        raise ReportError(f"{where}: expected object")
    keys(value, HOST_FIELDS, where)
    for field in HOST_FIELDS:
        if not isinstance(value[field], str):
            raise ReportError(f"{where}.{field}: expected string")


def validate_runner_repo(value: Any, where: str) -> None:
    if not isinstance(value, dict):
        raise ReportError(f"{where}: expected object")
    keys(value, RUNNER_REPO_FIELDS, where)
    commit = value["commit"]
    if commit is not None and (
        not isinstance(commit, str) or len(commit) != 40
        or any(character not in "0123456789abcdef" for character in commit)
    ):
        raise ReportError(f"{where}.commit: expected null or lowercase 40-hex commit")
    if value["dirty"] is not None and not isinstance(value["dirty"], bool):
        raise ReportError(f"{where}.dirty: expected null or boolean")


def validate_coverage(value: Any, expected: dict[str, Any], where: str) -> None:
    if not isinstance(value, dict):
        raise ReportError(f"{where}: expected object")
    keys(value, COVERAGE_FIELDS, where)
    measured = value["measured_by_role"]
    if not isinstance(measured, dict):
        raise ReportError(f"{where}.measured_by_role: expected object")
    keys(measured, ROLE_SET, f"{where}.measured_by_role")
    for field in ("common", "manifest_total", "selected_total"):
        integer(value[field], f"{where}.{field}")
    for role in ROLE_SET:
        integer(measured[role], f"{where}.measured_by_role.{role}")
    if value != expected:
        raise ReportError(f"{where}: does not match recomputed coverage")


def validate_identity(row: dict[str, Any], start: dict[str, Any], where: str) -> None:
    if row["run_id"] != start["run_id"] or row["schema_version"] != 4:
        raise ReportError(f"{where}: run/schema identity mismatch")


def validate_engine(
    role: str, engine: dict[str, Any], manifest: Manifest, where: str
) -> None:
    nonempty_string(role, f"{where}.role")
    identity = nonempty_string(engine["engine_identity"], f"{where}.engine_identity")
    if identity not in manifest.build_recipes:
        raise ReportError(f"{where}.engine_identity: unknown identity")
    adapter_id = nonempty_string(engine["adapter_id"], f"{where}.adapter_id")
    if adapter_id not in {"qjs-rust-raw", "qjs-file"}:
        raise ReportError(f"{where}.adapter_id: unknown adapter")
    if engine["provenance_status"] != "verified":
        raise ReportError(f"{where}.provenance_status: expected verified")
    for field in (
        "binary_source_path", "binary_snapshot_path",
        "binary_version_probe_snapshot_path",
    ):
        nonempty_string(engine[field], f"{where}.{field}")
    sha256(engine["binary_sha256"], f"{where}.binary_sha256")
    sha256(
        engine["binary_version_probe_pre_sha256"],
        f"{where}.binary_version_probe_pre_sha256",
    )
    sha256(
        engine["binary_version_probe_post_sha256"],
        f"{where}.binary_version_probe_post_sha256", nullable=True,
    )
    sha256(engine["receipt_sha256"], f"{where}.receipt_sha256")
    version = engine["binary_version"]
    if version is not None and (
        not isinstance(version, str) or not 1 <= len(version) <= 512
    ):
        raise ReportError(f"{where}.binary_version: expected null or 1..512 character string")
    if engine["binary_version_probe_pre_sha256"] != engine["binary_sha256"]:
        raise ReportError(f"{where}: version probe was not bound to binary hash")
    receipt = engine["receipt"]
    if not isinstance(receipt, dict):
        raise ReportError(f"{where}.receipt: verified engine requires receipt")
    try:
        canonical_digest = canonical_receipt_sha256(receipt)
    except ReceiptError as error:
        raise ReportError(f"{where}.receipt: {error}") from error
    if canonical_digest != engine["receipt_sha256"]:
        raise ReportError(f"{where}.receipt_sha256: canonical content digest mismatch")
    keys(
        receipt,
        {"schema_version", "engine_identity", "source", "profile_id", "build", "binary_sha256"},
        f"{where}.receipt",
    )
    if isinstance(receipt["schema_version"], bool) or receipt["schema_version"] != 1:
        raise ReportError(f"{where}.receipt.schema_version: invalid")
    if (
        receipt["engine_identity"] != identity
        or receipt["profile_id"] != manifest.profile.id
        or receipt["binary_sha256"] != engine["binary_sha256"]
    ):
        raise ReportError(f"{where}.receipt: identity mismatch")
    source = receipt["source"]
    if not isinstance(source, dict):
        raise ReportError(f"{where}.receipt.source: expected object")
    keys(source, {"repo", "revision", "dirty"}, f"{where}.receipt.source")
    revision = source["revision"]
    if (
        not isinstance(source["repo"], str) or not source["repo"]
        or not isinstance(revision, str) or len(revision) != 40
        or any(character not in "0123456789abcdef" for character in revision)
        or source["dirty"] is not False
    ):
        raise ReportError(f"{where}.receipt.source: invalid or dirty source")
    recipe = manifest.build_recipes[identity]
    expected_build = {
        key: list(value) if isinstance(value, tuple) else value
        for key, value in recipe.__dict__.items() if key != "engine_identity"
    }
    if receipt["build"] != expected_build:
        raise ReportError(f"{where}.receipt.build: recipe mismatch")
    if role == "quickjs-ng" and (
        source["repo"] != manifest.reference_repo
        or revision != manifest.reference_revision
    ):
        raise ReportError(f"{where}.receipt.source: reference pin mismatch")


def validate_sample_common(
    row: dict[str, Any], start: dict[str, Any], manifest: Manifest, where: str
) -> Any:
    keys(row, SAMPLE_FIELDS, where)
    validate_identity(row, start, where)
    exact = {
        "manifest_sha256": manifest.sha256,
        "series_id": manifest.series_id,
        "suite_id": manifest.suite_id,
        "protocol_id": manifest.protocol_id,
        "protocol_sha256": manifest.protocol_sha256,
        "lane_id": manifest.lane_id,
        "profile_id": manifest.profile.id,
        "metric": "outer_wall_time",
        "timer": "python.perf_counter_ns",
        "timer_phase_boundary": "before_process_spawn_to_wait_return",
        "host": start["host"],
        "runner_repo": start["runner_repo"],
    }
    for field, value in exact.items():
        if row[field] != value:
            raise ReportError(f"{where}.{field}: identity mismatch")
    for field in (
        "measurement_eligible", "timed_out", "stdout_truncated", "stderr_truncated",
    ):
        boolean(row[field], f"{where}.{field}")
    for field in (
        "binary_source_path", "binary_snapshot_path", "binary_version_probe_snapshot_path",
        "workload_source_path", "workload_snapshot_path",
    ):
        nonempty_string(row[field], f"{where}.{field}")
    case = next((case for case in manifest.cases if case.id == row["case_id"]), None)
    if case is None:
        raise ReportError(f"{where}.case_id: unknown case")
    if row["family"] != case.family or row["workload_sha256"] != case.workload_sha256:
        raise ReportError(f"{where}: case metadata mismatch")
    return case


def _expected_argv(row: dict[str, Any], case: Any, iterations: int) -> list[str]:
    argv = [row["binary_snapshot_path"]]
    if row["adapter_id"] == "qjs-rust-raw":
        argv.append("--raw")
    elif row["adapter_id"] != "qjs-file":
        raise ReportError("unknown adapter")
    argv.extend([row["workload_snapshot_path"], case.id, str(iterations)])
    return argv


def validate_success(row: dict[str, Any], case: Any, where: str) -> None:
    if row["status"] != "ok" or row["timed_out"]:
        raise ReportError(f"{where}: required sample is not successful")
    if row["stdout_truncated"] or row["stderr_truncated"]:
        raise ReportError(f"{where}: truncated sample is invalid")
    if (
        isinstance(row["exit_code"], bool) or not isinstance(row["exit_code"], int)
        or row["exit_code"] != 0
    ):
        raise ReportError(f"{where}.exit_code: successful sample requires integer 0")
    if row["error"] is not None:
        raise ReportError(f"{where}.error: successful sample requires null")
    if not isinstance(row["stdout"], str) or not isinstance(row["stderr"], str):
        raise ReportError(f"{where}: stdout and stderr must be strings")
    nonempty_string(row["started_at"], f"{where}.started_at")
    iterations = integer(row["iterations"], f"{where}.iterations")
    operations = integer(row["operations"], f"{where}.operations")
    integer(row["checksum"], f"{where}.checksum")
    integer(row["duration_ns"], f"{where}.duration_ns", minimum=1)
    if (
        operations != case.expected_operations(iterations)
        or row["checksum"] != case.expected_checksum(iterations)
    ):
        raise ReportError(f"{where}: operations/checksum does not match manifest")
    try:
        parsed = parse_result(row["stdout"])
    except (ValueError, json.JSONDecodeError) as error:
        raise ReportError(f"{where}.stdout: invalid workload result: {error}") from error
    if parsed != {
        "case_id": case.id, "iterations": iterations,
        "operations": operations, "checksum": row["checksum"],
    }:
        raise ReportError(f"{where}.stdout: result does not match record")
    if row["argv"] != _expected_argv(row, case, iterations):
        raise ReportError(f"{where}.argv: invocation does not match record")


def validate_non_success(row: dict[str, Any], case: Any, where: str) -> str:
    """Validate a durable runner failure and return its stable reason category."""
    status = row["status"]
    if row["measurement_eligible"] is not False or row["quality"] != "ineligible":
        raise ReportError(f"{where}: non-success must be ineligible")
    if status == "not_run":
        expected = {
            "argv": [], "iterations": None, "operations": None, "checksum": None,
            "duration_ns": None, "started_at": None, "exit_code": None,
            "timed_out": False, "stdout": "", "stderr": "",
            "stdout_truncated": False, "stderr_truncated": False,
            "error": "startup/calibration/warmup did not complete",
        }
        for field, value in expected.items():
            if row[field] != value:
                raise ReportError(f"{where}.{field}: invalid not_run record")
        return "not_run"
    if status not in {"failed", "timeout", "invalid"}:
        raise ReportError(f"{where}.status: unknown non-success state {status!r}")
    iterations = integer(row["iterations"], f"{where}.iterations")
    nonempty_string(row["started_at"], f"{where}.started_at")
    number(row["duration_ns"], f"{where}.duration_ns", positive=True)
    if not isinstance(row["stdout"], str) or not isinstance(row["stderr"], str):
        raise ReportError(f"{where}: stdout and stderr must be strings")
    nonempty_string(row["error"], f"{where}.error")
    if row["argv"] != _expected_argv(row, case, iterations):
        raise ReportError(f"{where}.argv: invocation does not match record")
    exit_code = row["exit_code"]
    if status == "failed" and exit_code is None:
        if (
            row["timed_out"] is not False
            or not row["stderr"]
            or row["error"] != f"engine could not start: {row['stderr']}"
            or row["operations"] is not None
            or row["checksum"] is not None
            or row["stdout"] != ""
            or row["stdout_truncated"]
            or row["stderr_truncated"]
        ):
            raise ReportError(f"{where}: invalid process-spawn failure record")
        return "spawn_failed"
    if isinstance(exit_code, bool) or not isinstance(exit_code, int):
        raise ReportError(f"{where}.exit_code: expected integer or spawn-failure null")
    if status == "timeout":
        if (
            row["timed_out"] is not True
            or row["error"] != f"timed out after {case.timeout_seconds}s"
            or row["operations"] is not None or row["checksum"] is not None
        ):
            raise ReportError(f"{where}: invalid timeout record")
        return "timeout"
    if row["timed_out"] is not False:
        raise ReportError(f"{where}: non-timeout record has timed_out=true")
    if status == "failed":
        if (
            exit_code == 0 or row["error"] != f"engine exited with status {exit_code}"
            or row["operations"] is not None or row["checksum"] is not None
        ):
            raise ReportError(f"{where}: invalid failed record")
        return "engine_failed"
    if exit_code != 0:
        raise ReportError(f"{where}: invalid-output record requires exit status zero")
    if row["stdout_truncated"] or row["stderr_truncated"]:
        streams = []
        if row["stdout_truncated"]:
            streams.append("stdout")
        if row["stderr_truncated"]:
            streams.append("stderr")
        if row["error"] != f"{' and '.join(streams)} exceeded validation limit":
            raise ReportError(f"{where}: invalid truncation error")
        return "output_truncated"
    parsed_operations = None
    parsed_checksum = None
    try:
        parsed = parse_result(row["stdout"])
        if parsed["case_id"] != case.id or parsed["iterations"] != iterations:
            expected_error = "result identity does not match invocation"
        else:
            parsed_operations = parsed["operations"]
            parsed_checksum = parsed["checksum"]
            if isinstance(parsed_operations, bool) or not isinstance(parsed_operations, int):
                expected_error = "operations must be an integer"
            elif isinstance(parsed_checksum, bool) or not isinstance(parsed_checksum, int):
                expected_error = "checksum must be an integer"
            elif parsed_operations != case.expected_operations(iterations):
                expected_error = (
                    f"operations mismatch: expected {case.expected_operations(iterations)}, "
                    f"got {parsed_operations}"
                )
            elif parsed_checksum != case.expected_checksum(iterations):
                expected_error = (
                    f"checksum mismatch: expected {case.expected_checksum(iterations)}, "
                    f"got {parsed_checksum}"
                )
            else:
                raise ReportError(f"{where}: valid result mislabeled invalid")
    except (ValueError, json.JSONDecodeError) as error:
        expected_error = str(error)
    if (
        row["error"] != expected_error
        or row["operations"] != parsed_operations
        or row["checksum"] != parsed_checksum
    ):
        raise ReportError(f"{where}: invalid result classification mismatch")
    return "invalid_result"
