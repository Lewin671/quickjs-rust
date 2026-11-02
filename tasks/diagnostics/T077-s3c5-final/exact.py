#!/usr/bin/env python3
"""Complete Test262 comparison through the existing harness's exact shards.

No filtering, exclusions or partial ledger records. Metadata preparation is
parallel; the complete unique path union and every configured pass are checked.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--mode', choices=['cached', 'ic-off'], required=True)
parser.add_argument('--binary', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--shards', type=int, default=6)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=False)
root = Path.cwd()
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
source = lambda: {str(p): sha(p) for p in sorted(Path('crates/qjs-runtime/src').rglob('*.rs'))}
env = {k:v for k,v in os.environ.items() if not k.startswith('QJS_')}
env.update(QJS_AGENTS='1', QJS_CLI_PROFILE='release', QJS_CLI_BIN=str(args.binary.resolve()),
           TEST262_BASELINE_JOBS='1', TEST262_CASE_TIMEOUT_SECONDS='20', TEST262_TIMEOUT_RETRIES='1')
if args.mode == 'ic-off': env['QJS_DISABLE_IC'] = '1'
request = {'commit': subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
           'mode':args.mode, 'binary_sha256':sha(args.binary), 'runtime_source_sha256':source(),
           'shards':args.shards, 'environment':{k:v for k,v in env.items() if k.startswith(('QJS_', 'TEST262_'))},
           'test262_commit':subprocess.check_output(['git','-C','third_party/test262','rev-parse','HEAD'],text=True).strip(),
           'ng_commit':subprocess.check_output(['git','-C','third_party/quickjs-ng','rev-parse','HEAD'],text=True).strip()}
(args.output/'request.json').write_text(json.dumps(request,indent=2)+'\n')
def run(index):
    command=['./scripts/test262-baseline.sh','--all','--engine','both','--shard',f'{index}/{args.shards}',
             '--summary-json',str(args.output/f'summary-{index}.json'),
             '--case-results-jsonl',str(args.output/f'cases-{index}.jsonl'),'--no-fail']
    with (args.output/f'shard-{index}.log').open('w') as stream:
        code=subprocess.run(command,env=env,stdout=stream,stderr=subprocess.STDOUT).returncode
    assert code==0,(index,code)
    print('complete shard',index,flush=True)
with ThreadPoolExecutor(max_workers=args.shards) as executor:
    list(executor.map(run,range(1,args.shards+1)))
expected={str(p.relative_to('third_party/test262')) for p in Path('third_party/test262/test').rglob('*.js')}
seen=set(); rows=[]
for index in range(1,args.shards+1):
    summary=json.loads((args.output/f'summary-{index}.json').read_text())
    assert summary['engine']=='both' and summary['limit']=='all' and not summary['filter']
    rows.append(summary)
    for line in (args.output/f'cases-{index}.jsonl').read_text().splitlines():
        item=json.loads(line); assert item['path'] not in seen,item['path'];seen.add(item['path'])
assert seen==expected,(len(seen),len(expected))
assert request['binary_sha256']==sha(args.binary) and request['runtime_source_sha256']==source()
result={'complete':True,'unique_cases':len(seen),'mode':args.mode,
        'rust':{key:sum(r['quickjs_rust'][key]for r in rows)for key in ['pass','fail','timeout','skipped']},
        'ng':{key:sum(r['quickjs_ng'][key]for r in rows)for key in ['pass','fail','timeout','skipped']},
        'comparison':{key:sum(r['comparison'][key]for r in rows)for key in rows[0]['comparison']}}
(args.output/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result),flush=True)
assert result['rust']['fail']==result['rust']['timeout']==0
assert result['comparison']['quickjs_ng_pass_rust_nonpass']==0
