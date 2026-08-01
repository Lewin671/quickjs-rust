from __future__ import annotations

import unittest

from tools.benchmark.preview_admission import AdmissionError, Budget, decide, main


class AdmissionTests(unittest.TestCase):
    """The rule that keeps an optional lane from being killed mid-run.

    Being skipped is acceptable; being cancelled is not, because the always-run
    publisher would then replace the broad lane's durable conclusion with a
    failure notice.
    """

    def budgets(self, *, step_elapsed: int, job_elapsed: int) -> list[Budget]:
        now = 10_000
        return [
            Budget("step", now - step_elapsed, 2820),
            Budget("job", now - job_elapsed, 3420),
        ]

    def test_a_fast_prefix_is_admitted(self) -> None:
        decision = decide(
            self.budgets(step_elapsed=300, job_elapsed=600), 10_000, 600, 120
        )
        self.assertTrue(decision.admitted)
        self.assertEqual(decision.remaining_seconds, 2400)

    def test_a_slow_prefix_is_refused(self) -> None:
        decision = decide(
            self.budgets(step_elapsed=2700, job_elapsed=2700), 10_000, 600, 120
        )
        self.assertFalse(decision.admitted)
        self.assertIn("step", decision.reason)

    def test_the_boundary_refuses_rather_than_gambling(self) -> None:
        # Exactly enough is admitted; one second less is not.
        exact = decide(self.budgets(step_elapsed=2100, job_elapsed=0), 10_000, 600, 120)
        self.assertTrue(exact.admitted)
        self.assertEqual(exact.remaining_seconds, 600)
        short = decide(self.budgets(step_elapsed=2101, job_elapsed=0), 10_000, 600, 120)
        self.assertFalse(short.admitted)

    def test_the_job_deadline_can_refuse_a_step_that_still_has_room(self) -> None:
        """The reason this is not step-only arithmetic.

        A job carries setup, cache, and publication steps the orchestrator
        never sees, so its clock can be far ahead of the step's. If only the
        step were consulted, a near-budget job would admit a lane and then be
        cancelled before the publisher ran.
        """
        decision = decide(
            self.budgets(step_elapsed=60, job_elapsed=3000), 10_000, 600, 120
        )
        self.assertFalse(decision.admitted)
        self.assertIn("job", decision.reason)

    def test_the_tightest_budget_decides(self) -> None:
        # Adding a budget can only make admission stricter, never looser.
        loose = [Budget("step", 9_400, 2820)]
        with_tight = loose + [Budget("job", 7_100, 3420)]
        self.assertTrue(decide(loose, 10_000, 600, 120).admitted)
        self.assertFalse(decide(with_tight, 10_000, 600, 120).admitted)

    def test_malformed_inputs_fail_closed(self) -> None:
        for budgets, needed, reserve, message in (
            ([], 600, 120, "at least one budget"),
            ([Budget("step", 0, 2820)], 0, 120, "must be positive"),
            ([Budget("step", 0, 2820)], 600, -1, "cannot be negative"),
        ):
            with self.assertRaisesRegex(AdmissionError, message):
                decide(budgets, 10_000, needed, reserve)

    def test_the_cli_reports_the_decision_through_its_exit_status(self) -> None:
        admit = [
            "--now", "10000", "--needed-seconds", "600", "--reserve-seconds", "120",
            "--step-started-at", "9700", "--step-budget-seconds", "2820",
            "--job-started-at", "9400", "--job-budget-seconds", "3420",
        ]
        self.assertEqual(main(admit), 0)
        refuse = list(admit)
        refuse[refuse.index("--step-started-at") + 1] = "7300"
        self.assertEqual(main(refuse), 1)
        malformed = list(admit)
        malformed[malformed.index("--step-budget-seconds") + 1] = "not-a-number"
        self.assertEqual(main(malformed), 2)


if __name__ == "__main__":
    unittest.main()
