"""Execute traceable black-box benchmark samples without building engines."""

from __future__ import annotations

import json
import platform
import statistics
import subprocess
from collections import defaultdict
from pathlib import Path
from typing import Any, TextIO
from uuid import uuid4

from .adapters import Engine, probe_version
from .planning import measurement_plan
from .process import ProcessResult, run_process
from .records import parse_result
from .schema import Case, Manifest, sha256_file
from .snapshots import SnapshotStore

def _runner_repo_metadata(root: Path) -> dict[str, Any]:
    def command(*args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", "-C", str(root), *args], capture_output=True, text=True, timeout=5, check=False
        )

    try:
        commit_result = command("rev-parse", "HEAD")
        dirty_result = command("status", "--porcelain")
    except (OSError, subprocess.TimeoutExpired):
        return {"commit": None, "dirty": None}
    commit = commit_result.stdout.strip() if commit_result.returncode == 0 else None
    dirty = bool(dirty_result.stdout) if dirty_result.returncode == 0 else None
    return {"commit": commit, "dirty": dirty}


def _host_metadata() -> dict[str, str]:
    return {
        "machine": platform.machine(),
        "node": platform.node(),
        "platform": platform.platform(),
        "processor": platform.processor(),
        "python": platform.python_version(),
        "system": platform.system(),
    }


