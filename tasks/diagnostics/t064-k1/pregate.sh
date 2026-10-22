#!/usr/bin/env bash
# Fixed stage-K instruction pre-gate; one pair, no timing decision or attempt.
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
. scripts/lib.sh
cargo_bin="$(qjs_resolve_cargo)"
base_sha=9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30
candidate_sha="$(git rev-parse HEAD)"
base="$(qjs_build_revision "$PWD" "$cargo_bin" "$base_sha")"
candidate="$(qjs_build_revision "$PWD" "$cargo_bin" "$candidate_sha")"
python3 - "$base" "$candidate" "$BENCH_OUT" "$base_sha" "$candidate_sha" <<'PY'
import hashlib, json, os, subprocess, sys
from pathlib import Path
base, candidate, out = map(Path, sys.argv[1:4])
assert 'no-timing' in (Path(os.environ['BENCH_OUT']).parent / 'status').read_text()
policy_path = Path('benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v6.json')
policy = json.loads(policy_path.read_text())['cases']
assert len(policy) == 86
identity = {'base': sys.argv[4], 'candidate': sys.argv[5],
            'binaries': {role: {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
                         for role, path in [('base', base), ('candidate', candidate)]},
            'policy_sha256': hashlib.sha256(policy_path.read_bytes()).hexdigest(),
            'timing_decision': False, 'attempts_consumed': 0}
(out / 'images.json').write_text(json.dumps(identity, indent=2) + '\n')
# Use the existing collector exactly as the preceding T056 instruction
# pre-gates did. Incidental wall/cycle fields cannot authorize a stage screen.
args = ['python3', '-m', 'tools.benchmark.screen', '--base', str(base),
        '--candidate', str(candidate), '--pairs', '1', '--symbols', '40',
        '--json', str(out / 'pregate.json')]
for case in policy:
    args.extend(['--case', case])
with (out / 'pregate.txt').open('x') as stdout, (out / 'pregate.err').open('x') as stderr:
    subprocess.run(args, stdout=stdout, stderr=stderr, check=True)
report = json.loads((out / 'pregate.json').read_text())
assert report['pairs'] == 1 and len(report['cases']) == 86
assert {c['id'] for c in report['cases']} == set(policy)
rows = []
for case in report['cases']:
    assert len(case['ratios']['instructions']) == 1
    ratio = case['ratios']['instructions'][0]
    cutoff = max(1.03, policy[case['id']]['instruction_band'])
    rows.append({'id': case['id'], 'ratio': ratio, 'cutoff': cutoff, 'pass': ratio <= cutoff})
rows.sort(key=lambda row: row['ratio'], reverse=True)
result = {'pass': all(row['pass'] for row in rows), 'cases': rows, 'images': identity}
(out / 'verdict.json').write_text(json.dumps(result, indent=2) + '\n')
for row in rows:
    print(f"{row['id']}\t{row['ratio']:.8f}\t{row['cutoff']:.8f}\t{row['pass']}")
print('PRE-GATE', 'PASS' if result['pass'] else 'BLOCK')
raise SystemExit(0 if result['pass'] else 3)
PY
