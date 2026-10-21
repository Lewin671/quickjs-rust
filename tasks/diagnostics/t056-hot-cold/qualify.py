"""Final exact-image instruction prequalification; never submits timing."""
import hashlib,json,os,subprocess,sys
from pathlib import Path
OUT=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (OUT.parent/'status').read_text()
ROOT=Path('tasks/diagnostics/t056-hot-cold');candidate=sys.argv[1]
subprocess.run([sys.executable,str(ROOT/'probe.py'),candidate],check=True)
probe=json.loads((OUT/'probe.json').read_text())
policy=json.loads(Path('benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v6.json').read_text())
args=[sys.executable,'-m','tools.benchmark.screen','--candidate',probe['images']['candidate']['path'],'--base',probe['images']['base']['path'],'--pairs','1','--symbols','40','--json',str(OUT/'snapshot86.json')]
for name in policy['cases']:args.extend(['--case',name])
with (OUT/'snapshot86.txt').open('w') as log,(OUT/'snapshot86.err').open('w') as err:subprocess.run(args,stdout=log,stderr=err,check=True)
snapshot=json.loads((OUT/'snapshot86.json').read_text());assert snapshot['pairs']==1 and len(snapshot['cases'])==86 and {r['id'] for r in snapshot['cases']}==set(policy['cases'])
ratios=[{'case':r['id'],'instructions':r['ratios']['instructions'][0],'limit':max(1.03,policy['cases'][r['id']]['instruction_band'])} for r in snapshot['cases']]
result={'diagnostic_only':True,'timing':False,'acceptance':False,'attempts_consumed':0,'candidate':candidate,
        'images':{r:v['sha256'] for r,v in probe['images'].items()},'instruction_ratios':ratios,
        'blocked':[r['case'] for r in ratios if r['instructions']>r['limit']],
        'canonical_medians':probe['medians'],'probe_sha256':hashlib.sha256((OUT/'probe.json').read_bytes()).hexdigest(),
        'snapshot_sha256':hashlib.sha256((OUT/'snapshot86.json').read_bytes()).hexdigest(),
        'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'scope':'Retired instructions only, wall fields discarded. Lead fresh exact+86 still required; PC changes are reported separately, never accepted by normalized bytes alone.'}
(OUT/'qualification.json').write_text(json.dumps(result,indent=2)+'\n');print('SNAPSHOT',len(ratios),'BLOCKED',result['blocked'],flush=True)
subprocess.run([sys.executable,str(ROOT/'native.py')],check=True)
if result['blocked']:raise SystemExit(3)
