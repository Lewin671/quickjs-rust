from __future__ import annotations

import collections
import unittest

from tools.benchmark.planning import measurement_plan, role_orders


class PlanningTests(unittest.TestCase):
    def test_three_role_latin_square_is_balanced_and_deterministic(self) -> None:
        roles = ["candidate", "base", "quickjs-ng"]
        first = role_orders(roles, 30, 17)
        self.assertEqual(first, role_orders(roles, 30, 17))
        for position in range(3):
            counts = collections.Counter(order[position] for order in first)
            self.assertEqual(counts, {role: 10 for role in roles})

    def test_one_and_two_role_orders_are_deterministic(self) -> None:
        self.assertEqual(role_orders(["candidate"], 3, 1), [["candidate"]] * 3)
        orders = role_orders(["candidate", "base"], 4, 1)
        self.assertEqual(orders[0], orders[2])
        self.assertEqual(orders[1], orders[3])
        self.assertNotEqual(orders[0], orders[1])

    def test_plan_contains_every_role_case_per_block(self) -> None:
        plan = measurement_plan(["candidate", "base"], ["a", "b"], 3, 9)
        self.assertEqual(len(plan), 12)
        for block in range(3):
            pairs = {(item.role, item.case_id) for item in plan if item.block == block}
            self.assertEqual(pairs, {("candidate", "a"), ("candidate", "b"), ("base", "a"), ("base", "b")})


if __name__ == "__main__":
    unittest.main()
