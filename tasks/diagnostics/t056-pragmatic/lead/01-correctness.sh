# No timing. Require complete exact evidence for this new image; older candidates cannot cover it.
python3 - <<'PY'
import os,subprocess,re
from common import bound,canonical_images,out,require
bound();canonical_images()
env=dict(os.environ)
for key in ['QJS_CLI_BIN','QJS_CLI_PROFILE','QJS_AGENTS']:env.pop(key,None)
env['QJS_CHECK_SKIP_TEST262']='0'
with (out()/'check.log').open('x') as f:subprocess.run(['./scripts/check.sh'],env=env,stdout=f,stderr=subprocess.STDOUT,check=True)
require(re.search(r'^ok: ran 5170 Test262 subset cases$',(out()/'check.log').read_text(),re.M),'full 5170 subset absent')
with (out()/'compare-qjs.log').open('x') as f:subprocess.run(['./scripts/compare-qjs.sh'],env=env,stdout=f,stderr=subprocess.STDOUT,check=True)
require(sum(s.startswith('ok: ') for s in (out()/'compare-qjs.log').read_text().splitlines())==255,'fixture inventory drift')
PY
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
. scripts/lib.sh
cargo_bin="$(qjs_resolve_cargo)"
cand_sha="$(git rev-parse HEAD)"
cand="$(qjs_build_revision "$PWD" "$cargo_bin" "$cand_sha")"
python3 - "$cand_sha" "$cand" "$BENCH_OUT/image.json" <<'PY'
import hashlib,json,sys
from pathlib import Path
p=Path(sys.argv[2]); Path(sys.argv[3]).write_text(json.dumps({'commit':sys.argv[1], 'path':str(p), 'binary_sha256':hashlib.sha256(p.read_bytes()).hexdigest()},indent=2)+'\n')
PY
mkdir -p "$BENCH_OUT/test262"
pids=()
for shard in 1 2 3 4 5 6 7 8; do
  QJS_AGENTS=1 QJS_CLI_PROFILE=release QJS_CLI_BIN="$cand" TEST262_BASELINE_JOBS=1 TEST262_TIMEOUT_RETRIES=1 TEST262_CASE_TIMEOUT_SECONDS=20 ./scripts/test262-baseline.sh --all --engine both --shard "$shard/8" --no-fail --summary-json "$BENCH_OUT/test262/summary-$shard.json" --case-results-jsonl "$BENCH_OUT/test262/cases-$shard.jsonl" > "$BENCH_OUT/test262/shard-$shard.log" 2>&1 &
  pids+=("$!")
done
status=0
for index in 0 1 2 3 4 5 6 7; do
  if wait "${pids[$index]}"; then echo "completed shard $((index+1))/8"; else status=1; fi
done
if [ "$status" -ne 0 ]; then exit "$status"; fi
./scripts/test262-burndown.sh --report "$BENCH_OUT/test262" --commit "$cand_sha" --source local-imac --dry-run > "$BENCH_OUT/burndown-entry.json"
python3 - "$BENCH_OUT" "$cand_sha" <<'PY'
import json,subprocess,sys
from pathlib import Path
from tools.benchmark.performance_evaluation import _test262_zero_gap
root=Path(sys.argv[1]);r=json.loads((root/'burndown-entry.json').read_text())
assert _test262_zero_gap(r,sys.argv[2]) and r['configured']==r['rust']['pass']==42672
paths=[]
for i in range(1,9):
    summary=json.loads((root/f'test262/summary-{i}.json').read_text())
    assert summary['engine']=='both' and summary['limit']=='all' and summary['filter']=='' and summary['shard']=={'index':i,'total':8}
    rows=[json.loads(line) for line in (root/f'test262/cases-{i}.jsonl').read_text().splitlines()]
    assert len(rows)==summary['total']
    assert all(row['rust']=='pass' for row in rows if row['quickjs_ng']!='skipped')
    paths.extend(row['path'] for row in rows)
pin=subprocess.check_output(['git','ls-tree','HEAD','third_party/test262'],text=True).split()[2]
pinned=subprocess.check_output(['git','-C','third_party/test262','ls-tree','-r','--name-only',pin,'--','test'],text=True).splitlines()
assert len(paths)==len(set(paths))==53572 and set(paths)=={p for p in pinned if p.endswith('.js')}
print(json.dumps(r,sort_keys=True));print('EXACT PASS: 8 disjoint shards, pinned corpus covered exactly once')
PY
python3 - <<'PY'
from common import CANDIDATE,CANDIDATE_HASH,out,read,write,sha,require
require(read(out()/'image.json')['binary_sha256']==CANDIDATE_HASH,'exact used a different image')
write(out()/'correctness-ready.json',{'candidate':CANDIDATE,'full_gate_passed':True,'ng_fixtures':255,'subset_cases':5170,
      'artifacts':{name:sha(out()/name) for name in ['check.log','compare-qjs.log','burndown-entry.json','image.json']}})
PY
