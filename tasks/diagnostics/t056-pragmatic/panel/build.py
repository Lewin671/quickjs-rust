"""Non-timing qualification only; build the sealed pragmatic placement variants."""
import json,os,subprocess,hashlib,shutil,platform
from pathlib import Path
from tools.benchmark.layout_variants import build
from tools.benchmark.layout_panel import matrix
from tools.benchmark.layout_identity import host_identity,OFFSETS
from tools.benchmark.screen import resolve_cases,DEFAULT_CACHE
import protocol as p
out=Path(os.environ['BENCH_OUT']);queue=Path(os.environ['BENCH_QUEUE'])
assert 'no-timing' in (out.parent/'status').read_text(),'qualification must be a non-timing job'
previous=queue/'20261021T182852Z-t056-pragmatic-final-instructions86';assert (previous/'status').read_text().strip()=='done 0'
receipt=json.loads((previous/'out/qualification.json').read_text());assert not receipt['blocked'] and receipt['images']['candidate']==p.HASHES['c']
root=queue/p.BASE_BUILD_JOB
assert (root/'status').read_text().strip()=='done 0' and 'DISTURBED' not in (root/'log').read_text()
base_path=root/'out/base000/identity.json';base=matrix(base_path)
assert base['revision']==p.BASE and base['host']==host_identity() and base['host_node']==platform.node()
assert base['variants'][0]['sha256']==p.HASHES['b']
candidate=build(p.CANDIDATE,out/'candidate')
assert candidate['variants'][0]['sha256']==p.HASHES['c']
work=out/'work';work.mkdir();cases=resolve_cases(p.MINI_CASES[:-1],DEFAULT_CACHE,work)
result={'diagnostic_only':True,'timing':False,'base_build_job':p.BASE_BUILD_JOB,'base_identity_sha256':hashlib.sha256(base_path.read_bytes()).hexdigest(),'offsets':list(OFFSETS),'cases':list(p.MINI_CASES),'images':{},'qualification':[],'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'host':host_identity()}
reference={}
for side,identity in [('b',base),('c',candidate)]:
 assert identity['static_pass'] and identity['revision']=={'b':p.BASE,'c':p.CANDIDATE}[side]
 for v in identity['variants']:
  key=f"{side}{v['index']}";source=Path(v['path']);target=out/'binaries'/key/'qjs';target.parent.mkdir(parents=True);shutil.copy2(source,target)
  assert hashlib.sha256(target.read_bytes()).hexdigest()==v['sha256']
  log=out/(key+'-ng.log');env={**os.environ,'QJS_CLI_BIN':str(target),'TZ':'UTC','LC_ALL':'C'}
  for name in p.ENV_KEYS:env.pop(name,None)
  with log.open('w') as stream:subprocess.run(['./scripts/compare-qjs.sh'],env=env,stdout=stream,stderr=subprocess.STDOUT,check=True)
  assert sum(x.startswith('ok: ') for x in log.read_text().splitlines())==255
  outputs={}
  for case in cases:
   argv=[str(target),'--raw',str(case.workload)]
   if case.kind=='internal':argv.extend([case.case_id,'1000'])
   outputs[case.id]=subprocess.check_output(argv,env=env,text=True,timeout=90)
  outputs['diagnostic/empty']=subprocess.check_output([str(target),'--raw','-e','0'],env=env,text=True,timeout=30)
  if reference:assert outputs==reference
  else:reference=outputs
  result['images'][key]={'sha256':v['sha256'],'revision':identity['revision'],'offset':v['pad_bytes'],'audit':v['audit']}
  result['qualification'].append({'image':key,'ng_fixtures':255,'outputs':outputs,'ng_log_sha256':hashlib.sha256(log.read_bytes()).hexdigest()})
  print('QUALIFIED',key,v['sha256'],flush=True)
result['all_outputs_match']=True
(out/'qualification.json').write_text(json.dumps(result,indent=2)+'\n')
