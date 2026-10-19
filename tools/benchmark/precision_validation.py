"""CPU-only precision-rule study from sealed T060 A/A (no engine launches).

Reports exact-bootstrap replay and deterministic Monte Carlo. The simulation
uses 20,000 seed-0 bootstrap *rank pairs*, reused across trials (the same
resampling indexes as layout_policy.analyze for one case). This is exactly
its even-N median percentile interval, without resampling values repeatedly.
"""
from __future__ import annotations

import argparse
from collections import Counter
from functools import lru_cache
import hashlib
import json
import math
from pathlib import Path
import random
import statistics

from .formal_precision import analyze_case, rank_bounds
from .layout_policy import analyze
from .layout_power import required_pairs


@lru_cache(maxsize=None)
def bootstrap_ranks(n):
    rng = random.Random(0)
    counts = Counter()
    for _ in range(20000):
        indexes = sorted(rng.randrange(n) for _ in range(n))
        counts[indexes[n//2-1], indexes[n//2]] += 1
    return tuple((a, b, count) for (a, b), count in counts.items())


def bootstrap(values):
    n = len(values)
    ordered = sorted(values)
    distribution = sorted(((ordered[a]+ordered[b])/2, count)
                          for a, b, count in bootstrap_ranks(n))
    bounds = []
    for quantile in (.025, .975):
        rank, cumulative = int(quantile*19999), 0
        for value, count in distribution:
            cumulative += count
            if cumulative > rank:
                bounds.append(value)
                break
    median = statistics.median(values)
    low, high = bounds
    return {'ratio': median, 'confidence_interval': {'lower': low, 'upper': high},
            'relative_half_width': max(median/low-1, high/median-1),
            'precise': max(median/low-1, high/median-1) <= .03}


def outcome(stats, limit, legacy):
    if legacy and not stats['precise']:
        return 'incomplete'
    ci = stats['confidence_interval']
    return 'pass' if ci['upper'] <= limit else 'fail' if ci['lower'] > limit else 'incomplete'


def simulate(trials, noise, truth, limit):
    rng = random.Random(62019)
    counts = {name: Counter() for name in ('old_cap', 'old_sequential', 'new_cap', 'new_sequential')}
    for _ in range(trials):
        stream = [truth*math.exp(rng.gauss(0, noise)) for _ in range(240)]
        counts['old_cap'][outcome(bootstrap(stream), limit, True)] += 1
        pilot = bootstrap(stream[:30])
        old = pilot
        if not pilot['precise']:
            predicted = required_pairs(pilot['relative_half_width'])
            old = bootstrap(stream[:predicted])
            if not old['precise']:
                old = bootstrap(stream)
        counts['old_sequential'][outcome(old, limit, True)] += 1
        counts['new_cap'][outcome(analyze_case(stream), limit, False)] += 1
        first = outcome(analyze_case(stream[:30]), limit, False)
        counts['new_sequential'][first if first != 'incomplete' else outcome(analyze_case(stream), limit, False)] += 1
    return {name: {**dict(c), 'false_pass_rate': c['pass']/trials if truth >= limit else None,
                   'false_incomplete_rate': c['incomplete']/trials if truth != limit else None}
            for name, c in counts.items()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--trials', type=int, default=10000)
    args = parser.parse_args()
    from .layout_formal_raw import extract
    from .layout_identity import ROOT
    policy_path = ROOT/'benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v5.json'
    policy = json.loads(policy_path.read_text())
    # Verify every sealed artifact locally; no live host or benchmark needed.
    summary = json.loads((args.bundle/'summary.json').read_text())
    for name, expected in summary['bundle']['artifacts'].items():
        if hashlib.sha256((args.bundle/name).read_bytes()).hexdigest() != expected:
            raise ValueError('sealed qualification artifact changed: '+name)
    ratios = extract(args.bundle, policy['host_node'])
    diagnosis = []
    for name, values in ratios.items():
        # Exact same 20k draws/statistic, using cached bootstrap rank pairs.
        stats = bootstrap(values)
        projected = stats['relative_half_width'] / math.sqrt(240/len(values))
        diagnosis.append({'id': name, 'pilot_half_width': stats['relative_half_width'],
                          'projected_cap_half_width': projected,
                          'required_blocks': required_pairs(stats['relative_half_width']),
                          'log_sd': statistics.stdev(map(math.log, values))})
    name = 'external/sunspider-1.0/bitops-3bit-bits-in-byte'
    assert bootstrap(ratios[name])['confidence_interval'] == analyze({name: ratios[name]})['confidence_interval']
    actual_noise = statistics.stdev(map(math.log, ratios[name]))
    result = {'trials': args.trials, 'seed': 62019, 'diagnosis': sorted(diagnosis,key=lambda x:x['pilot_half_width'],reverse=True),
              'cap_projection_failures': sum(r['projected_cap_half_width'] > .03 for r in diagnosis),
              'rank_bounds': {str(n): rank_bounds(n) for n in (30,60,240)}, 'simulations': []}
    for noise in (actual_noise, .20):
        for truth in (1., 1.05, 1.06, 1.10, .90):
            result['simulations'].append({'log_sd': noise, 'true_ratio': truth, 'limit': 1.05,
                                         **simulate(args.trials, noise, truth, 1.05)})
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == '__main__':
    main()
