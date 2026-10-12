"""Reproduce T038 pricing, paths, and sampling; run under the host lock.

Never time a perf-counters binary. The price mode enforces the exact recorded
Rust/NG executable hashes. Counter mode is diagnostic only. Outputs stay in
target/, and no command here changes runtime or third_party source.
"""
from __future__ import annotations

import argparse
from collections import Counter
from dataclasses import asdict
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
from tools.benchmark.measure_lock import busy_processes, wait_for_builds
from tools.benchmark.profile import import_profile
from tools.benchmark.screen import counter_tool, sample
from tofte_kernels import generate

RUST_SHA = '6878520ff5fc8fc480e9fd8086c2b5caeb4ba609236ea423cd5a6d573a6fc54b'
NG_SHA = 'cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0'
BASE = 'af4d0d3c81aad98721661689cf7224d2c17b9781'
GROUPS = ('component', 'format', 'extra', 'matched', 'search')


def quiet() -> None:
    busy = wait_for_builds(60)
    if busy:
        raise RuntimeError(f'competing processes remain: {busy}')


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def binaries(args: argparse.Namespace) -> dict[str, Path]:
    result = {}
    for role, source, expected in [('r', args.rust, RUST_SHA), ('n', args.ng, NG_SHA)]:
        if digest(source) != expected:
            raise ValueError(f'{role}: executable differs from T038; reprice at a new revision')
        destination = args.output_dir / role / 'qjs'
        destination.parent.mkdir(exist_ok=True)
        shutil.copy2(source, destination)
        result[role] = destination
    return result


def prices(args: argparse.Namespace) -> None:
    engines = binaries(args)
    tool = counter_tool()
    rows = {}
    groups = GROUPS if args.group == 'all' else (args.group,)
    for group in groups:
        counts = json.loads((args.output_dir / f'{group}-kernels.json').read_text())['N']
        for key in (['tofte'] if group == 'component' else []) + list(counts):
            quiet()
            forms = [(0, 0)] if key == 'tofte' else [(c, k) for c in (4, 8) for k in (4, 8)]
            rows[key] = {}
            for c, k in forms:
                path = args.output_dir / (f'{key}-{c}-{k}.js' if c else 'tofte.js')
                cell = {r: [] for r in engines}
                rows[key][f'{c},{k}'] = cell
                def one(role: str) -> dict:
                    flag = '--raw' if role == 'r' else '--script'
                    return asdict(sample(tool, [str(engines[role]), flag, str(path)], 120))
                for role in engines:
                    one(role)
                for pair in range(5):
                    quiet()
                    for role in (('r', 'n') if pair % 2 == 0 else ('n', 'r')):
                        cell[role].append(one(role))
                    if busy_processes():
                        raise RuntimeError('build appeared during pair; discard and rerun this case')
            result = {'samples': rows[key], 'N': counts.get(key)}
            if key != 'tofte':
                result['prices'] = {}
                for role in engines:
                    result['prices'][role] = {}
                    for metric in ('cycles', 'instructions'):
                        def median(c: int, k: int) -> float:
                            return statistics.median(v[metric] for v in rows[key][f'{c},{k}'][role])
                        d4 = (median(8, 4) - median(4, 4)) / (4 * counts[key])
                        d8 = (median(8, 8) - median(4, 8)) / (4 * counts[key])
                        result['prices'][role][metric] = {'A': 2*d4-d8, 'B': (d8-d4)/4,
                                                          'D4': d4, 'D8': d8}
                print(key, json.dumps(result['prices']), flush=True)
            else:
                print(key, {r: {m: statistics.median(v[m] for v in rows[key]['0,0'][r])
                                for m in ('cycles', 'instructions', 'wall_ns')} for r in engines}, flush=True)
            (args.output_dir / f'{key}-result.json').write_text(json.dumps(result, indent=2))


def counters(args: argparse.Namespace) -> None:
    files = [args.output_dir / 'tofte.js']
    files += sorted(args.output_dir.glob('*-4-4.js')) + sorted(args.output_dir.glob('*-8-8.js'))
    results = {}
    for path in files:
        quiet()
        completed = subprocess.run([str(args.counter), '--raw', str(path)], check=True,
                                   capture_output=True, text=True,
                                   env={**os.environ, 'QJS_CF_TRACE': '1', 'QJS_TL_TRACE': '1'})
        events = {m[1]: int(m[2]) for line in completed.stderr.splitlines()
                  if (m := re.match(r'^\s*(\w+) (\d+)$', line))}
        traces = Counter(line[:500] for line in completed.stderr.splitlines()
                         if line.startswith(('CFVM ', 'CFOK ', 'CFDECLINE ', 'CFEXIT ',
                                             'CFNATIVE ', 'TLFAIL ', 'TLGIVEUP ')))
        results[path.name] = {'counters': events, 'traces': dict(traces), 'stdout': completed.stdout}
        print(path.name, json.dumps(results[path.name]), flush=True)
    (args.output_dir / 'counters.json').write_text(json.dumps(results, indent=2))
    engines = binaries(args)
    for role, engine in engines.items():
        quiet()
        result = subprocess.run([str(engine), '--raw' if role == 'r' else '--script',
                                 str(args.output_dir / 'source-counts.js')],
                                check=True, capture_output=True, text=True)
        (args.output_dir / f'{role}-source-counts.stdout').write_text(result.stdout)


def profile(args: argparse.Namespace) -> None:
    engine = binaries(args)['r']
    workload = args.output_dir / 'amplified.js'
    workload.write_text(((args.output_dir / 'tofte.js').read_text() + '\n') * 128)
    quiet()
    with (args.output_dir / 'profile.stdout').open('w') as output:
        process = subprocess.Popen([str(engine), '--raw', str(workload)],
                                   stdout=output, stderr=subprocess.PIPE, text=True)
        command = ['sample', str(process.pid), '5', '1', '-mayDie', '-file',
                   str(args.output_dir / 'tofte.sample')]
        sampler = subprocess.run(command, check=True, capture_output=True, text=True)
        _, error = process.communicate(timeout=120)
        if process.returncode:
            raise RuntimeError(error)
    receipt = import_profile(argparse.Namespace(
        input=args.output_dir / 'tofte.sample', binary=engine, workload=workload,
        base_sha=BASE, opportunity_id=['external/sunspider-1.0/date-format-tofte'],
        tool='macOS sample 1 ms; amplified source diagnostic only',
        command_json=json.dumps(command), output_dir=args.output_dir / 'receipt'))
    print(json.dumps({'receipt': receipt, 'command': command, 'sampler': sampler.stdout}))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('generate', 'price', 'counter', 'profile'))
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--rust', type=Path)
    parser.add_argument('--ng', type=Path)
    parser.add_argument('--counter', type=Path)
    parser.add_argument('--group', choices=(*GROUPS, 'all'), default='all')
    parser.add_argument('--output-dir', type=Path, default=ROOT / 'target/tofte-reproduce')
    args = parser.parse_args()
    args.output_dir = args.output_dir.resolve()
    generate(args.output_dir, args.bundle.resolve())
    if args.mode != 'generate' and (args.rust is None or args.ng is None):
        parser.error('--rust and --ng are required for measurements')
    if args.mode == 'counter' and args.counter is None:
        parser.error('--counter is required for diagnostic counting')
    {'price': prices, 'counter': counters, 'profile': profile}.get(args.mode, lambda _: None)(args)


if __name__ == '__main__':
    main()
