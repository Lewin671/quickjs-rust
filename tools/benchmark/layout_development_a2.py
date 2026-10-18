"""Extend the measured placement envelope to an npm lane (development or held-out).

Compare every placement with natural; matching-layout optimization screens are
a different experiment. All paths are immutable, with raw/load conservation.
"""
from __future__ import annotations

import argparse
import json
import platform
import shutil
import statistics
from pathlib import Path

from . import screen
from .development import MANIFEST_PATH, opportunity_ids
from .layout_identity import OFFSETS, digest, host_identity, write_json
from .layout_panel import matrix
from .layout_policy import geomean, round_up, rows
from .pair_disturbance import Monitor, quieting_preflight, verify_run
from .pair_recording import measure
from .screen_replay import replay_case

LANES = {'development': ('quickjs-development-layout-a2', 'development_manifest_sha256'),
         'held-out': ('quickjs-held-out-layout-a2', 'held_out_manifest_sha256')}


def lane_inputs(lane):
    """Artifact type, manifest binding, manifest path and frozen ids of one lane."""
    if lane == 'held-out':
        from .held_out import MANIFEST_PATH as path, opportunity_ids as ids
    else:
        path, ids = MANIFEST_PATH, opportunity_ids
    return (*LANES[lane], path, ids())


def lane_cases(lane, work):
    if lane == 'held-out':
        from .screen_recording import held_out_cases
        return held_out_cases(screen.DEFAULT_CACHE, work)
    return screen.resolve_cases(['development'], screen.DEFAULT_CACHE, work)


def collect(matrix_path, output, expected_hash, lane='development'):
    artifact, key, manifest, _ = lane_inputs(lane)
    quieting = quieting_preflight()
    monitor = Monitor.from_queue()
    source = matrix(matrix_path)
    host = host_identity()
    if source['host'] != host or source['host_node'] != platform.node():
        raise ValueError('A2 matrix host mismatch')
    if source['variants'][0]['sha256'] != expected_hash:
        raise ValueError('A2 natural image differs from frozen source')
    out = output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    (out / 'work').mkdir()
    binaries = {}
    for k in range(6):
        binaries[k] = {}
        for role, variant in (('base0', 0), ('cand0', k)):
            target = out / role / f'variant{k}' / 'qjs'
            target.parent.mkdir(parents=True)
            shutil.copy2(source['variants'][variant]['path'], target)
            binaries[k]['base' if role == 'base0' else 'candidate'] = str(target)
    packets = []
    with screen.measurement_lock('development-a2'):
        if screen.wait_for_builds(300):
            raise ValueError('competing builds remain')
        screen.check_load(8, True, 300)
        preflight = screen.launch_preflight([Path(p) for pair in binaries.values() for p in pair.values()], 120)
        cases = lane_cases(lane, out / 'work')
        for k in range(6):
            folder = out / f'variant{k}'
            folder.mkdir()
            results = []
            for case in cases:
                print(lane.upper(), 'A2', k, case.id, flush=True)
                value = measure(case, Path(binaries[k]['candidate']), Path(binaries[k]['base']),
                                5, screen.counter_tool(), 120,
                                folder / (case.id.replace('/', '--') + '.json'), monitor)
                import dataclasses
                results.append(dataclasses.asdict(value))
            packets.append({'pairs': 5, 'cases': results})
    files = {str(p.relative_to(out)): digest(p) for p in out.glob('variant*/*.json')}
    load = monitor.snapshot()
    counts = verify_run([json.loads((out / p).read_text()) for p in files], load)
    value = {'schema_version': 1, 'artifact_type': artifact,
             'host': host, 'host_node': platform.node(), 'source': source,
             key: digest(manifest),
             'binaries': binaries, 'panels': packets, 'case_files': files,
             'quieting': quieting, 'launch_preflight': preflight, 'load': load, 'counts': counts}
    write_json(out / 'a2.json', value)
    replay(out / 'a2.json', expected_hash, lane)
    return out / 'a2.json'


