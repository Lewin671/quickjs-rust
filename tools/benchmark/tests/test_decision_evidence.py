from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.performance_evaluation import decide
from tools.benchmark.performance_schema import PerformanceDecisionError
from tools.benchmark.profile import verify_profiles
from tools.benchmark.tests import test_performance_decision as fixtures
from tools.benchmark.tests.decision_fixtures import effect, zero_gap


class DecisionEvidenceTests(unittest.TestCase):
    def setUp(self):
        from tools.benchmark.tests.decision_fixtures import isolate_test262_inventory
        isolate_test262_inventory(self)
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.helper = fixtures.PerformanceDecisionTests()
        self.queue, self.qsha, self.unit, self.usha = self.helper._queue_and_unit(self.root)
        self.summary = self.helper._summary(self.helper.candidate_sha, self.helper.base_sha)
        self.broad = self.helper._broad()
        self.external = self.helper._external()
        self.sentinel = self.helper._broad(lane="sentinel")

    def run_decision(self, mode="fast"):
        zero = zero_gap(self.helper.candidate_sha)
        return decide(self.unit, self.usha, self.queue, self.qsha, self.summary, "1" * 64,
                      self.broad, "2" * 64, self.external, "3" * 64, mode, (zero, "4" * 64),
                      sentinel=self.sentinel, sentinel_sha="5" * 64, profile_root=self.root)

    def test_healthy_evidence_is_retained(self):
        self.assertEqual(self.run_decision("promotion")["decision"], "retained")

    def test_report_from_another_revision_is_invalid(self):
        self.broad["run"]["engines"][0]["receipt"]["source"]["revision"] = "d" * 40
        with self.assertRaisesRegex(PerformanceDecisionError, "revision/binary mismatch"):
            self.run_decision()

    def test_external_from_another_executable_is_invalid(self):
        self.external["binary_sha256"]["candidate"] = "d" * 64
        with self.assertRaisesRegex(PerformanceDecisionError, "binary identities"):
            self.run_decision()

    def test_failed_linearity_cannot_retain(self):
        self.broad["health"]["linearity"]["status"] = "fail"
        self.assertEqual(self.run_decision()["decision"], "inconclusive")

    def test_missing_sentinel_cannot_retain(self):
        self.sentinel = None
        self.assertEqual(self.run_decision()["decision"], "inconclusive")

    def test_three_blocks_cannot_retain_even_if_point_estimate_is_fast(self):
        self.external["suites"][0]["cases"][0]["paired_comparisons"]["base"]["valid_blocks"] = 3
        self.assertEqual(self.run_decision()["decision"], "inconclusive")

    def test_interval_crossing_target_gate_is_inconclusive(self):
        row = self.external["suites"][0]["cases"][0]["paired_comparisons"]["base"]
        row.update(ratio=0.949, confidence_interval={"lower": 0.94, "upper": 0.96})
        self.assertEqual(self.run_decision()["decision"], "inconclusive")

    def test_unwatched_tenfold_regression_blocks_promotion(self):
        case = self.external["suites"][0]["cases"][2]
        case["candidate_over_base"] = 10
        case["paired_comparisons"]["base"] = effect(10)
        self.assertEqual(self.run_decision("promotion")["decision"], "rejected")

    def test_sentinel_regression_blocks_even_fast_mode(self):
        cases = self.sentinel["comparisons"]["candidate_vs_base"]["cases"]
        cases[next(iter(cases))] = effect(2)
        self.assertEqual(self.run_decision()["decision"], "rejected")

    def test_wrong_case_names_cannot_substitute_for_equal_case_count(self):
        cases = self.broad["comparisons"]["candidate_vs_base"]["cases"]
        cases["substitute"] = cases.pop("plain_function_call")
        self.assertEqual(self.run_decision("promotion")["decision"], "inconclusive")

    def test_missing_or_changed_profile_is_rejected(self):
        (self.root / "profile.sample").write_text("replaced profile")
        with self.assertRaisesRegex(PerformanceDecisionError, "missing, empty, or changed"):
            self.run_decision()

    def test_profile_paths_cannot_escape(self):
        unit = copy.deepcopy(self.unit)
        unit["profile_evidence"][0]["source"] = "../elsewhere.json"
        with self.assertRaisesRegex(PerformanceDecisionError, "inside the evidence directory"):
            verify_profiles(unit, self.queue, self.root)

    def test_same_revision_rebuild_must_match_profiled_binary(self):
        self.queue["engines"]["candidate"]["binary_sha256"] = "d" * 64
        with self.assertRaisesRegex(PerformanceDecisionError, "profiled queue candidate"):
            self.run_decision()

    def test_partial_zero_failure_scan_cannot_satisfy_parity(self):
        from tools.benchmark.performance_evaluation import _test262_zero_gap
        tiny = zero_gap(self.helper.candidate_sha)
        tiny.update(total=1, configured=1)
        tiny["rust"]["pass"] = tiny["ng"]["pass"] = 1
        with self.assertRaisesRegex(PerformanceDecisionError, "complete pinned inventory"):
            _test262_zero_gap(tiny, self.helper.candidate_sha)

    def test_invalid_lane_cannot_supply_a_regression_verdict(self):
        self.broad["health"]["linearity"]["status"] = "fail"
        self.broad["comparisons"]["candidate_vs_base"]["cases"]["plain_function_call"] = effect(10)
        self.assertEqual(self.run_decision()["decision"], "inconclusive")
