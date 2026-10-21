"""Lead-only 15-block data probe. Build and smoke precede the timed job."""
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
assert q['pairs']==5 and r['verdict']=='unclear' and r['attempts_consumed']==0 and r['budget_met']
assert chain.sha(triage/'triage/triage.json')==cfg['triage_result_sha256']
qualified=done(cfg['qualification_job']);assert sha(qualified/'qualified.json')==cfg['qualification_sha256']
qualification=read(qualified/'qualified.json');assert qualification['diagnostic_only'] and qualification['all_required_static_checks'] and qualification['case_outputs_match']==3 and qualification['ng_fixtures_per_bank_image']==255
build=done(cfg['build_job']);assert sha(build/'build-identity.json')==cfg['build_identity_sha256']
build_identity=read(build/'build-identity.json');assert qualification['build_job']==cfg['build_job'] and qualification['build_identity_sha256']==cfg['build_identity_sha256']
assert read(qualified/'canonical-byte-audit.json')['all_equivalent']
assert sha(qualified/'canonical-byte-audit.json')==cfg['canonical_byte_audit_sha256']
instruction_root=done(cfg['bank_instruction_job']);assert sha(instruction_root/'instruction-smoke.json')==cfg['bank_instruction_sha256']
instruction_smoke=read(instruction_root/'instruction-smoke.json');assert instruction_smoke['within_image_instruction_neutral'] and instruction_smoke['qualification_job']==cfg['qualification_job'] and instruction_smoke['qualification_sha256']==cfg['qualification_sha256']
roles=p.roles();pairs={case:p.comparisons(case) for case in p.CASE_IDS};assert len(roles)==16
paths={};physical={}
for role,meta in roles.items():
 image=meta['image'];source=build/'binaries'/image/'qjs';meta_key=image.replace('_bank','_page')  # original build's legacy metadata key
 expected=build_identity['binaries'][meta_key]['sha256'];assert sha(source)==expected
 assert qualification['images'][image]['sha256']==expected
 if image.endswith('_nat'):assert expected==p.NATURAL_HASH[meta['side']]
 else:assert build_identity['binaries'][meta_key]['requested_allocation_alignment']==4096 and build_identity['binaries'][meta_key]['window_offsets']==[0,64]
 target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True)
 if image in physical and meta['factor'] not in ['aa','bankaa']:os.link(physical[image],target)
 else:
  shutil.copy2(source,target)
  if meta['factor'] not in ['aa','bankaa']:physical[image]=target
 paths[role]=target
os.environ['TZ']='UTC';os.environ['LC_ALL']='C'
inherited_environment_sha=hashlib.sha256(json.dumps(dict(os.environ),sort_keys=True).encode()).hexdigest()
request={'executed_revision':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'candidate':p.CANDIDATE,'unit_sha256':p.UNIT_SHA,'blocks':p.BLOCKS,'workers':1,'case_ids':p.CASE_IDS,'roles':roles,'comparisons':pairs,'binary_sha256':{n:sha(x) for n,x in paths.items()},'role_inodes':{n:x.stat().st_ino for n,x in paths.items()},'literal_path':str(out/'active/qjs'),'collector_sha256':sha(__file__),'protocol_sha256':cfg['protocol_sha256'],'configuration':cfg,'host':host,'inherited_environment_sha256':inherited_environment_sha,'fixed_environment':{'TZ':'UTC','LC_ALL':'C'},'factor_keys_reset_each_role':p.ENV_KEYS,'collect_budget_seconds':p.COLLECT_SECONDS,'body_budget_seconds':p.BODY_SECONDS,'job_budget_seconds':p.JOB_SECONDS,'budget_basis':{'probe_A_complete_case_role_blocks':1215,'probe_A_budget_seconds':1800,'new_case_role_blocks':540,'nominal_scaled_seconds':800,'case_mix_adjustment':1.3,'calibration_analysis_reserve_seconds':120},'case_role_counts':{case:len(p.active_roles(case)) for case in p.CASE_IDS},'caveat':'single factors, not full factorial; canonical vs diagnostic bank00 includes source/layout/backing changes and is a bias control; bank64/bank00 uses identical image/allocation size and only shifts the window; not acceptance or proof of a common cause'}
(out/'request.json').write_text(json.dumps(request,indent=2)+'\n')
work=out/'work';work.mkdir();cases=s.resolve_cases(p.CASE_IDS,s.DEFAULT_CACHE,work)
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
with measurement_lock('T056 d130 data/launch probe'):
 for case in cases:
  current.update(case=case.id,block=None);activate('b_nat');current['phase']='calibration'
  counts[case.id]=s.calibrate(lambda n:take(case,n)) if case.kind=='internal' else None
 for block in range(p.BLOCKS):
  for case_index,case in enumerate(p.order(cases,block)):
   current.update(case=case.id,block=block);count=counts[case.id];sequence=p.order(p.active_roles(case.id),block,case_index);row={'block':block,'order':sequence,'samples':{}}
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
  print('BLOCK',block+1,'of 15; all cases and their 16/10/10 prespecified roles retained',flush=True)
 report=[]
 for case in cases:
  data=rows[case.id];assert len(data)==p.BLOCKS
  if counts[case.id] is not None:assert min(increment(row,'b_nat','wall_ns') for row in data)>=s.MIN_INCREMENT_NS
  effects={}
  for i,(name,weights) in enumerate(pairs[case.id].items()):
   effects[name]={}
   for metric in ('wall_ns','instructions'):
    logs={row['block']:sum(w*math.log(increment(row,role,metric)) for role,w in weights.items()) for row in data}
    lower,upper=paired_block_bootstrap({case.id:logs},samples=10000,seed=256+i,confidence=.95)
    effects[name][metric]={'median':math.exp(statistics.median(logs.values())),'lower':lower,'upper':upper}
  report.append({'case':case.id,'iterations':counts[case.id],'effects':effects})
 (out/'effects.json').write_text(json.dumps(report,indent=2)+'\n')
 p.remaining(deadline,time.monotonic(),1)
raw.close()
(out/'complete.json').write_text(json.dumps({'cases':3,'roles_total':16,'case_roles':[16,10,10],'blocks':15,'case_role_blocks':540,'seconds':time.monotonic()-start,'all_outputs_match':True,'decision_evidence':False,'attempts_consumed':0})+'\n')
print('COMPLETE: diagnostic only, no acceptance or attempt consumed',flush=True)
