#!/usr/bin/env bash
# T066 repair diagnosis H1/H2: non-timing rusage and allocator-page counters.
# Base M versus HEAD, five alternating pairs per case (rusage) and three
# counter-build runs per case. No timing decision and no attempt is consumed.
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
. scripts/lib.sh
cargo_bin="$(qjs_resolve_cargo)"
base_sha=9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30
candidate_sha="$(git rev-parse HEAD)"
base="$(qjs_build_revision "$PWD" "$cargo_bin" "$base_sha")"
candidate="$(qjs_build_revision "$PWD" "$cargo_bin" "$candidate_sha")"
# Counter build: a release CLI with the diagnostic feature. Counts describe the
# allocation mechanism; this build is never timed.
"$cargo_bin" build --release -p qjs-cli --features alloc-counters
counters="$PWD/target/release/qjs"
python3 - "$base" "$candidate" "$counters" "$BENCH_OUT" "$base_sha" "$candidate_sha" <<'PY'
import hashlib, json, os, signal, statistics, subprocess, sys, time
from pathlib import Path
from tools.benchmark.screen import DEFAULT_CACHE, _argv, _check_output, resolve_cases

base, candidate, counters, out = map(Path, sys.argv[1:5])
status = (out.parent / 'status').read_text()
assert 'no-timing' in status, status
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
workloads = out / 'workloads'
workloads.mkdir()
ids = [
    'external/kraken-1.1/imaging-darkroom',
    'external/kraken-1.1/imaging-desaturate',
    'sentinel/capturing_closure_call',
    'development/diff/compare',
    'development/graphlib/paths',
]
cases = {case.id: case for case in resolve_cases(ids, DEFAULT_CACHE, workloads)}
assert set(cases) == set(ids), sorted(cases)
identity = {
    'base': sys.argv[5], 'candidate': sys.argv[6],
    'binaries': {role: {'path': str(path), 'sha256': sha(path)}
                 for role, path in [('base', base), ('candidate', candidate), ('counters', counters)]},
    'workloads': {case_id: sha(case.workload) for case_id, case in cases.items()},
    'timing_decision': False, 'attempts_consumed': 0,
}
(out / 'images.json').write_text(json.dumps(identity, indent=2) + '\n')


def run(argv, timeout=1800):
    """Run one process, reap it with wait4 and return its rusage."""
    stdout = out / 'stdout.txt'
    stderr = out / 'stderr.txt'
    started = time.monotonic()
    with stdout.open('w') as out_f, stderr.open('w') as err_f:
        process = subprocess.Popen(argv, stdout=out_f, stderr=err_f)
    while True:
        tried, status_code, usage = os.wait4(process.pid, os.WNOHANG)
        if tried == process.pid:
            break
        if time.monotonic() - started > timeout:
            os.kill(process.pid, signal.SIGKILL)
            os.wait4(process.pid, 0)
            raise SystemExit(f'timeout: {argv}')
        time.sleep(0.01)
    process.returncode = os.waitstatus_to_exitcode(status_code)
    return {
        'exit_code': process.returncode,
        'stdout': stdout.read_text(),
        'stderr': stderr.read_text(),
        'wall_s': time.monotonic() - started,
        'minflt': usage.ru_minflt,
        'majflt': usage.ru_majflt,
        'utime': usage.ru_utime,
        'stime': usage.ru_stime,
        'maxrss': usage.ru_maxrss,
    }


def argv_for(binary, case):
    iterations = 3_000_000 if case.kind == 'internal' else None
    return _argv(binary, case, iterations)


rusage_runs = []
for case_id, case in cases.items():
    for repeat in range(5):
        order = [('base', base), ('candidate', candidate)]
        if repeat % 2:
            order.reverse()
        for role, binary in order:
            sample = run(argv_for(binary, case))
            assert sample['exit_code'] == 0, (case_id, role, sample['exit_code'], sample['stderr'][-400:])
            _check_output(case, sample['stdout'])
            rusage_runs.append({'id': case_id, 'role': role, 'repeat': repeat, **sample})
            (out / 'rusage.json').write_text(json.dumps(rusage_runs, indent=2) + '\n')

rows = []
for case_id, case in cases.items():
    medians = {}
    for role in ('base', 'candidate'):
        samples = [r for r in rusage_runs if r['id'] == case_id and r['role'] == role]
        assert len(samples) == 5
        medians[role] = {key: statistics.median(s[key] for s in samples)
                         for key in ('minflt', 'majflt', 'utime', 'stime', 'maxrss')}
    rows.append({
        'id': case_id,
        'base': medians['base'],
        'candidate': medians['candidate'],
        'ratio': {key: (medians['candidate'][key] / medians['base'][key]
                        if medians['base'][key] else None)
                  for key in medians['base']},
    })

counter_runs = []
for case_id, case in cases.items():
    for repeat in range(3):
        sample = run(argv_for(counters, case))
        assert sample['exit_code'] == 0, (case_id, sample['exit_code'], sample['stderr'][-400:])
        line = next((line for line in sample['stderr'].splitlines()
                     if line.startswith('QJS_ALLOC_COUNTERS')), '')
        assert line, (case_id, sample['stderr'][-400:])
        values = {key: int(value) for key, value in
                  (field.split('=') for field in line.split()[1:])}
        cpu_s = sample['utime'] + sample['stime']
        counter_runs.append({'id': case_id, 'repeat': repeat, **values,
                             'wall_s': sample['wall_s'], 'cpu_s': cpu_s,
                             'recycles_per_cpu_s': values['page_recycles'] / cpu_s if cpu_s else None,
                             'system_allocs_per_cpu_s': values['system_page_allocs'] / cpu_s if cpu_s else None})
        (out / 'counters.json').write_text(json.dumps(counter_runs, indent=2) + '\n')

summary = {'rusage': rows, 'counters': [
    {'id': case_id,
     **{key: statistics.median(r[key] for r in counter_runs if r['id'] == case_id)
        for key in ('system_page_allocs', 'system_page_frees', 'empty_pool_hits',
                    'page_recycles', 'recycles_per_cpu_s', 'system_allocs_per_cpu_s')}}
    for case_id in cases], 'images': identity}
(out / 'h1.json').write_text(json.dumps(summary, indent=2) + '\n')
for row in rows:
    ratio = row['ratio']
    print(f"{row['id']}\tminflt {row['base']['minflt']:.0f}->{row['candidate']['minflt']:.0f} "
          f"({ratio['minflt']:.3f})\tstime {row['base']['stime']:.3f}->{row['candidate']['stime']:.3f} "
          f"({ratio['stime']:.3f})\tutime {ratio['utime']:.3f}\tmaxrss {ratio['maxrss']:.3f}")
for row in summary['counters']:
    print(f"{row['id']}\tpages +{row['system_page_allocs']:.0f} -{row['system_page_frees']:.0f} "
          f"pool {row['empty_pool_hits']:.0f} recycles {row['page_recycles']:.0f} "
          f"({row['recycles_per_cpu_s']:.0f}/cpu-s)")
PY
