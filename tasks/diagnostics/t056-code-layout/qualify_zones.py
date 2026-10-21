"""Non-timing correctness smoke for all eight code x zone cells."""
import os,json,hashlib,subprocess
from pathlib import Path
from tools.benchmark.screen import resolve_cases,DEFAULT_CACHE
import protocol as p
out=Path(os.environ['BENCH_OUT']);cfg=json.loads((out/'zone-config.json').read_text())
assert 'no-timing' in (out.parent/'status').read_text()
root=Path(os.environ['BENCH_QUEUE'])/cfg['build_job'];assert (root/'status').read_text().strip()=='done 0'
root=root/'out';assert hashlib.sha256((root/'qualification.json').read_bytes()).hexdigest()==cfg['build_sha256']
q=json.loads((root/'qualification.json').read_text());work=out/'work';work.mkdir();cases=resolve_cases(p.CROSS_CASES[:-1],DEFAULT_CACHE,work)
result={'diagnostic_only':True,'timing':False,'build_job':cfg['build_job'],'build_sha256':cfg['build_sha256'],'cells':[]};reference=None
for side in ('b','c'):
 for code in (0,1):
  binary=root/'binaries'/f'{side}{code}'/'qjs';assert hashlib.sha256(binary.read_bytes()).hexdigest()==q['images'][f'{side}{code}']['sha256']
  for zone in (0,1):
   env=dict(os.environ)
   for key in p.ENV_KEYS:env.pop(key,None)
   env.update(MallocNanoZone=str(zone),QJS_CLI_BIN=str(binary),TZ='UTC',LC_ALL='C')
   log=out/f'{side}{code}z{zone}-ng.log'
   with log.open('w') as f:subprocess.run(['./scripts/compare-qjs.sh'],env=env,stdout=f,stderr=subprocess.STDOUT,check=True)
   assert sum(l.startswith('ok: ') for l in log.read_text().splitlines())==255
   outputs={}
   for case in cases:
    argv=[str(binary),'--raw',str(case.workload)]
    if case.kind=='internal':argv.extend([case.case_id,'1000'])
    outputs[case.id]=subprocess.check_output(argv,env=env,text=True,timeout=90)
   outputs['diagnostic/empty']=subprocess.check_output([str(binary),'--raw','-e','0'],env=env,text=True)
   if reference is not None:assert outputs==reference
   else:reference=outputs
   result['cells'].append({'side':side,'code':code,'zone':zone,'image_sha256':q['images'][f'{side}{code}']['sha256'],'ng_fixtures':255,'outputs':outputs,'log_sha256':hashlib.sha256(log.read_bytes()).hexdigest()})
   print('ZONE QUALIFIED',side,code,zone,flush=True)
result['all_outputs_match']=True
(out/'zones.json').write_text(json.dumps(result,indent=2)+'\n')
