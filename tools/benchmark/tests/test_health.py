from __future__ import annotations

import unittest

from tools.benchmark.analysis_schema import HealthPolicy
from tools.benchmark.health import block_health, precision_health
from tools.benchmark.statistics import relative_half_width


POLICY = HealthPolicy(
    initial_blocks=30,
    extension_blocks=30,
    max_blocks=60,
    max_relative_half_width=0.03,
    max_invalid_block_fraction=0.10,
    block_invalidation="portfolio-whole-block",
    outlier_policy="retain",
    retry_policy="never",
)


class HealthPolicyTests(unittest.TestCase):
    def test_thirty_block_loss_boundary_is_inclusive(self) -> None:
        for count in (0, 3):
            with self.subTest(count=count):
                result = block_health(30, range(count), POLICY)
                self.assertEqual(result["status"], "pass")
                self.assertEqual(result["total_invalid_limit"], 3)
        self.assertEqual(block_health(30, range(4), POLICY)["status"], "invalid")

    def test_sixty_block_loss_boundary_is_inclusive(self) -> None:
        self.assertEqual(block_health(60, range(30, 36), POLICY)["status"], "pass")
        self.assertEqual(block_health(60, range(30, 37), POLICY)["status"], "invalid")

    def test_first_cohort_failure_cannot_be_diluted_by_extension(self) -> None:
        result = block_health(60, [0, 1, 2, 3, 30, 31], POLICY)
        self.assertEqual(result["status"], "invalid")
        self.assertTrue(result["initial_limit_exceeded"])
        self.assertFalse(result["total_limit_exceeded"])

    def test_wide_thirty_blocks_require_exact_extension_ids(self) -> None:
        result = precision_health(30, [0.02, 0.031], True, POLICY)
        self.assertEqual(result["status"], "extension_required")
        self.assertEqual(result["extension_block_ids"], list(range(30, 60)))

    def test_narrow_thirty_blocks_are_healthy(self) -> None:
        computed_boundary = relative_half_width(1.0, 0.98, 1.03)
        self.assertGreater(computed_boundary, 0.03)
        result = precision_health(30, [computed_boundary, 0.01], True, POLICY)
        self.assertEqual(result["status"], "healthy")
        self.assertEqual(result["extension_block_ids"], [])

    def test_clearly_over_limit_thirty_blocks_remain_wide(self) -> None:
        result = precision_health(30, [0.030_001], True, POLICY)
        self.assertEqual(result["status"], "extension_required")

    def test_wide_sixty_blocks_are_inconclusive(self) -> None:
        result = precision_health(60, [0.031], True, POLICY)
        self.assertEqual(result["status"], "inconclusive")
        self.assertEqual(result["extension_block_ids"], [])

    def test_nonstandard_smoke_is_inconclusive(self) -> None:
        block_result = block_health(3, [], POLICY)
        self.assertEqual(block_result["status"], "non_claim")
        self.assertEqual(precision_health(3, [0.0], True, POLICY)["status"], "inconclusive")

    def test_relative_half_width_is_multiplicative(self) -> None:
        self.assertAlmostEqual(relative_half_width(1.0, 0.98, 1.03), 0.03)
        with self.assertRaisesRegex(ValueError, "positive"):
            relative_half_width(0.0, 0.9, 1.1)


if __name__ == "__main__":
    unittest.main()
