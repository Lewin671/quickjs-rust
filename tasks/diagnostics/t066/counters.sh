#!/usr/bin/env bash
# T066 repair diagnosis: one non-timing run per case with the alloc-counters
# build, printing the allocator counters and user CPU time.
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
. scripts/lib.sh
cargo_bin="$(qjs_resolve_cargo)"
candidate_sha="$(git rev-parse HEAD)"
"$cargo_bin" build --release -p qjs-cli --features alloc-counters
binary="$PWD/target/release/qjs"
python3 - "$binary" "$BENCH_OUT" "$candidate_sha" <<'PY'
import json, os, subprocess, sys, time
from pathlib import Path
from tools.benchmark.screen import DEFAULT_CACHE, _argv, _check_output, resolve_cases

binary, out = map(Path, sys.argv[1:3])
out.mkdir(parents=True, exist_ok=True)
ids = [
    'external/kraken-1.1/imaging-darkroom',
    'external/kraken-1.1/imaging-desaturate',
    'sentinel/capturing_closure_call',
    'development/diff/compare',
    'development/graphlib/paths',
]
workloads = out / 'workloads'
workloads.mkdir(exist_ok=True)
cases = {case.id: case for case in resolve_cases(ids, DEFAULT_CACHE, workloads)}
rows = []
for case_id, case in cases.items():
    iterations = 3_000_000 if case.kind == 'internal' else None
    stdout = out / 'stdout.txt'
    stderr = out / 'stderr.txt'
    started = time.monotonic()
    with stdout.open('w') as out_f, stderr.open('w') as err_f:
        process = subprocess.Popen(_argv(binary, case, iterations), stdout=out_f, stderr=err_f)
    waited, status_code, usage = os.wait4(process.pid, 0)
    assert waited == process.pid
    assert os.waitstatus_to_exitcode(status_code) == 0
    _check_output(case, stdout.read_text())
    line = next(line for line in stderr.read_text().splitlines()
                if line.startswith('QJS_ALLOC_COUNTERS'))
    values = {key: int(value) for key, value in
              (field.split('=') for field in line.split()[1:])}
    cpu = usage.ru_utime + usage.ru_stime
    row = {'id': case_id, 'cpu_s': cpu, 'wall_s': time.monotonic() - started,
           'minflt': usage.ru_minflt, 'stime': usage.ru_stime, 'utime': usage.ru_utime,
           **values}
    rows.append(row)
    (out / 'counters.json').write_text(json.dumps(rows, indent=2) + '\n')
    print(case_id, json.dumps({k: row[k] for k in (
        'utime', 'stime', 'minflt', 'block_allocs', 'block_frees',
        'scan_visits', 'system_page_allocs', 'empty_pool_hits', 'page_recycles')}))
PY
