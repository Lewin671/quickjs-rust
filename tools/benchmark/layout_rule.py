"""Dual-metric layout-robust verdicts; no benchmark-specific exceptions.

A single placement can open an event, never establish persistent case cost.
Targets, whole-lane ceilings and held-out acceptance remain independent gates.
"""
from __future__ import annotations

import statistics

from .layout_policy import geomean, rows


def evaluate(policy, screen, targets, payoff, *, panel=None, held_out=None,
             instruction_blind=False, memory_targets=(), promotion=False, panel_required=False, stage=False, stage_limit=1.25):
    data = rows(screen, policy['cases'])
    if screen['pairs'] < 5:
        raise ValueError('layout verdict needs at least five pairs')
    targets = set(targets)
    if not targets <= set(data):
        raise ValueError('unknown layout target')
    if instruction_blind and (not memory_targets or not set(memory_targets) <= targets):
        raise ValueError('instruction-blind unit must predeclare memory-sensitive wall targets')
    failures, missing = [], []
    events, cases, lanes = {}, {}, {}
    panels = None
    if panel is not None:
        if len(panel) != 6:
            raise ValueError('layout panel requires exactly six matching variants')
        panels = [rows(packet, policy['cases']) for packet in panel]
        if panel[0]['cases'] != screen['cases']:
            raise ValueError('natural screen differs from panel variant zero')
        if any(packet['pairs'] != screen['pairs'] for packet in panel):
            raise ValueError('panel pair counts differ')
    for name, ratios in data.items():
        band = policy['cases'][name]['wall_band']
        if stage:
            band = min(band, stage_limit)
        instruction_limit = max(1.01, policy['cases'][name]['instruction_band'])
        wall = statistics.median(ratios['wall_ns'])
        instructions = ratios['instructions']
        label = 'pass'
        target_aggregate = None
        if name in targets and not stage:
            limit = min(payoff, 1 / band)
            if wall > limit or max(ratios['wall_ns']) >= 1:
                failures.append(name + ': target payoff/noise/all-pairs gate')
                label = 'reject'
            if panels is not None and (panel_required or instruction_blind):
                target_aggregate = geomean([statistics.median(p[name]['wall_ns']) for p in panels])
                all_pairs = [v for p in panels for v in p[name]['wall_ns']]
                if target_aggregate > limit or max(all_pairs) >= 1:
                    failures.append(name + ': predeclared target aggregate/all-pairs gate')
                    label = 'reject'
        else:
            if min(instructions) > instruction_limit and statistics.median(instructions) > 1.03:
                failures.append(name + ': consistent instruction excess exceeds median hard cap')
                label = 'reject'
            elif min(instructions) > instruction_limit:
                events[name] = label = 'control event'
            elif statistics.median(instructions) > instruction_limit:
                missing.append(name + ': inconsistent instruction alert needs longer frozen evidence')
                label = 'incomplete'
            if wall > band and label != 'reject':
                events.setdefault(name, 'wall event')
                if label == 'pass':
                    label = 'wall event'
            if panels:
                if any(min(p[name]['instructions']) > instruction_limit
                       and statistics.median(p[name]['instructions']) > 1.03 for p in panels):
                    failures.append(name + ': panel instruction hard cap exceeded')
                    label = 'reject'
                walls = [statistics.median(p[name]['wall_ns']) for p in panels]
                if all(value > band for value in walls):
                    failures.append(name + ': persistent six-layout wall cost')
                    label = 'reject'
                elif name in events and label != 'reject':
                    label = 'pass'
        cases[name] = {'label': label, 'wall_median': wall, 'wall_band': band,
                       'instructions_median': statistics.median(instructions),
                       'instructions_min': min(instructions), 'instruction_ceiling': instruction_limit,
                       'target_aggregate': target_aggregate}
    for k, values in enumerate(panels or [data]):
        for lane in ('broad', 'sentinel', 'external', 'portfolio'):
            value = geomean([statistics.median(r['wall_ns']) for n, r in values.items()
                             if lane == 'portfolio' or n.startswith(lane + '/')])
            ceiling = policy['lanes'][lane]
            lanes[f'{k}/{lane}'] = {'ratio': value, 'ceiling': ceiling}
            if value > ceiling:
                failures.append(f'layout {k} {lane}: wall lane ceiling exceeded')
    required = bool(events) or instruction_blind or panel_required
    if required and panels is None:
        missing.append('complete K=6 matching-layout panel required')
    held = None
    if held_out is not None:
        values = rows(held_out, policy['held_out'])
        if held_out['pairs'] < (30 if promotion else 5):
            raise ValueError('insufficient held-out pairs')
        held = {n: statistics.median(r['wall_ns']) for n, r in values.items()}
        for name, value in held.items():
            if value > policy['held_out'][name]['wall_band']:
                failures.append(name + ': held-out program band exceeded')
        aggregate = geomean(list(held.values()))
        if aggregate > policy['lanes']['held-out']:
            failures.append('held-out lane ceiling exceeded')
        held = {'cases': held, 'ratio': aggregate, 'ceiling': policy['lanes']['held-out']}
    elif promotion:
        missing.append('mandatory held-out promotion gate missing')
    if failures:
        label = 'reject'
    elif missing:
        label = ('control event' if any(v == 'control event' for v in events.values()) else
                 'wall event' if events else 'incomplete')
    else:
        label = 'pass'
    return {'label': label, 'complete': not missing, 'passed': label == 'pass',
            'failures': failures, 'missing': missing, 'events': events,
            'event_resolution': 'nonpersistent' if events and panels and not failures else None,
            'panel_required': required, 'cases': cases, 'lanes': lanes, 'held_out': held}
