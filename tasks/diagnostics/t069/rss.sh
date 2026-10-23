#!/usr/bin/env bash
# T069 peak RSS: all development programs, frozen plan base versus HEAD.
# Fifteen alternating pairs; median(candidate)/median(base), per-case <= 1.10
# and development geomean <= 1.05. This rule precedes candidate measurements.
# One direct process at a time; ru_maxrss from wait4, never a timing claim.
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
. scripts/lib.sh
cargo_bin="$(qjs_resolve_cargo)"
base_sha="$(python3 -c 'import json; print(json.load(open("tasks/performance-units/positional-call-activation.json"))["base_sha"])')"
candidate_sha="$(git rev-parse HEAD)"
base="$(qjs_build_revision "$PWD" "$cargo_bin" "$base_sha")"
candidate="$(qjs_build_revision "$PWD" "$cargo_bin" "$candidate_sha")"
python3 - "$base" "$candidate" "$BENCH_OUT" "$base_sha" "$candidate_sha" <<'PY'
import hashlib, json, math, os, statistics, sys
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
targets = json.loads(Path("tasks/performance-units/positional-call-activation.json").read_text())["fast_gate"]["target_ids"]
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
                 'target': case.id in targets, 'over_case_ceiling': ratio > 1.10})
geomean = math.exp(sum(math.log(row['ratio']) for row in rows) / len(rows))
within_rule = all(not row['over_case_ceiling'] for row in rows) and geomean <= 1.05
result = {'cases': rows, 'targets': targets, 'case_ceiling': 1.10,
          'development_geomean': geomean, 'geomean_ceiling': 1.05,
          'all_within_rule': within_rule, 'images': identity}
(out / 'verdict.json').write_text(json.dumps(result, indent=2) + '\n')
for row in sorted(rows, key=lambda row: row['ratio'], reverse=True):
    print(f"{row['id']}\t{row['base_median_bytes']}\t{row['candidate_median_bytes']}\t"
          f"{row['ratio']:.6f}\tpaired {row['paired_ratio_median']:.4f} [{row['paired_ratio_q1']:.4f},{row['paired_ratio_q3']:.4f}]\t"
          f"{'OVER' if row['over_case_ceiling'] else 'ok'}")
print(f"RSS development geomean {geomean:.6f} (ceiling 1.05)",
      'PASS' if within_rule else 'OVER-RULE')
raise SystemExit(0 if within_rule else 3)
PY
