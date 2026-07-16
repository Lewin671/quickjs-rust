from __future__ import annotations

import json
import hashlib
import tempfile
import unittest
from dataclasses import replace
from fractions import Fraction
from pathlib import Path

from tools.benchmark.schema import (
    ManifestError,
    load_manifest,
    next_calibration_iterations,
    sha256_file,
)


ROOT = Path(__file__).resolve().parents[3]


class ManifestTests(unittest.TestCase):
    def test_repository_manifest_and_hashes_validate(self) -> None:
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        self.assertEqual(manifest.schema_version, 4)
        self.assertEqual(manifest.series_id, "broad-black-box-v2")
        self.assertEqual(manifest.protocol_id, "quickjs-measurement-protocol-v6")
        self.assertEqual(manifest.lane_id, "throughput/wall_ns_per_operation")
        self.assertEqual(
            [case.id for case in manifest.cases],
            [
                "plain_function_call", "method_call", "captured_read",
                "captured_write", "many_locals_call", "property_read", "array_read",
                "function_call_two_args", "function_call_reordered",
                "top_level_function_call", "dynamic_method_call", "local_read",
                "global_read", "property_dynamic_read", "property_write",
                "array_dynamic_read", "array_write", "empty_loop",
                "branch_arithmetic", "math_abs", "array_index_of", "string_slice",
                "object_allocation", "array_allocation", "closure_allocation_call",
            ],
        )
        family_counts: dict[str, int] = {}
        for case in manifest.cases:
            family_counts[case.family] = family_counts.get(case.family, 0) + 1
        self.assertEqual(
            family_counts,
            {
                "call": 6,
                "binding": 5,
                "property": 3,
                "array": 3,
                "control": 2,
                "builtin": 2,
                "string": 1,
                "allocation": 3,
            },
        )
        self.assertTrue(all(case.critical for case in manifest.cases))
        cases = {case.id: case for case in manifest.cases}
        for case_id in ("property_write", "array_write"):
            self.assertEqual(cases[case_id].checksum_model, "triangular")
            self.assertEqual(cases[case_id].checksum_factor, 1)
            self.assertEqual(cases[case_id].expected_checksum(1024), 524800)
        self.assertTrue(
            all(
                abs(case.expected_checksum(case.max_iterations)) <= (2**53 - 1)
                for case in manifest.cases
            ),
            "every maximum-iteration checksum must remain an exact JavaScript number",
        )
        self.assertTrue(all(not Path(identifier).is_absolute() for identifier in manifest.protocol_file_ids))
        self.assertEqual(
            manifest.protocol_file_ids,
            (
                "benchmarks/workloads/broad-micro.js",
                "scripts/benchmark.sh",
                "tools/__init__.py",
                "tools/benchmark/__init__.py",
                "tools/benchmark/__main__.py",
                "tools/benchmark/adapters.py",
                "tools/benchmark/planning.py",
                "tools/benchmark/process.py",
                "tools/benchmark/receipts.py",
                "tools/benchmark/records.py",
                "tools/benchmark/runner.py",
                "tools/benchmark/schema.py",
                "tools/benchmark/snapshots.py",
            ),
        )
        self.assertTrue(all(sha256_file(case.workload) == case.workload_sha256 for case in manifest.cases))
        self.assertTrue(
            all(case.calibration_safety_factor == Fraction(5, 4) for case in manifest.cases)
        )

    def test_calibration_target_applies_factor_and_rounds_up(self) -> None:
        case = load_manifest(ROOT / "benchmarks/manifest.json").cases[0]
        case = replace(
            case,
            min_window_ms=1,
            startup_max_fraction=Fraction(3, 10),
            calibration_safety_factor=Fraction(11, 10),
        )
        self.assertEqual(case.calibration_target_ns(1_000_000), 3_666_667)

    def test_calibration_progression_is_proportional_monotonic_and_capped(self) -> None:
        self.assertEqual(
            next_calibration_iterations(100, 12_500_000, 11_000_000, 1_000),
            114,
        )
        self.assertEqual(next_calibration_iterations(100, 10**30, 1, 10_000), 1_600)
        self.assertEqual(next_calibration_iterations(100, 10**30, 1, 113), 113)

    def _temporary_manifest(self) -> tuple[tempfile.TemporaryDirectory[str], Path, dict]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        (root / "benchmarks/workloads").mkdir(parents=True)
        workload = root / "benchmarks/workloads/work.js"
        workload.write_text("0;\n", encoding="utf-8")
        relative_workload = "benchmarks/workloads/work.js"
        protocol_digest = hashlib.sha256()
        protocol_digest.update(relative_workload.encode())
        protocol_digest.update(b"\0")
        protocol_digest.update(bytes.fromhex(sha256_file(workload)))
        protocol_digest.update(b"\n")
        data = {
            "schema_version": 4,
            "series": {"id": "test", "suite_id": "test@1"},
            "lane": {"id": "throughput/wall_ns_per_operation"},
            "protocol": {
                "id": "test-protocol-v1",
                "files": [relative_workload],
                "aggregate_sha256": protocol_digest.hexdigest(),
            },
            "reference_engine": {
                "identity": "quickjs-ng",
                "source_repo": "https://example.invalid/quickjs-ng.git",
                "revision": "a" * 40,
            },
            "profile": {
                "id": "test", "platform": "test-host",
            },
            "build_recipes": [
                {
                    "engine_identity": identity,
                    "build_mode": "release",
                    "toolchain": f"{identity}-toolchain",
                    "target": "test-target",
                    "features": [],
                    "flags": [],
                    "lto": "off",
                    "strip": "none",
                    "allocator": "system",
                    "host_features": "baseline",
                }
                for identity in ("qjs-rust", "quickjs-ng")
            ],
            "cases": [{
                "id": "case", "family": "family", "critical": True,
                "workload": "benchmarks/workloads/work.js",
                "workload_sha256": sha256_file(workload),
                "operations_per_iteration": 1,
                "checksum": {"model": "linear", "factor": 1},
                "measurement": {
                    "initial_iterations": 1, "max_iterations": 2, "min_window_ms": 1,
                    "startup_max_fraction": 0.01, "calibration_safety_factor": 1.25,
                    "warmup_runs": 0, "timeout_seconds": 1,
                },
            }],
        }
        path = root / "benchmarks/manifest.json"
        return temporary, path, data

    def test_unknown_field_fails_closed(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        data["surprise"] = True
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "unknown"):
            load_manifest(path)

    def test_calibration_safety_factor_is_required_and_bounded(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        del data["cases"][0]["measurement"]["calibration_safety_factor"]
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "missing.*calibration_safety_factor"):
            load_manifest(path)

        data["cases"][0]["measurement"]["calibration_safety_factor"] = 0.99
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "1 <= value <= 4"):
            load_manifest(path)

        data["cases"][0]["measurement"]["calibration_safety_factor"] = 4.01
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "1 <= value <= 4"):
            load_manifest(path)

        data["cases"][0]["measurement"]["calibration_safety_factor"] = 1e309
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "non-standard numeric constant"):
            load_manifest(path)

    def test_exact_decimal_bounds_do_not_round_through_float(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        encoded = json.dumps(data)
        path.write_text(
            encoded.replace(
                '"calibration_safety_factor": 1.25',
                '"calibration_safety_factor": 0.99999999999999999',
            ),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ManifestError, "1 <= value <= 4"):
            load_manifest(path)

        path.write_text(
            encoded.replace(
                '"startup_max_fraction": 0.01',
                '"startup_max_fraction": 0.10000000000000001',
            ),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ManifestError, "0 < value <= 0.1"):
            load_manifest(path)

    def test_extreme_numbers_fail_as_manifest_errors(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        data["cases"][0]["measurement"]["calibration_safety_factor"] = 10**1000
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "1 <= value <= 4"):
            load_manifest(path)

        data["cases"][0]["measurement"]["calibration_safety_factor"] = 1.25
        encoded = json.dumps(data).replace(
            '"calibration_safety_factor": 1.25',
            '"calibration_safety_factor": 1e308',
        )
        path.write_text(encoded, encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "1 <= value <= 4"):
            load_manifest(path)

        encoded = json.dumps(data).replace(
            '"calibration_safety_factor": 1.25',
            '"calibration_safety_factor": 1e99999999999999999999',
        )
        path.write_text(encoded, encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "cannot read manifest"):
            load_manifest(path)

    def test_duplicate_json_key_fails_closed(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        encoded = json.dumps(data)
        path.write_text(encoded.replace('{"schema_version": 4,', '{"schema_version": 4, "schema_version": 4,'), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "duplicate key"):
            load_manifest(path)

    def test_hash_mismatch_fails_closed(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        data["cases"][0]["workload_sha256"] = "0" * 64
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "mismatch"):
            load_manifest(path)

    def test_protocol_hash_mismatch_fails_closed(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        data["protocol"]["aggregate_sha256"] = "0" * 64
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "protocol.*mismatch"):
            load_manifest(path)

    def test_protocol_file_edit_requires_manifest_update(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        path.write_text(json.dumps(data), encoding="utf-8")
        (path.parent / "workloads/work.js").write_text("1;\n", encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "protocol.*mismatch"):
            load_manifest(path)

    def test_boolean_schema_version_fails_closed(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        data["schema_version"] = True
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "integer version"):
            load_manifest(path)

    def test_duplicate_id_and_escape_fail_closed(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        data["cases"].append(dict(data["cases"][0]))
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "duplicate"):
            load_manifest(path)
        data["cases"] = [data["cases"][0]]
        data["cases"][0]["workload"] = "../outside.js"
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "inside"):
            load_manifest(path)


if __name__ == "__main__":
    unittest.main()
