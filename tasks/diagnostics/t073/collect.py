"""T073 diagnostic only: retain every fresh-process sample; never gate or retry.

Run from the repository root through imac-bench.sh run --timing. Uses the
unchanged T072 audited matrices, verifies their natural hashes, and preserves
the screen's /usr/bin/time launch/counter path. Extra environment padding and
an optional SETEXEC launcher are explicit interventions, not acceptance data.
"""
import argparse
import dataclasses
import hashlib
import json
import os
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from tools.benchmark import screen
from tools.benchmark.process import run_process

HASHES = {'base': 'cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e',
          'candidate': 'f6d5220992f3dd7d05afaad4af2e169a01258294e7995347d28f81e4ca74a5e3'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--repeats', type=int, default=60)
    parser.add_argument('--variant', type=int, choices=range(6), default=0)
    parser.add_argument('--pads', default='-1')
    parser.add_argument('--counts', default='23068672,46137344')
    parser.add_argument('--case', default='local_read')
    parser.add_argument('--launcher', type=Path)
    parser.add_argument('--aslr', choices=['on', 'off', 'both'], default='on')
    parser.add_argument('--probe-identity', '--binary-identity', type=Path)
    parser.add_argument('--within-candidate', nargs=2, type=int, choices=range(6))
    parser.add_argument('--matrix-job', default='20261023T154044Z-t072-screen-attempt2')
    args = parser.parse_args()
    pads = list(range(0, 4096, 16)) if args.pads == 'page' else list(map(int, args.pads.split(',')))
    assert os.environ.get('BENCH_OUT'), 'use the timing queue'
    args.output.mkdir(parents=True, exist_ok=False)
    matrix = Path(os.environ['BENCH_QUEUE']) / args.matrix_job / 'out/attempt2'
    binaries = {}
    for role, directory in [('base', 'base000'), ('candidate', 'cand000')]:
        matrix_file = matrix / directory / 'identity.json'
        expected_matrix = {'base': '585cfe4ed2eb97c2be2f5e3564be515b9bc41cabd01326fa26067aaca10a4d89',
                           'candidate': '99687a16d1050dcfff8047d24a24e1c7317b7217f0b98671c240af91c3eab2de'}
        assert hashlib.sha256(matrix_file.read_bytes()).hexdigest() == expected_matrix[role]
        identity = json.loads(matrix_file.read_text())
        variant = identity['variants'][args.variant]
        binary = Path(variant['path'])
        actual = hashlib.sha256(binary.read_bytes()).hexdigest()
        assert actual == variant['sha256']
        if args.variant == 0:
            assert actual == HASHES[role]
        binaries[role] = binary
    if args.within_candidate:
        assert not args.probe_identity and args.variant == 0
        for role, index in zip(['base', 'candidate'], args.within_candidate):
            variant = identity['variants'][index]  # last verified matrix is candidate
            binaries[role] = Path(variant['path'])
            assert hashlib.sha256(binaries[role].read_bytes()).hexdigest() == variant['sha256']
    if args.probe_identity:
        assert args.variant == 0
        receipt = json.loads(args.probe_identity.read_text())
        kind = receipt.get('kind', 'address-probe')
        assert kind in ['address-probe', 'pie-header-control']
        if kind == 'address-probe':
            assert os.environ.get('T073_PROBE') == '1'
        else:
            assert not os.environ.get('T073_PROBE')
        for role in binaries:
            binaries[role] = Path(receipt[role]['binary'])
            assert hashlib.sha256(binaries[role].read_bytes()).hexdigest() == receipt[role]['sha256']
    tool = screen.counter_tool()
    spec = args.case if '/' in args.case else 'broad/' + args.case
    case = screen.resolve_cases([spec], screen.DEFAULT_CACHE, args.output)[0]
    metadata = {'arguments': vars(args) | {'output': str(args.output), 'launcher': str(args.launcher), 'probe_identity': str(args.probe_identity)},
                'hashes': {k: hashlib.sha256(v.read_bytes()).hexdigest() for k, v in binaries.items()}, 'binaries': {k: str(v) for k, v in binaries.items()},
                'environment_bytes': sum(len(k) + len(v) + 2 for k, v in os.environ.items()),
                'cwd': os.getcwd(), 'cycles_trusted': tool.cycles}
    (args.output / 'request.json').write_text(json.dumps(metadata, indent=2) + '\n')
    preflight = []
    for binary in binaries.values():
        argv = [str(binary), '-e', '1;']
        if args.launcher:
            argv = [str(args.launcher.resolve()), 'on' if args.aslr == 'both' else args.aslr, '-1', *argv]
        preflight.append(dataclasses.asdict(run_process([*tool.prefix, *argv], 30)))
        assert preflight[-1]['exit_code'] == 0 and not preflight[-1]['timed_out']
    (args.output / 'preflight.json').write_text(json.dumps(preflight, indent=2) + '\n')
    expected = {}
    with (args.output / 'samples.jsonl').open('x') as stream:
        for repeat in range(args.repeats):
            for pad in pads:
                roles = ['base', 'candidate'] if repeat % 2 == 0 else ['candidate', 'base']
                modes = (['on', 'off'] if repeat % 2 == 0 else ['off', 'on']) if args.aslr == 'both' else [args.aslr]
                for mode, role in ((m, r) for m in modes for r in roles):
                    for count in map(int, args.counts.split(',')):
                        argv = screen._argv(binaries[role], case, count)
                        if args.launcher:
                            argv = [str(args.launcher.resolve()), mode, str(pad), *argv]
                        elif pad >= 0:
                            argv = ['/usr/bin/env', 'T073_PAD=' + 'x' * pad, *argv]
                        result = run_process([*tool.prefix, *argv], 120)
                        row = {'repeat': repeat, 'pad': pad, 'role': role, 'mode': mode, 'count': count,
                               'result': dataclasses.asdict(result)}
                        stream.write(json.dumps(row) + '\n'); stream.flush()
                        assert result.exit_code == 0 and not result.timed_out, row
                        checked = screen._check_output(case, result.stdout)
                        assert checked == expected.setdefault(count, checked)
                        if case.id == 'broad/local_read':
                            assert json.loads(checked) == {'operations': count * 2, 'checksum': count * 3}
                        counters = tool.parse(result.stderr)
                        print(repeat, pad, mode, role, count, result.duration_ns,
                              counters['user_ns'], counters['instructions'], flush=True)


if __name__ == '__main__':
    main()
