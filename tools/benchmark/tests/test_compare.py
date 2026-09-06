import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark.compare import summary_for, markdown_for
from tools.benchmark.performance_evidence import load_archived_manifest
from tools.benchmark.tests.decision_fixtures import summary, internal, external, ROOT, BROAD


class CompareTests(unittest.TestCase):
    def setUp(self):
        self.engines = summary("c" * 40, "a" * 40)["engines"]
        self.broad = internal(self.engines)
        self.sentinel = internal(self.engines, lane="sentinel")
        self.external = external(self.engines)
        # Renderer fields, normally assembled by the external reporter.
        for suite in self.external["suites"]:
            suite.update(name=suite["id"], case_count=len(suite["cases"]),
                         base_comparable_case_count=len(suite["cases"]),
                         comparable_case_count=len(suite["cases"]),
                         diagnostic_comparable_case_geomean_ratio=0.5,
                         diagnostic_candidate_over_base_geomean_ratio=0.99,
                         base_wins={"candidate": 1, "base": 0},
                         wins={"candidate": 1, "quickjs-ng": 0})
            for case in suite["cases"]:
                case["median_duration_ns"] = dict.fromkeys(self.engines, 100000)

    def test_complete_bundle_has_separate_lanes_and_no_headline_score(self):
        result = summary_for(self.broad, self.external, self.sentinel)
        self.assertEqual(result["decision_readiness"], "ready_for_unit_gates")
        self.assertFalse(result["claim_eligible"])
        text = markdown_for(result, self.broad, self.external, self.sentinel)
        for name in ("Specializer coverage", "Generic-path sentinels", "External Benchmark Preview"):
            self.assertIn(name, text)

    def test_complete_inventory_with_failed_reference_is_not_ready(self):
        self.external["suites"][0]["complete_comparison"] = False
        result = summary_for(self.broad, self.external, self.sentinel)
        self.assertEqual(result["decision_readiness"], "inconclusive")

    def test_preview_is_not_ready_for_decisions(self):
        self.broad["health"]["blocks"]["valid"] = 3
        result = summary_for(self.broad, self.external, self.sentinel)
        self.assertEqual(result["decision_readiness"], "inconclusive")

    def test_archived_manifest_loads_away_from_source_tree(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "manifest.json"
            path.write_bytes((ROOT / "benchmarks/manifest.json").read_bytes())
            manifest = load_archived_manifest(path)
            self.assertEqual(len(manifest.cases), 25)
            self.assertFalse(manifest.path.exists())
            self.assertEqual(manifest.cases[0].workload.parent, ROOT / "benchmarks/workloads")

    def test_archived_manifest_cannot_change_operation_denominator(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "manifest.json"
            data = copy.deepcopy(BROAD)
            data["cases"][0]["operations_per_iteration"] *= 100
            path.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, "semantics differ"):
                load_archived_manifest(path)

    def test_invalid_linearity_hides_lane_ratios(self):
        self.broad["health"]["linearity"]["status"] = "fail"
        result = summary_for(self.broad, self.external, self.sentinel)
        text = markdown_for(result, self.broad, self.external, self.sentinel)
        self.assertIn("No ratios:", text)
        self.assertNotIn("`plain_function_call`", text)
