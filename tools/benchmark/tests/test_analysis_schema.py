from __future__ import annotations

import ast
import json
import shutil
import tempfile
import unittest
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
        data["compatible_measurement"]["protocol_id"] = "wrong"
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
