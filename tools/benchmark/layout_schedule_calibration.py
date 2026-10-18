"""Queue-stage orchestration for a frozen case-sharded calibration plan.

Every stage is a separate queue job; prior inputs are immutable paths. Only
load-disturbed runs may be replaced, at the frozen cap. Qualification never
changes the seed and leaves publication to a reviewed repository change.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

from .layout_identity import digest, host_identity, write_json


def load(path):
    return json.loads(path.read_text())


def retry(output, count, run):
    for attempt in range(count + 1):
        target = output / f'attempt{attempt}'
        try:
            result = run(target)
            write_json(output / 'result.json', {'path': str(result), 'sha256': digest(result)})
            return result
        except (ValueError, subprocess.CalledProcessError) as error:
            # Subprocess errors need their retained stderr checked by the caller;
            # they are not presumed to be load failures.
            if not isinstance(error, ValueError) or not str(error).startswith('DISTURBED:') or attempt == count:
                raise


def receipt(folder):
    value = load(folder / 'result.json')
    path = Path(value['path'])
    if digest(path) != value['sha256']:
        raise ValueError('calibration stage receipt changed')
    return path


def clone_matrix(source, root):
    from .layout_panel import matrix
    value = matrix(source)
    root.mkdir(parents=True)
    for k, variant in enumerate(value['variants']):
        target = root / 'binaries' / f'variant{k}' / 'qjs'
        target.parent.mkdir(parents=True)
        shutil.copy2(variant['path'], target)
        variant['path'] = str(target)
    path = root / 'identity.json'
    write_json(path, value)
    matrix(path)
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--stage', choices=('prepare','a2','estimator','seed','pilot','formal','formal-power','qualify','speed'), required=True)
    parser.add_argument('--inputs', type=Path, help='prepared shared root, normally earlier queue job/out')
    parser.add_argument('--seed', type=Path)
    parser.add_argument('--lane', choices=('portfolio','development','held-out'))
    parser.add_argument('--workers', type=int, choices=(1,4), default=4)
    parser.add_argument('--a2', type=Path, action='append', default=[])
    parser.add_argument('--estimator', type=Path)
    parser.add_argument('--pilot', type=Path, action='append', default=[])
    parser.add_argument('--formal', type=Path)
    parser.add_argument('--formal-power', type=Path)
    args = parser.parse_args()
    plan = load(args.plan)
    from .case_schedule import binding
    if plan['measurement_schedule'] != binding(4):
        raise ValueError('this qualification protocol requires workers=4')
    out = Path(os.environ['BENCH_OUT'])
    queue = Path(os.environ['BENCH_QUEUE'])
    stage = args.stage
    if stage == 'prepare':
        from .layout_variants import build
        matrix = build(plan['portfolio_source_revision'], out/'portfolio-matrix')
        if matrix['variants'][0]['sha256'] != plan['portfolio_image']:
            raise ValueError('rebuilt band-source image changed')
        npm = clone_matrix(queue/plan['npm_matrix'], out/'npm-matrix')
        if load(npm)['revision'] != plan['npm_source_revision'] or load(npm)['variants'][0]['sha256'] != plan['npm_image']:
            raise ValueError('npm source image changed')
        clone_matrix(npm, out/'base000')
        clone_matrix(npm, out/'cand000')
        return 0
    if stage == 'a2':
        from .layout_development_a2 import collect
        source = args.inputs / ('portfolio-matrix' if args.lane == 'portfolio' else 'npm-matrix') / 'identity.json'
        image = plan['portfolio_image'] if args.lane == 'portfolio' else plan['npm_image']
        retry(out, plan['max_load_replacements'], lambda target: collect(source, target, image, args.lane, 4))
    elif stage in ('estimator', 'pilot'):
        binary = args.inputs/'npm-matrix/binaries/variant0/qjs'
        from .layout_calibration import collect
        retry(out, plan['max_load_replacements'], lambda target: collect(binary, plan['npm_image'], args.lane, target, 4))
    elif stage == 'seed':
        from .layout_schedule_bands import inputs
        lanes = {load(receipt(p))['artifact_type'].removeprefix('quickjs-').removesuffix('-layout-a2'): receipt(p) for p in args.a2}
        seed = inputs(lanes, receipt(args.estimator),
                      {'portfolio': plan['portfolio_image'], 'development': plan['npm_image'], 'held-out': plan['npm_image']}, plan['parent_sha256'])
        seed['calibration_plan'] = {'path': str(args.plan), 'sha256': digest(args.plan)}
        write_json(out/'seed.json', seed)
    elif stage == 'formal':
        from .layout_schedule_publish import formal_stage
        retry(out, plan['max_load_replacements'], lambda target: formal_stage(plan, target))
    elif stage == 'formal-power':
        subprocess.run([sys.executable, '-m', 'tools.benchmark.layout_formal_continuation',
                        '--bundle',str(receipt(args.formal).parent),'--output',str(out/'power')], check=True)
    elif stage == 'qualify':
        from .layout_schedule_publish import qualify_stage
        return qualify_stage(args, plan, out)
    elif stage == 'speed':
        from .layout_panel import measure_panel
        root = out/'panel'
        base, cand = [args.inputs/p/'identity.json' for p in ('base000','cand000')]
        natural = measure_panel(base, cand, root, natural=True, development=True, workers=args.workers)
        panel = measure_panel(base, cand, root, complete=True, development=True, workers=args.workers)
        write_json(out/'speed.json', {'workers': args.workers, 'screen_seconds': natural['collection_seconds'],
                   'panel_seconds': sum(r['collection_seconds'] for r in panel['disturbance_runs']),
                   'natural_sha256':digest(root/'panel.json'), 'panel_sha256':digest(root/'complete.json')})
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
