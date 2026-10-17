"""Dual-metric layout-robust verdicts; no benchmark-specific exceptions.

A single placement can open an event, never establish persistent case cost.
Targets, whole-lane ceilings and held-out acceptance remain independent gates.
Policies declaring HELD_OUT_RULE apply the same persistence test to held-out.
"""
from __future__ import annotations

import statistics

from .layout_policy import HELD_OUT_RULE, geomean, rows, selection_lanes
from .layout_power import diagnostic_only, needs_panel


def evaluate(policy, screen, targets, payoff, *, panel=None, held_out=None, held_out_panel=None,
             instruction_blind=False, memory_targets=(), promotion=False, panel_required=False, stage=False, stage_limit=1.25):
    data = rows(screen, policy['cases'])
    if screen['pairs'] < 5:
        raise ValueError('layout verdict needs at least five pairs')
    fallback_panel = needs_panel(policy)
    mandatory_panel = panel_required or instruction_blind or fallback_panel
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
        diagnostic = diagnostic_only(policy,name)
        band = policy['cases'][name]['wall_band']
        if stage:
            band = stage_limit if diagnostic else min(band, stage_limit)
        instruction_limit = max(1.01, policy['cases'][name]['instruction_band'])
        wall = statistics.median(ratios['wall_ns'])
        instructions = ratios['instructions']
        label = 'pass'
        target_aggregate = None
        if name in targets and not stage:
            limit = payoff if diagnostic else min(payoff, 1 / band)
            if wall > limit or max(ratios['wall_ns']) >= 1:
                failures.append(name + ': target payoff/noise/all-pairs gate')
                label = 'reject'
            if panels is not None and mandatory_panel:
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
                if (not diagnostic or stage) and all(value > band for value in walls):
                    failures.append(name + ': persistent six-layout wall cost')
                    label = 'reject'
                elif name in events and label != 'reject':
                    label = 'pass'
        cases[name] = {'label': label, 'wall_median': wall, 'wall_band': band,
                       'instructions_median': statistics.median(instructions),
                       'instructions_min': min(instructions), 'instruction_ceiling': instruction_limit,
                       'target_aggregate': target_aggregate, 'diagnostic_only':diagnostic}
    persistence = policy.get('held_out_rule') == HELD_OUT_RULE
    for lane in selection_lanes(policy):
        ceiling = policy['lanes'][lane]
        values = []
        for k, layout in enumerate(panels or [data]):
            values.append(geomean([statistics.median(r['wall_ns']) for n, r in layout.items()
                                   if lane == 'portfolio' or n.startswith(lane + '/')]))
            lanes[f'{k}/{lane}'] = {'ratio': values[-1], 'ceiling': ceiling}
            if values[-1] > ceiling and not persistence:
                failures.append(f'layout {k} {lane}: wall lane ceiling exceeded')
        if persistence:
            lane_event(lane, values, ceiling, panels is not None, events, failures)
    required = bool(events) or mandatory_panel
    if required and panels is None:
        missing.append('complete K=6 matching-layout panel required')
    held = None
    if held_out_panel is not None and (held_out is None or not persistence):
        raise ValueError('held-out persistence panel requires a v4 policy and its natural screen')
    if held_out is not None:
        values = rows(held_out, policy['held_out'])
        if held_out['pairs'] < (30 if promotion else 5):
            raise ValueError('insufficient held-out pairs')
    if held_out is not None and persistence:
        held = held_out_persistence(policy, held_out, values, held_out_panel, failures, missing)
    elif held_out is not None:
        held = {n: statistics.median(r['wall_ns']) for n, r in values.items()}
        for name, value in held.items():
            if not diagnostic_only(policy,name,held=True) and value > policy['held_out'][name]['wall_band']:
                failures.append(name + ': held-out program band exceeded')
        aggregate = geomean(list(held.values()))
        if aggregate > policy['lanes']['held-out']:
            failures.append('held-out lane ceiling exceeded')
        held = {'cases': held, 'ratio': aggregate, 'ceiling': policy['lanes']['held-out']}
    elif promotion:
        missing.append('mandatory held-out promotion gate missing')
    held_events = bool(held and held.get('events'))
    if failures:
        label = 'reject'
    elif missing:
        label = ('control event' if any(v == 'control event' for v in events.values()) else
                 'wall event' if events or held_events else 'incomplete')
    else:
        label = 'pass'
    return {'label': label, 'complete': not missing, 'passed': label == 'pass',
            'failures': failures, 'missing': missing, 'events': events,
            'event_resolution': 'nonpersistent' if events and panels and not failures else None,
            'panel_required': required, 'cases': cases, 'lanes': lanes, 'held_out': held}


