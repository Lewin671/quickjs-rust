"""Prospective formal case precision: two fixed looks, exact median coverage.

Only a policy explicitly freezing RULE opts in. Qualification, joint lane
bootstrap intervals and historical policies retain their original statistics.
"""
from __future__ import annotations

import math
import statistics
from functools import lru_cache

from .layout_power import CAP

RULE = 'threshold-median-v2'


@lru_cache(maxsize=None)
def rank_bounds(n):
    """Central order statistics with <=.0125 binomial tail on each side.

    Two looks * two tails * .0125 <= .05. Ties only make coverage conservative.
    Independent paired blocks from a stationary distribution are required.
    """
    if n not in (30, 60, CAP):
        raise ValueError('v2 formal precision requires frozen pilot or 240 blocks')
    tail, k = 0., 0
    for j in range(n // 2):
        candidate = tail + math.comb(n, j) / 2**n
        if candidate > .0125:
            break
        tail, k = candidate, j + 1
    return k - 1, n - k


def analyze_case(values):
    if any(type(v) not in (float, int) or not math.isfinite(v) or v <= 0 for v in values):
        raise ValueError('formal precision requires positive finite ratios')
    low, high = rank_bounds(len(values))
    ordered = sorted(values)
    ratio = statistics.median(values)
    lower, upper = ordered[low], ordered[high]
    width = max(ratio / lower - 1, upper / ratio - 1)
    return {'ratio': ratio, 'confidence_interval': {'lower': lower, 'upper': upper},
            'relative_half_width': width, 'blocks': len(values), 'precise': width <= .03,
            'interval_method': RULE, 'look_coverage': .975, 'maximum_looks': 2}


def threshold(unit, policy, case, mode):
    band = policy['cases'][case]['wall_band']
    from .layout_power import diagnostic_only
    diagnostic = diagnostic_only(policy, case)
    if mode == 'stage':
        budget = unit['migration']['stage_max_candidate_over_base']
        return budget if diagnostic else min(band, budget)
    if case in unit['fast_gate']['target_ids']:
        payoff = unit['fast_gate']['target_max_candidate_over_base']
        return payoff if diagnostic else min(payoff, 1 / band)
    return band


def ready(stats, limit):
    """Width is required only when the interval has not resolved the threshold."""
    ci = stats['confidence_interval']
    return stats['precise'] or ci['upper'] <= limit or ci['lower'] > limit


def enabled(policy):
    rule = policy.get('formal_precision_rule')
    if rule is not None and rule != RULE:
        raise ValueError('unsupported formal precision rule')
    return rule == RULE


def next_count(pilot, current):
    if pilot not in (30, 60) or current not in (pilot, CAP):
        raise ValueError('v2 continuation count differs from fixed two-look schedule')
    return CAP
