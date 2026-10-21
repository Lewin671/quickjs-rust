"""Lead-only architecture robustness probe. Never an acceptance screen."""
import dataclasses,hashlib,json,math,os,re,shutil,statistics,subprocess,time
from pathlib import Path
from tools.benchmark import screen as s
from tools.benchmark.measure_lock import measurement_lock
from tools.benchmark.process import run_process
from tools.benchmark.statistics import paired_block_bootstrap
from tools.benchmark.layout_identity import host_identity
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.performance_decision import load_unit
import chain_common as chain
import protocol as p
out=Path(os.environ['BENCH_OUT']);start=time.monotonic();deadline=start+p.COLLECT_SECONDS
cfg=json.loads((out/'probe-config.json').read_text())
assert re.fullmatch(r'running [0-9TZ:-]+',(out.parent/'status').read_text().strip()), 'requires --timing runner, not a no-timing launch'
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def read(path):return json.loads(Path(path).read_text())
def done(identifier):
 assert re.fullmatch(r'[0-9]{8}T[0-9]{6}Z-[a-z0-9._-]+',identifier)
 root=Path(os.environ['BENCH_QUEUE'])/identifier
 assert (root/'status').read_text().strip()=='done 0' and 'DISTURBED' not in (root/'log').read_text()
 return root/'out'
assert p.BLOCKS==15 and tuple(cfg['case_ids'])==p.CASE_IDS
subprocess.run(['git','merge-base','--is-ancestor',cfg['protocol_revision'],'HEAD'],check=True)
assert sha(__file__)==cfg['collector_sha256'] and sha(Path(__file__).with_name('protocol.py'))==cfg['protocol_sha256']
assert sha('tasks/performance-units/interpreter-core-v2-layout-edges.json')==p.UNIT_SHA
subprocess.run(['git','diff','--exit-code',p.CANDIDATE,'HEAD','--','crates','tools','scripts','.cargo','Cargo.toml','Cargo.lock'],check=True)
assert os.sysconf('SC_PAGE_SIZE')==4096
host=host_identity();assert host['hardware']=='iMac20,1' and 'i7-10700' in host['cpu']
unit,h=load_unit(chain.PLAN);assert h==p.UNIT_SHA
policy=load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256'])
chain.correctness(cfg['correctness_job']);chain.pregate(cfg['pregate_job'],policy)
triage=chain.job(cfg['triage_job']);q=read(triage/'triage/request.json');r=read(triage/'triage/triage.json')
assert q['pairs']==5 and r['verdict']=='likely loss' and r['attempts_consumed']==0 and r['budget_met']
assert sha(triage/'triage/triage.json')==cfg['triage_result_sha256']
build=done(cfg['pad_build_job']);assert sha(build/'pad-build.json')==cfg['pad_build_sha256']
identity=read(build/'pad-build.json');assert identity['diagnostic_only'] and identity['runtime_source_unchanged'] and identity['all_outputs_match']
assert identity['pad_count']==64 and identity['pad_modes']=={'pad0128':128,'pad0144':144,'pad1536':1536,'pada128':128}
assert len(identity['qualification'])==8
assert all(x['ng_fixtures']==255 for x in identity['qualification'] if x['mode']!='pada128')
roles=p.roles();pairs=p.comparisons();assert len(roles)==12
paths={};physical={}
for role,meta in roles.items():
 side=meta['side'];image=meta['image'];version={'b':'aligned','c':'allocation'}[side]
 if image.endswith('_pad'):
  source=build/'binaries'/version/'qjs';expected=identity['images'][version]['sha256']
 else:
  root=done(cfg['natural_jobs'][side]);receipt=read(root/'triage/triage.json')['builds']['candidate'];source=root/'triage/cand000/qjs';expected=p.NATURAL_HASH[side]
  assert receipt['sha256']==expected
 assert sha(source)==expected
 target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True)
 if image in physical and meta['factor'] not in ('aa','padaa'):os.link(physical[image],target)
 else:
  shutil.copy2(source,target)
  if meta['factor'] not in ('aa','padaa'):physical[image]=target
 paths[role]=target
