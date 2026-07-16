from __future__ import annotations

import io
import json
import shutil
import statistics
import subprocess
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from unittest import mock

from tools.benchmark.adapters import load_engine
from tools.benchmark.analysis_schema import load_analysis_manifest
from tools.benchmark.process import ProcessResult
from tools.benchmark.receipts import BuildReceipt, canonical_receipt_sha256
from tools.benchmark.report import ReportError, build_report
from tools.benchmark.runner import BenchmarkRun, JsonlWriter
from tools.benchmark.schema import load_manifest


ROOT = Path(__file__).resolve().parents[3]


class ReportTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        loaded_analysis = load_analysis_manifest(
            ROOT / "benchmarks/analysis.json", self.manifest
        )
        self.analysis = replace(loaded_analysis, bootstrap_samples=200)
        self.input = self.directory / "raw.jsonl"
        self._write_rows(self._complete_rows())

    def _engine(self, role: str):
        path = self.directory / f"{role}-engine"
        path.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        path.chmod(0o755)
        identity = "quickjs-ng" if role == "quickjs-ng" else "qjs-rust"
        engine = load_engine(role, "qjs-file", identity, path)
        recipe = self.manifest.build_recipes[identity]
        source_repo = (
            self.manifest.reference_repo
            if role == "quickjs-ng" else "https://example.invalid/qjs-rust.git"
        )
        revision = self.manifest.reference_revision if role == "quickjs-ng" else "a" * 40
        content = {
            "schema_version": 1,
            "engine_identity": identity,
            "source": {"repo": source_repo, "revision": revision, "dirty": False},
            "profile_id": self.manifest.profile.id,
            "build": {
                key: value for key, value in recipe.__dict__.items()
                if key != "engine_identity"
            },
            "binary_sha256": engine.binary_sha256,
        }
        receipt = BuildReceipt(
            path=path,
            sha256=canonical_receipt_sha256(content),
            content=content,
            engine_identity=identity,
            source_repo=source_repo,
            source_revision=revision,
            source_dirty=False,
            profile_id=self.manifest.profile.id,
            binary_sha256=engine.binary_sha256,
        )
        return replace(engine, receipt=receipt)

    def _complete_rows(self, blocks: int = 2, startup_ns: int = 1_000_000) -> list[dict]:
        engines = [self._engine(role) for role in ("candidate", "base", "quickjs-ng")]
        per_op = {"candidate": 120, "base": 100, "quickjs-ng": 150}
        case_by_id = {case.id: case for case in self.manifest.cases}

        def process(argv: list[str], _timeout: float) -> ProcessResult:
            role = Path(argv[0]).name.split("-")[0]
            # Snapshot names preserve the role as their directory component.
            for known_role in per_op:
                if known_role in argv[0]:
                    role = known_role
                    break
            case = case_by_id[argv[-2]]
            iterations = int(argv[-1])
            operations = case.expected_operations(iterations)
            duration = startup_ns if iterations == 0 else per_op[role] * operations
            return ProcessResult(
                started_at="2026-01-01T00:00:00+00:00",
                duration_ns=duration,
                exit_code=0,
                timed_out=False,
                stdout=(
                    "QJS_BENCH_RESULT "
                    + json.dumps({
                        "case_id": case.id,
                        "iterations": iterations,
                        "operations": operations,
                        "checksum": case.expected_checksum(iterations),
                    }, separators=(",", ":"))
                    + "\n"
                ),
                stderr="",
                stdout_truncated=False,
                stderr_truncated=False,
            )

        output = io.StringIO()
        with mock.patch("tools.benchmark.runner.probe_version", return_value="test"), mock.patch(
            "tools.benchmark.runner.run_process", side_effect=process
        ):
            run = BenchmarkRun(
                self.manifest, engines, list(self.manifest.cases), blocks, 11,
                JsonlWriter(output), ROOT,
            )
            self.assertTrue(run.execute())
        return [json.loads(line) for line in output.getvalue().splitlines()]

    def _write_rows(self, rows: list[dict]) -> None:
        self.input.write_text(
            "".join(json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n" for row in rows),
            encoding="utf-8",
        )

    def _build_report(self):
        return build_report(self.input, self.manifest, self.analysis)

    def _retarget(self, row: dict, iterations: int) -> None:
        case = next(case for case in self.manifest.cases if case.id == row["case_id"])
        row["iterations"] = iterations
        row["operations"] = case.expected_operations(iterations)
        row["checksum"] = case.expected_checksum(iterations)
        row["argv"][-1] = str(iterations)
        row["stdout"] = (
            "QJS_BENCH_RESULT "
            + json.dumps({
                "case_id": case.id,
                "iterations": iterations,
                "operations": row["operations"],
                "checksum": row["checksum"],
            }, separators=(",", ":"))
            + "\n"
        )

    def test_complete_report_is_deterministic_and_paired(self) -> None:
        raw_start = json.loads(self.input.read_text(encoding="utf-8").splitlines()[0])
        self.assertEqual(raw_start["protocol_files"], list(self.manifest.protocol_file_ids))
        self.assertEqual(raw_start["lane_id"], "throughput/wall_ns_per_operation")
        self.assertTrue(all(not Path(value).is_absolute() for value in raw_start["protocol_files"]))
        first = self._build_report()
        second = self._build_report()
        self.assertEqual(first, second)
        self.assertFalse(first["claim_eligible"])
        self.assertEqual(
            first["measurement_contract"]["lane_id"],
            "throughput/wall_ns_per_operation",
        )
        self.assertTrue(first["coverage"]["physical_plan_complete"])
        self.assertTrue(first["coverage"]["comparison_input_complete"])
        self.assertEqual(first["coverage"]["runner_end_status"], "complete")
        self.assertEqual(first["health"]["status"], "inconclusive")
        comparison = first["comparisons"]["candidate_vs_base"]
        self.assertAlmostEqual(comparison["overall"]["ratio"], 1.2)
        self.assertAlmostEqual(comparison["cases"]["plain_function_call"]["ratio"], 1.2)
        interval = comparison["overall"]["confidence_interval"]
        self.assertAlmostEqual(interval["lower"], 1.2)
        self.assertAlmostEqual(interval["upper"], 1.2)

    def test_narrow_thirty_block_report_is_healthy(self) -> None:
        self._write_rows(self._complete_rows(30))
        report = self._build_report()
        self.assertEqual(report["health"]["status"], "healthy")
        self.assertEqual(report["health"]["blocks"]["requested"], 30)
        self.assertEqual(report["health"]["blocks"]["valid"], 30)
        self.assertEqual(report["health"]["blocks"]["invalid"], 0)
        self.assertEqual(
            report["coverage"]["attempted_measurement_records"],
            len(self.manifest.cases) * 3 * 30,
        )
        self.assertEqual(report["health"]["policy"]["retry_policy"], "never")
        self.assertEqual(report["health"]["policy"]["outlier_policy"], "retain")
        self.assertFalse(report["claim_eligible"])

    def test_wide_thirty_and_sixty_block_reports_apply_bounded_policy(self) -> None:
        for blocks, expected in ((30, "extension_required"), (60, "inconclusive")):
            with self.subTest(blocks=blocks):
                rows = self._complete_rows(blocks)
                for row in rows:
                    if (
                        row.get("phase") == "measurement"
                        and row["role"] == "candidate" and row["block"] % 2
                    ):
                        row["duration_ns"] *= 2
                self._write_rows(rows)
                report = self._build_report()
                self.assertEqual(report["health"]["status"], expected)
                extension = report["health"]["precision"]["extension_block_ids"]
                self.assertEqual(extension, list(range(30, 60)) if blocks == 30 else [])
                self.assertGreater(
                    report["health"]["precision"][
                        "maximum_critical_family_relative_half_width"
                    ],
                    0.03,
                )

    def test_timing_quality_is_recomputed_at_both_boundaries(self) -> None:
        case = next(
            case for case in self.manifest.cases if case.id == "plain_function_call"
        )
        minimum_duration = case.min_window_ms * 1_000_000
        startup_boundary = int(minimum_duration * case.startup_max_fraction)
        rows = self._complete_rows(startup_ns=startup_boundary)
        measurement = next(
            row for row in rows
            if row.get("phase") == "measurement"
            and row["role"] == "candidate"
            and row["case_id"] == "plain_function_call"
        )
        measurement["duration_ns"] = minimum_duration
        self._write_rows(rows)
        self.assertTrue(self._build_report()["coverage"]["comparison_input_complete"])

        measurement["duration_ns"] = minimum_duration - 1
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "timing quality mismatch"):
            self._build_report()

        rows = self._complete_rows(startup_ns=startup_boundary + 1)
        measurement = next(
            row for row in rows
            if row.get("phase") == "measurement"
            and row["role"] == "candidate"
            and row["case_id"] == "plain_function_call"
        )
        measurement["duration_ns"] = minimum_duration
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "timing quality mismatch"):
            self._build_report()

    def test_same_raw_bytes_are_report_path_independent(self) -> None:
        first = self._build_report()
        renamed = self.directory / "another-worktree" / "renamed-evidence.data"
        renamed.parent.mkdir()
        renamed.write_bytes(self.input.read_bytes())
        second = build_report(renamed, self.manifest, self.analysis)
        self.assertEqual(first, second)
        self.assertEqual(set(first["input"]), {"sha256", "byte_length"})

    def test_analysis_manifest_can_change_without_invalidating_raw(self) -> None:
        repository_analysis = json.loads(
            (ROOT / "benchmarks/analysis.json").read_text(encoding="utf-8")
        )
        repository_analysis["bootstrap"]["samples"] = 201
        temporary_root = self.directory / "analysis-revision"
        analysis_path = temporary_root / "benchmarks/analysis.json"
        analysis_path.parent.mkdir(parents=True)
        for relative in repository_analysis["protocol"]["files"]:
            destination = temporary_root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, destination)
        analysis_path.write_text(json.dumps(repository_analysis), encoding="utf-8")
        revised = load_analysis_manifest(analysis_path, self.manifest)
        original_report = self._build_report()
        revised_report = build_report(self.input, self.manifest, revised)
        self.assertEqual(
            original_report["measurement_contract"],
            revised_report["measurement_contract"],
        )
        self.assertEqual(original_report["input"], revised_report["input"])
        self.assertNotEqual(
            original_report["analysis_contract"]["manifest_sha256"],
            revised_report["analysis_contract"]["manifest_sha256"],
        )

    def test_provenance_metadata_types_fail_closed(self) -> None:
        rows = self._complete_rows()
        role = rows[0]["engines"][0]["role"]
        rows[0]["engines"][0]["binary_version_probe_post_sha256"] = None
        rows[0]["engines"][0]["binary_version"] = None
        for row in rows:
            if row.get("record_type") == "sample" and row["role"] == role:
                row["binary_version_probe_post_sha256"] = None
                row["binary_version"] = None
        self._write_rows(rows)
        self.assertTrue(self._build_report()["coverage"]["comparison_input_complete"])

        for field, value, expected in (
            ("binary_version_probe_post_sha256", "bad", "post_sha256"),
            ("binary_version", "", "binary_version"),
            ("binary_version", "v" * 513, "binary_version"),
            ("binary_source_path", "", "binary_source_path"),
            ("binary_snapshot_path", "", "binary_snapshot_path"),
            ("binary_version_probe_snapshot_path", "", "probe_snapshot_path"),
        ):
            rows = self._complete_rows()
            rows[0]["engines"][0][field] = value
            self._write_rows(rows)
            with self.assertRaisesRegex(ReportError, expected):
                self._build_report()

        rows = self._complete_rows()
        sample = next(row for row in rows if row.get("record_type") == "sample")
        sample["workload_source_path"] = ""
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "workload_source_path"):
            self._build_report()

        rows = self._complete_rows()
        role = rows[0]["engines"][0]["role"]
        rows[0]["engines"][0]["receipt_sha256"] = "0" * 64
        for row in rows:
            if row.get("record_type") == "sample" and row["role"] == role:
                row["receipt_sha256"] = "0" * 64
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "canonical content digest mismatch"):
            self._build_report()

    def test_host_runner_repo_and_coverage_tampering_fail_closed(self) -> None:
        mutations = (
            (lambda rows: rows[0]["host"].__setitem__("extra", "value"), "run_start.host"),
            (lambda rows: rows[0]["host"].__setitem__("machine", 1), "host.machine"),
            (lambda rows: rows[0]["runner_repo"].__setitem__("extra", None), "runner_repo"),
            (lambda rows: rows[0]["runner_repo"].__setitem__("commit", True), "commit"),
            (lambda rows: rows[0]["runner_repo"].__setitem__("commit", "A" * 40), "commit"),
            (lambda rows: rows[0]["runner_repo"].__setitem__("dirty", "false"), "dirty"),
            (lambda rows: rows[0]["coverage"].__setitem__("extra", 0), "coverage"),
            (lambda rows: rows[0]["coverage"].__setitem__("common", True), "coverage.common"),
            (
                lambda rows: rows[0]["coverage"]["measured_by_role"].__setitem__(
                    "candidate", 1
                ),
                "run_start.coverage",
            ),
            (
                lambda rows: rows[-1]["coverage"]["measured_by_role"].pop("quickjs-ng"),
                "measured_by_role",
            ),
            (lambda rows: rows[-1]["coverage"].__setitem__("common", 6), "run_end.coverage"),
        )
        for mutate, expected in mutations:
            with self.subTest(expected=expected):
                rows = self._complete_rows()
                mutate(rows)
                self._write_rows(rows)
                with self.assertRaisesRegex(ReportError, expected):
                    self._build_report()

    def test_duplicate_json_key_and_duplicate_record_fail_closed(self) -> None:
        encoded = self.input.read_text(encoding="utf-8")
        self.input.write_text(
            encoded.replace('"record_type":"run_start"', '"record_type":"run_start","run_id":"duplicate"', 1),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ReportError, "duplicate key"):
            self._build_report()

        rows = self._complete_rows()
        measurement = next(row for row in rows if row.get("phase") == "measurement")
        rows.insert(-1, measurement)
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "duplicate"):
            self._build_report()

    def test_missing_block_and_dynamic_intersection_fail_closed(self) -> None:
        rows = self._complete_rows()
        rows = [
            row for row in rows
            if not (
                row.get("phase") == "measurement"
                and row["role"] == "candidate"
                and row["case_id"] == "captured_read"
                and row["block"] == 1
            )
        ]
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "physical plan|portfolio mismatch"):
            self._build_report()

    def test_timer_limited_is_durable_and_identity_mismatch_fails_closed(self) -> None:
        rows = self._complete_rows()
        measurement = next(row for row in rows if row.get("phase") == "measurement")
        case = next(
            case for case in self.manifest.cases
            if case.id == measurement["case_id"]
        )
        measurement["measurement_eligible"] = False
        measurement["quality"] = "timer_limited"
        measurement["duration_ns"] = case.min_window_ms * 1_000_000 - 1
        role = measurement["role"]
        rows[-1]["comparison_input_complete"] = False
        rows[-1]["coverage"]["measured_by_role"][role] -= 1
        rows[-1]["coverage"]["common"] -= 1
        self._write_rows(rows)
        report = self._build_report()
        invalid = report["health"]["blocks"]["invalid_blocks"]
        self.assertEqual(len(invalid), 1)
        self.assertEqual(invalid[0]["triggers"][0]["reason"], "timer_limited")
        records_per_block = len(self.manifest.cases) * 3
        self.assertEqual(
            report["coverage"]["valid_measurement_records"], records_per_block
        )
        self.assertEqual(
            report["coverage"]["invalid_measurement_records"], records_per_block
        )
        self.assertEqual(
            {trigger["case_id"] for trigger in invalid[0]["triggers"]},
            {measurement["case_id"]},
        )

        rows = self._complete_rows()
        rows[1]["series_id"] = "wrong-series"
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "identity mismatch"):
            self._build_report()

    def test_measurement_failure_states_are_classified_without_losing_plan(self) -> None:
        mutations = {
            "failed": {
                "exit_code": 7, "timed_out": False,
                "error": "engine exited with status 7", "stdout": "",
            },
            "timeout": {
                "exit_code": -15, "timed_out": True,
                "error": "timed out after 30s", "stdout": "",
            },
            "invalid": {
                "exit_code": 0, "timed_out": False,
                "error": "expected exactly one QJS_BENCH_RESULT line, got 0",
                "stdout": "garbage\n",
            },
            "spawn_failed": {
                "exit_code": None, "timed_out": False,
                "error": "engine could not start: Exec format error",
                "stdout": "", "stderr": "Exec format error",
            },
        }
        expected_reason = {
            "failed": "engine_failed", "timeout": "timeout", "invalid": "invalid_result",
            "spawn_failed": "spawn_failed",
        }
        for status, fields in mutations.items():
            with self.subTest(status=status):
                rows = self._complete_rows()
                sample = next(row for row in rows if row.get("phase") == "measurement")
                sample.update(fields)
                sample.update({
                    "status": "failed" if status == "spawn_failed" else status,
                    "quality": "ineligible",
                    "measurement_eligible": False, "operations": None, "checksum": None,
                })
                role = sample["role"]
                rows[-1]["status"] = "failed"
                rows[-1]["comparison_input_complete"] = False
                rows[-1]["coverage"]["measured_by_role"][role] -= 1
                rows[-1]["coverage"]["common"] -= 1
                self._write_rows(rows)
                report = self._build_report()
                triggers = report["health"]["blocks"]["invalid_blocks"][0]["triggers"]
                self.assertEqual(triggers[0]["reason"], expected_reason[status])

                sample["error"] = "forged"
                self._write_rows(rows)
                with self.assertRaises(ReportError):
                    self._build_report()

    def test_failed_setup_and_not_run_measurements_make_health_invalid(self) -> None:
        rows = self._complete_rows()
        role = "candidate"
        case_id = "plain_function_call"
        diagnostics = [
            row for row in rows
            if row.get("record_type") == "sample"
            and row["role"] == role and row["case_id"] == case_id
            and row["phase"] != "measurement"
        ]
        first = diagnostics[0]
        first.update({
            "status": "failed", "quality": "ineligible",
            "measurement_eligible": False, "exit_code": 9,
            "error": "engine exited with status 9", "operations": None,
            "checksum": None, "stdout": "",
        })
        removed = set(id(row) for row in diagnostics[1:])
        rows = [row for row in rows if id(row) not in removed]
        for row in rows:
            if (
                row.get("phase") == "measurement" and row["role"] == role
                and row["case_id"] == case_id
            ):
                row.update({
                    "argv": [], "iterations": None, "operations": None, "checksum": None,
                    "duration_ns": None, "started_at": None, "exit_code": None,
                    "timed_out": False, "stdout": "", "stderr": "",
                    "stdout_truncated": False, "stderr_truncated": False,
                    "error": "startup/calibration/warmup did not complete",
                    "status": "not_run", "quality": "ineligible",
                    "measurement_eligible": False,
                })
        rows[-1]["status"] = "failed"
        rows[-1]["comparison_input_complete"] = False
        rows[-1]["coverage"]["measured_by_role"][role] -= 1
        rows[-1]["coverage"]["common"] -= 1
        self._write_rows(rows)
        report = self._build_report()
        self.assertEqual(report["health"]["status"], "invalid")
        self.assertEqual(report["health"]["linearity"]["status"], "fail")
        self.assertEqual(len(report["health"]["blocks"]["invalid_blocks"]), 2)
        self.assertTrue(report["coverage"]["physical_plan_complete"])
        self.assertFalse(report["coverage"]["comparison_input_complete"])
        self.assertEqual(report["coverage"]["runner_end_status"], "failed")

        first["iterations"] = 1
        first["argv"][-1] = "1"
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "diagnostic iterations mismatch"):
            self._build_report()

    def test_failure_after_reached_calibration_target_is_unreachable(self) -> None:
        rows = self._complete_rows()
        role = "candidate"
        case_id = "plain_function_call"
        pair_diagnostics = [
            row for row in rows
            if row.get("record_type") == "sample" and row["role"] == role
            and row["case_id"] == case_id and row["phase"] != "measurement"
        ]
        reached = [
            row for row in pair_diagnostics if row["phase"] == "calibration"
        ][-1]
        impossible_failure = dict(reached)
        impossible_failure.update({
            "status": "failed", "quality": "ineligible",
            "measurement_eligible": False, "exit_code": 9,
            "error": "engine exited with status 9", "operations": None,
            "checksum": None, "stdout": "",
        })
        discarded = {
            id(row) for row in pair_diagnostics
            if row["phase"] in {"warmup", "linearity"}
        }
        rewritten = []
        for row in rows:
            if id(row) in discarded:
                continue
            rewritten.append(row)
            if row is reached:
                rewritten.append(impossible_failure)
        rows = rewritten
        for row in rows:
            if (
                row.get("phase") == "measurement" and row["role"] == role
                and row["case_id"] == case_id
            ):
                row.update({
                    "argv": [], "iterations": None, "operations": None, "checksum": None,
                    "duration_ns": None, "started_at": None, "exit_code": None,
                    "timed_out": False, "stdout": "", "stderr": "",
                    "stdout_truncated": False, "stderr_truncated": False,
                    "error": "startup/calibration/warmup did not complete",
                    "status": "not_run", "quality": "ineligible",
                    "measurement_eligible": False,
                })
        rows[-1]["status"] = "failed"
        rows[-1]["comparison_input_complete"] = False
        rows[-1]["coverage"]["measured_by_role"][role] -= 1
        rows[-1]["coverage"]["common"] -= 1
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "diagnostic state mismatch"):
            self._build_report()

    def test_raw_replay_requires_safety_adjusted_calibration_target(self) -> None:
        rows = self._complete_rows()
        role = "candidate"
        case_id = "plain_function_call"
        calibration = [
            row for row in rows
            if row.get("phase") == "calibration" and row["role"] == role
            and row["case_id"] == case_id
        ]
        self.assertGreaterEqual(len(calibration), 2)
        old_last, adjusted_last = calibration[-2:]
        case = next(case for case in self.manifest.cases if case.id == case_id)
        startup = [
            row["duration_ns"] for row in rows
            if row.get("phase") == "startup" and row["role"] == role
            and row["case_id"] == case_id
        ]
        old_target = max(
            case.min_window_ms * 1_000_000,
            statistics.median(startup) / case.startup_max_fraction,
        )
        adjusted_target = case.calibration_target_ns(int(statistics.median(startup)))
        self.assertLess(old_target, adjusted_target)
        self.assertLess(old_last["duration_ns"], adjusted_target)
        self.assertGreaterEqual(adjusted_last["duration_ns"], adjusted_target)
        rows.remove(adjusted_last)
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "expected calibration, got warmup"):
            self._build_report()

    def test_each_reachable_diagnostic_failure_phase_is_accepted(self) -> None:
        role = "candidate"
        case_id = "plain_function_call"
        for phase in (
            "startup", "calibration", "warmup", "linearity",
        ):
            with self.subTest(phase=phase):
                rows = self._complete_rows()
                diagnostics = [
                    row for row in rows
                    if row.get("record_type") == "sample" and row["role"] == role
                    and row["case_id"] == case_id and row["phase"] != "measurement"
                ]
                target = next(row for row in diagnostics if row["phase"] == phase)
                target_index = next(
                    index for index, row in enumerate(diagnostics) if row is target
                )
                later = {
                    id(row) for index, row in enumerate(diagnostics)
                    if index > target_index
                }
                rows = [row for row in rows if id(row) not in later]
                target.update({
                    "status": "failed", "quality": "ineligible",
                    "measurement_eligible": False, "exit_code": 9,
                    "error": "engine exited with status 9", "operations": None,
                    "checksum": None, "stdout": "",
                })
                for row in rows:
                    if (
                        row.get("phase") == "measurement" and row["role"] == role
                        and row["case_id"] == case_id
                    ):
                        row.update({
                            "argv": [], "iterations": None, "operations": None,
                            "checksum": None, "duration_ns": None, "started_at": None,
                            "exit_code": None, "timed_out": False, "stdout": "",
                            "stderr": "", "stdout_truncated": False,
                            "stderr_truncated": False,
                            "error": "startup/calibration/warmup did not complete",
                            "status": "not_run", "quality": "ineligible",
                            "measurement_eligible": False,
                        })
                rows[-1]["status"] = "failed"
                rows[-1]["comparison_input_complete"] = False
                rows[-1]["coverage"]["measured_by_role"][role] -= 1
                rows[-1]["coverage"]["common"] -= 1
                self._write_rows(rows)
                report = self._build_report()
                failure = report["health"]["linearity"]["roles"][role][case_id][
                    "execution_failure"
                ]
                self.assertEqual(failure["phase"], phase)

    def test_record_and_phase_order_fail_closed(self) -> None:
        rows = self._complete_rows()
        rows[0], rows[1] = rows[1], rows[0]
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "record order"):
            self._build_report()

        rows = self._complete_rows()
        first_index = next(
            i
            for i, row in enumerate(rows)
            if row.get("diagnostic_point") == "2n:0"
        )
        second_index = next(
            i for i, row in enumerate(rows)
            if row.get("diagnostic_point") == "2n:1"
            and row["role"] == rows[first_index]["role"]
            and row["case_id"] == rows[first_index]["case_id"]
        )
        rows[first_index], rows[second_index] = rows[second_index], rows[first_index]
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "diagnostic point mismatch"):
            self._build_report()

    def test_seed_and_seeded_measurement_plan_fail_closed(self) -> None:
        rows = self._complete_rows()
        rows[0]["seed"] = True
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "seed: expected integer"):
            self._build_report()

        rows = self._complete_rows()
        self._write_rows(rows)
        self.input.write_text(
            self.input.read_text(encoding="utf-8").replace('"seed":11', '"seed":NaN', 1),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ReportError, "non-standard numeric constant"):
            self._build_report()

        rows = self._complete_rows()
        indices = [i for i, row in enumerate(rows) if row.get("phase") == "measurement"][:2]
        rows[indices[0]]["order"], rows[indices[1]]["order"] = (
            rows[indices[1]]["order"], rows[indices[0]]["order"]
        )
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "seeded physical plan"):
            self._build_report()

        rows = self._complete_rows()
        rows[indices[0]], rows[indices[1]] = rows[indices[1]], rows[indices[0]]
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "seeded physical plan"):
            self._build_report()

    def test_phase_iteration_bindings_fail_closed(self) -> None:
        rows = self._complete_rows()
        startup = next(row for row in rows if row.get("phase") == "startup")
        self._retarget(startup, 1)
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "startup requires zero"):
            self._build_report()

        rows = self._complete_rows()
        measurements = [
            row for row in rows
            if row.get("phase") == "measurement"
            and row["role"] == "candidate"
            and row["case_id"] == "plain_function_call"
        ]
        self._retarget(measurements[-1], measurements[-1]["iterations"] + 1)
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "measurement iterations mismatch"):
            self._build_report()

        rows = self._complete_rows()
        point = next(row for row in rows if row.get("diagnostic_point") == "n:0")
        self._retarget(point, point["iterations"] - 1)
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "diagnostic iterations mismatch"):
            self._build_report()

        for phase, expected in (
            ("warmup", "diagnostic iterations mismatch"),
            ("calibration", "calibration"),
        ):
            rows = self._complete_rows()
            matches = [row for row in rows if row.get("phase") == phase]
            row = matches[-1]
            self._retarget(row, row["iterations"] + 1)
            self._write_rows(rows)
            with self.assertRaisesRegex(ReportError, expected):
                self._build_report()

    def test_success_record_binds_exit_error_and_stdout(self) -> None:
        for field, value, expected in (
            ("exit_code", 7, "exit_code"),
            ("error", "hidden failure", "error"),
            ("stdout", "garbage\n", "stdout"),
        ):
            rows = self._complete_rows()
            measurement = next(row for row in rows if row.get("phase") == "measurement")
            measurement[field] = value
            self._write_rows(rows)
            with self.assertRaisesRegex(ReportError, expected):
                self._build_report()

    def test_linearity_missing_duplicate_invalid_and_failed_health(self) -> None:
        rows = self._complete_rows()
        point = next(row for row in rows if row.get("diagnostic_point") == "n:0")
        rows.remove(point)
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "diagnostic iterations mismatch"):
            self._build_report()

        rows = self._complete_rows()
        point = next(row for row in rows if row.get("diagnostic_point") == "n:0")
        rows.insert(rows.index(point) + 1, dict(point))
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "duplicate linearity"):
            self._build_report()

        rows = self._complete_rows()
        point = next(row for row in rows if row.get("diagnostic_point") == "2n:0")
        point["diagnostic_point"] = "n"
        self._write_rows(rows)
        with self.assertRaisesRegex(ReportError, "invalid linearity point"):
            self._build_report()

        rows = self._complete_rows()
        points = [
            row
            for row in rows
            if row.get("diagnostic_point") in {"2n:0", "2n:1"}
            and row["role"] == "candidate"
            and row["case_id"] == "plain_function_call"
        ]
        self.assertEqual(len(points), 2)
        for point in points:
            point["duration_ns"] *= 3
        self._write_rows(rows)
        report = self._build_report()
        self.assertEqual(report["health"]["status"], "invalid")
        self.assertFalse(report["claim_eligible"])

    def test_linearity_median_retains_one_fixed_probe_outlier(self) -> None:
        rows = self._complete_rows()
        point = next(
            row
            for row in rows
            if row.get("diagnostic_point") == "2n:0"
            and row["role"] == "candidate"
            and row["case_id"] == "plain_function_call"
        )
        point["duration_ns"] *= 3
        self._write_rows(rows)
        report = self._build_report()
        self.assertEqual(report["health"]["linearity"]["status"], "pass")
        self.assertEqual(report["health"]["status"], "inconclusive")

    def test_report_cli_writes_atomically_and_refuses_overwrite(self) -> None:
        # Use the repository contract for the subprocess by regenerating raw
        # records with its unchanged 20k bootstrap setting.
        repository_manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        self.manifest = repository_manifest
        self._write_rows(self._complete_rows())
        output = self.directory / "report.json"
        command = [
            str(ROOT / "scripts/benchmark-report.sh"),
            "--analysis-manifest", str(ROOT / "benchmarks/analysis.json"),
            "--input", str(self.input), "--output", str(output),
        ]
        completed = subprocess.run(command, capture_output=True, text=True, timeout=30, check=False)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(json.loads(output.read_text())["schema_id"], "quickjs-benchmark-report")
        rejected = subprocess.run(command, capture_output=True, text=True, timeout=30, check=False)
        self.assertEqual(rejected.returncode, 2)
        self.assertIn("refusing to overwrite", rejected.stderr)

        invalid_output = self.directory / "invalid.json"
        self.input.write_text("{}\n", encoding="utf-8")
        invalid = subprocess.run(
            [
                str(ROOT / "scripts/benchmark-report.sh"),
                "--input", str(self.input), "--output", str(invalid_output),
            ],
            capture_output=True, text=True, timeout=10, check=False,
        )
        self.assertEqual(invalid.returncode, 2)
        self.assertFalse(invalid_output.exists())


if __name__ == "__main__":
    unittest.main()
