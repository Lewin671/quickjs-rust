import unittest

from tools.benchmark.external_report import paired_effect


def samples(values):
    return [{"block": i, "duration_ns": v, "status": "ok"} for i, v in enumerate(values)]


class ExternalStatisticsTests(unittest.TestCase):
    def test_uses_paired_ratios_not_ratio_of_marginal_medians(self):
        result = paired_effect(samples([10, 2, 3]), samples([1, 2, 30]), 3)
        self.assertEqual(result["ratio"], 1)
        self.assertEqual(result["status"], "inconclusive")

    def test_common_block_drift_cancels_in_paired_estimate(self):
        base = [1000 + i * 100 for i in range(30)]
        result = paired_effect(samples([v * 0.9 for v in base]), samples(base), 30)
        self.assertAlmostEqual(result["ratio"], 0.9)
        self.assertEqual(result["status"], "healthy")

    def test_noise_is_retained_and_makes_decision_inconclusive(self):
        result = paired_effect(samples([50, 150] * 15), samples([100] * 30), 30)
        self.assertEqual(result["status"], "inconclusive")
        self.assertGreater(result["relative_half_width"], 0.03)

    def test_missing_duplicate_or_failed_block_cannot_be_scored(self):
        valid = samples([100] * 30)
        for kind in ("missing", "duplicate", "failed"):
            candidate = samples([90] * 30)
            if kind == "missing":
                candidate.pop()
            elif kind == "duplicate":
                candidate[-1]["block"] = 0
            else:
                candidate[-1]["status"] = "timeout"
            with self.subTest(kind=kind):
                self.assertIsNone(paired_effect(candidate, valid, 30))
