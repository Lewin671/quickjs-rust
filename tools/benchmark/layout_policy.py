"""Versioned, hash-bound policy loading and exact decision-statistic analysis.

Policies are measured envelopes for one host/toolchain/recipe, not confidence
limits or portable constants. Formal precision is checked independently.
"""
from __future__ import annotations

import json
import math
import random
import platform
import statistics
from pathlib import Path

from .layout_identity import OFFSETS, digest, host_identity
from .statistics import relative_half_width


def geomean(values):
    return math.exp(statistics.fmean(math.log(x) for x in values))


def round_up(value):
    return math.ceil(value * 10000 - 1e-9) / 10000


def rows(packet, expected=None, pairs=None):
    result = {}
    for row in packet['cases']:
        name = row['id']
        if name in result:
            raise ValueError('duplicate layout case: ' + name)
        ratios = row['ratios']
        for metric in ('wall_ns', 'instructions'):
            values = ratios[metric]
            if len(values) != (pairs or packet['pairs']) or any(
                    type(v) not in (int, float) or not math.isfinite(v) or v <= 0 for v in values):
                raise ValueError(f'{name}: incomplete/nonpositive {metric} pairs')
        result[name] = ratios
    if expected is not None and set(result) != set(expected):
        raise ValueError('layout case inventory mismatch')
    return result


def statistic(ratios):
    return geomean([statistics.median(v) for v in ratios.values()])


def analyze(ratios, draws=20000):
    """Joint block bootstrap of exactly geomean(case median paired ratio)."""
    lengths = {len(v) for v in ratios.values()}
    if len(lengths) != 1 or not lengths or next(iter(lengths)) < 1:
        raise ValueError('bootstrap requires equal nonempty paired block sets')
    n = next(iter(lengths))
    estimate = statistic(ratios)
    randomizer = random.Random(0)
    simulated = []
    for _ in range(draws):
        indexes = [randomizer.randrange(n) for _ in range(n)]
        simulated.append(geomean([statistics.median([v[i] for i in indexes]) for v in ratios.values()]))
    simulated.sort()
    lower, upper = simulated[int(.025 * (draws-1))], simulated[int(.975 * (draws-1))]
    width = relative_half_width(estimate, lower, upper)
    return {'ratio': estimate, 'confidence_interval': {'lower': lower, 'upper': upper},
            'relative_half_width': width, 'blocks': n, 'precise': n >= 30 and width <= .03}


def load_policy(path: Path, sha256: str, *, live=True):
    if digest(path) != sha256:
        raise ValueError('layout policy hash mismatch')
    policy = json.loads(path.read_text())
    if policy.get('schema_version') != 1 or policy.get('artifact_type') != 'quickjs-layout-policy':
        raise ValueError('unsupported layout policy schema')
    if policy.get('qualified') is not True:
        raise ValueError('layout policy is not qualified')
    if policy.get('offsets') != list(OFFSETS):
        raise ValueError('layout policy offsets mismatch')
    if live and (policy['host'] != host_identity() or policy.get('host_node') != platform.node()):
        raise ValueError('layout policy host/recipe mismatch: recalibration required')
    for group in ('cases', 'held_out'):
        if not policy[group]:
            raise ValueError('empty layout policy inventory')
        for bands in policy[group].values():
            for value in bands.values():
                if type(value) not in (int, float) or not math.isfinite(value) or value < 1:
                    raise ValueError('invalid layout policy band')
    if set(policy['lanes']) != {'broad', 'sentinel', 'external', 'portfolio', 'held-out'}:
        raise ValueError('invalid layout lane inventory')
    if any(not math.isfinite(v) or v < 1 for v in policy['lanes'].values()):
        raise ValueError('invalid layout lane ceiling')
    return policy
