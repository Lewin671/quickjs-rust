"""Fail-closed validation for portable measurement JSONL evidence."""

from __future__ import annotations

import hashlib
import json
import statistics
from collections import defaultdict
from dataclasses import dataclass
from fractions import Fraction
from pathlib import Path
from typing import Any

from .planning import measurement_plan
from .raw_contract import (
    ENGINE_FIELDS,
    ROLE_SET,
    RUN_END_FIELDS,
    RUN_START_FIELDS,
    ReportError,
    boolean,
    integer,
    keys,
    nonempty_string,
    reject_constant,
    unique_object,
    validate_coverage,
    validate_engine,
    validate_host,
    validate_identity,
    validate_non_success,
    validate_runner_repo,
    validate_sample_common,
    validate_success,
)
from .schema import Manifest, next_calibration_iterations


@dataclass(frozen=True)
class ValidatedRun:
    input_sha256: str
    input_bytes: int
    start: dict[str, Any]
    engines: dict[str, dict[str, Any]]
    blocks: int
    measurements: dict[tuple[str, str, int], float]
    measurement_records: dict[tuple[str, str, int], dict[str, Any]]
    valid_blocks: tuple[int, ...]
    invalid_blocks: tuple[dict[str, Any], ...]
    linearity: dict[tuple[str, str, str], dict[str, Any]]
    startup: dict[tuple[str, str], tuple[int, int, int]]
    diagnostic_failures: tuple[dict[str, Any], ...]
    runner_end_status: str
    comparison_input_complete: bool
    runner_coverage: dict[str, Any]


def _read_jsonl(path: Path) -> tuple[list[dict[str, Any]], str, int]:
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise ReportError(f"cannot read input {path}: {error}") from error
    rows = []
    for line_number, raw_line in enumerate(raw.splitlines(), 1):
        if not raw_line.strip():
            raise ReportError(f"line {line_number}: blank lines are not allowed")
        try:
            value = json.loads(
                raw_line, object_pairs_hook=unique_object, parse_constant=reject_constant
            )
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise ReportError(f"line {line_number}: invalid JSON: {error}") from error
        if not isinstance(value, dict):
            raise ReportError(f"line {line_number}: expected object")
        rows.append(value)
    if not rows:
        raise ReportError("input is empty")
    return rows, hashlib.sha256(raw).hexdigest(), len(raw)