os.environ['TZ']='UTC';os.environ['LC_ALL']='C'
inherited_environment_sha=hashlib.sha256(json.dumps(dict(os.environ),sort_keys=True).encode()).hexdigest()
request={'executed_revision':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'candidate':p.CANDIDATE,'unit_sha256':p.UNIT_SHA,'blocks':p.BLOCKS,'workers':1,'case_ids':p.CASE_IDS,'roles':roles,'comparisons':pairs,'binary_sha256':{n:sha(x) for n,x in paths.items()},'role_inodes':{n:x.stat().st_ino for n,x in paths.items()},'literal_path':str(out/'active/qjs'),'collector_sha256':sha(__file__),'protocol_sha256':cfg['protocol_sha256'],'configuration':cfg,'host':host,'inherited_environment_sha256':inherited_environment_sha,'fixed_environment':{'TZ':'UTC','LC_ALL':'C'},'factor_keys_reset_each_role':p.ENV_KEYS,'collect_budget_seconds':p.COLLECT_SECONDS,'body_budget_seconds':p.BODY_SECONDS,'job_budget_seconds':p.JOB_SECONDS,'budget_basis':{'reference_units':540,'reference_seconds':401,'new_units':1080,'scaled_seconds':802,'reserve_factor':1.4,'setup_reserve_seconds':120},'fixed_counts':p.COUNTS,'caveat':'same-image padding contrasts isolate allocation-size effects; canonical/diagnostic includes codegen and host setup bias; empty reports whole-process setup cost, never automatically subtracted; no acceptance claim'}
(out/'request.json').write_text(json.dumps(request,indent=2)+'\n')
work=out/'work';work.mkdir();cases=s.resolve_cases(p.CASE_IDS[:-1],s.DEFAULT_CACHE,work)
empty=work/'empty.js';empty.write_text('0;\n')
cases.append(dataclasses.replace(cases[-1],id='diagnostic/empty',workload=empty,expected_stdout=None))
active=out/'active/qjs';active.parent.mkdir();temporary=active.with_name('next');preflights=[];current={};tool=s.counter_tool(trust_cycles=False);raw=(out/'raw.jsonl').open('x');rows={c.id:[] for c in cases};checks={};counts={}
def runner(argv,timeout):
 timeout=p.remaining(deadline,time.monotonic(),timeout)
 result=run_process(argv,timeout)
 raw.write(json.dumps({**current,'argv':argv,'environment_overrides':roles.get(current.get('role'),{}).get('environment',{}),'result':dataclasses.asdict(result)})+'\n');raw.flush();return result

def activate(name):
 current['role']=name
 for key,value in roles[name]['environment'].items():
  if value is None:os.environ.pop(key,None)
  else:os.environ[key]=value
 temporary.unlink(missing_ok=True);os.link(paths[name],temporary);os.replace(temporary,active);temporary.unlink(missing_ok=True)
 current['phase']='warm';preflights.extend(s.launch_preflight([active],120,runner=runner));(out/'preflights.json').write_text(json.dumps(preflights,indent=2)+'\n')
def take(case,n):return s.sample(tool,s._argv(active,case,n),120,runner=runner)
def increment(row,role,metric):
 values=row['samples'][role];v=values['2N'][metric]-values['N'][metric] if 'N' in values else values['whole'][metric]
 assert v>0,('nonpositive increment',row['block'],role,metric,v)
 return v
with measurement_lock('T056 architecture robustness diagnostic'):
 counts.update(p.COUNTS)
 for block in range(p.BLOCKS):
  for case_index,case in enumerate(p.order(cases,block)):
   current.update(case=case.id,block=block);count=counts[case.id];sequence=p.order(roles,block,case_index);row={'block':block,'order':sequence,'samples':{}}
   for role in sequence:
    activate(role);current['phase']='measured';values={};sizes=[('N',count),('2N',2*count)] if count is not None else [('whole',None)]
    if block%2:sizes.reverse()
    for size,n in sizes:
     current['size']=size;value=take(case,n);check=s._check_output(case,value.stdout);key=(case.id,size)
     if key in checks:assert checks[key]==check,('stdout mismatch',case.id,role,size)
     else:checks[key]=check
     values[size]=dataclasses.asdict(value)
    row['samples'][role]=values
   rows[case.id].append(row)
   (out/(case.id.replace('/','--')+'.json')).write_text(json.dumps({'case':case.id,'iterations':count,'rows':rows[case.id]},indent=2)+'\n')
  print('BLOCK',block+1,'of 15; all six cases and twelve roles retained',flush=True)
 report=[]
 for case in cases:
  data=rows[case.id];assert len(data)==p.BLOCKS
  if counts[case.id] is not None:assert min(increment(row,'b_nat','wall_ns') for row in data)>=s.MIN_INCREMENT_NS
  effects={}
  for i,(name,weights) in enumerate(pairs.items()):
   effects[name]={}
   for metric in ('wall_ns','instructions'):
    logs={row['block']:sum(w*math.log(increment(row,role,metric)) for role,w in weights.items()) for row in data}
    lower,upper=paired_block_bootstrap({case.id:logs},samples=10000,seed=256+i,confidence=.95)
    effects[name][metric]={'median':math.exp(statistics.median(logs.values())),'lower':lower,'upper':upper}
  report.append({'case':case.id,'iterations':counts[case.id],'effects':effects})
 (out/'effects.json').write_text(json.dumps(report,indent=2)+'\n')
 p.remaining(deadline,time.monotonic(),1)
raw.close()
(out/'complete.json').write_text(json.dumps({'cases':6,'roles_total':12,'case_roles':[12]*6,'blocks':15,'case_role_blocks':1080,'seconds':time.monotonic()-start,'all_outputs_match':True,'decision_evidence':False,'attempts_consumed':0})+'\n')
print('COMPLETE: diagnostic only, no acceptance or attempt consumed',flush=True)
