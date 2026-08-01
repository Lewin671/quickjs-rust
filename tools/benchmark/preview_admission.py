"""Whether an optional preview lane has room to run.

The hosted preview's sentinel lane is allowed to be skipped but never allowed
to be *killed*: if the surrounding step or job expires while it is running, the
always-run publisher sees a non-success status and replaces the broad lane's
durable conclusion with "no performance conclusion". Refusing to start is
cheap; being cancelled costs the answer the run already had.

Two weaker rules were tried first and both were wrong, which is why this
exists as testable logic rather than shell arithmetic:

* bounding the lane alone does not bound the step, because the lane's deadline
  starts only after everything before it; and
* raising the step budget alone only moves the cliff, because nothing bounds
  how long that prefix takes.

What holds is measuring what is actually left. A lane is admitted only when
*both* the step and the job have room for its deadline plus a reserve covering
the fallback write and the publisher -- the job matters separately because a
job carries setup, cache, and publication steps the script never sees.
"""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass


class AdmissionError(ValueError):
    """An admission input is missing or malformed."""


@dataclass(frozen=True)
class Budget:
    """One deadline the lane must fit inside."""

    name: str
    started_at: int
    total_seconds: int

    def remaining(self, now: int, reserve_seconds: int) -> int:
        return self.total_seconds - (now - self.started_at) - reserve_seconds


@dataclass(frozen=True)
class Decision:
    admitted: bool
    reason: str
    remaining_seconds: int


def decide(
    budgets: list[Budget], now: int, needed_seconds: int, reserve_seconds: int
) -> Decision:
    """Admits the lane only when every budget has room for it.

    The tightest budget decides, so adding a budget can only ever make
    admission stricter. That is deliberate: a missing budget should degrade
    into "run anyway" only where the caller has said so explicitly, never
    silently through an unchecked default.
    """
    if needed_seconds <= 0:
        raise AdmissionError("needed seconds must be positive")
    if reserve_seconds < 0:
        raise AdmissionError("reserve seconds cannot be negative")
    if not budgets:
        raise AdmissionError("at least one budget is required")
    tightest = min(budgets, key=lambda budget: budget.remaining(now, reserve_seconds))
    remaining = tightest.remaining(now, reserve_seconds)
    if remaining < needed_seconds:
        return Decision(
            admitted=False,
            reason=(
                f"{remaining}s of the {tightest.name} budget "
                f"({tightest.total_seconds}s) remained, below the {needed_seconds}s "
                f"the lane needs"
            ),
            remaining_seconds=remaining,
        )
    return Decision(admitted=True, reason="", remaining_seconds=remaining)


def _integer(value: str, where: str, minimum: int) -> int:
    try:
        parsed = int(value)
    except ValueError as error:
        raise AdmissionError(f"{where}: expected an integer") from error
    if parsed < minimum:
        raise AdmissionError(f"{where}: must be at least {minimum}")
    return parsed


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="decide whether an optional preview lane has room to run"
    )
    parser.add_argument("--now", required=True)
    parser.add_argument("--needed-seconds", required=True)
    parser.add_argument("--reserve-seconds", required=True)
    parser.add_argument("--step-started-at", required=True)
    parser.add_argument("--step-budget-seconds", required=True)
    parser.add_argument("--job-started-at", required=True)
    parser.add_argument("--job-budget-seconds", required=True)
    try:
        args = parser.parse_args(argv)
        budgets = [
            Budget(
                "step",
                _integer(args.step_started_at, "step started at", 0),
                _integer(args.step_budget_seconds, "step budget", 1),
            ),
            Budget(
                "job",
                _integer(args.job_started_at, "job started at", 0),
                _integer(args.job_budget_seconds, "job budget", 1),
            ),
        ]
        decision = decide(
            budgets,
            _integer(args.now, "now", 0),
            _integer(args.needed_seconds, "needed seconds", 1),
            _integer(args.reserve_seconds, "reserve seconds", 0),
        )
    except AdmissionError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    if decision.admitted:
        print(f"admit {decision.remaining_seconds}")
        return 0
    print(f"refuse {decision.reason}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
