"""Frozen 30-pair A/A qualification at the layout decision statistic.

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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--sha256', required=True)
    parser.add_argument('--lane', choices=('portfolio', 'held-out'), required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
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
    write_json(out / 'identity.json', {'host': identity, 'sha256': args.sha256,
        'binaries': list(map(str, binaries)), 'pairs': 30, 'lane': args.lane})
    tool = screen.counter_tool()
    (out / 'cases').mkdir()
    (out / 'work').mkdir()
    with screen.measurement_lock('layout-calibration'):
        if screen.wait_for_builds(300):
            raise ValueError('competing builds remain')
        screen.check_load(8, True, 300)
        preflight = screen.launch_preflight(binaries, 120)
        cases = (held_out_cases(screen.DEFAULT_CACHE, out / 'work') if args.lane == 'held-out'
                 else screen.resolve_cases(['broad', 'sentinel', 'external'], screen.DEFAULT_CACHE, out / 'work'))
        results = []
        for case in cases:
            print('CALIBRATE', case.id, flush=True)
            result = measure(case, binaries[1], binaries[0], 30, tool, 120,
                             out / 'cases' / (case.id.replace('/', '--') + '.json'))
            results.append(dataclasses_asdict(result))
    write_json(out / 'screen.json', {'artifact_type': 'quickjs-layout-calibration',
        'pairs': 30, 'cases': results, 'launch_preflight': preflight, 'host': identity})


def dataclasses_asdict(value):
    import dataclasses
    return dataclasses.asdict(value)


if __name__ == '__main__':
    main()
