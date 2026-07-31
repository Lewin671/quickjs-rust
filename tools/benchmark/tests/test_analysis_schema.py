from __future__ import annotations

import ast
import json
import shutil
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

from tools.benchmark.analysis_schema import AnalysisManifestError, load_analysis_manifest
from tools.benchmark.schema import load_manifest


ROOT = Path(__file__).resolve().parents[3]


class AnalysisManifestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.measurement = load_manifest(ROOT / "benchmarks/manifest.json")

    def _copy_manifest(self) -> tuple[tempfile.TemporaryDirectory[str], Path, dict]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        data = json.loads((ROOT / "benchmarks/analysis.json").read_text(encoding="utf-8"))
        for relative in data["protocol"]["files"]:
            destination = root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, destination)
        path = root / "benchmarks/analysis.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        return temporary, path, data

    def test_repository_analysis_manifest_is_strict_and_compatible(self) -> None:
        analysis = load_analysis_manifest(ROOT / "benchmarks/analysis.json", self.measurement)
        self.assertEqual(analysis.schema_version, 2)
        self.assertEqual(analysis.bootstrap_samples, 20_000)
        self.assertEqual(analysis.compatible_measurement_schema, 4)
        self.assertEqual(analysis.health.initial_blocks, 30)
        self.assertEqual(analysis.health.max_blocks, 60)
        self.assertEqual(analysis.health.retry_policy, "never")
        self.assertTrue(all(not Path(value).is_absolute() for value in analysis.protocol_file_ids))
        self.assertEqual(
            analysis.protocol_file_ids,
            (
                "scripts/benchmark-report.sh",
                "tools/__init__.py",
                "tools/benchmark/__init__.py",
                "tools/benchmark/analysis.py",
                "tools/benchmark/analysis_schema.py",
                "tools/benchmark/artifact.py",
                "tools/benchmark/health.py",
                "tools/benchmark/linearity.py",
                "tools/benchmark/planning.py",
                "tools/benchmark/raw_contract.py",
                "tools/benchmark/raw_validation.py",
                "tools/benchmark/receipts.py",
                "tools/benchmark/records.py",
                "tools/benchmark/report.py",
                "tools/benchmark/schema.py",
                "tools/benchmark/statistics.py",
            ),
        )

    def test_raw_contract_local_dependencies_are_analysis_protocol_inputs(self) -> None:
        analysis = load_analysis_manifest(ROOT / "benchmarks/analysis.json", self.measurement)
        tree = ast.parse(
            (ROOT / "tools/benchmark/raw_contract.py").read_text(encoding="utf-8")
        )
        dependencies = {
            f"tools/benchmark/{node.module}.py"
            for node in ast.walk(tree)
            if isinstance(node, ast.ImportFrom) and node.level == 1 and node.module
        }
        self.assertIn("tools/benchmark/receipts.py", dependencies)
        self.assertLessEqual(dependencies, set(analysis.protocol_file_ids))

    def test_unknown_bool_duplicate_and_incompatible_fail_closed(self) -> None:
        temporary, path, data = self._copy_manifest()
        self.addCleanup(temporary.cleanup)
        data["surprise"] = True
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(AnalysisManifestError, "unknown"):
            load_analysis_manifest(path, self.measurement)
        del data["surprise"]
        data["bootstrap"]["samples"] = True
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(AnalysisManifestError, "expected integer"):
            load_analysis_manifest(path, self.measurement)
        data["bootstrap"]["samples"] = 20_000
        data["compatible_measurement"]["protocol_ids"] = ["wrong"]
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(AnalysisManifestError, "incompatible"):
            load_analysis_manifest(path, self.measurement)

        encoded = json.dumps(data).replace(
            '{"schema_version": 2,', '{"schema_version": 2, "schema_version": 2,', 1
        )
        path.write_text(encoded, encoding="utf-8")
        with self.assertRaisesRegex(AnalysisManifestError, "duplicate key"):
            load_analysis_manifest(path, self.measurement)


if __name__ == "__main__":
    unittest.main()


class CompatibleProtocolListTests(unittest.TestCase):
    """One analysis policy, more than one measurement series.

    The broad portfolio and the generic-path sentinels differ in workload and
    therefore in measurement protocol, while sharing every rule that turns raw
    records into ratios. Listing the protocols is what lets the sentinels reach
    the same analysis without a second, drifting copy of that policy.
    """

    def test_repository_policy_serves_both_measurement_series(self) -> None:
        data = json.loads((ROOT / "benchmarks/analysis.json").read_text(encoding="utf-8"))
        self.assertEqual(
            data["compatible_measurement"]["protocol_ids"],
            ["quickjs-generic-sentinel-protocol-v1", "quickjs-measurement-protocol-v8"],
        )
        for manifest_path in (
            "benchmarks/manifest.json",
            "benchmarks/generic-sentinels-manifest.json",
        ):
            measurement = load_manifest(ROOT / manifest_path)
            analysis = load_analysis_manifest(ROOT / "benchmarks/analysis.json", measurement)
            analysis.assert_compatible(measurement)

    def test_unlisted_protocol_still_fails_closed(self) -> None:
        measurement = load_manifest(ROOT / "benchmarks/manifest.json")
        analysis = load_analysis_manifest(ROOT / "benchmarks/analysis.json", measurement)
        stranger = replace(measurement, protocol_id="quickjs-unlisted-protocol-v1")
        with self.assertRaisesRegex(AnalysisManifestError, "incompatible"):
            analysis.assert_compatible(stranger)

    def test_protocol_list_must_be_non_empty_unique_and_sorted(self) -> None:
        measurement = load_manifest(ROOT / "benchmarks/manifest.json")
        helper = AnalysisManifestTests()
        temporary, path, data = helper._copy_manifest()
        self.addCleanup(temporary.cleanup)
        for value, message in (
            ([], "non-empty array"),
            (["b", "a"], "unique and sorted"),
            (["a", "a"], "unique and sorted"),
        ):
            data["compatible_measurement"]["protocol_ids"] = value
            path.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaisesRegex(AnalysisManifestError, message):
                load_analysis_manifest(path, measurement)