def replay(path, expected_hash, lane='development'):
    artifact, key, manifest, ids = lane_inputs(lane)
    value = json.loads(path.read_text())
    if (value.get('artifact_type') != artifact or value.get('schema_version') != 1
            or value.get(key) != digest(manifest)):
        raise ValueError(lane + ' A2 contract mismatch')
    source = value['source']
    if (not source['static_pass'] or source['offsets'] != list(OFFSETS)
            or source['variants'][0]['sha256'] != expected_hash or len(value['panels']) != 6
            or source['host'] != value['host'] or source['host_node'] != value['host_node']):
        raise ValueError('development A2 matrix mismatch')
    files, raw = set(), {}
    for k, packet in enumerate(value['panels']):
        rows(packet, ids, 5)
        binaries = value['binaries'][str(k)]
        if len({len(str(p).encode()) for p in binaries.values()}) != 1 or len(set(binaries.values())) != 2:
            raise ValueError('A2 paths must be distinct/equal-length')
        for role, index in (('base', 0), ('candidate', k)):
            if digest(Path(binaries[role])) != source['variants'][index]['sha256']:
                raise ValueError('A2 executable changed')
        for case in packet['cases']:
            name = f"variant{k}/{case['id'].replace('/', '--')}.json"
            files.add(name)
            if digest(path.parent / name) != value['case_files'].get(name):
                raise ValueError('A2 raw hash mismatch')
            record = json.loads((path.parent / name).read_text())
            if record['id'] != case['id'] or record['ratios'] != case['ratios']:
                raise ValueError('A2 raw/report mismatch')
            replay_case(record, 5, binaries)
            raw[(k, case['id'])] = record
    if files != set(value['case_files']) or verify_run(list(raw.values()), value['load']) != value['counts']:
        raise ValueError('A2 inventory/load conservation mismatch')
    return value, raw


def bands(parent, original, raw):
    """A2's two layout estimators; longer A/A never enlarges these budgets."""
    old = {c['id']: c['metrics'] for c in original['cases']}
    ids = sorted(opportunity_ids())
    if set(old) != set(parent['cases']) or set(raw) != {(k, n) for k in range(6) for n in ids}:
        raise ValueError('A2 extension case inventory mismatch')
    reciprocal = lambda v: max(v, 1 / v)
    spread = lambda vs: max(vs) / min(vs)
    bias = {m: max([reciprocal(old[n][m]['aa_median']) for n in old] +
                  [reciprocal(statistics.median(raw[(0, n)]['ratios'][m])) for n in ids])
            for m in ('wall_ns', 'instructions')}
    paired = {n: old[n]['wall_ns']['paired_median_ratios'] for n in old}
    cases = dict(parent['cases'])
    for n in ids:
        paired[n] = [statistics.median(raw[(k, n)]['ratios']['wall_ns']) for k in range(6)]
        normalized = [1.] + [statistics.median(raw[(k, n)]['costs']['candidate']['wall_ns']) /
                            statistics.median(raw[(k, n)]['costs']['base']['wall_ns']) for k in range(1, 6)]
        instructions = raw[(0, n)]['ratios']['instructions']
        cases[n] = {'wall_band': max(spread(normalized), spread([1.] + paired[n][1:]), bias['wall_ns']),
                    'instruction_band': round_up(max(bias['instructions'], max(instructions), 1 / min(instructions)))}
    lanes = dict(parent['lanes'])
    for lane, members in (('development', ids), ('portfolio', sorted(paired))):
        placements = [1.] + [geomean([paired[n][k] for n in members]) for k in range(1, 6)]
        lanes[lane] = max(spread(placements), reciprocal(geomean([paired[n][0] for n in members])))
    return {'cases': cases, 'lanes': lanes, 'development_manifest_sha256': digest(MANIFEST_PATH)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--matrix', type=Path, required=True)
    parser.add_argument('--sha256', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    collect(args.matrix, args.output, args.sha256)


if __name__ == '__main__':
    main()
