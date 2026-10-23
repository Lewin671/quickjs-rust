"""T076 non-timing instruction and exact-base profile collector.

Run through imac-bench.sh run (without --timing). Inventory and fixed instruction
counts are declared before execution. Wall/rusage bytes are retained only for
audit; no timing calibration, performance acceptance or held-out selection.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess

from tools.benchmark import screen
from tools.benchmark.triage_build import build


POLICY = Path('benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v6.json')
BASE = '19125d48c626869d776df9a501c44f3a00566e32'
PAIRS = 5
ITERATIONS = 1 << 20
TARGETS = {
    'u1': ['development/graphlib/paths', 'development/immutable/persistent',
           'development/lodash/transform', 'development/matter-js/simulate'],
    'u2': ['development/big.js/arith'],
    'u3': ['development/immutable/persistent', 'development/js-yaml/document',
           'development/lodash/transform', 'development/papaparse/csv'],
}
UNION = sorted({name for names in TARGETS.values() for name in names})


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', default=BASE)
    parser.add_argument('--candidate')
    parser.add_argument('--profiles', action='store_true')
    args = parser.parse_args()
    out = Path(os.environ['BENCH_OUT']).resolve()
    status = (out.parent / 'status').read_text()
    assert 'no-timing' in status, 'instruction/profile collection must be non-timing'
    revisions = {'base': args.base}
    if args.candidate:
        revisions['candidate'] = args.candidate
    revisions = {role: subprocess.check_output(
        ['git', 'rev-parse', ref + '^{commit}'], text=True).strip()
        for role, ref in revisions.items()}
    ids = list(json.loads(POLICY.read_text())['cases'])
    assert len(ids) == 86 and set(UNION) <= set(ids)
    request = dict(revisions=revisions, cases=ids, targets=TARGETS, pairs=PAIRS,
                   internal_iterations=ITERATIONS, timing=False,
                   metric='median of per-pair candidate/base instructions; N/2N increment for internal cases',
                   policy_sha256=digest(POLICY), profile_targets=UNION if args.profiles else [])
    write(out / 'request.json', request)
    binaries = {}
    for role, revision in revisions.items():
        folder = out / ('base000' if role == 'base' else 'cand000')
        folder.mkdir()
        binaries[role] = folder / 'qjs'
        identity = build(revision, binaries[role])
        write(folder / 'identity.json', identity)
        for command, suffix in ((['nm', '-n'], 'nm'), (['otool', '-tvV'], 'asm')):
            with (folder / (suffix + '.txt')).open('w') as stream:
                subprocess.run([*command, str(binaries[role])], stdout=stream, check=True)
    work = out / 'workloads'
    work.mkdir()
    cases = screen.resolve_cases(ids, screen.DEFAULT_CACHE, work)
    rows = []
    signatures = {}
    for case in cases:
        for pair in range(PAIRS):
            roles = list(binaries) if pair % 2 == 0 else list(reversed(binaries))
            for role in roles:
                counts = []
                sizes = [ITERATIONS, 2 * ITERATIONS] if case.kind == 'internal' else [None]
                for size in sizes:
                    stem = case.id.replace('/', '--') + f'.{pair}.{role}.{size}'
                    argv = screen._argv(binaries[role], case, size)
                    process = subprocess.run(['/usr/bin/time', '-l', *argv],
                                             capture_output=True, text=True, timeout=1800)
                    (out / (stem + '.stdout')).write_text(process.stdout)
                    raw = out / (stem + '.rusage')
                    raw.write_text(process.stderr)
                    assert process.returncode == 0, (case.id, process.stderr[-1500:])
                    signature = screen._check_output(case, process.stdout)
                    key = (case.id, size)
                    assert signatures.setdefault(key, signature) == signature, (key, "checksum drift")
                    measured = screen.parse_macos_time(process.stderr, cycles=False)
                    instructions = measured['instructions']
                    assert instructions > 0
                    counts.append(instructions)
                    rows.append(dict(id=case.id, pair=pair, role=role, iterations=size,
                                     instructions=instructions, argv=argv,
                                     binary_sha256=digest(binaries[role]),
                                     workload_sha256=digest(case.workload),
                                     stdout_sha256=digest(out / (stem + '.stdout')),
                                     raw_sha256=digest(raw), raw=raw.name))
                    write(out / 'processes.json', rows)
                assert len(counts) == 1 or counts[1] > counts[0], (case.id, counts)
        print('COUNTED', case.id, flush=True)
    summary = []
    for case in cases:
        counts = {}
        for role in binaries:
            counts[role] = []
            for pair in range(PAIRS):
                values = [r['instructions'] for r in rows
                          if r['id'] == case.id and r['role'] == role and r['pair'] == pair]
                counts[role].append(values[-1] - values[0] if len(values) == 2 else values[0])
        row = dict(id=case.id, counts=counts)
        if 'candidate' in counts:
            row['ratios'] = [c / b for b, c in zip(counts['base'], counts['candidate'])]
            row['ratio'] = statistics.median(row['ratios'])
        summary.append(row)
    write(out / 'instructions.json', summary)
    if args.profiles:
        for case in cases:
            if case.id not in UNION:
                continue
            for repeat in range(3):
                folder = out / 'profiles' / case.id.replace('/', '--') / str(repeat)
                folder.mkdir(parents=True)
                argv = screen._argv(binaries['base'], case, None)
                with (folder / 'stdout').open('w') as stdout, (folder / 'stderr').open('w') as stderr:
                    process = subprocess.Popen(argv, stdout=stdout, stderr=stderr)
                    raw = folder / 'raw.sample'
                    command = ['sample', str(process.pid), '60', '1', '-mayDie', '-file', str(raw)]
                    sampler = subprocess.run(command, capture_output=True, text=True, timeout=100)
                    result = process.wait(timeout=180)
                (folder / 'sampler.log').write_text(sampler.stdout + sampler.stderr)
                assert result == 0 and sampler.returncode == 0
                screen._check_output(case, (folder / 'stdout').read_text())
                subprocess.run([
                    'python3', '-m', 'tools.benchmark.profile', '--input', str(raw),
                    '--binary', str(binaries['base']), '--workload', str(case.workload),
                    '--base-sha', revisions['base'], '--opportunity-id', case.id,
                    '--tool', 'macOS sample 1ms complete process',
                    '--command-json', json.dumps(command), '--output-dir', str(folder / 'receipt'),
                ], check=True)
                print('PROFILE', case.id, repeat, flush=True)
    write(out / 'complete.json', dict(processes=len(rows), cases=len(cases),
                                      instructions_sha256=digest(out / 'instructions.json')))


if __name__ == '__main__':
    main()