class JsonlWriter:
    def __init__(self, handle: TextIO):
        self.handle = handle

    def write(self, value: dict[str, Any]) -> None:
        self.handle.write(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")
        self.handle.flush()


class BenchmarkRun:
    def __init__(
        self,
        manifest: Manifest,
        engines: list[Engine],
        cases: list[Case],
        blocks: int,
        seed: int,
        writer: JsonlWriter,
        root: Path,
    ):
        self.manifest = manifest
        self.engines = {engine.role: engine for engine in engines}
        self.cases = {case.id: case for case in cases}
        self.blocks = blocks
        self.seed = seed
        self.writer = writer
        self.run_id = str(uuid4())
        self.runner_repo = _runner_repo_metadata(root)
        self.host = _host_metadata()
        self.snapshot_store = SnapshotStore(root, self.run_id)
        self._closed = False
        try:
            # Probe disposable copies before measurement copies exist. A
            # version handler may rewrite its own executable; measurement can
            # never execute that probe copy, and the later source-to-measurement
            # copy still has to match the originally validated hash.
            self.engine_probe_snapshots = {
                engine.role: self.snapshot_store.snapshot_version_probe(
                    engine.role, engine.binary, engine.binary_sha256
                )
                for engine in self.engines.values()
            }
            self.engine_versions = {
                role: probe_version(snapshot)
                for role, snapshot in self.engine_probe_snapshots.items()
            }
            self.engine_probe_post_sha256 = {
                role: self._snapshot_sha256_or_none(snapshot)
                for role, snapshot in self.engine_probe_snapshots.items()
            }
            self.engine_snapshots = {
                engine.role: self.snapshot_store.snapshot_engine(
                    engine.role, engine.binary, engine.binary_sha256
                )
                for engine in self.engines.values()
            }
            self.workload_snapshots = {}
            for case in self.cases.values():
                if case.workload not in self.workload_snapshots:
                    self.workload_snapshots[case.workload] = self.snapshot_store.snapshot_workload(
                        case.workload, case.workload_sha256
                    )
        except Exception:
            self.snapshot_store.cleanup()
            raise
        self.iterations: dict[tuple[str, str], int] = {}
        self.startup_ns: dict[tuple[str, str], int] = {}
        self.measurement_counts: dict[tuple[str, str], int] = defaultdict(int)
        self.linearity_counts: dict[tuple[str, str], int] = defaultdict(int)
        self.failed = False
        self.portfolio_complete = tuple(self.cases) == tuple(case.id for case in manifest.cases)
        self.provenance_verified = all(
            engine.provenance_status == "verified" for engine in self.engines.values()
        )
        self.comparison_input_preconditions_met = (
            set(self.engines) == {"candidate", "base", "quickjs-ng"}
            and self.portfolio_complete
            and self.provenance_verified
        )

    @staticmethod
    def _snapshot_sha256_or_none(path: Path) -> str | None:
        try:
            return sha256_file(path)
        except OSError:
            # Version metadata is bounded best-effort and never blocks the
            # benchmark. The measurement copy is verified independently.
            return None

    def _engine_fields(self, engine: Engine) -> dict[str, Any]:
        receipt = engine.receipt
        return {
            "binary_source_path": str(engine.binary),
            "binary_snapshot_path": str(self.engine_snapshots[engine.role]),
            "binary_sha256": engine.binary_sha256,
            "binary_version": self.engine_versions[engine.role],
            "binary_version_probe_snapshot_path": str(
                self.engine_probe_snapshots[engine.role]
            ),
            "binary_version_probe_pre_sha256": engine.binary_sha256,
            "binary_version_probe_post_sha256": self.engine_probe_post_sha256[
                engine.role
            ],
            "adapter_id": engine.adapter_id,
            "engine_identity": engine.engine_identity,
            "provenance_status": engine.provenance_status,
            "receipt": receipt.content if receipt is not None else None,
            "receipt_sha256": receipt.sha256 if receipt is not None else None,
        }

    def _sample_record(
        self,
        engine: Engine,
        case: Case,
        *,
        iterations: int | None,
        phase: str,
        block: int | None,
        order: int | None,
        result: ProcessResult | None,
        status: str,
        quality: str,
        operations: int | None,
        checksum: int | None,
        error: str | None,
        argv: list[str],
        diagnostic_point: str | None = None,
    ) -> dict[str, Any]:
        record = {
            "argv": argv,
            "block": block,
            "case_id": case.id,
            "checksum": checksum,
            "measurement_eligible": (
                phase == "measurement" and status == "ok" and quality == "eligible"
            ),
            "duration_ns": result.duration_ns if result is not None else None,
            "diagnostic_point": diagnostic_point,
            "error": error,
            "exit_code": result.exit_code if result is not None else None,
            "family": case.family,
            "host": self.host,
            "iterations": iterations,
            "lane_id": self.manifest.lane_id,
            "manifest_sha256": self.manifest.sha256,
            "metric": "outer_wall_time",
            "operations": operations,
            "order": order,
            "phase": phase,
            "profile_id": self.manifest.profile.id,
            "protocol_id": self.manifest.protocol_id,
            "protocol_sha256": self.manifest.protocol_sha256,
            "quality": quality,
            "record_type": "sample",
            "role": engine.role,
            "run_id": self.run_id,
            "runner_repo": self.runner_repo,
            "schema_version": 4,
            "series_id": self.manifest.series_id,
            "started_at": result.started_at if result is not None else None,
            "status": status,
            "stderr": result.stderr if result is not None else "",
            "stderr_truncated": result.stderr_truncated if result is not None else False,
            "stdout": result.stdout if result is not None else "",
            "stdout_truncated": result.stdout_truncated if result is not None else False,
            "suite_id": self.manifest.suite_id,
            "timed_out": result.timed_out if result is not None else False,
            "timer": "python.perf_counter_ns",
            "timer_phase_boundary": "before_process_spawn_to_wait_return",
            "workload_source_path": str(case.workload),
            "workload_snapshot_path": str(self.workload_snapshots[case.workload]),
            "workload_sha256": case.workload_sha256,
        }
        record.update(self._engine_fields(engine))
        return record

    def _sample(
        self,
        engine: Engine,
        case: Case,
        iterations: int,
        phase: str,
        block: int | None,
        order: int | None,
        quality: str,
        startup_ns: int | None = None,
        diagnostic_point: str | None = None,
    ) -> tuple[ProcessResult, str, str]:
        argv = engine.command(
            self.workload_snapshots[case.workload],
            case.id,
            iterations,
            binary=self.engine_snapshots[engine.role],
        )
        result = run_process(argv, case.timeout_seconds)
        status = "ok"
        error: str | None = None
        operations: int | None = None
        checksum: int | None = None
        if result.timed_out:
            status = "timeout"
            error = f"timed out after {case.timeout_seconds}s"
        elif result.exit_code is None:
            status = "failed"
            error = f"engine could not start: {result.stderr}"
        elif result.exit_code != 0:
            status = "failed"
            error = f"engine exited with status {result.exit_code}"
        elif result.stdout_truncated or result.stderr_truncated:
            status = "invalid"
            streams = []
            if result.stdout_truncated:
                streams.append("stdout")
            if result.stderr_truncated:
                streams.append("stderr")
            error = f"{' and '.join(streams)} exceeded validation limit"
        else:
            try:
                parsed = parse_result(result.stdout)
                if parsed["case_id"] != case.id or parsed["iterations"] != iterations:
                    raise ValueError("result identity does not match invocation")
                operations = parsed["operations"]
                checksum = parsed["checksum"]
                if isinstance(operations, bool) or not isinstance(operations, int):
                    raise ValueError("operations must be an integer")
                if isinstance(checksum, bool) or not isinstance(checksum, int):
                    raise ValueError("checksum must be an integer")
                if operations != case.expected_operations(iterations):
                    raise ValueError(
                        f"operations mismatch: expected {case.expected_operations(iterations)}, got {operations}"
                    )
                if checksum != case.expected_checksum(iterations):
                    raise ValueError(
                        f"checksum mismatch: expected {case.expected_checksum(iterations)}, got {checksum}"
                    )
            except (ValueError, json.JSONDecodeError) as parse_error:
                status = "invalid"
                error = str(parse_error)
        if status != "ok":
            quality = "ineligible"
            self.failed = True
        elif startup_ns is not None and (
            result.duration_ns < case.min_window_ms * 1_000_000
            or startup_ns / max(1, result.duration_ns) > case.startup_max_fraction
        ):
            quality = "timer_limited"
        if phase == "measurement" and status == "ok" and quality == "eligible":
            self.measurement_counts[(engine.role, case.id)] += 1
        if phase in {"linearity_n", "linearity_2n"} and status == "ok":
            self.linearity_counts[(engine.role, case.id)] += 1
        self.writer.write(self._sample_record(
            engine,
            case,
            iterations=iterations,
            phase=phase,
            block=block,
            order=order,
            result=result,
            status=status,
            quality=quality,
            operations=operations,
            checksum=checksum,
            error=error,
            argv=argv,
            diagnostic_point=diagnostic_point,
        ))
        return result, status, quality

    def _not_run(self, engine: Engine, case: Case, block: int, order: int) -> None:
        self.writer.write(self._sample_record(
            engine,
            case,
            iterations=None,
            phase="measurement",
            block=block,
            order=order,
            result=None,
            status="not_run",
            quality="ineligible",
            operations=None,
            checksum=None,
            error="startup/calibration/warmup did not complete",
            argv=[],
        ))
        self.failed = True

    def _calibrate(self, engine: Engine, case: Case) -> bool:
        key = (engine.role, case.id)
        startup_durations = []
        for _ in range(3):
            result, status, _quality = self._sample(
                engine, case, 0, "startup", None, None, "diagnostic"
            )
            if status != "ok":
                return False
            startup_durations.append(result.duration_ns)
        startup_ns = int(statistics.median(startup_durations))
        target_ns = max(case.min_window_ms * 1_000_000, int(startup_ns / case.startup_max_fraction))
        iterations = case.initial_iterations
        while True:
            result, status, _quality = self._sample(
                engine, case, iterations, "calibration", None, None, "diagnostic"
            )
            if status != "ok":
                return False
            if result.duration_ns >= target_ns or iterations >= case.max_iterations:
                break
            scale = max(2, min(16, int(target_ns / max(1, result.duration_ns)) + 1))
            iterations = min(case.max_iterations, iterations * scale)
        for _ in range(case.warmup_runs):
            _result, status, _quality = self._sample(
                engine, case, iterations, "warmup", None, None, "diagnostic"
            )
            if status != "ok":
                return False
        # Use dedicated samples rather than reinterpreting measurement blocks.
        # N is chosen after calibration and constrained so that 2N is always
        # exact and never exceeds the manifest maximum.
        diagnostic_n = max(1, min(iterations, case.max_iterations // 2))
        for diagnostic_iterations, phase, point in (
            (diagnostic_n, "linearity_n", "n"),
            (diagnostic_n * 2, "linearity_2n", "2n"),
        ):
            _result, status, _quality = self._sample(
                engine,
                case,
                diagnostic_iterations,
                phase,
                None,
                None,
                "diagnostic",
                diagnostic_point=point,
            )
            if status != "ok":
                return False
        # Commit calibration only after every prerequisite succeeds.
        self.startup_ns[key] = startup_ns
        self.iterations[key] = iterations
        return True

    def _coverage(self) -> dict[str, Any]:
        measured_by_role = {}
        complete_sets = []
        for role in self.engines:
            complete = {
                case_id
                for case_id in self.cases
                if self.measurement_counts[(role, case_id)] == self.blocks
            }
            complete_sets.append(complete)
            measured_by_role[role] = len(complete)
        common = set.intersection(*complete_sets) if complete_sets else set()
        return {
            "common": len(common),
            "manifest_total": len(self.manifest.cases),
            "measured_by_role": measured_by_role,
            "selected_total": len(self.cases),
        }

    def _comparison_input_complete(self) -> bool:
        coverage = self._coverage()
        measurements_complete = (
            coverage["common"] == len(self.cases)
            and all(count == len(self.cases) for count in coverage["measured_by_role"].values())
        )
        linearity_complete = all(
            self.linearity_counts[(role, case_id)] == 2
            for role in self.engines
            for case_id in self.cases
        )
        return (
            self.comparison_input_preconditions_met
            and measurements_complete
            and linearity_complete
            and not self.failed
        )

    def execute(self) -> bool:
        try:
            return self._execute()
        finally:
            self.close()

    def close(self) -> None:
        if not self._closed:
            self.snapshot_store.cleanup()
            self._closed = True

    def __del__(self) -> None:
        if hasattr(self, "snapshot_store"):
            self.close()

    def _execute(self) -> bool:
        empty_coverage = {
            "common": 0,
            "manifest_total": len(self.manifest.cases),
            "measured_by_role": {role: 0 for role in self.engines},
            "selected_total": len(self.cases),
        }
        self.writer.write({
            "blocks": self.blocks,
            "build_recipes": {
                identity: recipe.__dict__
                for identity, recipe in self.manifest.build_recipes.items()
            },
            "claim_eligible": False,
            "comparison_input_complete": False,
            "comparison_input_preconditions_met": self.comparison_input_preconditions_met,
            "coverage": empty_coverage,
            "engines": [
                {"role": engine.role, **self._engine_fields(engine)}
                for engine in self.engines.values()
            ],
            "host": self.host,
            "manifest": str(self.manifest.path),
            "manifest_cases": [case.id for case in self.manifest.cases],
            "manifest_sha256": self.manifest.sha256,
            "lane_id": self.manifest.lane_id,
            "portfolio_complete": self.portfolio_complete,
            "profile": self.manifest.profile.__dict__,
            "protocol_files": list(self.manifest.protocol_file_ids),
            "protocol_id": self.manifest.protocol_id,
            "protocol_sha256": self.manifest.protocol_sha256,
            "provenance_status": "verified" if self.provenance_verified else "unverified",
            "record_type": "run_start",
            "run_id": self.run_id,
            "runner_repo": self.runner_repo,
            "snapshot_root": str(self.snapshot_store.path),
            "schema_version": 4,
            "seed": self.seed,
            "selected_cases": list(self.cases),
            "series_id": self.manifest.series_id,
            "suite_id": self.manifest.suite_id,
        })
        for engine in self.engines.values():
            for case in self.cases.values():
                self._calibrate(engine, case)
        plan = measurement_plan(list(self.engines), list(self.cases), self.blocks, self.seed)
        for item in plan:
            engine = self.engines[item.role]
            case = self.cases[item.case_id]
            key = (engine.role, case.id)
            if key not in self.iterations or key not in self.startup_ns:
                self._not_run(engine, case, item.block, item.order)
                continue
            self._sample(
                engine,
                case,
                self.iterations[key],
                "measurement",
                item.block,
                item.order,
                "eligible",
                self.startup_ns[key],
            )
        coverage = self._coverage()
        comparison_input_complete = self._comparison_input_complete()
        self.writer.write({
            "claim_eligible": False,
            "comparison_input_complete": comparison_input_complete,
            "coverage": coverage,
            "portfolio_complete": self.portfolio_complete,
            "lane_id": self.manifest.lane_id,
            "provenance_status": "verified" if self.provenance_verified else "unverified",
            "record_type": "run_end",
            "run_id": self.run_id,
            "schema_version": 4,
            "status": "failed" if self.failed else "complete",
        })
        return not self.failed
