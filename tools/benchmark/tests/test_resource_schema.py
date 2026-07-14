from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.resource_analysis_schema import load_resource_analysis
from tools.benchmark.resource_schema import (
    LANE_SHAPES,
    ResourceManifestError,
    load_resource_manifest,
)


ROOT = Path(__file__).resolve().parents[3]


class ResourceSchemaTests(unittest.TestCase):
    def test_repository_resource_contracts_are_strict_and_independent(self) -> None:
        measurement = load_resource_manifest(ROOT / "benchmarks/resources.json")
        analysis = load_resource_analysis(
            ROOT / "benchmarks/resource-analysis.json", measurement
        )
        self.assertEqual(measurement.schema_version, 1)
        self.assertEqual(set(measurement.lanes), set(LANE_SHAPES))
        self.assertEqual(measurement.profile.platform, "darwin")
        self.assertEqual(measurement.profile.machine, "arm64")
        self.assertEqual(measurement.profile.rss_raw_unit, "bytes")
        self.assertEqual(
            measurement.protocol_file_ids,
            (
                "benchmarks/workloads/resource-probe.js",
                "scripts/resource-benchmark.sh",
                "tools/__init__.py",
                "tools/benchmark/__init__.py",
                "tools/benchmark/adapters.py",
                "tools/benchmark/planning.py",
                "tools/benchmark/process.py",
                "tools/benchmark/receipts.py",
                "tools/benchmark/records.py",
                "tools/benchmark/resource_cli.py",
                "tools/benchmark/resource_process.py",
                "tools/benchmark/resource_runner.py",
                "tools/benchmark/resource_schema.py",
                "tools/benchmark/resource_wall_process.py",
                "tools/benchmark/schema.py",
                "tools/benchmark/snapshots.py",
            ),
        )
        self.assertEqual(analysis.bootstrap_samples, 20_000)
        self.assertEqual(analysis.health.initial_blocks, 30)
        self.assertEqual(analysis.health.max_blocks, 60)
        self.assertEqual(
            analysis.protocol_file_ids,
            (
                "scripts/resource-benchmark-report.sh",
                "tools/__init__.py",
                "tools/benchmark/__init__.py",
                "tools/benchmark/planning.py",
                "tools/benchmark/receipts.py",
                "tools/benchmark/records.py",
                "tools/benchmark/resource_analysis.py",
                "tools/benchmark/resource_analysis_schema.py",
                "tools/benchmark/resource_artifact.py",
                "tools/benchmark/resource_process.py",
                "tools/benchmark/resource_report.py",
                "tools/benchmark/resource_schema.py",
                "tools/benchmark/resource_validation.py",
                "tools/benchmark/schema.py",
                "tools/benchmark/statistics.py",
            ),
        )
        self.assertNotEqual(measurement.protocol_id, analysis.protocol_id)

    def test_protocol_hashes_bind_every_inventory_file(self) -> None:
        measurement = load_resource_manifest(ROOT / "benchmarks/resources.json")
        analysis = load_resource_analysis(
            ROOT / "benchmarks/resource-analysis.json", measurement
        )
        self.assertEqual(
            measurement.protocol_sha256,
            "67eee418bcb19a1ea45813f109b5fdcb0058393528052aeaa6657f1ca2d99bac",
        )
        self.assertEqual(
            analysis.protocol_sha256,
            "4d28b20cd3090bdc74a4350774c1d12807458a957ebfe85ae03d286f4b59e14a",
        )
        self.assertNotIn("tools/benchmark/resource_runner.py", analysis.protocol_file_ids)

    def test_json_contract_rejects_duplicate_keys(self) -> None:
        text = (ROOT / "benchmarks/resources.json").read_text(encoding="utf-8")
        duplicate = text.replace(
            '"schema_version": 1,', '"schema_version": 1, "schema_version": 1,', 1
        )
        self.assertEqual(json.loads(duplicate)["schema_version"], 1)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "resources.json"
            path.write_text(duplicate, encoding="utf-8")
            with self.assertRaisesRegex(ResourceManifestError, "duplicate key"):
                load_resource_manifest(path)

    def test_resource_shell_dry_run_selects_exactly_one_lane(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.run(
                [str(ROOT / "scripts/resource-benchmark.sh"), "--lane", "fresh", "--dry-run"],
                cwd=directory, capture_output=True, text=True, timeout=10, check=False,
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        plan = json.loads(result.stdout)
        self.assertEqual(plan["lane_id"], "fresh_process_latency/wall_ns_per_process")
        self.assertEqual(plan["blocks"], 30)
        self.assertEqual(len(plan["plan"]), 90)
        self.assertFalse(plan["claim_eligible"])


if __name__ == "__main__":
    unittest.main()
