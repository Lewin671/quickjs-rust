# Argument: CORRECTNESS_JOB. No timing, frozen 86-case instruction admission.
python3 - "$@" <<'PY'
import sys
from common import bound,correctness,require,canonical_images
require(len(sys.argv)==2,'usage: pre-gate CORRECTNESS_JOB')
bound();correctness(sys.argv[1]);canonical_images()
PY
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
exact_job="$1"
python3 - "$BENCH_QUEUE/$exact_job" "$(git rev-parse HEAD)" <<'PY'
import json,sys
from pathlib import Path
from tools.benchmark.performance_evaluation import _test262_zero_gap
p=Path(sys.argv[1]); assert (p/'status').read_text().strip()=='done 0'
assert 'DISTURBED' not in (p/'log').read_text()
r=json.loads((p/'out/burndown-entry.json').read_text()); assert _test262_zero_gap(r,sys.argv[2])
PY
. scripts/lib.sh
cargo_bin="$(qjs_resolve_cargo)"
base_sha=f58c696e8379261ed726f0cea1051ed6d34f431a
cand_sha="$(git rev-parse HEAD)"
base="$(qjs_build_revision "$PWD" "$cargo_bin" "$base_sha")"
cand="$(qjs_build_revision "$PWD" "$cargo_bin" "$cand_sha")"
python3 - "$BENCH_QUEUE/$exact_job/out/image.json" "$cand" <<'PY'
import hashlib,json,sys
from pathlib import Path
r=json.loads(Path(sys.argv[1]).read_text()); assert r['binary_sha256']==hashlib.sha256(Path(sys.argv[2]).read_bytes()).hexdigest()
PY
{ echo "base $base_sha"; echo "candidate $cand_sha"; shasum -a 256 "$base" "$cand"; uname -m; } > "$BENCH_OUT/images.txt"
cases=()
while IFS= read -r case_id; do cases+=(--case "$case_id"); done < <(python3 -c 'import json; print("\n".join(json.load(open("benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v6.json"))["cases"]))')
echo "cases $((${#cases[@]} / 2))" >> "$BENCH_OUT/images.txt"
python3 -m tools.benchmark.screen --candidate "$cand" --base "$base" --pairs 1 "${cases[@]}" --symbols 40 --json "$BENCH_OUT/pregate.json" > "$BENCH_OUT/pregate.txt" 2> "$BENCH_OUT/pregate.err"
python3 - "$BENCH_OUT/pregate.json" > "$BENCH_OUT/verdict.txt" <<'PY'
import json,statistics,sys
r=json.load(open(sys.argv[1]));policy=json.load(open('benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v6.json'))['cases']
rows=[];blocked=[]
assert r['pairs']==1 and len(r['cases'])==86 and len({c['id'] for c in r['cases']})==86
for c in r['cases']:
    assert len(c['ratios']['instructions'])==1
    ratio=statistics.median(c['ratios']['instructions']);cutoff=max(1.03,policy[c['id']]['instruction_band'])
    rows.append((ratio,c['id'],cutoff))
    if ratio>cutoff:blocked.append(c['id'])
missing=sorted(set(policy)-{c['id'] for c in r['cases']})
for ratio,name,cutoff in sorted(rows,reverse=True):print('%s\t%.8f\tcutoff %.4f%s'%(name,ratio,cutoff,'\tBLOCK' if ratio>cutoff else ''))
print('cases',len(rows),'missing',missing);print('VERDICT','pass' if not blocked and not missing else 'block',blocked)
if blocked or missing:sys.exit(3)
PY
cat "$BENCH_OUT/verdict.txt"
