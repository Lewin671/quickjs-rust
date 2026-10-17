"""Precision allocation and diagnostic-only fallback for host qualification.

Counts depend on the pilot interval width, never candidate/base direction.
Original equal-cohort lane statistics remain separate from extended case data.
"""
from __future__ import annotations

import math

LIMIT = .03
PILOT = 30
CAP = 240


def required_pairs(half_width):
    if type(half_width) not in (int,float) or not math.isfinite(half_width) or half_width < 0:
        raise ValueError('invalid pilot relative half-width')
    return min(CAP, max(PILOT, math.ceil(PILOT * (half_width / LIMIT) ** 2)))


def allocation(pilot_stats, current_pairs=PILOT, current_precise=False):
    """One predicted stage, then the fixed cap if precision still fails."""
    predicted = required_pairs(pilot_stats['relative_half_width'])
    if current_pairs == PILOT:
        return predicted
    if current_pairs not in (predicted, CAP):
        raise ValueError('case count differs from frozen power allocation')
    return current_pairs if current_precise or current_pairs == CAP else CAP


def diagnostic_only(policy, name, *, held=False):
    return policy.get('precision', {}).get('held_out' if held else 'cases', {}).get(name, {}).get('status') == 'imprecise on this host'


def needs_panel(policy, *, held=False):
    return any(diagnostic_only(policy,n,held=held) for n in policy['held_out' if held else 'cases'])


def precision_record(pilot, measured, count):
    predicted = required_pairs(pilot.get('relative_half_width',0))
    if count not in (predicted,CAP):
        return None
    status = ('precise' if measured['precise'] else
              'imprecise on this host' if count == CAP else 'needs cap continuation')
    return {'pairs':count,'required_pairs':predicted,'status':status,
            'relative_half_width':measured.get('relative_half_width',0),
            'precise':measured['precise']}


def validate_precision(policy):
    if 'precision' not in policy:
        return
    for group in ('cases','held_out'):
        records = policy['precision'][group]
        if set(records) != set(policy[group]):
            raise ValueError('precision policy inventory mismatch')
        for name, record in records.items():
            count = record['pairs']; width = record['relative_half_width']
            if type(count) is not int or not PILOT <= count <= CAP or not math.isfinite(width) or width < 0:
                raise ValueError('invalid policy precision record')
            if record['status'] == 'imprecise on this host':
                if count != CAP or width <= LIMIT or record['precise']:
                    raise ValueError('diagnostic fallback requires imprecise 240-pair evidence')
            elif record['status'] != 'precise' or width > LIMIT or not record['precise']:
                raise ValueError('qualified precision record is incomplete')
