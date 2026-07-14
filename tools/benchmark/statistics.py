"""Pure paired-analysis primitives shared by strict benchmark reporting."""

from __future__ import annotations

import math
import random
import statistics
from collections.abc import Mapping


def paired_log_ratios(candidate: Mapping[int, float], baseline: Mapping[int, float]) -> list[float]:
    if set(candidate) != set(baseline) or not candidate:
        raise ValueError("candidate and baseline must contain the same non-empty block set")
    ratios = []
    for block in sorted(candidate):
        candidate_value = candidate[block]
        baseline_value = baseline[block]
        if candidate_value <= 0 or baseline_value <= 0:
            raise ValueError("timings must be positive")
        ratios.append(math.log(candidate_value / baseline_value))
    return ratios


def case_effect(candidate: Mapping[int, float], baseline: Mapping[int, float]) -> float:
    """Candidate/baseline ratio from the median paired block log effect."""
    return math.exp(statistics.median(paired_log_ratios(candidate, baseline)))


def family_effect(case_log_effects: Mapping[str, float]) -> float:
    """Equal-case geometric aggregation; inputs are case-level log effects."""
    if not case_log_effects:
        raise ValueError("a family needs at least one case")
    if any(not math.isfinite(value) for value in case_log_effects.values()):
        raise ValueError("case log effects must be finite")
    return math.exp(statistics.fmean(case_log_effects.values()))


def relative_half_width(estimate: float, lower: float, upper: float) -> float:
    """Largest multiplicative distance from a positive estimate to its CI."""
    if not all(math.isfinite(value) and value > 0 for value in (estimate, lower, upper)):
        raise ValueError("estimate and confidence bounds must be finite and positive")
    if lower > upper:
        raise ValueError("confidence interval bounds are reversed")
    return max(upper / estimate - 1, estimate / lower - 1)


def paired_block_bootstrap(
    case_block_logs: Mapping[str, Mapping[int, float]],
    *,
    samples: int = 20_000,
    seed: int = 0,
    confidence: float = 0.95,
) -> tuple[float, float]:
    """Jointly resample shared block IDs across fixed cases."""
    if samples < 1 or not case_block_logs or not 0 < confidence < 1:
        raise ValueError("positive samples and at least one case are required")
    block_sets = {frozenset(values) for values in case_block_logs.values()}
    if len(block_sets) != 1 or not next(iter(block_sets)):
        raise ValueError("every case must contain the same non-empty block set")
    if any(not math.isfinite(value) for values in case_block_logs.values() for value in values.values()):
        raise ValueError("log effects must be finite")
    blocks = sorted(next(iter(block_sets)))
    randomizer = random.Random(seed)
    draws: list[float] = []
    for _ in range(samples):
        sampled_blocks = [blocks[randomizer.randrange(len(blocks))] for _ in blocks]
        case_effects = []
        for values in case_block_logs.values():
            resampled = [values[block] for block in sampled_blocks]
            case_effects.append(statistics.median(resampled))
        draws.append(math.exp(statistics.fmean(case_effects)))
    draws.sort()
    tail = (1 - confidence) / 2
    lower = draws[int(tail * (samples - 1))]
    upper = draws[int((1 - tail) * (samples - 1))]
    return lower, upper
