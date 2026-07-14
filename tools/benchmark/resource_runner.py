"""Execute one independently selected resource lane without building engines."""

from __future__ import annotations

import json
import platform
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable, TextIO
from uuid import uuid4

from .adapters import Engine
from .planning import measurement_plan
from .records import parse_result
from .resource_process import ResourceProcessResult, run_process_wait4
from .resource_schema import ResourceLane, ResourceManifest, sha256_file
from .resource_wall_process import ResourceWallResult, run_fresh_process
from .snapshots import SnapshotStore


class ResourceRunError(ValueError):
    """A resource run cannot produce evidence under the selected contract."""


def _runner_repo(root: Path) -> dict[str, Any]:
    try:
        commit = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "HEAD"], capture_output=True,
            text=True, timeout=5, check=False,
        )
        dirty = subprocess.run(
            ["git", "-C", str(root), "status", "--porcelain"], capture_output=True,
            text=True, timeout=5, check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return {"commit": None, "dirty": None}
    return {
        "commit": commit.stdout.strip() if commit.returncode == 0 else None,
        "dirty": bool(dirty.stdout) if dirty.returncode == 0 else None,
    }


def _host(machine_name: str) -> dict[str, str]:
    return {
        "machine": machine_name, "node": platform.node(),
        "platform": platform.platform(), "processor": platform.processor(),
        "python": platform.python_version(), "system": platform.system(),
    }


class ResourceJsonlWriter:
    def __init__(self, handle: TextIO):
        self.handle = handle

    def write(self, value: dict[str, Any]) -> None:
        self.handle.write(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")
        self.handle.flush()


class ResourceRun:
    def __init__(
        self,
        manifest: ResourceManifest,
        lane: ResourceLane,
        engines: list[Engine],
        blocks: int,
        seed: int,
        writer: ResourceJsonlWriter,
        root: Path,
        *,
        platform_name: str | None = None,
        machine_name: str | None = None,
        wall_runner: Callable[[list[str], int], ResourceWallResult] = run_fresh_process,
        rss_runner: Callable[..., ResourceProcessResult] = run_process_wait4,
    ):
        self.manifest = manifest
        self.lane = lane
        self.engines = {engine.role: engine for engine in engines}
        if len(self.engines) != len(engines) or not self.engines:
            raise ResourceRunError("resource run requires unique non-empty roles")
        self.blocks = blocks
        self.seed = seed
        self.writer = writer
        self.root = root
        self.platform_name = platform_name or sys.platform
        self.machine_name = machine_name or platform.machine()
        if self.platform_name != manifest.profile.platform:
            raise ResourceRunError(
                f"host platform {self.platform_name!r} does not match profile "
                f"{manifest.profile.platform!r}"
            )
        if self.machine_name != manifest.profile.machine:
            raise ResourceRunError(
                f"host machine {self.machine_name!r} does not match profile "
                f"{manifest.profile.machine!r}"
            )
        expected_identity = {
            "candidate": "qjs-rust", "base": "qjs-rust",
            "quickjs-ng": manifest.reference_identity,
        }
        for engine in engines:
            if engine.role not in expected_identity:
                raise ResourceRunError(f"unknown resource role {engine.role!r}")
            if engine.engine_identity != expected_identity[engine.role]:
                raise ResourceRunError(
                    f"{engine.role}: identity must be {expected_identity[engine.role]!r}"
                )
        if lane.kind == "dynamic" and blocks not in {lane.initial_blocks, lane.max_blocks}:
            raise ResourceRunError("dynamic resource lane requires exact 30 or 60 blocks")
        if lane.kind == "dynamic" and seed != lane.seed:
            raise ResourceRunError("dynamic resource lane requires the manifest seed")
        if lane.kind == "static" and blocks != 1:
            raise ResourceRunError("binary-size lane requires exactly one sample per role")
        self.wall_runner = wall_runner
        self.rss_runner = rss_runner
        self.run_id = str(uuid4())
        self.host = _host(self.machine_name)
        self.runner_repo = _runner_repo(root)
        self.snapshot_store = SnapshotStore(root, self.run_id)
        self.failed = False
        self._closed = False
        try:
            self.engine_snapshots = {
                engine.role: self.snapshot_store.snapshot_engine(
                    engine.role, engine.binary, engine.binary_sha256
                )
                for engine in engines
            }
            self.workload_snapshot = None
            if lane.case is not None:
                self.workload_snapshot = self.snapshot_store.snapshot_workload(
                    lane.case.workload, lane.case.workload_sha256
                )
        except Exception:
            self.snapshot_store.cleanup()
            raise
        self.roles_complete = set(self.engines) == {"candidate", "base", "quickjs-ng"}
        self.provenance_verified = all(
            engine.provenance_status == "verified" for engine in self.engines.values()
        )
        self.preconditions = self.roles_complete and self.provenance_verified

    def _engine_fields(self, engine: Engine) -> dict[str, Any]:
        receipt = engine.receipt
        return {
            "adapter_id": engine.adapter_id,
            "binary_sha256": engine.binary_sha256,
            "binary_snapshot_path": str(self.engine_snapshots[engine.role]),
            "binary_source_path": str(engine.binary),
            "engine_identity": engine.engine_identity,
            "provenance_status": engine.provenance_status,
            "receipt": receipt.content if receipt is not None else None,
            "receipt_sha256": receipt.sha256 if receipt is not None else None,
        }

    def _sample_base(
        self, engine: Engine, *, block: int | None, order: int, argv: list[str]
    ) -> dict[str, Any]:
        case = self.lane.case
        result = {
            "argv": argv,
            "block": block,
            "case_id": case.id if case else None,
            "checksum": None,
            "duration_ns": None,
            "descendants_detected": False,
            "error": None,
            "exit_code": None,
            "host": self.host,
            "iterations": case.iterations if case else None,
            "lane_id": self.lane.id,
            "manifest_sha256": self.manifest.sha256,
            "measurement_eligible": False,
            "metric": self.lane.metric,
            "monitor_error": None,
            "operations": None,
            "order": order,
            "phase": "measurement",
            "profile_id": self.manifest.profile.id,
            "protocol_id": self.manifest.protocol_id,
            "protocol_sha256": self.manifest.protocol_sha256,
            "raw_rss": None,
            "raw_rss_unit": None,
            "record_type": "resource_sample",
            "role": engine.role,
            "run_id": self.run_id,
            "runner_repo": self.runner_repo,
            "sample_id": str(uuid4()),
            "schema_version": 1,
            "series_id": self.manifest.series_id,
            "started_at": None,
            "status": "invalid",
            "stderr": "",
            "stderr_truncated": False,
            "stdout": "",
            "stdout_truncated": False,
            "suite_id": self.manifest.suite_id,
            "timed_out": False,
            "timer": None,
            "timer_phase_boundary": None,
            "unit": self.lane.unit,
            "value": None,
            "workload_sha256": case.workload_sha256 if case else None,
            "workload_snapshot_path": str(self.workload_snapshot) if case else None,
            "workload_source_path": str(case.workload) if case else None,
        }
        result.update(self._engine_fields(engine))
        return result

    def _validate_output(
        self, record: dict[str, Any], process: ResourceWallResult | ResourceProcessResult
    ) -> None:
        case = self.lane.case
        if case is None:
            raise AssertionError("dynamic sample requires a case")
        if getattr(process, "monitor_error", None) is not None:
            record["status"] = "failed"
            record["error"] = f"resource monitor failed: {process.monitor_error}"
        elif process.timed_out:
            record["status"] = "timeout"
            record["error"] = f"timed out after {case.timeout_seconds}s"
        elif process.exit_code is None:
            record["status"] = "failed"
            record["error"] = f"engine could not start: {process.stderr}"
        elif process.exit_code != 0:
            record["status"] = "failed"
            record["error"] = f"engine exited with status {process.exit_code}"
        elif process.stdout_truncated or process.stderr_truncated:
            record["status"] = "invalid"
            record["error"] = "process output exceeded validation limit"
        elif getattr(process, "descendants_detected", False):
            record["status"] = "invalid"
            record["error"] = "engine spawned descendant processes"
        else:
            try:
                parsed = parse_result(process.stdout)
                expected = {
                    "case_id": case.id,
                    "iterations": case.iterations,
                    "operations": case.operations,
                    "checksum": case.expected_checksum(),
                }
                if parsed != expected:
                    raise ValueError(f"result mismatch: expected {expected}, got {parsed}")
                record["operations"] = parsed["operations"]
                record["checksum"] = parsed["checksum"]
                record["status"] = "ok"
            except (ValueError, json.JSONDecodeError) as error:
                record["status"] = "invalid"
                record["error"] = str(error)
        if record["status"] != "ok":
            self.failed = True
        record["measurement_eligible"] = record["status"] == "ok"

    def _dynamic_sample(self, engine: Engine, block: int, order: int) -> dict[str, Any]:
        case = self.lane.case
        if case is None or self.workload_snapshot is None:
            raise AssertionError("dynamic lane is missing workload snapshot")
        argv = engine.command(
            self.workload_snapshot, case.id, case.iterations,
            binary=self.engine_snapshots[engine.role],
        )
        record = self._sample_base(engine, block=block, order=order, argv=argv)
        if self.lane.id == "fresh_process_latency/wall_ns_per_process":
            process = self.wall_runner(argv, case.timeout_seconds)
            record["timer"] = "python.perf_counter_ns"
            record["timer_phase_boundary"] = "before_process_spawn_to_wait_return"
            record["value"] = process.duration_ns
        elif self.lane.id == "peak_rss/bytes":
            process = self.rss_runner(
                argv, case.timeout_seconds, platform_name=self.platform_name
            )
            record["raw_rss"] = process.peak_rss_raw
            record["raw_rss_unit"] = self.manifest.profile.rss_raw_unit
            record["value"] = process.peak_rss_bytes
            record["descendants_detected"] = process.descendants_detected
        else:
            raise AssertionError("unknown dynamic resource lane")
        for field in (
            "descendants_detected", "duration_ns", "exit_code", "monitor_error",
            "started_at", "stderr", "stderr_truncated",
            "stdout", "stdout_truncated", "timed_out",
        ):
            default = False if field == "descendants_detected" else None
            record[field] = getattr(process, field, default)
        self._validate_output(record, process)
        if record["status"] != "ok":
            record["value"] = None
            if self.lane.id == "peak_rss/bytes":
                record["raw_rss"] = process.peak_rss_raw
        return record

    def _size_sample(self, engine: Engine, order: int) -> dict[str, Any]:
        record = self._sample_base(engine, block=None, order=order, argv=[])
        try:
            if sha256_file(self.engine_snapshots[engine.role]) != engine.binary_sha256:
                raise ResourceRunError("binary snapshot hash changed before size measurement")
            value = self.engine_snapshots[engine.role].stat().st_size
            if value <= 0:
                raise ResourceRunError("binary snapshot has non-positive logical size")
            record.update({"measurement_eligible": True, "status": "ok", "value": value})
        except (OSError, ResourceRunError) as error:
            record.update({"status": "failed", "error": str(error)})
            self.failed = True
        return record

    def _plan(self) -> list[tuple[int | None, int, str]]:
        if self.lane.kind == "static":
            roles = [role for role in ("candidate", "base", "quickjs-ng") if role in self.engines]
            return [(None, order, role) for order, role in enumerate(roles)]
        case = self.lane.case
        if case is None:
            raise AssertionError("dynamic lane is missing case")
        return [
            (item.block, item.order, item.role)
            for item in measurement_plan(
                list(self.engines), [case.id], self.blocks, self.seed
            )
        ]

    def execute(self) -> bool:
        try:
            plan = self._plan()
            self.writer.write({
                "blocks": self.blocks,
                "build_recipes": {
                    identity: recipe.__dict__
                    for identity, recipe in self.manifest.build_recipes.items()
                },
                "claim_eligible": False,
                "comparison_input_complete": False,
                "comparison_input_preconditions_met": self.preconditions,
                "engines": [
                    {"role": engine.role, **self._engine_fields(engine)}
                    for engine in self.engines.values()
                ],
                "expected_samples": len(plan),
                "host": self.host,
                "lane_id": self.lane.id,
                "manifest_sha256": self.manifest.sha256,
                "profile": self.manifest.profile.__dict__,
                "protocol_files": list(self.manifest.protocol_file_ids),
                "protocol_id": self.manifest.protocol_id,
                "protocol_sha256": self.manifest.protocol_sha256,
                "record_type": "resource_run_start",
                "run_id": self.run_id,
                "runner_repo": self.runner_repo,
                "schema_version": 1,
                "seed": self.seed if self.lane.kind == "dynamic" else None,
                "series_id": self.manifest.series_id,
                "snapshot_root": str(self.snapshot_store.path),
                "suite_id": self.manifest.suite_id,
            })
            valid = 0
            by_role = {role: 0 for role in self.engines}
            for block, order, role in plan:
                engine = self.engines[role]
                record = (
                    self._size_sample(engine, order)
                    if self.lane.kind == "static"
                    else self._dynamic_sample(engine, int(block), order)
                )
                self.writer.write(record)
                if record["status"] == "ok":
                    valid += 1
                    by_role[role] += 1
            complete = self.preconditions and not self.failed and valid == len(plan)
            self.writer.write({
                "claim_eligible": False,
                "comparison_input_complete": complete,
                "coverage": {
                    "attempted": len(plan), "planned": len(plan), "valid": valid,
                    "valid_by_role": by_role,
                },
                "lane_id": self.lane.id,
                "physical_plan_complete": True,
                "record_type": "resource_run_end",
                "run_id": self.run_id,
                "schema_version": 1,
                "status": "failed" if self.failed else "complete",
            })
            return not self.failed
        finally:
            self.close()

    def close(self) -> None:
        if not self._closed:
            self.snapshot_store.cleanup()
            self._closed = True

    def __del__(self) -> None:
        if hasattr(self, "snapshot_store"):
            self.close()
