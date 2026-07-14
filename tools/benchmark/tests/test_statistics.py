from __future__ import annotations

import math
import unittest

from tools.benchmark.statistics import (
    case_effect,
    family_effect,
    paired_block_bootstrap,
    paired_log_ratios,
)


class StatisticsTests(unittest.TestCase):
    def test_case_and_family_effects(self) -> None:
        candidate = {0: 20.0, 1: 30.0, 2: 40.0}
        baseline = {0: 10.0, 1: 15.0, 2: 20.0}
        self.assertEqual(case_effect(candidate, baseline), 2.0)
        self.assertAlmostEqual(family_effect({"a": math.log(2), "b": math.log(8)}), 4.0)

    def test_incomplete_or_nonpositive_pairs_are_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "same"):
            paired_log_ratios({0: 1.0}, {1: 1.0})
        with self.assertRaisesRegex(ValueError, "positive"):
            paired_log_ratios({0: 0.0}, {0: 1.0})

    def test_paired_bootstrap_is_deterministic_and_keeps_fixed_cases(self) -> None:
        values = {
            "a": {10: math.log(0.9), 20: math.log(1.0), 30: math.log(1.1)},
            "b": {10: math.log(1.0), 20: math.log(1.0), 30: math.log(1.0)},
        }
        first = paired_block_bootstrap(values, samples=1000, seed=5)
        self.assertEqual(first, paired_block_bootstrap(values, samples=1000, seed=5))
        self.assertLessEqual(first[0], 1.0)
        self.assertGreaterEqual(first[1], 1.0)
        with self.assertRaisesRegex(ValueError, "same"):
            paired_block_bootstrap({"a": {0: 1.0}, "b": {0: 1.0, 1: 2.0}}, samples=2)

    def test_bootstrap_preserves_fully_correlated_case_blocks(self) -> None:
        # Opposite independent resampling could collapse these cases toward one;
        # joint cluster resampling must preserve their identical block motion.
        correlated = {
            "a": {0: math.log(0.5), 1: math.log(2.0)},
            "b": {0: math.log(0.5), 1: math.log(2.0)},
        }
        lower, upper = paired_block_bootstrap(correlated, samples=2000, seed=3)
        self.assertLessEqual(lower, 0.5)
        self.assertGreaterEqual(upper, 2.0)


if __name__ == "__main__":
    unittest.main()
