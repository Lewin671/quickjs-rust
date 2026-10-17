"""Reference-derived hang limits, independent of candidate/base results.

The frozen manifest carries every reference median and its source hashes.
Replay recomputes limits from those medians; a timeout remains a failed sample.
"""
from __future__ import annotations

from dataclasses import dataclass

import math
import re
import statistics

@dataclass(frozen=True)
class Measurement:
    blocks: int
    timeout_seconds: int
    metric: str
    phase_boundary: str
    order: str
    seed: int
    case_timeouts: dict[str, float] | None = None

    def timeout_for(self, suite, case):
        return self.case_timeouts[suite + '/' + case] if self.case_timeouts is not None else self.timeout_seconds


ID = 'quickjs-reference-timeout-v1'


def derive(reference_rows, *, provenance):
    groups = {}
    binary = provenance['reference_binary_sha256']
    for row in reference_rows:
        if row.get('phase') != 'measurement' or row.get('role') != 'quickjs-ng':
            continue
        if row['status'] != 'ok' or row['binary_sha256'] != binary:
            raise ValueError('timeout reference must be complete and pinned')
        key = row['suite_id'] + '/' + row['case_id']
        group = groups.setdefault(key, {})
        if row['block'] in group or row['duration_ns'] <= 0:
            raise ValueError('duplicate/nonpositive timeout reference')
        group[row['block']] = row['duration_ns']
    cases = {}
    for key, values in groups.items():
        if set(values) != set(range(provenance['blocks'])):
            raise ValueError('timeout reference block inventory mismatch')
        median = statistics.median(values.values())
        cases[key] = {'reference_median_ns': median,
                      'timeout_seconds': max(60, 4 * median / 1e9)}
    return {'id': ID, 'minimum_seconds': 60, 'multiplier': 4,
            'reference': provenance, 'cases': cases}


def validate(data, case_ids):
    if not isinstance(data, dict) or set(data) != {'id','minimum_seconds','multiplier','reference','cases'}:
        raise ValueError('invalid reference timeout policy fields')
    if data['id'] != ID or data['minimum_seconds'] != 60 or data['multiplier'] != 4:
        raise ValueError('reference timeout rule must be max(60 s, 4 x NG median)')
    reference = data['reference']
    if set(reference) != {'job','raw_sha256','summary_sha256','reference_revision',
                          'reference_binary_sha256','blocks','selection'}:
        raise ValueError('invalid timeout reference provenance')
    for name in ('raw_sha256','summary_sha256','reference_binary_sha256'):
        if not re.fullmatch('[0-9a-f]{64}', reference[name]):
            raise ValueError('invalid timeout reference digest')
    if (not re.fullmatch('[0-9a-f]{40}', reference['reference_revision'])
            or reference['blocks'] < 30 or reference['selection'] != 'latest-valid-canonical-NG'):
        raise ValueError('invalid timeout reference identity/selection')
    if set(data['cases']) != set(case_ids):
        raise ValueError('timeout policy case inventory mismatch')
    result = {}
    for name, row in data['cases'].items():
        if set(row) != {'reference_median_ns','timeout_seconds'}:
            raise ValueError('invalid case timeout fields')
        median = row['reference_median_ns']
        if type(median) not in (int,float) or not math.isfinite(median) or median <= 0:
            raise ValueError('invalid reference median')
        expected = max(60, 4 * median / 1e9)
        if type(row['timeout_seconds']) not in (int,float) or row['timeout_seconds'] != expected:
            raise ValueError('case timeout differs from reference derivation')
        result[name] = expected
    return result


def attach(measurement, data, case_ids):
    from dataclasses import replace
    limits = validate(data, case_ids)
    if measurement.timeout_seconds != 60:
        raise ValueError('reference timeout floor must be 60 seconds')
    return replace(measurement, case_timeouts=limits)
