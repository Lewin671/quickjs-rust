"""A/A qualification: frozen 30-pair pilot with automatic precision continuation.

Collect only through the site's serialized timing queue. This command never
widens an existing lane budget or silently replaces a failed run.
"""
from __future__ import annotations

import argparse
import shutil
from pathlib import Path

from . import screen
from .layout_identity import digest, host_identity, write_json
from .screen_recording import held_out_cases, measure


def collect(binary, sha256, lane, output, workers=1):
    from types import SimpleNamespace
    args = SimpleNamespace(binary=binary, sha256=sha256, lane=lane, output=output, workers=workers)
    from .case_schedule import binding, collect
    schedule = binding(args.workers)
    from .pair_disturbance import Monitor, quieting_preflight, verify_run
    quieting = quieting_preflight()
    monitor = Monitor.from_queue()
    identity = host_identity()
    if digest(args.binary) != args.sha256:
        raise ValueError('calibration binary hash mismatch')
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binaries = []
    for role in ('base0', 'cand0'):
        target = out / role / 'qjs'
        target.parent.mkdir()
        shutil.copy2(args.binary, target)
        binaries.append(target)
    binding = {}
    if args.lane == 'development':
        from .development import MANIFEST_PATH
        binding['development_manifest_sha256'] = digest(MANIFEST_PATH)
    write_json(out / 'identity.json', {'host': identity, 'sha256': args.sha256,
        'binaries': list(map(str, binaries)), 'measurement_schedule': schedule, 'pairs': 30, 'lane': args.lane, **binding})
    tool = screen.counter_tool()
    (out / 'work').mkdir()
    with screen.measurement_lock('layout-calibration'):
        if screen.wait_for_builds(300):
            raise ValueError('competing builds remain')
        screen.check_load(8, True, 300)
        preflight = screen.launch_preflight(binaries, 120)
        cases = (held_out_cases(screen.DEFAULT_CACHE, out / 'work') if args.lane == 'held-out'
                 else screen.resolve_cases(['development'] if args.lane == 'development' else
                    ['broad', 'sentinel', 'external'], screen.DEFAULT_CACHE, out / 'work'))
        packets, scheduling = collect(cases, [0], {0: {'base': binaries[0], 'candidate': binaries[1]}},
                                       30, tool, 120, out, monitor, args.workers)
        results = packets[0]['cases']
        (out / 'variant0').rename(out / 'cases')
    import json
    disturbance = monitor.snapshot()
    conservation = verify_run([json.loads(p.read_text()) for p in (out / 'cases').glob('*.json')], disturbance)
    write_json(out / 'screen.json', {'artifact_type': 'quickjs-layout-calibration',
        'pairs': 30, 'cases': results, **scheduling, 'disturbance': disturbance,
        'disturbance_counts': conservation, 'quieting': quieting, 'launch_preflight': preflight, 'host': identity, **binding})

    if any('health' in row for row in results):
        # Retain the completed inventory, but do not allocate precision retries
        # or calculate any pilot statistic from a failed required case.
        return out / 'screen.json'

    # The pilot stays sealed. Each deficient case gets a separate immutable
    # chain; original joint lane cohorts never acquire fabricated observations.
    from .layout_continuation import complete
    from .layout_policy import analyze
    from .layout_identity import command
    context=command(['git','rev-parse','HEAD'])
    deficient = [r['id'] for r in results if analyze({r['id']:r['ratios']['wall_ns']})['relative_half_width']>.03]
    if args.workers == 1:
        for name in deficient:
            complete(out/'screen.json', name, context, out/'power'/name.replace('/','--'))
    elif deficient:
        from functools import partial
        from .worker_pool import Task, run_tasks
        with screen.measurement_lock('layout-power-allocation'):
            run_tasks([Task('power ' + name, partial(complete, out/'screen.json', name,
                      context, out/'power'/name.replace('/','--'), locked=False)) for name in deficient], args.workers)
    continuations=[]
    for name in deficient:
        path=out/'power'/name.replace('/','--')/'continuation.json'
        continuations.append({'path':str(path),'sha256':digest(path)})
    write_json(out/'power.json',{'continuations':continuations})
    return out/'screen.json'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--sha256', required=True)
    parser.add_argument('--lane', choices=('portfolio', 'held-out', 'development'), required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--workers', type=int, choices=range(1, 9), default=1)
    args = parser.parse_args()
    collect(args.binary, args.sha256, args.lane, args.output, args.workers)


if __name__ == '__main__':
    main()
