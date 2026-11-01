"""Recalibrate all layout envelopes from one case-sharded schedule.

No serial observations or old ceilings enter the estimator. The independent
held-out A/A supplies its reciprocal confidence floor before qualification.
"""
from __future__ import annotations

import statistics

from .case_schedule import binding, require_same
from .development import MANIFEST_PATH as DEVELOPMENT
from .held_out import MANIFEST_PATH as HELD_OUT
from .layout_identity import digest, same_node
from .layout_policy import geomean, round_up, rows
from .layout_held_out_policy import reciprocal, spread, aa_envelope


def estimate(raw, held_aa, selection_ids, held_ids):
    ids = set(selection_ids) | set(held_ids)
    if set(raw) != {(k, n) for k in range(6) for n in ids}:
        raise ValueError('schedule calibration A2 inventory mismatch')
    from .case_health import require_healthy
    for record in raw.values():
        require_healthy(record)
    pairs = rows(held_aa, held_ids, 30)
    bias = {m: max(reciprocal(statistics.median(raw[(0, n)]['ratios'][m])) for n in ids)
            for m in ('wall_ns', 'instructions')}
    paired, bands = {}, {}
    for n in sorted(ids):
        paired[n] = [statistics.median(raw[(k, n)]['ratios']['wall_ns']) for k in range(6)]
        normalized = [1.] + [statistics.median(raw[(k, n)]['costs']['candidate']['wall_ns']) /
                             statistics.median(raw[(k, n)]['costs']['base']['wall_ns']) for k in range(1, 6)]
        instructions = raw[(0, n)]['ratios']['instructions'] + (pairs[n]['instructions'] if n in held_ids else [])
        band = max(spread(normalized), spread([1.] + paired[n][1:]), bias['wall_ns'])
        if n in held_ids:
            band = max(band, aa_envelope({n: pairs[n]['wall_ns']}))
        bands[n] = {'wall_band': band,
                    'instruction_band': round_up(max(bias['instructions'], max(instructions), 1 / min(instructions)))}
    lanes = {}
    for lane in ('broad', 'sentinel', 'external', 'development', 'portfolio', 'held-out'):
        members = sorted(selection_ids) if lane == 'portfolio' else sorted(n for n in ids if n.startswith(lane + '/'))
        if not members:
            raise ValueError('empty calibration lane: ' + lane)
        placements = [1.] + [geomean([paired[n][k] for n in members]) for k in range(1, 6)]
        lanes[lane] = max(spread(placements), reciprocal(geomean([paired[n][0] for n in members])))
    lanes['held-out'] = max(lanes['held-out'], aa_envelope({n: pairs[n]['wall_ns'] for n in held_ids}))
    return {'cases': {n: bands[n] for n in sorted(selection_ids)},
            'held_out': {n: bands[n] for n in sorted(held_ids)}, 'lanes': lanes,
            'development_manifest_sha256': digest(DEVELOPMENT), 'held_out_manifest_sha256': digest(HELD_OUT)}


def inputs(a2_paths, held_aa, images, parent_sha):
    from .layout_development_a2 import replay
    from .layout_qualification import replay_calibration
    if set(a2_paths) != {'portfolio', 'development', 'held-out'} or set(images) != set(a2_paths):
        raise ValueError('need all three frozen A2 lane sources')
    raw, packets, evidence = {}, [], {}
    for lane, path in a2_paths.items():
        packet, records = replay(path, images[lane], lane)
        if set(raw) & set(records):
            raise ValueError('duplicate calibration case')
        raw.update(records); packets.append(packet)
        evidence[lane + '-a2'] = {'path': str(path), 'sha256': digest(path)}
    pilot = replay_calibration(held_aa, images['held-out'])
    schedule = {'measurement_schedule': binding(4)}
    require_same(schedule, pilot, *packets)
    if any(p['host'] != packets[0]['host']
           or not same_node(p['host_node'], packets[0]['host_node']) for p in packets):
        raise ValueError('A2 calibration host mismatch')
    if pilot['host'] != packets[0]['host']:
        raise ValueError('held-out estimator host mismatch')
    selection = {n for _, n in raw if not n.startswith('held-out/')}
    held = {n for _, n in raw if n.startswith('held-out/')}
    evidence['held-out-estimator-aa'] = {'path': str(held_aa), 'sha256': digest(held_aa)}
    return {'schema_version': 1, 'artifact_type': 'quickjs-schedule-a2-bands',
            'parent_policy_sha256': parent_sha, 'host': packets[0]['host'],
            'host_node': packets[0]['host_node'], **schedule, 'formal_lane_schedule': 'workers-4',
            'source_images': images, 'evidence': evidence, **estimate(raw, pilot, selection, held)}