def _replay_setup(
    samples: list[tuple[dict[str, Any], str | None]], case: Any, role: str
) -> tuple[bool, int | None, tuple[int, ...], dict[str, Any] | None]:
    """Replay the runner's exact startup/calibration/warmup/linearity machine."""
    pair = f"{role}/{case.id}"
    index = 0
    startup_durations: list[int] = []

    def take(phase: str, iterations: int) -> tuple[dict[str, Any], str | None]:
        nonlocal index
        if index >= len(samples):
            raise ReportError(f"diagnostic portfolio silently incomplete for {pair}")
        row, reason = samples[index]
        index += 1
        if row["phase"] != phase:
            raise ReportError(
                f"diagnostic state mismatch for {pair}: expected {phase}, got {row['phase']}"
            )
        if row["iterations"] != iterations:
            raise ReportError(
                f"diagnostic iterations mismatch for {pair}/{phase}: "
                f"expected {iterations}, got {row['iterations']}"
            )
        return row, reason

    def failed(
        row: dict[str, Any], reason: str, formal_iterations: int | None
    ) -> tuple[bool, int | None, tuple[int, ...], dict[str, Any]]:
        if index != len(samples):
            raise ReportError(f"diagnostic failure must terminate {pair}")
        return (
            False,
            formal_iterations,
            tuple(startup_durations),
            {
                "role": role,
                "case_id": case.id,
                "phase": row["phase"],
                "status": row["status"],
                "reason": reason,
            },
        )

    for _ in range(3):
        row, reason = take("startup", 0)
        if reason is not None:
            return failed(row, reason, None)
        startup_durations.append(row["duration_ns"])

    startup_ns = int(statistics.median(startup_durations))
    target_ns = case.calibration_target_ns(startup_ns)
    expected_iterations = case.initial_iterations
    while True:
        row, reason = take("calibration", expected_iterations)
        if reason is not None:
            return failed(row, reason, None)
        reached = (
            row["duration_ns"] >= target_ns
            or expected_iterations >= case.max_iterations
        )
        if reached:
            formal_iterations = expected_iterations
            break
        expected_iterations = next_calibration_iterations(
            expected_iterations,
            target_ns,
            row["duration_ns"],
            case.max_iterations,
        )

    for _ in range(case.warmup_runs):
        row, reason = take("warmup", formal_iterations)
        if reason is not None:
            return failed(row, reason, formal_iterations)

    diagnostic_n = max(1, min(formal_iterations, case.max_iterations // 2))
    row, reason = take("linearity_n", diagnostic_n)
    if reason is not None:
        return failed(row, reason, formal_iterations)
    row, reason = take("linearity_2n", diagnostic_n * 2)
    if reason is not None:
        return failed(row, reason, formal_iterations)
    if index != len(samples):
        raise ReportError(f"diagnostic portfolio has extra records for {pair}")
    return True, formal_iterations, tuple(startup_durations), None


def validate_run(input_path: Path, manifest: Manifest) -> ValidatedRun:
    rows, input_sha256, input_bytes = _read_jsonl(input_path)
    if len(rows) < 3:
        raise ReportError("input must contain run_start, samples, and run_end")
    start, end = rows[0], rows[-1]
    if start.get("record_type") != "run_start" or end.get("record_type") != "run_end":
        raise ReportError("record order must begin with run_start and end with run_end")
    keys(start, RUN_START_FIELDS, "run_start")
    keys(end, RUN_END_FIELDS, "run_end")
    if any(row.get("record_type") != "sample" for row in rows[1:-1]):
        raise ReportError("only sample records may appear between run_start and run_end")
    validate_identity(start, start, "run_start")
    validate_identity(end, start, "run_end")
    nonempty_string(start["run_id"], "run_start.run_id")
    nonempty_string(start["manifest"], "run_start.manifest")
    nonempty_string(start["snapshot_root"], "run_start.snapshot_root")
    validate_host(start["host"], "run_start.host")
    validate_runner_repo(start["runner_repo"], "run_start.runner_repo")
    exact_start = {
        "manifest_sha256": manifest.sha256,
        "manifest_cases": [case.id for case in manifest.cases],
        "selected_cases": [case.id for case in manifest.cases],
        "series_id": manifest.series_id,
        "suite_id": manifest.suite_id,
        "lane_id": manifest.lane_id,
        "protocol_id": manifest.protocol_id,
        "protocol_sha256": manifest.protocol_sha256,
        "profile": manifest.profile.__dict__,
        "provenance_status": "verified",
        "build_recipes": {
            identity: {
                key: list(value) if isinstance(value, tuple) else value
                for key, value in recipe.__dict__.items()
            }
            for identity, recipe in manifest.build_recipes.items()
        },
        "protocol_files": list(manifest.protocol_file_ids),
    }
    for field, value in exact_start.items():
        if start[field] != value:
            raise ReportError(f"run_start.{field}: identity or completeness mismatch")
    for field, expected in (
        ("portfolio_complete", True), ("comparison_input_preconditions_met", True),
        ("comparison_input_complete", False), ("claim_eligible", False),
    ):
        if boolean(start[field], f"run_start.{field}") is not expected:
            raise ReportError(f"run_start.{field}: invalid completeness state")
    blocks = integer(start["blocks"], "run_start.blocks", 1)
    seed = start["seed"]
    if isinstance(seed, bool) or not isinstance(seed, int):
        raise ReportError("run_start.seed: expected integer")
    empty_coverage = {
        "common": 0,
        "manifest_total": len(manifest.cases),
        "measured_by_role": {role: 0 for role in ("candidate", "base", "quickjs-ng")},
        "selected_total": len(manifest.cases),
    }
    validate_coverage(start["coverage"], empty_coverage, "run_start.coverage")
    if end["claim_eligible"] is not False or end["portfolio_complete"] is not True:
        raise ReportError("run_end: invalid claim or portfolio state")
    if end["provenance_status"] != "verified" or end["lane_id"] != manifest.lane_id:
        raise ReportError("run_end: provenance or lane identity mismatch")

    if not isinstance(start["engines"], list) or len(start["engines"]) != 3:
        raise ReportError("run_start.engines: expected exactly three roles")
    if [engine.get("role") for engine in start["engines"]] != [
        "candidate", "base", "quickjs-ng",
    ]:
        raise ReportError("run_start.engines: roles must be ordered candidate/base/quickjs-ng")
    engines = {}
    for index, engine in enumerate(start["engines"]):
        where = f"run_start.engines[{index}]"
        if not isinstance(engine, dict):
            raise ReportError(f"{where}: expected object")
        keys(engine, ENGINE_FIELDS, where)
        role = engine["role"]
        validate_engine(role, engine, manifest, where)
        engines[role] = engine
    if (
        engines["candidate"]["engine_identity"] != "qjs-rust"
        or engines["base"]["engine_identity"] != "qjs-rust"
        or engines["quickjs-ng"]["engine_identity"] != manifest.reference_identity
    ):
        raise ReportError("run_start.engines: engine identity mismatch")

    eligible_measurements: dict[tuple[str, str, int], float] = {}
    measurement_records: dict[tuple[str, str, int], dict[str, Any]] = {}
    measurement_reasons: dict[tuple[str, str, int], str] = {}
    measurement_rows = []
    measurement_orders: dict[int, set[int]] = defaultdict(set)
    diagnostic_rows: dict[tuple[str, str], list[tuple[dict[str, Any], str | None]]] = defaultdict(list)
    linearity: dict[tuple[str, str, str], dict[str, Any]] = {}
    startup: dict[tuple[str, str], list[int]] = defaultdict(list)
    seen_measurement = False
    phase_rank_by_case: dict[tuple[str, str], int] = defaultdict(lambda: -1)
    phase_ranks = {
        "startup": 0, "calibration": 1, "warmup": 2,
        "linearity_n": 3, "linearity_2n": 4, "measurement": 5,
    }
    failed_sample_seen = False
    for index, row in enumerate(rows[1:-1], 2):
        where = f"line {index}"
        case = validate_sample_common(row, start, manifest, where)
        role = row["role"]
        if role not in engines:
            raise ReportError(f"{where}.role: undeclared role")
        for field in ENGINE_FIELDS - {"role"}:
            if row[field] != engines[role][field]:
                raise ReportError(f"{where}.{field}: engine identity mismatch")
        phase = row["phase"]
        if phase not in phase_ranks:
            raise ReportError(f"{where}.phase: unknown phase {phase!r}")
        pair = (role, case.id)
        if phase_ranks[phase] < phase_rank_by_case[pair]:
            raise ReportError(f"{where}.phase: invalid phase order for {role}/{case.id}")
        phase_rank_by_case[pair] = phase_ranks[phase]

        if phase == "measurement":
            seen_measurement = True
            if row["diagnostic_point"] is not None:
                raise ReportError(f"{where}: measurement cannot have diagnostic point")
            block = integer(row["block"], f"{where}.block")
            order = integer(row["order"], f"{where}.order")
            if block >= blocks:
                raise ReportError(f"{where}: invalid block")
            if order in measurement_orders[block]:
                raise ReportError(f"{where}: duplicate order in block")
            measurement_orders[block].add(order)
            key = (role, case.id, block)
            if key in measurement_records:
                raise ReportError(f"{where}: duplicate measurement record {key}")
            measurement_records[key] = row
            measurement_rows.append((block, order, role, case.id))
            if row["status"] == "ok":
                validate_success(row, case, where)
                if row["quality"] == "eligible" and row["measurement_eligible"] is True:
                    eligible_measurements[key] = row["duration_ns"] / row["operations"]
                elif row["quality"] == "timer_limited" and row["measurement_eligible"] is False:
                    measurement_reasons[key] = "timer_limited"
                else:
                    raise ReportError(f"{where}: invalid successful measurement eligibility")
            else:
                reason = validate_non_success(row, case, where)
                measurement_reasons[key] = reason
                failed_sample_seen = True
        else:
            if seen_measurement:
                raise ReportError(f"{where}: diagnostic appears after measurement")
            if row["block"] is not None or row["order"] is not None:
                raise ReportError(f"{where}: diagnostic cannot have block/order")
            point = None
            if phase in {"linearity_n", "linearity_2n"}:
                point = "n" if phase == "linearity_n" else "2n"
                if row["diagnostic_point"] != point:
                    raise ReportError(f"{where}: invalid linearity point")
            elif row["diagnostic_point"] is not None:
                raise ReportError(f"{where}: setup cannot have diagnostic point")
            reason = None
            if row["status"] == "ok":
                validate_success(row, case, where)
                if row["quality"] != "diagnostic" or row["measurement_eligible"] is not False:
                    raise ReportError(f"{where}: invalid diagnostic eligibility")
                if phase == "startup" and row["iterations"] != 0:
                    raise ReportError(f"{where}.iterations: startup requires zero")
                if phase == "startup":
                    startup[pair].append(row["duration_ns"])
                if point is not None:
                    linearity_key = (role, case.id, point)
                    if linearity_key in linearity:
                        raise ReportError(f"{where}: duplicate linearity point {linearity_key}")
                    linearity[linearity_key] = row
            else:
                reason = validate_non_success(row, case, where)
                if reason == "not_run":
                    raise ReportError(f"{where}: runner never emits not_run diagnostics")
                failed_sample_seen = True
            diagnostic_rows[pair].append((row, reason))

    expected_plan = [
        (item.block, item.order, item.role, item.case_id)
        for item in measurement_plan(
            ["candidate", "base", "quickjs-ng"],
            [case.id for case in manifest.cases], blocks, seed,
        )
    ]
    if measurement_rows != expected_plan:
        raise ReportError("measurement records do not match the seeded physical plan order")
    expected_measurements = {
        (role, case.id, block)
        for role in engines for case in manifest.cases for block in range(blocks)
    }
    if set(measurement_records) != expected_measurements:
        raise ReportError("measurement portfolio mismatch")
    expected_orders = set(range(len(engines) * len(manifest.cases)))
    if any(measurement_orders[block] != expected_orders for block in range(blocks)):
        raise ReportError("measurement order portfolio is incomplete or invalid")

    diagnostic_failures = []
    completed_pairs: set[tuple[str, str]] = set()
    for role in ("candidate", "base", "quickjs-ng"):
        for case in manifest.cases:
            pair = (role, case.id)
            samples = diagnostic_rows[pair]
            if not samples:
                raise ReportError(f"diagnostic portfolio silently missing for {role}/{case.id}")
            completed, formal_iterations, replayed_startup, failure = _replay_setup(
                samples, case, role
            )
            if replayed_startup != tuple(startup[pair]):
                raise ReportError(f"startup replay mismatch for {role}/{case.id}")
            if failure is not None:
                diagnostic_failures.append(failure)
            if completed:
                completed_pairs.add(pair)

            pair_measurements = [
                measurement_records[(role, case.id, block)] for block in range(blocks)
            ]
            if completed:
                if any(row["iterations"] != formal_iterations for row in pair_measurements):
                    raise ReportError(f"measurement iterations mismatch for {role}/{case.id}")
                startup_ns = statistics.median(startup[pair])
                for block, row in enumerate(pair_measurements):
                    if row["status"] != "ok":
                        continue
                    timing_good = (
                        row["duration_ns"] >= case.min_window_ms * 1_000_000
                        and Fraction(startup_ns, row["duration_ns"])
                        <= case.startup_max_fraction
                    )
                    if timing_good != (row["quality"] == "eligible"):
                        raise ReportError(
                            f"measurement timing quality mismatch for {role}/{case.id}/{block}"
                        )
            elif any(row["status"] != "not_run" for row in pair_measurements):
                raise ReportError(f"failed setup must make every measurement not_run for {role}/{case.id}")

    completed_by_role = {
        role: {
            case.id for case in manifest.cases
            if all((role, case.id, block) in eligible_measurements for block in range(blocks))
        }
        for role in ROLE_SET
    }
    common = set.intersection(*completed_by_role.values())
    recomputed_coverage = {
        "common": len(common),
        "manifest_total": len(manifest.cases),
        "measured_by_role": {
            role: len(completed_by_role[role])
            for role in ("candidate", "base", "quickjs-ng")
        },
        "selected_total": len(manifest.cases),
    }
    validate_coverage(end["coverage"], recomputed_coverage, "run_end.coverage")
    comparison_complete = (
        set(eligible_measurements) == expected_measurements
        and len(completed_pairs) == len(engines) * len(manifest.cases)
        and not failed_sample_seen
    )
    if end["comparison_input_complete"] is not comparison_complete:
        raise ReportError("run_end.comparison_input_complete: does not match records")
    expected_end_status = "failed" if failed_sample_seen else "complete"
    if end["status"] != expected_end_status:
        raise ReportError("run_end.status: does not match durable failures")

    invalid_blocks = []
    valid_blocks = []
    for block in range(blocks):
        triggers = []
        for case in manifest.cases:
            for role in ("candidate", "base", "quickjs-ng"):
                key = (role, case.id, block)
                if key not in eligible_measurements:
                    row = measurement_records[key]
                    triggers.append({
                        "case_id": case.id,
                        "role": role,
                        "status": row["status"],
                        "quality": row["quality"],
                        "reason": measurement_reasons[key],
                    })
        if triggers:
            invalid_blocks.append({"block": block, "triggers": triggers})
        else:
            valid_blocks.append(block)
    valid_block_set = set(valid_blocks)
    filtered_measurements = {
        key: value for key, value in eligible_measurements.items()
        if key[2] in valid_block_set
    }
    return ValidatedRun(
        input_sha256=input_sha256,
        input_bytes=input_bytes,
        start=start,
        engines=engines,
        blocks=blocks,
        measurements=filtered_measurements,
        measurement_records=measurement_records,
        valid_blocks=tuple(valid_blocks),
        invalid_blocks=tuple(invalid_blocks),
        linearity=linearity,
        startup={key: tuple(values) for key, values in startup.items()},
        diagnostic_failures=tuple(diagnostic_failures),
        runner_end_status=end["status"],
        comparison_input_complete=end["comparison_input_complete"],
        runner_coverage=end["coverage"],
    )
