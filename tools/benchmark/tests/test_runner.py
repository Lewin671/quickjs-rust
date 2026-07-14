from __future__ import annotations

import io
import json
import os
import tempfile
import time
import unittest
from dataclasses import replace
from pathlib import Path
from unittest import mock

from tools.benchmark.adapters import Engine, load_engine, probe_version
from tools.benchmark.process import ProcessResult, run_process
from tools.benchmark.receipts import BuildReceipt, canonical_receipt_sha256
from tools.benchmark.runner import BenchmarkRun, JsonlWriter
from tools.benchmark.schema import load_manifest, sha256_file
from tools.benchmark.snapshots import SnapshotError


ROOT = Path(__file__).resolve().parents[3]


class RunnerTests(unittest.TestCase):
    def setUp(self) -> None:
        # Runner semantic tests must not depend on spawning a dynamic-loader
        # process. Dedicated process/version tests below retain real children.
        version_patcher = mock.patch(
            "tools.benchmark.runner.probe_version", return_value="fake-version"
        )
        version_patcher.start()
        self.addCleanup(version_patcher.stop)

    def _engine(
        self,
        body: str,
        *,
        role: str = "candidate",
        adapter_id: str = "qjs-file",
        engine_identity: str = "quickjs-ng",
    ):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        path = Path(temporary.name) / "engine"
        path.write_text("#!/usr/bin/env python3\n" + body, encoding="utf-8")
        path.chmod(0o755)
        return load_engine(role, adapter_id, engine_identity, path)

    @staticmethod
    def _process_result(
        *,
        stdout: str = "",
        stderr: str = "",
        duration_ns: int = 1_000_000,
        exit_code: int | None = 0,
        timed_out: bool = False,
        stdout_truncated: bool = False,
        stderr_truncated: bool = False,
    ) -> ProcessResult:
        return ProcessResult(
            started_at="2026-01-01T00:00:00+00:00",
            duration_ns=duration_ns,
            exit_code=exit_code,
            timed_out=timed_out,
            stdout=stdout,
            stderr=stderr,
            stdout_truncated=stdout_truncated,
            stderr_truncated=stderr_truncated,
        )

    def _valid_result(self, case, iterations: int, *, duration_ns: int = 1_000_000):
        return self._process_result(
            duration_ns=duration_ns,
            stdout=(
                "QJS_BENCH_RESULT "
                + json.dumps({
                    "case_id": case.id,
                    "iterations": iterations,
                    "operations": case.expected_operations(iterations),
                    "checksum": case.expected_checksum(iterations),
                }, separators=(",", ":"))
                + "\n"
            ),
        )

    def _verified(self, engine, manifest):
        recipe = manifest.build_recipes[engine.engine_identity]
        content = {
            "schema_version": 1,
            "engine_identity": engine.engine_identity,
            "source": {
                "repo": (
                    manifest.reference_repo
                    if engine.role == "quickjs-ng"
                    else "https://example.invalid/engine.git"
                ),
                "revision": (
                    manifest.reference_revision if engine.role == "quickjs-ng" else "a" * 40
                ),
                "dirty": False,
            },
            "profile_id": manifest.profile.id,
            "build": {
                key: value for key, value in recipe.__dict__.items() if key != "engine_identity"
            },
            "binary_sha256": engine.binary_sha256,
        }
        receipt = BuildReceipt(
            path=engine.binary,
            sha256=canonical_receipt_sha256(content),
            content=content,
            engine_identity=engine.engine_identity,
            source_repo=content["source"]["repo"],
            source_revision=content["source"]["revision"],
            source_dirty=False,
            profile_id=manifest.profile.id,
            binary_sha256=engine.binary_sha256,
        )
        return replace(engine, receipt=receipt)

    def test_timeout_is_reported(self) -> None:
        result = run_process(["python3", "-c", "import time; time.sleep(2)"], 0.05)
        self.assertTrue(result.timed_out)
        self.assertNotEqual(result.exit_code, 0)

    def test_output_is_bounded(self) -> None:
        result = run_process(["python3", "-c", "print('x' * 70000)"], 1)
        self.assertEqual(result.exit_code, 0)
        self.assertTrue(result.stdout_truncated)
        self.assertEqual(len(result.stdout.encode()), 64 * 1024)

    def test_invalid_utf8_and_noisy_stderr_stay_bounded(self) -> None:
        result = run_process(
            ["python3", "-c", "import os; os.write(1, b'\\xff' * 70000); os.write(2, b'e' * 70000)"],
            1,
        )
        self.assertTrue(result.stdout_truncated)
        self.assertTrue(result.stderr_truncated)
        self.assertLessEqual(len(result.stdout.encode()), 64 * 1024)
        self.assertLessEqual(len(result.stderr.encode()), 64 * 1024)

    @unittest.skipUnless(os.name == "posix", "process-group assertion is POSIX-specific")
    def test_timeout_kills_spawned_child_process_group(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        marker = Path(temporary.name) / "child-finished"
        child = f"import time, pathlib; time.sleep(.3); pathlib.Path({str(marker)!r}).write_text('bad')"
        parent = "import subprocess, sys, time; subprocess.Popen([sys.executable, '-c', sys.argv[1]]); time.sleep(2)"
        result = run_process(["python3", "-c", parent, child], 0.05)
        self.assertTrue(result.timed_out)
        time.sleep(0.4)
        self.assertFalse(marker.exists())

    @unittest.skipUnless(os.name == "posix", "process-group assertion is POSIX-specific")
    def test_normal_parent_exit_contains_pipe_inheriting_child(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        marker = Path(temporary.name) / "orphan-finished"
        child = f"import time, pathlib; time.sleep(.3); pathlib.Path({str(marker)!r}).write_text('bad')"
        parent = "import subprocess, sys; subprocess.Popen([sys.executable, '-c', sys.argv[1]])"
        result = run_process(["python3", "-c", parent, child], 1)
        self.assertEqual(result.exit_code, 0)
        time.sleep(0.35)
        self.assertFalse(marker.exists())

    def test_version_probe_uses_requested_path_flag_and_bounded_output(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        binary = Path(temporary.name) / "engine"
        binary.write_text("not executed by this unit test\n", encoding="utf-8")
        binary.chmod(0o755)
        engine = load_engine("candidate", "qjs-file", "quickjs-ng", binary)
        completed = self._process_result(stdout="v" * 600)
        with mock.patch(
            "tools.benchmark.adapters.run_process", return_value=completed
        ) as process:
            version = probe_version(engine.binary)
        self.assertEqual(version, "v" * 512)
        process.assert_called_once_with([str(engine.binary), "--version"], 2)

    def test_version_probe_timeout_and_process_error_are_unknown(self) -> None:
        binary = Path("/isolated/probe-copy")
        timed_out = self._process_result(exit_code=-9, timed_out=True)
        with mock.patch(
            "tools.benchmark.adapters.run_process",
            side_effect=[timed_out, OSError("cannot execute")],
        ) as process:
            self.assertIsNone(probe_version(binary))
        self.assertEqual(
            process.call_args_list,
            [
                mock.call([str(binary), "--version"], 2),
                mock.call([str(binary), "-v"], 2),
            ],
        )

    def test_load_engine_does_not_execute_source_binary(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        marker = Path(temporary.name) / "executed"
        binary = Path(temporary.name) / "engine"
        binary.write_text(
            "#!/bin/sh\n"
            f"printf executed > {str(marker)!r}\n"
            "printf '%s\\n' source-version\n",
            encoding="utf-8",
        )
        binary.chmod(0o755)
        engine = load_engine("candidate", "qjs-file", "quickjs-ng", binary)
        self.assertEqual(engine.binary, binary.resolve())
        self.assertFalse(marker.exists())

    def test_failed_sample_is_durable(self) -> None:
        engine = self._engine("import sys\nsys.stderr.write('nope')\nsys.exit(7)\n")
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        output = io.StringIO()
        run = BenchmarkRun(manifest, [engine], [manifest.cases[0]], 1, 1, JsonlWriter(output), ROOT)
        self.addCleanup(run.close)
        result = self._process_result(stderr="nope", exit_code=7)
        with mock.patch("tools.benchmark.runner.run_process", return_value=result):
            _result, status, _quality = run._sample(
                engine, manifest.cases[0], 1, "measurement", 0, 0, "eligible"
            )
        self.assertEqual(status, "failed")
        record = json.loads(output.getvalue())
        self.assertEqual(record["status"], "failed")
        self.assertEqual(record["exit_code"], 7)
        self.assertEqual(record["quality"], "ineligible")

    def test_exec_format_spawn_failure_is_durable_and_bound_to_stderr(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        binary = Path(temporary.name) / "bad-engine"
        binary.write_bytes(b"this is not an executable format\n")
        binary.chmod(0o755)
        engine = load_engine("candidate", "qjs-file", "quickjs-ng", binary)
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        output = io.StringIO()
        run = BenchmarkRun(
            manifest, [engine], [manifest.cases[0]], 1, 1, JsonlWriter(output), ROOT
        )
        self.addCleanup(run.close)
        result, status, quality = run._sample(
            engine, manifest.cases[0], 1, "measurement", 0, 0, "eligible"
        )
        self.assertIsNone(result.exit_code)
        self.assertEqual((status, quality), ("failed", "ineligible"))
        record = json.loads(output.getvalue())
        self.assertTrue(record["stderr"])
        self.assertEqual(record["error"], f"engine could not start: {record['stderr']}")

    def test_invalid_checksum_is_rejected(self) -> None:
        engine = self._engine(
            "import json, sys\n"
            "case, n = sys.argv[-2], int(sys.argv[-1])\n"
            "print('QJS_BENCH_RESULT ' + json.dumps({'case_id': case, 'iterations': n, 'operations': n, 'checksum': 0}))\n"
        )
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        output = io.StringIO()
        run = BenchmarkRun(manifest, [engine], [manifest.cases[0]], 1, 1, JsonlWriter(output), ROOT)
        self.addCleanup(run.close)
        result = self._process_result(
            stdout=(
                'QJS_BENCH_RESULT {"case_id":"plain_function_call","iterations":2,'
                '"operations":2,"checksum":0}\n'
            )
        )
        with mock.patch("tools.benchmark.runner.run_process", return_value=result):
            _result, status, _quality = run._sample(
                engine, manifest.cases[0], 2, "measurement", 0, 0, "eligible"
            )
        self.assertEqual(status, "invalid")
        self.assertIn("checksum mismatch", json.loads(output.getvalue())["error"])

    def test_truncated_stderr_makes_sample_invalid(self) -> None:
        engine = self._engine(
            "import json, sys\n"
            "case, n = sys.argv[-2], int(sys.argv[-1])\n"
            "sys.stderr.write('x' * 70000)\n"
            "print('QJS_BENCH_RESULT ' + json.dumps({'case_id': case, 'iterations': n, 'operations': n, 'checksum': n * (n + 1) // 2}))\n"
        )
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        output = io.StringIO()
        run = BenchmarkRun(manifest, [engine], [manifest.cases[0]], 1, 1, JsonlWriter(output), ROOT)
        self.addCleanup(run.close)
        result = self._valid_result(manifest.cases[0], 2)
        result = replace(result, stderr="x" * 1024, stderr_truncated=True)
        with mock.patch("tools.benchmark.runner.run_process", return_value=result):
            _result, status, quality = run._sample(
                engine, manifest.cases[0], 2, "measurement", 0, 0, "eligible"
            )
        self.assertEqual((status, quality), ("invalid", "ineligible"))

    def test_warmup_failure_writes_not_run_measurement(self) -> None:
        engine = self._engine("raise SystemExit(1)\n")
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        case = replace(
            manifest.cases[0], initial_iterations=1, max_iterations=1,
            min_window_ms=1, warmup_runs=1, timeout_seconds=1,
        )
        output = io.StringIO()
        run = BenchmarkRun(manifest, [engine], [case], 1, 1, JsonlWriter(output), ROOT)
        self.addCleanup(run.close)
        calls = 0

        def fail_warmup(argv: list[str], _timeout: float) -> ProcessResult:
            nonlocal calls
            calls += 1
            iterations = int(argv[-1])
            if calls == 5:
                return self._process_result(exit_code=9)
            return self._valid_result(case, iterations)

        with mock.patch("tools.benchmark.runner.run_process", side_effect=fail_warmup):
            self.assertFalse(run.execute())
        rows = [json.loads(line) for line in output.getvalue().splitlines()]
        measurement = [row for row in rows if row.get("phase") == "measurement"]
        self.assertEqual(len(measurement), 1)
        self.assertEqual(measurement[0]["status"], "not_run")
        self.assertFalse(measurement[0]["measurement_eligible"])
        self.assertNotIn((engine.role, case.id), run.iterations)

    def test_calibration_emits_dedicated_exact_n_and_2n_linearity_phases(self) -> None:
        engine = self._engine("raise SystemExit(1)\n")
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        case = replace(
            manifest.cases[0], initial_iterations=1, max_iterations=8,
            min_window_ms=1, startup_max_fraction=0.1, warmup_runs=1,
        )
        output = io.StringIO()
        run = BenchmarkRun(manifest, [engine], [case], 1, 1, JsonlWriter(output), ROOT)
        self.addCleanup(run.close)

        def successful(argv: list[str], _timeout: float) -> ProcessResult:
            iterations = int(argv[-1])
            duration = 1_000_000 if iterations == 0 else 20_000_000
            return self._valid_result(case, iterations, duration_ns=duration)

        with mock.patch("tools.benchmark.runner.run_process", side_effect=successful):
            self.assertTrue(run._calibrate(engine, case))
        rows = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(
            [row["phase"] for row in rows],
            ["startup", "startup", "startup", "calibration", "warmup", "linearity_n", "linearity_2n"],
        )
        n, twice = rows[-2:]
        self.assertEqual((n["diagnostic_point"], twice["diagnostic_point"]), ("n", "2n"))
        self.assertEqual(twice["iterations"], n["iterations"] * 2)
        self.assertFalse(n["measurement_eligible"])

    def test_failed_linearity_is_durable_and_prevents_complete_input(self) -> None:
        engine = self._engine("raise SystemExit(1)\n")
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        case = replace(
            manifest.cases[0], initial_iterations=1, max_iterations=8,
            min_window_ms=1, startup_max_fraction=0.1, warmup_runs=1,
        )
        output = io.StringIO()
        run = BenchmarkRun(manifest, [engine], [case], 1, 1, JsonlWriter(output), ROOT)
        self.addCleanup(run.close)
        calls = 0

        def fail_second_point(argv: list[str], _timeout: float) -> ProcessResult:
            nonlocal calls
            calls += 1
            if calls == 7:
                return self._process_result(exit_code=9)
            iterations = int(argv[-1])
            duration = 1_000_000 if iterations == 0 else 20_000_000
            return self._valid_result(case, iterations, duration_ns=duration)

        with mock.patch("tools.benchmark.runner.run_process", side_effect=fail_second_point):
            self.assertFalse(run.execute())
        rows = [json.loads(line) for line in output.getvalue().splitlines()]
        failed = next(row for row in rows if row.get("phase") == "linearity_2n")
        self.assertEqual((failed["status"], failed["diagnostic_point"]), ("failed", "2n"))
        self.assertFalse(rows[-1]["comparison_input_complete"])
        self.assertEqual(
            next(row for row in rows if row.get("phase") == "measurement")["status"],
            "not_run",
        )

    def test_focused_portfolio_is_never_complete_comparison_input(self) -> None:
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        output = io.StringIO()
        engine = self._engine("raise SystemExit(1)\n")
        run = BenchmarkRun(manifest, [engine], [manifest.cases[0]], 1, 1, JsonlWriter(output), ROOT)
        self.addCleanup(run.close)
        self.assertFalse(run.portfolio_complete)
        self.assertFalse(run.comparison_input_preconditions_met)

    def test_verified_receipt_is_recorded_without_runner_git_substitution(self) -> None:
        engine = self._engine(
            "import json, sys\n"
            "case, n = sys.argv[-2], int(sys.argv[-1])\n"
            "print('QJS_BENCH_RESULT ' + json.dumps({'case_id': case, 'iterations': n, 'operations': n, 'checksum': n * (n + 1) // 2}))\n"
        )
        receipt_content = {
            "schema_version": 1,
            "engine_identity": "quickjs-ng",
            "source": {"repo": "repo", "revision": "a" * 40, "dirty": False},
            "profile_id": "macos-arm64-release-v1",
            "build": {},
            "binary_sha256": engine.binary_sha256,
        }
        receipt = BuildReceipt(
            path=engine.binary,
            sha256=canonical_receipt_sha256(receipt_content),
            content=receipt_content,
            engine_identity="quickjs-ng",
            source_repo="repo",
            source_revision="a" * 40,
            source_dirty=False,
            profile_id="macos-arm64-release-v1",
            binary_sha256=engine.binary_sha256,
        )
        engine = replace(engine, receipt=receipt)
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        output = io.StringIO()
        run = BenchmarkRun(manifest, [engine], list(manifest.cases), 1, 1, JsonlWriter(output), ROOT)
        self.addCleanup(run.close)
        self.assertFalse(run.comparison_input_preconditions_met)
        result = self._valid_result(manifest.cases[0], 2)
        with mock.patch("tools.benchmark.runner.run_process", return_value=result):
            run._sample(engine, manifest.cases[0], 2, "measurement", 0, 0, "eligible")
        record = json.loads(output.getvalue())
        self.assertTrue(record["measurement_eligible"])
        self.assertNotIn("claim_eligible", record)
        self.assertEqual(record["receipt"], receipt_content)
        self.assertEqual(
            record["receipt_sha256"], canonical_receipt_sha256(receipt_content)
        )
        self.assertIn("runner_repo", record)
        self.assertNotIn("git", record)

    def test_exact_verified_three_roles_meet_only_raw_input_preconditions(self) -> None:
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        body = "print('fake'); raise SystemExit(0)\n"
        engines = [
            self._verified(
                self._engine(
                    body,
                    role=role,
                    engine_identity="quickjs-ng" if role == "quickjs-ng" else "qjs-rust",
                ),
                manifest,
            )
            for role in ("candidate", "base", "quickjs-ng")
        ]
        run = BenchmarkRun(
            manifest, engines, list(manifest.cases), 1, 1, JsonlWriter(io.StringIO()), ROOT
        )
        self.addCleanup(run.close)
        self.assertTrue(run.comparison_input_preconditions_met)
        for engine in engines:
            for case in manifest.cases:
                run.measurement_counts[(engine.role, case.id)] = 1
                run.linearity_counts[(engine.role, case.id)] = 2
        self.assertTrue(run._comparison_input_complete())
        run.failed = True
        self.assertFalse(run._comparison_input_complete())

    def test_measurement_eligibility_does_not_predict_later_failure(self) -> None:
        engine = self._engine("raise SystemExit(1)\n")
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        output = io.StringIO()
        run = BenchmarkRun(manifest, [engine], [manifest.cases[0]], 2, 1, JsonlWriter(output), ROOT)
        self.addCleanup(run.close)
        results = [
            self._valid_result(manifest.cases[0], 2),
            self._process_result(exit_code=8),
        ]
        with mock.patch("tools.benchmark.runner.run_process", side_effect=results):
            run._sample(engine, manifest.cases[0], 2, "measurement", 0, 0, "eligible")
            run._sample(engine, manifest.cases[0], 2, "measurement", 1, 1, "eligible")
        records = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertTrue(records[0]["measurement_eligible"])
        self.assertFalse(records[1]["measurement_eligible"])
        self.assertNotIn("claim_eligible", records[0])

    def test_diagnostic_sample_is_not_measurement_eligible(self) -> None:
        engine = self._engine(
            "import json, sys\n"
            "case, n = sys.argv[-2], int(sys.argv[-1])\n"
            "print('QJS_BENCH_RESULT ' + json.dumps({'case_id': case, 'iterations': n, 'operations': n, 'checksum': n * (n + 1) // 2}))\n"
        )
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        output = io.StringIO()
        run = BenchmarkRun(manifest, [engine], list(manifest.cases), 1, 1, JsonlWriter(output), ROOT)
        self.addCleanup(run.close)
        result = self._valid_result(manifest.cases[0], 2)
        with mock.patch("tools.benchmark.runner.run_process", return_value=result):
            run._sample(engine, manifest.cases[0], 2, "calibration", None, None, "diagnostic")
        self.assertFalse(json.loads(output.getvalue())["measurement_eligible"])

    def test_single_role_full_portfolio_never_claims(self) -> None:
        engine = self._engine(
            "import json, sys\n"
            "case, n = sys.argv[-2], int(sys.argv[-1])\n"
            "factor = {'property_read': 3, 'array_read': 4}.get(case, 1)\n"
            "checksum = {'property_read': 6 * n, 'array_read': 10 * n}.get(case, n * (n + 1) // 2)\n"
            "print('QJS_BENCH_RESULT ' + json.dumps({'case_id': case, 'iterations': n, 'operations': factor * n, 'checksum': checksum}))\n"
        )
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        cases = [
            replace(
                case, initial_iterations=1, max_iterations=1, min_window_ms=1,
                startup_max_fraction=0.1, warmup_runs=0, timeout_seconds=1,
            )
            for case in manifest.cases
        ]
        output = io.StringIO()
        run = BenchmarkRun(manifest, [engine], cases, 1, 1, JsonlWriter(output), ROOT)
        self.addCleanup(run.close)
        case_by_id = {case.id: case for case in cases}

        def successful_sample(argv: list[str], _timeout: float) -> ProcessResult:
            case = case_by_id[argv[-2]]
            return self._valid_result(case, int(argv[-1]))

        with mock.patch("tools.benchmark.runner.run_process", side_effect=successful_sample):
            self.assertTrue(run.execute())
        rows = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertFalse(rows[0]["claim_eligible"])
        self.assertFalse(rows[0]["comparison_input_complete"])
        self.assertFalse(rows[-1]["claim_eligible"])
        self.assertFalse(rows[-1]["comparison_input_complete"])

    def test_binary_and_workload_mutation_after_run_creation_uses_snapshots(self) -> None:
        engine_temporary = tempfile.TemporaryDirectory()
        self.addCleanup(engine_temporary.cleanup)
        engine_path = Path(engine_temporary.name) / "engine"
        engine_path.write_text(
            "#!/bin/sh\n"
            "if [ \"$1\" = '--version' ] || [ \"$1\" = '-v' ]; then\n"
            "    printf '%s\\n' engine-a\n"
            "    exit 0\n"
            "fi\n"
            "n=$3\n"
            "checksum=$((n * (n + 1) / 2))\n"
            "printf 'QJS_BENCH_RESULT {\"case_id\":\"%s\",\"iterations\":%s,"
            "\"operations\":%s,\"checksum\":%s}\\n' \"$2\" \"$n\" \"$n\" \"$checksum\"\n",
            encoding="utf-8",
        )
        engine_path.chmod(0o755)
        engine = Engine(
            role="candidate",
            adapter_id="qjs-file",
            engine_identity="quickjs-ng",
            binary=engine_path.resolve(),
            binary_sha256=sha256_file(engine_path),
            receipt=None,
        )
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        workload = Path(temporary.name) / "workload.js"
        workload.write_bytes((ROOT / "benchmarks/workloads/core-micro.js").read_bytes())
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        case = replace(
            manifest.cases[0], workload=workload, workload_sha256=sha256_file(workload)
        )
        output = io.StringIO()
        probed_paths: list[Path] = []

        def malicious_snapshot_version(snapshot: Path) -> str:
            """Model a version handler that chmods and overwrites itself."""
            probed_paths.append(snapshot)
            self.assertNotEqual(snapshot, engine.binary)
            self.assertIn(b"engine-a", snapshot.read_bytes())
            snapshot.chmod(0o700)
            snapshot.write_bytes(b"#!/bin/sh\nexit 91\n")
            return "engine-a"

        with mock.patch(
            "tools.benchmark.runner.probe_version", side_effect=malicious_snapshot_version
        ):
            run = BenchmarkRun(manifest, [engine], [case], 1, 1, JsonlWriter(output), ROOT)
        self.addCleanup(run.close)
        binary_snapshot = run.engine_snapshots[engine.role]
        probe_snapshot = run.engine_probe_snapshots[engine.role]
        workload_snapshot = run.workload_snapshots[workload]
        self.assertEqual(probed_paths, [probe_snapshot])
        self.assertNotEqual(probe_snapshot, binary_snapshot)
        self.assertEqual(probe_snapshot.read_bytes(), b"#!/bin/sh\nexit 91\n")
        self.assertIn(b"engine-a", binary_snapshot.read_bytes())

        engine.binary.write_text("#!/bin/sh\nexit 9\n", encoding="utf-8")
        engine.binary.chmod(0o755)
        workload.write_text("throw new Error('mutated');\n", encoding="utf-8")

        def fake_run_process(argv: list[str], _timeout_seconds: float) -> ProcessResult:
            self.assertEqual(argv[0], str(binary_snapshot))
            self.assertEqual(argv[1], str(workload_snapshot))
            self.assertIn(b"engine-a", Path(argv[0]).read_bytes())
            self.assertEqual(
                Path(argv[1]).read_bytes(),
                (ROOT / "benchmarks/workloads/core-micro.js").read_bytes(),
            )
            return ProcessResult(
                started_at="2026-01-01T00:00:00+00:00",
                duration_ns=1_000_000,
                exit_code=0,
                timed_out=False,
                stdout=(
                    'QJS_BENCH_RESULT {"case_id":"plain_function_call","iterations":2,'
                    '"operations":2,"checksum":3}\n'
                ),
                stderr="",
                stdout_truncated=False,
                stderr_truncated=False,
            )

        with mock.patch("tools.benchmark.runner.run_process", side_effect=fake_run_process):
            _result, status, quality = run._sample(
                engine, case, 2, "measurement", 0, 0, "eligible"
            )
        self.assertEqual((status, quality), ("ok", "eligible"))
        record = json.loads(output.getvalue())
        self.assertEqual(record["argv"][0], str(binary_snapshot))
        self.assertEqual(record["argv"][1], str(workload_snapshot))
        self.assertEqual(record["binary_source_path"], str(engine.binary))
        self.assertEqual(record["binary_snapshot_path"], str(binary_snapshot))
        self.assertEqual(record["binary_version"], "engine-a")
        self.assertEqual(
            record["binary_version_probe_snapshot_path"], str(probe_snapshot)
        )
        self.assertEqual(
            record["binary_version_probe_pre_sha256"], engine.binary_sha256
        )
        self.assertEqual(
            record["binary_version_probe_post_sha256"], sha256_file(probe_snapshot)
        )
        self.assertNotEqual(
            record["binary_version_probe_pre_sha256"],
            record["binary_version_probe_post_sha256"],
        )
        self.assertEqual(record["workload_source_path"], str(workload))
        self.assertEqual(record["workload_snapshot_path"], str(workload_snapshot))
        self.assertEqual(record["binary_sha256"], sha256_file(binary_snapshot))
        self.assertEqual(record["workload_sha256"], sha256_file(workload_snapshot))

    def test_mutation_before_snapshot_fails_closed(self) -> None:
        engine = self._engine("print('engine-a')\n")
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        engine.binary.write_text("#!/usr/bin/env python3\nprint('engine-b')\n", encoding="utf-8")
        engine.binary.chmod(0o755)
        with self.assertRaisesRegex(SnapshotError, "snapshot hash mismatch"):
            BenchmarkRun(
                manifest, [engine], [manifest.cases[0]], 1, 1, JsonlWriter(io.StringIO()), ROOT
            )

    def test_workload_hash_failure_during_snapshot_fails_closed(self) -> None:
        engine = self._engine("print('engine-a')\n")
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        workload = Path(temporary.name) / "changed.js"
        workload.write_text("changed\n", encoding="utf-8")
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        case = replace(manifest.cases[0], workload=workload)
        with self.assertRaisesRegex(SnapshotError, "snapshot hash mismatch"):
            BenchmarkRun(manifest, [engine], [case], 1, 1, JsonlWriter(io.StringIO()), ROOT)


if __name__ == "__main__":
    unittest.main()
