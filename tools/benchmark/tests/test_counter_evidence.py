"""Hardware-counter evidence: collection, raw contract, reports and decisions."""
from __future__ import annotations

import copy
import sys
import unittest

from tools.benchmark import counters
from tools.benchmark.external_report import paired_effect
from tools.benchmark.performance_evidence import comparison_index, promotion_metric
from tools.benchmark.process import run_process
from tools.benchmark.report import ReportError
from tools.benchmark.tests.test_report import ReportTests


@unittest.skipUnless(counters.supported(), "host cannot read per-process counters")
class CollectionTests(unittest.TestCase):
    def test_run_process_reads_counters_without_changing_the_result(self):
        result = run_process([sys.executable, "-c", "print(sum(range(100000)))"], 30)
        self.assertEqual((result.exit_code, result.stdout), (0, "4999950000\n"))
        self.assertGreater(result.instructions, 1_000_000)
        self.assertGreater(result.cycles, 100_000)

    def test_timed_out_process_still_reports_consistent_fields(self):
        result = run_process([sys.executable, "-c", "import time; time.sleep(5)"], 0.5)
        self.assertTrue(result.timed_out)
        self.assertEqual(result.instructions is None, result.cycles is None)


class RawCounterContractTests(ReportTests):
    """Reuses the report fixture to build complete raw runs with counters."""

    def _measurement_rows(self, rows):
        return [row for row in rows if row.get("phase") == "measurement"]

    def test_complete_counters_add_a_cycles_comparison(self):
        self._write_rows(self._complete_rows(cycles_per_op={"candidate": 90, "base": 100,
                                                            "quickjs-ng": 50}))
        report = self._build_report()
        self.assertEqual(report["schema_version"], 4)
        cycles = report["counter_comparisons"]["cycles"]["candidate_vs_base"]
        self.assertAlmostEqual(cycles["cases"]["plain_function_call"]["ratio"], 0.9)
        self.assertIn("candidate_median_cycles_per_op", cycles["cases"]["plain_function_call"])
        # Wall time keeps its own, independent comparison.
        self.assertAlmostEqual(report["comparisons"]["candidate_vs_base"]["overall"]["ratio"], 1.2)

    def test_without_counters_there_is_no_cycles_comparison(self):
        report = self._build_report()
        self.assertEqual(report["counter_comparisons"]["cycles"],
                         {"candidate_vs_base": None, "candidate_vs_quickjs_ng": None})

    def test_one_sample_without_counters_drops_the_whole_cycles_comparison(self):
        rows = self._complete_rows(cycles_per_op={"candidate": 90, "base": 100, "quickjs-ng": 50})
        row = self._measurement_rows(rows)[0]
        row["instructions"] = row["cycles"] = None
        self._write_rows(rows)
        report = self._build_report()
        self.assertIsNone(report["counter_comparisons"]["cycles"]["candidate_vs_base"])
        self.assertIsNotNone(report["comparisons"]["candidate_vs_base"])

    def test_counters_are_both_present_and_positive(self):
        for instructions, cycles in ((None, 5), (5, None), (0, 5), (5, 0), (5, 1.5)):
            rows = self._complete_rows(cycles_per_op={"candidate": 90, "base": 100,
                                                      "quickjs-ng": 50})
            row = self._measurement_rows(rows)[0]
            row["instructions"], row["cycles"] = instructions, cycles
            self._write_rows(rows)
            with self.subTest(instructions=instructions, cycles=cycles):
                with self.assertRaises(ReportError):
                    self._build_report()


def _row(block, cycles, status="ok", duration=100):
    return {"block": block, "status": status, "duration_ns": duration, "cycles": cycles}


class ExternalCounterTests(unittest.TestCase):
    def test_paired_cycles_use_the_same_block_statistics(self):
        candidate = [_row(b, 90) for b in range(30)]
        base = [_row(b, 100) for b in range(30)]
        effect = paired_effect(candidate, base, 30, "cycles")
        self.assertAlmostEqual(effect["ratio"], 0.9)
        self.assertEqual(effect["status"], "healthy")

    def test_any_block_without_cycles_yields_no_cycles_effect(self):
        candidate = [_row(b, 90) for b in range(30)]
        candidate[7]["cycles"] = None
        base = [_row(b, 100) for b in range(30)]
        self.assertIsNone(paired_effect(candidate, base, 30, "cycles"))
        self.assertIsNotNone(paired_effect(candidate, base, 30))


class PromotionMetricTests(unittest.TestCase):
    def setUp(self):
        from tools.benchmark.tests import decision_fixtures as fx
        self.fx = fx
        self.engines = fx.summary("c" * 40, "b" * 40)["engines"]

    def _external_cycles(self, ratio=0.95):
        return {c["id"]: ratio for s in self.fx.EXTERNAL["suites"] for c in s["cases"]}

    def test_cycles_when_every_lane_carries_them(self):
        broad = self.fx.internal(self.engines, cycles=0.95)
        sentinel = self.fx.internal(self.engines, lane="sentinel", cycles=0.95)
        external = self.fx.external(self.engines, cycles=self._external_cycles())
        self.assertEqual(promotion_metric(broad, external, sentinel), "cycles")
        rows = comparison_index(broad, external, sentinel, "cycles")
        self.assertAlmostEqual(rows["broad/plain_function_call"]["ratio"], 0.95)

    def test_wall_time_when_any_case_lacks_cycles(self):
        broad = self.fx.internal(self.engines, cycles=0.95)
        sentinel = self.fx.internal(self.engines, lane="sentinel", cycles=0.95)
        partial = self._external_cycles()
        partial.pop(next(iter(partial)))
        external = self.fx.external(self.engines, cycles=partial)
        self.assertEqual(promotion_metric(broad, external, sentinel), "wall_time")
        no_sentinel_cycles = self.fx.internal(self.engines, lane="sentinel")
        external = self.fx.external(self.engines, cycles=self._external_cycles())
        self.assertEqual(promotion_metric(broad, external, no_sentinel_cycles), "wall_time")

    def test_unknown_metric_is_rejected(self):
        from tools.benchmark.performance_schema import PerformanceDecisionError
        with self.assertRaises(PerformanceDecisionError):
            comparison_index({}, {"suites": []}, None, "instructions")


def load_tests(loader, _tests, _pattern):
    """Collect only this module's tests; RawCounterContractTests borrows the
    report fixture by inheritance and must not rerun ReportTests."""
    suite = unittest.TestSuite()
    for case in (CollectionTests, RawCounterContractTests, ExternalCounterTests, PromotionMetricTests):
        for name in sorted(vars(case)):
            if name.startswith("test_"):
                suite.addTest(case(name))
    return suite


if __name__ == "__main__":
    unittest.main()
