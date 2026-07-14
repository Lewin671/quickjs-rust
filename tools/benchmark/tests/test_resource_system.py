from __future__ import annotations

import hashlib
import io
import json
import os
import shutil
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

from tools.benchmark.adapters import Engine
from tools.benchmark.process import ProcessResult
from tools.benchmark.receipts import BuildReceipt, canonical_receipt_sha256
from tools.benchmark.resource_analysis import _block_health, _precision
from tools.benchmark.resource_analysis_schema import load_resource_analysis
from tools.benchmark.resource_artifact import build_resource_report
from tools.benchmark.resource_process import ResourceProcessResult
from tools.benchmark.resource_report import write_resource_report
from tools.benchmark.resource_runner import ResourceJsonlWriter, ResourceRun
from tools.benchmark.resource_runner import ResourceRunError
from tools.benchmark.resource_schema import load_resource_manifest
from tools.benchmark.resource_validation import ResourceReportError, validate_resource_run
from tools.benchmark.snapshots import SnapshotError


ROOT = Path(__file__).resolve().parents[3]


class ResourceSystemTests(unittest.TestCase):
    def setUp(self) -> None:
        self.manifest = load_resource_manifest(ROOT / "benchmarks/resources.json")
        self.analysis = load_resource_analysis(
            ROOT / "benchmarks/resource-analysis.json", self.manifest
        )
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.temp = Path(self.temporary.name)

    def _engine(
        self, role: str, size: int, *, verified: bool = True
    ) -> Engine:
        identity = "quickjs-ng" if role == "quickjs-ng" else "qjs-rust"
        prefix = b"#!/bin/sh\nexit 0\n"
        binary = self.temp / f"{role}.bin"
        binary.write_bytes(prefix + b"#" * (size - len(prefix)))
        binary.chmod(0o700)
        digest = hashlib.sha256(binary.read_bytes()).hexdigest()
        receipt = None
        if verified:
            recipe = self.manifest.build_recipes[identity]
            content = {
                "schema_version": 1,
                "engine_identity": identity,
                "source": {
                    "repo": self.manifest.reference_repo if role == "quickjs-ng"
                    else "https://example.invalid/quickjs-rust.git",
                    "revision": self.manifest.reference_revision if role == "quickjs-ng"
                    else ({"candidate": "a", "base": "b"}[role] * 40),
                    "dirty": False,
                },
                "profile_id": self.manifest.profile.id,
                "build": {
                    key: list(value) if isinstance(value, tuple) else value
                    for key, value in recipe.__dict__.items()
                    if key != "engine_identity"
                },
                "binary_sha256": digest,
            }
            receipt = BuildReceipt(
                path=self.temp / f"{role}.receipt.json",
                sha256=canonical_receipt_sha256(content), content=content,
                engine_identity=identity, source_repo=content["source"]["repo"],
                source_revision=content["source"]["revision"], source_dirty=False,
                profile_id=self.manifest.profile.id, binary_sha256=digest,
            )
        return Engine(role, "qjs-file" if role == "quickjs-ng" else "qjs-rust-raw",
                      identity, binary, digest, receipt)

    @staticmethod
    def _stdout(argv: list[str]) -> str:
        case_id = argv[-2]
        iterations = int(argv[-1])
        factor = 7 if case_id == "fresh_process_probe" else 3
        return "QJS_BENCH_RESULT " + json.dumps({
            "case_id": case_id, "iterations": iterations,
            "operations": iterations, "checksum": iterations * factor,
        }, separators=(",", ":")) + "\n"

    def _wall_result(
        self, argv: list[str], *, duration: int = 100, exit_code: int = 0,
        stdout: str | None = None,
    ) -> ProcessResult:
        return ProcessResult(
            started_at="2026-07-13T00:00:00+00:00", duration_ns=duration,
            exit_code=exit_code, timed_out=False,
            stdout=self._stdout(argv) if stdout is None and exit_code == 0 else (stdout or ""),
            stderr="", stdout_truncated=False, stderr_truncated=False,
        )

    def _run(
        self, lane_id: str, engines: list[Engine], *, blocks: int | None = None,
        wall_runner=None, rss_runner=None,
    ) -> tuple[Path, list[dict]]:
        lane = self.manifest.lanes[lane_id]
        output = io.StringIO()
        kwargs = {}
        if wall_runner is not None:
            kwargs["wall_runner"] = wall_runner
        if rss_runner is not None:
            kwargs["rss_runner"] = rss_runner
        ResourceRun(
            self.manifest, lane, engines, blocks or lane.initial_blocks, lane.seed,
            ResourceJsonlWriter(output), ROOT,
            platform_name=self.manifest.profile.platform,
            machine_name=self.manifest.profile.machine, **kwargs,
        ).execute()
        path = self.temp / f"{lane_id.split('/')[0]}-{len(list(self.temp.glob('*.jsonl')))}.jsonl"
        path.write_text(output.getvalue(), encoding="utf-8")
        return path, [json.loads(line) for line in output.getvalue().splitlines()]

    def _write_rows(self, name: str, rows: list[dict]) -> Path:
        path = self.temp / name
        path.write_text(
            "".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows),
            encoding="utf-8",
        )
        return path

    def test_fresh_lane_has_exact_new_spawn_plan_and_no_setup_phases(self) -> None:
        calls = []

        def wall(argv, timeout):
            calls.append(tuple(argv))
            return self._wall_result(argv, duration=100 + len(calls))

        path, rows = self._run(
            "fresh_process_latency/wall_ns_per_process",
            [self._engine("candidate", 128, verified=False)], wall_runner=wall,
        )
        validated = validate_resource_run(path, self.manifest)
        self.assertEqual(len(calls), 30)
        self.assertEqual(len({row["sample_id"] for row in rows[1:-1]}), 30)
        self.assertEqual({row["phase"] for row in rows[1:-1]}, {"measurement"})
        encoded = path.read_text(encoding="utf-8")
        self.assertNotIn("warmup", encoded)
        self.assertNotIn("calibration", encoded)
        self.assertFalse(validated.comparison_input_complete)
        self.assertEqual(validated.end["status"], "complete")

    def test_real_fresh_process_rejects_and_kills_redirected_descendants(self) -> None:
        binary = self.temp / "fresh-descendant-engine.sh"
        binary.write_text(
            "#!/bin/sh\n"
            "sleep 30 >/dev/null 2>&1 &\n"
            "child=$!\n"
            "printf 'CHILD %s\\n' \"$child\"\n"
            "printf 'QJS_BENCH_RESULT {\"case_id\":\"fresh_process_probe\","
            "\"iterations\":1,\"operations\":1,\"checksum\":7}\\n'\n",
            encoding="utf-8",
        )
        binary.chmod(0o700)
        digest = hashlib.sha256(binary.read_bytes()).hexdigest()
        engine = Engine("candidate", "qjs-file", "qjs-rust", binary, digest, None)
        path, rows = self._run(
            "fresh_process_latency/wall_ns_per_process", [engine]
        )
        validate_resource_run(path, self.manifest)
        samples = rows[1:-1]
        self.assertEqual({row["status"] for row in samples}, {"invalid"})
        self.assertTrue(all(row["descendants_detected"] for row in samples))
        self.assertTrue(all(row["duration_ns"] < 5_000_000_000 for row in samples))
        child_pids = [
            int(line.split()[1]) for row in samples for line in row["stdout"].splitlines()
            if line.startswith("CHILD ")
        ]
        deadline = time.monotonic() + 2
        remaining = set(child_pids)
        while remaining and time.monotonic() < deadline:
            for pid in list(remaining):
                try:
                    os.kill(pid, 0)
                except ProcessLookupError:
                    remaining.remove(pid)
            time.sleep(0.01)
        self.assertFalse(remaining, f"descendants survived containment: {remaining}")

    def test_rss_lane_uses_separate_wait4_result_and_raw_unit(self) -> None:
        calls = []

        def wall_forbidden(argv, timeout):
            raise AssertionError("RSS lane must not use wall runner")

        def rss(argv, timeout, *, platform_name):
            calls.append(tuple(argv))
            return ResourceProcessResult(
                exit_code=0, timed_out=False, stdout=self._stdout(argv), stderr="",
                stdout_truncated=False, stderr_truncated=False, duration_ns=500,
                started_at="2026-07-13T00:00:00+00:00", peak_rss_raw=4096,
                peak_rss_bytes=4096, descendants_detected=False,
            )

        path, rows = self._run(
            "peak_rss/bytes", [self._engine("candidate", 128, verified=False)],
            wall_runner=wall_forbidden, rss_runner=rss,
        )
        validate_resource_run(path, self.manifest)
        self.assertEqual(len(calls), 30)
        sample = rows[1]
        self.assertEqual(sample["raw_rss_unit"], "bytes")
        self.assertEqual(sample["value"], 4096)
        self.assertIsNone(sample["timer"])

    def test_binary_size_uses_hash_verified_snapshot_and_has_no_ci(self) -> None:
        engines = [
            self._engine("candidate", 128), self._engine("base", 256),
            self._engine("quickjs-ng", 512),
        ]
        path, rows = self._run("binary_size/bytes", engines)
        validated = validate_resource_run(path, self.manifest)
        report = build_resource_report(validated, self.manifest, self.analysis)
        self.assertEqual([row["value"] for row in rows[1:-1]], [128, 256, 512])
        self.assertEqual(report["health"]["status"], "healthy")
        self.assertEqual(report["comparisons"]["candidate_vs_base"]["ratio"], 0.5)
        self.assertEqual(
            report["comparisons"]["candidate_vs_quickjs_ng"]["ratio"], 0.25
        )
        self.assertNotIn("confidence_interval", json.dumps(report))
        self.assertFalse(report["claim_eligible"])

    def test_binary_size_preserves_integers_above_ieee754_exact_range(self) -> None:
        path, rows = self._run(
            "binary_size/bytes",
            [self._engine("candidate", 128), self._engine("base", 256),
             self._engine("quickjs-ng", 512)],
        )
        huge = 2**53 + 1
        rows[1]["value"] = huge
        synthetic = self._write_rows("huge-size.jsonl", rows)
        validated = validate_resource_run(synthetic, self.manifest)
        self.assertIsInstance(validated.values[("candidate", None)], int)
        self.assertEqual(validated.values[("candidate", None)], huge)
        report = build_resource_report(validated, self.manifest, self.analysis)
        self.assertEqual(
            report["comparisons"]["candidate_vs_base"]["candidate_bytes"], huge
        )
        self.assertNotIn("confidence_interval", json.dumps(report))

    def test_size_snapshot_ignores_later_source_mutation_and_rejects_prior_mutation(self) -> None:
        engine = self._engine("candidate", 160, verified=False)
        output = io.StringIO()
        run = ResourceRun(
            self.manifest, self.manifest.lanes["binary_size/bytes"], [engine], 1,
            self.manifest.lanes["binary_size/bytes"].seed, ResourceJsonlWriter(output), ROOT,
            platform_name=self.manifest.profile.platform,
            machine_name=self.manifest.profile.machine,
        )
        engine.binary.write_bytes(b"changed after snapshot")
        run.execute()
        rows = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(rows[1]["value"], 160)

        stale = self._engine("base", 160, verified=False)
        stale.binary.write_bytes(b"changed before snapshot")
        with self.assertRaisesRegex(SnapshotError, "hash mismatch"):
            ResourceRun(
                self.manifest, self.manifest.lanes["binary_size/bytes"], [stale], 1,
                self.manifest.lanes["binary_size/bytes"].seed,
                ResourceJsonlWriter(io.StringIO()), ROOT,
                platform_name=self.manifest.profile.platform,
                machine_name=self.manifest.profile.machine,
            )

    def test_invalid_sample_invalidates_whole_block_but_plan_is_durable(self) -> None:
        engines = [
            self._engine("candidate", 128), self._engine("base", 256),
            self._engine("quickjs-ng", 512),
        ]
        calls = 0

        def wall(argv, timeout):
            nonlocal calls
            calls += 1
            if calls == 5:
                return self._wall_result(argv, exit_code=9)
            role_duration = 100 if "candidate-" in argv[0] else (
                110 if "base-" in argv[0] else 120
            )
            return self._wall_result(argv, duration=role_duration)

        path, rows = self._run(
            "fresh_process_latency/wall_ns_per_process", engines, wall_runner=wall
        )
        validated = validate_resource_run(path, self.manifest)
        report = build_resource_report(validated, self.manifest, self.analysis)
        self.assertEqual(calls, 90)
        self.assertEqual(len(rows), 92)
        self.assertEqual(len(validated.invalid_blocks), 1)
        bad_block = validated.invalid_blocks[0]["block"]
        self.assertTrue(all((role, bad_block) not in validated.values for role in (
            "candidate", "base", "quickjs-ng"
        )))
        self.assertTrue(report["coverage"]["physical_plan_complete"])
        self.assertFalse(report["coverage"]["comparison_input_complete"])
        self.assertEqual(report["health"]["status"], "healthy")

    def test_30_60_health_policy_and_three_percent_boundary(self) -> None:
        policy = self.analysis.health
        self.assertEqual(_precision(30, [0.03], True, policy)["status"], "healthy")
        self.assertEqual(_precision(30, [0.031], True, policy)["status"], "extension_required")
        self.assertEqual(_precision(60, [0.031], True, policy)["status"], "inconclusive")
        self.assertEqual(_block_health(30, [0, 1, 2], policy)["status"], "pass")
        self.assertEqual(_block_health(30, [0, 1, 2, 3], policy)["status"], "invalid")
        self.assertEqual(_block_health(60, [30, 31, 32, 33, 34, 35], policy)["status"], "pass")

    def test_sixty_block_dynamic_plan_is_exact(self) -> None:
        calls = 0

        def wall(argv, timeout):
            nonlocal calls
            calls += 1
            return self._wall_result(argv)

        path, rows = self._run(
            "fresh_process_latency/wall_ns_per_process",
            [self._engine("candidate", 128, verified=False)], blocks=60,
            wall_runner=wall,
        )
        validated = validate_resource_run(path, self.manifest)
        self.assertEqual(calls, 60)
        self.assertEqual(len(rows), 62)
        self.assertEqual(validated.valid_blocks, tuple(range(60)))

    def test_binary_size_smoke_is_invalid_not_a_partial_claim(self) -> None:
        path, _rows = self._run(
            "binary_size/bytes", [self._engine("candidate", 128, verified=False)]
        )
        report = build_resource_report(
            validate_resource_run(path, self.manifest), self.manifest, self.analysis
        )
        self.assertEqual(report["health"]["status"], "invalid")
        self.assertIsNone(report["comparisons"]["candidate_vs_base"])
        self.assertFalse(report["claim_eligible"])

    def test_non_ok_process_states_are_durable_and_errors_are_recomputed(self) -> None:
        lane_id = "fresh_process_latency/wall_ns_per_process"

        def result_for(kind, argv):
            base = self._wall_result(argv)
            values = base.__dict__.copy()
            if kind == "timeout":
                values.update(exit_code=-9, timed_out=True, stdout="")
            elif kind == "nonzero":
                values.update(exit_code=7, stdout="")
            elif kind == "truncated":
                values.update(stdout_truncated=True)
            elif kind == "malformed":
                values.update(stdout="not a result\n")
            elif kind == "mismatch":
                values.update(stdout=base.stdout.replace('"checksum":7', '"checksum":8'))
            elif kind == "spawn":
                values.update(exit_code=None, stdout="", stderr="exec format error")
            return ProcessResult(**values)

        expected_status = {
            "timeout": "timeout", "nonzero": "failed", "truncated": "invalid",
            "malformed": "invalid", "mismatch": "invalid", "spawn": "failed",
        }
        for kind, status in expected_status.items():
            with self.subTest(kind=kind):
                path, rows = self._run(
                    lane_id, [self._engine("candidate", 128, verified=False)],
                    wall_runner=lambda argv, timeout, kind=kind: result_for(kind, argv),
                )
                validated = validate_resource_run(path, self.manifest)
                self.assertEqual({row["status"] for row in rows[1:-1]}, {status})
                self.assertEqual(validated.end["status"], "failed")
                rows[1]["error"] = "forged"
                tampered = self.temp / f"forged-{kind}.jsonl"
                tampered.write_text(
                    "".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows),
                    encoding="utf-8",
                )
                with self.assertRaisesRegex(ResourceReportError, "status/error"):
                    validate_resource_run(tampered, self.manifest)

    def test_rss_timeout_keeps_wait4_rusage_but_never_a_value(self) -> None:
        def rss(argv, timeout, *, platform_name):
            return ResourceProcessResult(
                exit_code=-9, timed_out=True, stdout="", stderr="",
                stdout_truncated=False, stderr_truncated=False, duration_ns=500,
                started_at="2026-07-13T00:00:00+00:00", peak_rss_raw=4096,
                peak_rss_bytes=4096, descendants_detected=False,
            )

        path, rows = self._run(
            "peak_rss/bytes", [self._engine("candidate", 128, verified=False)],
            rss_runner=rss,
        )
        validated = validate_resource_run(path, self.manifest)
        self.assertEqual(validated.end["status"], "failed")
        self.assertTrue(all(row["raw_rss"] == 4096 for row in rows[1:-1]))
        self.assertTrue(all(row["value"] is None for row in rows[1:-1]))

    def test_rss_monitor_failure_is_durable_and_has_no_rss_value(self) -> None:
        def rss(argv, timeout, *, platform_name):
            return ResourceProcessResult(
                exit_code=-9, timed_out=False, stdout="", stderr="",
                stdout_truncated=False, stderr_truncated=False, duration_ns=500,
                started_at="2026-07-13T00:00:00+00:00", peak_rss_raw=None,
                peak_rss_bytes=None, descendants_detected=False,
                monitor_error="OSError: wait4 failed",
            )

        path, rows = self._run(
            "peak_rss/bytes", [self._engine("candidate", 128, verified=False)],
            rss_runner=rss,
        )
        validated = validate_resource_run(path, self.manifest)
        self.assertEqual(validated.end["status"], "failed")
        self.assertTrue(all(row["raw_rss"] is None for row in rows[1:-1]))
        self.assertTrue(all(row["value"] is None for row in rows[1:-1]))
        self.assertEqual(
            {row["error"] for row in rows[1:-1]},
            {"resource monitor failed: OSError: wait4 failed"},
        )

    def test_rss_descendant_evidence_is_invalid_and_contained_state_is_durable(self) -> None:
        def rss(argv, timeout, *, platform_name):
            return ResourceProcessResult(
                exit_code=0, timed_out=False, stdout=self._stdout(argv), stderr="",
                stdout_truncated=False, stderr_truncated=False, duration_ns=500,
                started_at="2026-07-13T00:00:00+00:00", peak_rss_raw=4096,
                peak_rss_bytes=4096, descendants_detected=True,
            )

        path, rows = self._run(
            "peak_rss/bytes", [self._engine("candidate", 128, verified=False)],
            rss_runner=rss,
        )
        validate_resource_run(path, self.manifest)
        self.assertEqual({row["status"] for row in rows[1:-1]}, {"invalid"})
        self.assertEqual(
            {row["error"] for row in rows[1:-1]}, {"engine spawned descendant processes"}
        )

    def test_strict_validation_rejects_plan_receipt_and_status_tampering(self) -> None:
        path, original = self._run(
            "fresh_process_latency/wall_ns_per_process",
            [self._engine("candidate", 128), self._engine("base", 256),
             self._engine("quickjs-ng", 512)],
            wall_runner=lambda argv, timeout: self._wall_result(argv),
        )
        mutations = {
            "argv": lambda rows: rows[1]["argv"].append("forged"),
            "receipt": lambda rows: rows[0]["engines"][0].__setitem__("receipt_sha256", "0" * 64),
            "status": lambda rows: rows[1].update({"status": "failed", "error": "forged", "value": None,
                                                    "measurement_eligible": False,
                                                    "operations": None, "checksum": None}),
            "unknown": lambda rows: rows[1].__setitem__("surprise", True),
            "coverage": lambda rows: rows[-1]["coverage"].__setitem__("valid", 0),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                rows = json.loads(json.dumps(original))
                mutate(rows)
                tampered = self.temp / f"tampered-{name}.jsonl"
                tampered.write_text(
                    "".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows),
                    encoding="utf-8",
                )
                with self.assertRaises(ResourceReportError):
                    validate_resource_run(tampered, self.manifest)
        validate_resource_run(path, self.manifest)

        first_line, remainder = path.read_text(encoding="utf-8").split("\n", 1)
        duplicate = first_line.replace(
            '"blocks":30,', '"blocks":30,"blocks":30,', 1
        ) + "\n" + remainder
        duplicate_path = self.temp / "duplicate-raw-key.jsonl"
        duplicate_path.write_text(duplicate, encoding="utf-8")
        with self.assertRaisesRegex(ResourceReportError, "duplicate key"):
            validate_resource_run(duplicate_path, self.manifest)

    def test_role_identity_is_rejected_by_cli_constructor_and_raw_validator(self) -> None:
        binary = self.temp / "attacker.bin"
        binary.write_bytes(b"#!/bin/sh\nexit 0\n")
        binary.chmod(0o700)
        digest = hashlib.sha256(binary.read_bytes()).hexdigest()
        recipe = self.manifest.build_recipes["quickjs-ng"]
        content = {
            "schema_version": 1, "engine_identity": "quickjs-ng",
            "source": {
                "repo": "https://attacker.invalid/fork.git", "revision": "c" * 40,
                "dirty": False,
            },
            "profile_id": self.manifest.profile.id,
            "build": {
                key: list(value) if isinstance(value, tuple) else value
                for key, value in recipe.__dict__.items() if key != "engine_identity"
            },
            "binary_sha256": digest,
        }
        receipt = BuildReceipt(
            path=self.temp / "attacker-receipt.json",
            sha256=canonical_receipt_sha256(content), content=content,
            engine_identity="quickjs-ng", source_repo=content["source"]["repo"],
            source_revision="c" * 40, source_dirty=False,
            profile_id=self.manifest.profile.id, binary_sha256=digest,
        )
        receipt.path.write_text(json.dumps(content), encoding="utf-8")
        attacker = Engine("candidate", "qjs-file", "quickjs-ng", binary, digest, receipt)
        with self.assertRaisesRegex(ResourceRunError, "candidate: identity"):
            ResourceRun(
                self.manifest, self.manifest.lanes["binary_size/bytes"], [attacker], 1,
                self.manifest.lanes["binary_size/bytes"].seed,
                ResourceJsonlWriter(io.StringIO()), ROOT,
                platform_name=self.manifest.profile.platform,
                machine_name=self.manifest.profile.machine,
            )
        cli = subprocess.run(
            [str(ROOT / "scripts/resource-benchmark.sh"), "--lane", "size",
             "--candidate", str(binary), "--candidate-identity", "quickjs-ng",
             "--candidate-receipt", str(receipt.path), "--output", str(self.temp / "attack.jsonl")],
            capture_output=True, text=True, timeout=10, check=False,
        )
        self.assertEqual(cli.returncode, 2)
        self.assertIn("candidate: identity must be 'qjs-rust'", cli.stderr)

        _path, rows = self._run(
            "binary_size/bytes", [self._engine("candidate", 128)]
        )
        forged_engine = {
            **rows[0]["engines"][0], "engine_identity": "quickjs-ng",
            "receipt": content, "receipt_sha256": canonical_receipt_sha256(content),
            "binary_sha256": digest, "binary_source_path": str(binary),
            "binary_snapshot_path": "/tmp/attacker-snapshot",
        }
        rows[0]["engines"][0] = forged_engine
        for sample in rows[1:-1]:
            for field in forged_engine:
                if field != "role":
                    sample[field] = forged_engine[field]
        forged = self._write_rows("forged-role.jsonl", rows)
        with self.assertRaisesRegex(ResourceReportError, "expected 'qjs-rust'"):
            validate_resource_run(forged, self.manifest)

    def test_raw_control_fields_reject_bool_integer_aliases(self) -> None:
        _size_path, size_rows = self._run(
            "binary_size/bytes", [self._engine("candidate", 128, verified=False)]
        )
        size_attacks = {
            "lane_id_container": lambda r: r[0].__setitem__("lane_id", []),
            "adapter_id_container": lambda r: r[0]["engines"][0].__setitem__(
                "adapter_id", []
            ),
            "provenance_status_container": lambda r: r[0]["engines"][0].__setitem__(
                "provenance_status", []
            ),
            "start_schema": lambda r: r[0].__setitem__("schema_version", True),
            "start_claim": lambda r: r[0].__setitem__("claim_eligible", 0),
            "start_readiness": lambda r: r[0].__setitem__("comparison_input_complete", 0),
            "start_preconditions": lambda r: r[0].__setitem__("comparison_input_preconditions_met", 0),
            "blocks": lambda r: r[0].__setitem__("blocks", True),
            "expected_samples": lambda r: r[0].__setitem__("expected_samples", True),
            "sample_schema": lambda r: r[1].__setitem__("schema_version", True),
            "sample_status_container": lambda r: r[1].__setitem__("status", []),
            "sample_eligible": lambda r: r[1].__setitem__("measurement_eligible", 1),
            "sample_order": lambda r: r[1].__setitem__("order", False),
            "sample_value": lambda r: r[1].__setitem__("value", True),
            "end_schema": lambda r: r[-1].__setitem__("schema_version", True),
            "end_claim": lambda r: r[-1].__setitem__("claim_eligible", 0),
            "end_readiness": lambda r: r[-1].__setitem__("comparison_input_complete", 0),
            "physical": lambda r: r[-1].__setitem__("physical_plan_complete", 1),
            "attempted": lambda r: r[-1]["coverage"].__setitem__("attempted", True),
            "planned": lambda r: r[-1]["coverage"].__setitem__("planned", True),
            "valid": lambda r: r[-1]["coverage"].__setitem__("valid", True),
            "valid_by_role": lambda r: r[-1]["coverage"]["valid_by_role"].__setitem__(
                "candidate", True
            ),
        }
        for name, mutate in size_attacks.items():
            with self.subTest(name=name):
                rows = json.loads(json.dumps(size_rows))
                mutate(rows)
                with self.assertRaises(ResourceReportError):
                    validate_resource_run(self._write_rows(f"type-{name}.jsonl", rows), self.manifest)

        _fresh_path, fresh_rows = self._run(
            "fresh_process_latency/wall_ns_per_process",
            [self._engine("candidate", 128, verified=False)],
            wall_runner=lambda argv, timeout: self._wall_result(argv),
        )
        block_one = next(index for index, row in enumerate(fresh_rows) if row.get("block") == 1)
        dynamic_attacks = {
            "seed": lambda r: r[0].__setitem__("seed", True),
            "block": lambda r: r[block_one].__setitem__("block", True),
            "iterations": lambda r: r[1].__setitem__("iterations", True),
            "duration": lambda r: r[1].__setitem__("duration_ns", True),
            "exit": lambda r: r[1].__setitem__("exit_code", False),
            "operations": lambda r: r[1].__setitem__("operations", True),
            "checksum": lambda r: r[1].__setitem__("checksum", True),
            "timed_out": lambda r: r[1].__setitem__("timed_out", 0),
            "stdout_truncated": lambda r: r[1].__setitem__("stdout_truncated", 0),
            "stderr_truncated": lambda r: r[1].__setitem__("stderr_truncated", 0),
            "descendants": lambda r: r[1].__setitem__("descendants_detected", 0),
        }
        for name, mutate in dynamic_attacks.items():
            with self.subTest(name=name):
                rows = json.loads(json.dumps(fresh_rows))
                mutate(rows)
                with self.assertRaises(ResourceReportError):
                    validate_resource_run(self._write_rows(f"type-dynamic-{name}.jsonl", rows), self.manifest)

    def test_numeric_measurements_are_bounded_before_float_conversion(self) -> None:
        huge = 10**1000
        _fresh_path, fresh_rows = self._run(
            "fresh_process_latency/wall_ns_per_process",
            [self._engine("candidate", 128, verified=False)],
            wall_runner=lambda argv, timeout: self._wall_result(argv),
        )
        attacks = {
            "duration": lambda rows: rows[1].__setitem__("duration_ns", huge),
            "dynamic_value": lambda rows: rows[1].__setitem__("value", huge),
        }
        for name, mutate in attacks.items():
            with self.subTest(name=name):
                rows = json.loads(json.dumps(fresh_rows))
                mutate(rows)
                with self.assertRaises(ResourceReportError):
                    validate_resource_run(
                        self._write_rows(f"huge-{name}.jsonl", rows), self.manifest
                    )

        cli_rows = json.loads(json.dumps(fresh_rows))
        cli_rows[1]["value"] = huge
        cli_input = self._write_rows("huge-cli.jsonl", cli_rows)
        cli = subprocess.run(
            [str(ROOT / "scripts/resource-benchmark-report.sh"),
             "--input", str(cli_input), "--output", str(self.temp / "huge-report.json")],
            capture_output=True, text=True, timeout=10, check=False,
        )
        self.assertEqual(cli.returncode, 2)
        self.assertNotIn("Traceback", cli.stderr)
        self.assertIn("expected integer", cli.stderr)

        def rss(argv, timeout, *, platform_name):
            return ResourceProcessResult(
                exit_code=0, timed_out=False, stdout=self._stdout(argv), stderr="",
                stdout_truncated=False, stderr_truncated=False, duration_ns=500,
                started_at="2026-07-13T00:00:00+00:00", peak_rss_raw=4096,
                peak_rss_bytes=4096, descendants_detected=False,
            )

        _rss_path, rss_rows = self._run(
            "peak_rss/bytes", [self._engine("candidate", 128, verified=False)],
            rss_runner=rss,
        )
        rss_rows[1]["raw_rss"] = huge
        with self.assertRaises(ResourceReportError):
            validate_resource_run(self._write_rows("huge-rss.jsonl", rss_rows), self.manifest)

        _size_path, size_rows = self._run(
            "binary_size/bytes", [self._engine("candidate", 128, verified=False)]
        )
        size_rows[1]["value"] = 1 << 63
        with self.assertRaises(ResourceReportError):
            validate_resource_run(self._write_rows("huge-size.jsonl", size_rows), self.manifest)

    def test_profile_machine_mismatch_fails_runner_and_raw(self) -> None:
        engine = self._engine("candidate", 128, verified=False)
        with self.assertRaisesRegex(ResourceRunError, "host machine 'x86_64'"):
            ResourceRun(
                self.manifest, self.manifest.lanes["binary_size/bytes"], [engine], 1,
                self.manifest.lanes["binary_size/bytes"].seed,
                ResourceJsonlWriter(io.StringIO()), ROOT,
                platform_name="darwin", machine_name="x86_64",
            )
        _path, rows = self._run("binary_size/bytes", [engine])
        rows[0]["host"]["machine"] = "x86_64"
        for row in rows[1:-1]:
            row["host"]["machine"] = "x86_64"
        with self.assertRaisesRegex(ResourceReportError, "host.machine"):
            validate_resource_run(self._write_rows("wrong-machine.jsonl", rows), self.manifest)

    def test_dynamic_evidence_reanalyzes_from_second_checkout_root(self) -> None:
        path, _rows = self._run(
            "fresh_process_latency/wall_ns_per_process",
            [self._engine("candidate", 128, verified=False)],
            wall_runner=lambda argv, timeout: self._wall_result(argv),
        )
        second = self.temp / "second-root"
        files = set(self.manifest.protocol_file_ids) | set(self.analysis.protocol_file_ids)
        for relative in files:
            destination = second / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, destination)
        for name in ("resources.json", "resource-analysis.json"):
            destination = second / "benchmarks" / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / "benchmarks" / name, destination)
        second_measurement = load_resource_manifest(second / "benchmarks/resources.json")
        second_analysis = load_resource_analysis(
            second / "benchmarks/resource-analysis.json", second_measurement
        )
        original_report = build_resource_report(
            validate_resource_run(path, self.manifest), self.manifest, self.analysis
        )
        second_report = build_resource_report(
            validate_resource_run(path, second_measurement), second_measurement, second_analysis
        )
        self.assertEqual(original_report, second_report)

    def test_report_is_path_independent_atomic_and_never_overwrites(self) -> None:
        path, _rows = self._run(
            "binary_size/bytes",
            [self._engine("candidate", 128), self._engine("base", 256),
             self._engine("quickjs-ng", 512)],
        )
        renamed = self.temp / "renamed-evidence.jsonl"
        renamed.write_bytes(path.read_bytes())
        first = build_resource_report(
            validate_resource_run(path, self.manifest), self.manifest, self.analysis
        )
        second = build_resource_report(
            validate_resource_run(renamed, self.manifest), self.manifest, self.analysis
        )
        self.assertEqual(first, second)
        output = self.temp / "report.json"
        write_resource_report(output, first)
        with self.assertRaisesRegex(ResourceReportError, "overwrite"):
            write_resource_report(output, first)


if __name__ == "__main__":
    unittest.main()
