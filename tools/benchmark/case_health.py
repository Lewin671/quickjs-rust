"""Terminal case failures and diagnostics; never an alternate cost estimator.

Only typed engine-process/correctness failures and replayed invalid increments
are containable. Infrastructure failures still abort the run. Healthy packets
keep their historical shape; failed packets carry no usable ratios or costs.
"""
from __future__ import annotations

import statistics
from dataclasses import dataclass


class CaseHealthFailure(ValueError):
    def __init__(self, health):
        self.health = health
        super().__init__('case health failure: ' + ', '.join(health['reasons']))


@dataclass
class FailedCase:
    id: str
    kind: str
    iterations: int | None
    ratios: dict
    health: dict


def failed_result(raw):
    return FailedCase(raw['id'], raw['kind'], raw['iterations'], {}, raw['health'])


def diagnostics(raw, flags, reasons, *, collection='complete'):
    """Keep signed per-role increments and all fresh-process distributions."""
    increments, populations = [], {}
    for position, attempt in enumerate(raw['attempts']):
        roles = ('base', 'candidate') if attempt['index'] % 2 == 0 else ('candidate', 'base')
        width = 2 if raw['kind'] == 'internal' else 1
        for index, row in enumerate(attempt['samples']):
            role = roles[index // width]
            count = index % width + 1
            if 'sample' in row:
                for metric in ('wall_ns', 'user_ns', 'instructions'):
                    value = row['sample'].get(metric)
                    if value is not None:
                        populations.setdefault(f'{role}/{count}/{metric}', []).append(value)
        if width == 2:
            for index, role in enumerate(roles):
                taken = attempt['samples'][index*2:index*2+2]
                if len(taken) == 2 and all('sample' in r for r in taken):
                    increments.append({'attempt_index': attempt['index'], 'role': role,
                        'load_valid': flags[position], **{metric: taken[1]['sample'].get(metric, 0) -
                            taken[0]['sample'].get(metric, 0) for metric in ('wall_ns', 'user_ns', 'instructions')}})
    distribution = {key: {'count': len(v), 'min': min(v), 'median': statistics.median(v), 'max': max(v)}
                    for key, v in populations.items()}
    return {'collection': collection, 'analysis': 'failed', 'reasons': reasons,
            'increments': increments, 'process_distribution': distribution}


def validate_health(health):
    if (not isinstance(health, dict) or health.get('collection') not in ('complete', 'failed')
            or health.get('analysis') != 'failed' or not isinstance(health.get('reasons'), list)
            or not health['reasons'] or any(not isinstance(reason, str) or not reason for reason in health['reasons'])
            or not isinstance(health.get('increments'), list)
            or not isinstance(health.get('process_distribution'), dict)):
        raise ValueError('invalid case health state')


def incomplete(packets, inventories):
    """Validate every inventory before returning a verdict without aggregates."""
    from .layout_policy import rows
    failures, states = {}, {}
    for label, (packet, expected) in enumerate(zip(packets, inventories)):
        rows(packet, expected, allow_health=True)
        for case in packet['cases']:
            health = case.get('health', {'collection': 'complete', 'analysis': 'complete'})
            states[f'{label}/{case["id"]}'] = {key: health[key] for key in ('collection', 'analysis')}
            if 'health' in case:
                failures[f'{label}/{case["id"]}'] = case['health']
    if not failures:
        return None
    return {'label': 'incomplete', 'complete': False, 'passed': False,
            'inventory_complete': True, 'failures': [],
            'missing': [name + ': case health failure' for name in failures],
            'events': {}, 'event_resolution': None, 'panel_required': False,
            'cases': states, 'lanes': {}, 'held_out': None, 'case_health': failures}


INCOMPLETE_EXIT = 4


def first_health(value):
    """Find terminal health in a case, screen, or complete panel inventory."""
    if not isinstance(value, dict):
        return None
    if value.get('analysis') == 'failed' and 'reasons' in value:
        validate_health(value)
        return value
    health = value.get('health')
    if isinstance(health, dict) and health.get('analysis') == 'failed':
        validate_health(health)
        return health
    children = []
    for key in ('cases', 'panels', 'case_health'):
        group = value.get(key, [])
        children.extend(group.values() if isinstance(group, dict) else group)
    for child in children:
        health = first_health(child)
        if health is not None:
            return health
    return None


def mark_readiness(value):
    """Add job-facing status only to failed artifacts; preserve healthy bytes."""
    if first_health(value) is not None:
        value.update(inventory_complete=True, readiness='incomplete')
    return value


def exit_status(value):
    return INCOMPLETE_EXIT if value.get('readiness') == 'incomplete' or first_health(value) is not None else 0


def require_healthy(value):
    health = first_health(value)
    if health is not None:
        raise CaseHealthFailure(health)
