#!/usr/bin/env bash
# T066 non-timing peak-RSS cohort, precision rerun: every development case, base M versus HEAD.
# Lead-declared before running (2026-10-22): 15 alternating pairs per case; the plan metric is
# median(candidate)/median(base); paired-ratio median and quartiles are reported as noise context only.
# One direct process at a time; ru_maxrss from wait4, never a timing claim.
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
. scripts/lib.sh
cargo_bin="$(qjs_resolve_cargo)"
base_sha=9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30
candidate_sha="$(git rev-parse HEAD)"
base="$(qjs_build_revision "$PWD" "$cargo_bin" "$base_sha")"
candidate="$(qjs_build_revision "$PWD" "$cargo_bin" "$candidate_sha")"
python3 - "$base" "$candidate" "$BENCH_OUT" "$base_sha" "$candidate_sha" <<'PY'
import hashlib, json, os, statistics, sys
from pathlib import Path
from tools.benchmark.resource_process import run_process_wait4
from tools.benchmark.screen import DEFAULT_CACHE, _argv, _check_output, resolve_cases

base, candidate, out = map(Path, sys.argv[1:4])
REPEATS = 15
status = (out.parent / 'status').read_text()
assert 'no-timing' in status, status
workloads = out / 'workloads'
workloads.mkdir()
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
cases = resolve_cases(['development'], DEFAULT_CACHE, workloads)
assert len(cases) == 10, len(cases)
targets = [
    'development/diff/compare',
    'development/graphlib/paths',
    'development/js-yaml/document',
    'development/immutable/persistent',
]
identity = {
    'base': sys.argv[4], 'candidate': sys.argv[5],
    'binaries': {role: {'path': str(path), 'sha256': sha(path)}
                 for role, path in [('base', base), ('candidate', candidate)]},
    'workloads': {case.id: sha(case.workload) for case in cases},
    'timing_decision': False, 'attempts_consumed': 0,
}
(out / 'images.json').write_text(json.dumps(identity, indent=2) + '\n')
runs = []
for case in cases:
    for repeat in range(REPEATS):
        order = [('base', base), ('candidate', candidate)]
        if repeat % 2:
            order.reverse()
        for role, binary in order:
            result = run_process_wait4(_argv(binary, case, None), 1800)
            assert result.exit_code == 0 and not result.timed_out, (
                case.id, role, result.exit_code, result.timed_out, result.stderr[-400:])
            _check_output(case, result.stdout)
            assert result.peak_rss_bytes is not None, (case.id, role)
            runs.append({'id': case.id, 'role': role, 'repeat': repeat,
                         'peak_rss_bytes': result.peak_rss_bytes,
                         'descendants_detected': result.descendants_detected,
                         'exit_code': result.exit_code})
            (out / 'rss.json').write_text(json.dumps(runs, indent=2) + '\n')
rows = []
for case in cases:
    medians = {}
    for role in ('base', 'candidate'):
        values = [run['peak_rss_bytes'] for run in runs
                  if run['id'] == case.id and run['role'] == role]
        assert len(values) == REPEATS, (case.id, role, len(values))
        medians[role] = int(statistics.median(values))
    ratio = medians['candidate'] / medians['base']
    pairs = sorted(
        next(r['peak_rss_bytes'] for r in runs if r['id'] == case.id and r['role'] == 'candidate' and r['repeat'] == k)
        / next(r['peak_rss_bytes'] for r in runs if r['id'] == case.id and r['role'] == 'base' and r['repeat'] == k)
        for k in range(REPEATS))
    quart = statistics.quantiles(pairs, n=4)
    rows.append({'paired_ratio_median': statistics.median(pairs), 'paired_ratio_q1': quart[0], 'paired_ratio_q3': quart[2],
                 'base_spread': [min(r['peak_rss_bytes'] for r in runs if r['id'] == case.id and r['role'] == 'base'),
                                 max(r['peak_rss_bytes'] for r in runs if r['id'] == case.id and r['role'] == 'base')],
                 'id': case.id, 'base_median_bytes': medians['base'],
                 'candidate_median_bytes': medians['candidate'], 'ratio': ratio,
                 'target': case.id in targets, 'over_1_05': ratio > 1.05})
result = {'cases': rows, 'targets': targets, 'ceiling': 1.05,
          'all_within_ceiling': all(not row['over_1_05'] for row in rows),
          'images': identity}
(out / 'verdict.json').write_text(json.dumps(result, indent=2) + '\n')
for row in sorted(rows, key=lambda row: row['ratio'], reverse=True):
    print(f"{row['id']}\t{row['base_median_bytes']}\t{row['candidate_median_bytes']}\t"
          f"{row['ratio']:.6f}\tpaired {row['paired_ratio_median']:.4f} [{row['paired_ratio_q1']:.4f},{row['paired_ratio_q3']:.4f}]\t"
          f"{'OVER' if row['over_1_05'] else 'ok'}")
print('RSS', 'PASS' if result['all_within_ceiling'] else 'OVER-CEILING')
raise SystemExit(0 if result['all_within_ceiling'] else 3)
PY