def held_out_persistence(policy, natural, values, panel, failures, missing):
    """Held-out wall events resolve by six matching layouts, like controls.

    Instructions only explain a wall excess: a consistent excess above
    max(1.01, I_i) with the wall above B_i rejects in any layout. A wall-only
    excess rejects only when all six medians exceed B_i; the lane likewise.
    """
    panels = None
    if panel is not None:
        if len(panel) != 6:
            raise ValueError('held-out panel requires exactly six matching variants')
        if panel[0]['cases'] != natural['cases']:
            raise ValueError('natural held-out screen differs from held-out panel variant zero')
        if any(packet['pairs'] != natural['pairs'] for packet in panel):
            raise ValueError('held-out panel pair counts differ')
        panels = [rows(packet, policy['held_out']) for packet in panel]
    events, programs, lanes = {}, {}, {}
    for name, ratios in values.items():
        bands = policy['held_out'][name]
        band, ceiling = bands['wall_band'], max(1.01, bands['instruction_band'])
        diagnostic = diagnostic_only(policy, name, held=True)
        layouts = panels or [values]
        walls = [statistics.median(p[name]['wall_ns']) for p in layouts]
        label = 'pass'
        if any(wall > band and min(p[name]['instructions']) > ceiling
               for wall, p in zip(walls, layouts)):
            failures.append(name + ': held-out wall excess with consistent instruction regression')
            label = 'reject'
        elif walls[0] > band:
            events[name] = label = 'held-out wall event'
            if panels is not None and not diagnostic and all(w > band for w in walls):
                failures.append(name + ': persistent six-layout held-out wall cost')
                label = 'reject'
            elif panels is not None:
                label = 'pass'
        programs[name] = {'label': label, 'wall_median': walls[0], 'wall_band': band,
                          'layout_wall_medians': walls if panels else None,
                          'instructions_median': statistics.median(ratios['instructions']),
                          'instructions_min': min(ratios['instructions']),
                          'instruction_ceiling': ceiling, 'diagnostic_only': diagnostic}
    ceiling = policy['lanes']['held-out']
    for k, layout in enumerate(panels or [values]):
        lanes[str(k)] = geomean([statistics.median(r['wall_ns']) for r in layout.values()])
    before = len(failures)
    lane_event('held-out', list(lanes.values()), ceiling, panels is not None, events, failures)
    required = bool(events) or needs_panel(policy, held=True)
    if required and panels is None:
        missing.append('complete K=6 held-out panel required')
    resolved = bool(events) and panels is not None and not any(
        p['label'] == 'reject' for p in programs.values()) and len(failures) == before
    return {'rule': HELD_OUT_RULE, 'cases': {n: p['wall_median'] for n, p in programs.items()},
            'programs': programs, 'ratio': lanes['0'], 'ceiling': ceiling, 'layout_lanes': lanes,
            'events': events, 'panel_required': required,
            'event_resolution': 'nonpersistent' if resolved else None}


def lane_event(lane, values, ceiling, complete, events, failures):
    """v4: a natural lane excess is an event; six neutral layouts set the ceiling.

    The ceiling is the maximum of six neutral layouts, so a neutral seventh
    placement exceeds it about one time in seven. With the complete panel the
    lane rejects only if its six-layout geomean or every layout exceeds it.
    """
    if values[0] > ceiling:
        events[lane + ' lane'] = 'lane event'
    if complete and (geomean(values) > ceiling or all(v > ceiling for v in values)):
        failures.append(f'{lane}: six-layout lane geomean or every layout exceeds ceiling')
