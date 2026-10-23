"""Measure the fixed six-layout candidate/base panel, or held-out acceptance.

All variants are built/audited before timing. Paths are distinct, equal-length
and created once; variant k is compared only to baseline variant k. Completion
appends variants 1..5 to an immutable natural screen, selection or held-out.
"""
from __future__ import annotations

import argparse
import platform
import json
import os
from pathlib import Path

from . import screen
from .case_health import mark_readiness, exit_status
from .layout_identity import OFFSETS, digest, host_identity, write_json
from .layout_policy import load_policy
from .screen_recording import held_out_cases, measure


def matrix(path):
    value = json.loads(path.read_text())
    if value.get('static_pass') is not True or value.get('offsets') != list(OFFSETS):
        raise ValueError('unaudited/incomplete layout build')
    if len(value['variants']) != 6:
        raise ValueError('layout build must contain six variants')
    for k, variant in enumerate(value['variants']):
        if variant['index'] != k or variant['pad_bytes'] != OFFSETS[k]:
            raise ValueError('layout variant ordering mismatch')
        if digest(Path(variant['path'])) != variant['sha256']:
            raise ValueError('layout variant hash mismatch')
    return value


def measure_panel(base_path, candidate_path, output, pairs=5, held=False, natural=False, complete=False, held_panel=False, development=False, workers=1):
    from .pair_disturbance import Monitor, quieting_preflight, verify_run
    if held_panel and (not held or natural or complete):
        raise ValueError('held-out panel requires --held-out alone')
    from .case_schedule import binding, collect, require_same
    schedule = binding(workers)
    quieting = quieting_preflight()
    monitor = Monitor.from_queue()
    host = host_identity()
    base, candidate = matrix(base_path), matrix(candidate_path)
    if (base['host'] != host or candidate['host'] != host
            or base['host_node'] != platform.node() or candidate['host_node'] != platform.node()):
        raise ValueError('layout build host/recipe differs from measurement host')
    binaries = [Path(v['path']) for m in (base, candidate) for v in m['variants']]
    if len(set(binaries)) != 12 or len({len(os.fsencode(p)) for p in binaries}) != 1:
        raise ValueError('panel needs twelve distinct stable equal-length binary paths')
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=complete)
    if not complete:
        (output / 'work').mkdir()
    previous = None
    if complete:
        previous = json.loads((output / 'panel.json').read_text())
        if (previous['base'] != base or previous['candidate'] != candidate
                or previous['pairs'] != pairs or len(previous['panels']) != 1 or previous['held_out'] != held
                or previous.get('development', False) != development):
            raise ValueError('completion must preserve the exact natural screen')
        require_same(previous, {'measurement_schedule': schedule})
        from .layout_evidence import replay
        ids = {c['id']: {} for c in previous['panels'][0]['cases']}
        replay(output / 'panel.json', {'host': host, 'host_node': platform.node(),
               'held_out' if held else 'cases': ids, 'measurement_schedule': schedule}, held=held)
    tool = screen.counter_tool()
    with screen.measurement_lock('layout-panel'):
        if screen.wait_for_builds(300):
            raise ValueError('competing builds remain')
        screen.check_load(8, True, 300)
        preflight = screen.launch_preflight(binaries, 120)
        cases = (held_out_cases(screen.DEFAULT_CACHE, output / 'work') if held else
                 screen.resolve_cases(['broad', 'sentinel', 'external'] + (['development'] if development else []), screen.DEFAULT_CACHE, output / 'work'))
        packets = list(previous['panels']) if previous else []
        variants = list(range(1, 6) if complete else [0] if (held and not held_panel) or natural else range(6))
        role_paths = {k: {'base': binaries[k], 'candidate': binaries[6+k]} for k in variants}
        measured, scheduling = collect(cases, variants, role_paths, pairs, tool, 120,
                                       output, monitor, workers, measure=measure)
        packets.extend(measured)
    disturbance = monitor.snapshot()
    new_files = list(output.glob('variant[1-5]/*.json')) if complete else list(output.glob('variant*/*.json'))
    counts = verify_run([json.loads(p.read_text()) for p in new_files], disturbance)
    evidence = {'schema_version': 1, 'artifact_type': 'quickjs-layout-panel',
                'host': host, 'host_node': platform.node(), 'base': base, 'candidate': candidate, 'held_out': held,
                'pairs': pairs, 'panels': packets, 'quieting': quieting, **scheduling,
                'disturbance_runs': [*(previous.get('disturbance_runs', []) if previous else []),
                    {'collection_seconds': scheduling['collection_seconds'], 'variants': list(range(1,6)) if complete else [0] if (held and not held_panel) or natural else list(range(6)),
                     'load': disturbance, 'counts': counts}], 'launch_preflight': preflight,
                'case_files': {str(p.relative_to(output)): digest(p)
                               for p in output.glob('variant*/*.json')}}
    if development and not held:
        from .development import MANIFEST_PATH
        evidence.update(development=True, development_manifest_sha256=digest(MANIFEST_PATH))
    mark_readiness(evidence)
    write_json(output / ('complete.json' if complete else 'panel.json'), evidence)
    return evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base-matrix', type=Path, required=True)
    parser.add_argument('--candidate-matrix', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--held-out-panel', action='store_true', help='all six held-out layouts for host precision fallback')
    parser.add_argument('--workers', type=int, choices=range(1, 9), default=1)
    parser.add_argument('--pairs', type=int, default=5)
    parser.add_argument('--development', action='store_true', help='include the selectable npm lane (v3 policy)')
    parser.add_argument('--held-out', action='store_true', help='held-out acceptance; with --complete, the v4 K=6 held-out panel')
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument('--natural', action='store_true')
    modes.add_argument('--complete', action='store_true', help='append variants 1..5 to an immutable natural screen')
    args = parser.parse_args()
    if args.held_out and args.natural:
        parser.error('--held-out measures natural placement already')
    if args.pairs < (30 if args.held_out else 5):
        raise ValueError('need >=5 panel pairs or >=30 held-out promotion pairs')
    evidence = measure_panel(args.base_matrix, args.candidate_matrix, args.output, args.pairs, args.held_out, args.natural, args.complete, args.held_out_panel, args.development, args.workers)
    return exit_status(evidence)


if __name__ == '__main__':
    raise SystemExit(main())
