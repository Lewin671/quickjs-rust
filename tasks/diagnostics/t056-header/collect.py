"""Lead-only timing of header-factor images against f58; no builds in this job."""
import os,json,hashlib,re,subprocess,shutil,dataclasses,time
from pathlib import Path
from tools.benchmark import screen as s
from tools.benchmark.process import run_process
from tools.benchmark.measure_lock import measurement_lock
from tools.benchmark.pair_disturbance import Monitor,heavy_samples
from tools.benchmark.layout_identity import host_identity
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.performance_decision import load_unit
import protocol as p,analyze,chain_common as chain
out=Path(os.environ['BENCH_OUT']);start=time.monotonic();deadline=start+p.COLLECT_SECONDS
assert re.fullmatch(r'running [0-9TZ:-]+',(out.parent/'status').read_text().strip()),'lead --timing job required; no owner timing'
def sha(x):return hashlib.sha256(Path(x).read_bytes()).hexdigest()
def read(x):return json.loads(Path(x).read_text())
def done(identifier):
 assert re.fullmatch(r'[0-9]{8}T[0-9]{6}Z-[a-z0-9._-]+',identifier)
 root=Path(os.environ['BENCH_QUEUE'])/identifier
 assert (root/'status').read_text().strip()=='done 0' and 'DISTURBED' not in (root/'log').read_text()
 return root/'out'
cfg=read(out/'config.json')
for name,h in cfg['files'].items():assert sha(Path(__file__).with_name(name))==h
subprocess.run(['git','diff','--exit-code',p.CANDIDATE,'HEAD','--','crates','tools','scripts','.cargo','Cargo.toml','Cargo.lock'],check=True)
unit,h=load_unit(chain.PLAN);assert h==chain.UNIT_HASH
policy=load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256'])
chain.correctness(cfg['correctness_job']);chain.pregate(cfg['pregate_job'],policy)
host=host_identity();assert host['hardware']=='iMac20,1' and 'i7-10700' in host['cpu']
root=done(cfg['build_job']);assert sha(root/'images.json')==cfg['images_sha256'];images=read(root/'images.json');assert images['parts_sha256']==sha(out/'parts.json')
layouts=read(root/'layouts.json');assert sha(root/'layouts.json')==cfg['layouts_sha256'] and layouts['initial_images_sha256']==cfg['images_sha256']
assert set(layouts['roles'])=={'aligned','full_a',*p.VARIANTS}
assert images['all_outputs_match'] and images['timing'] is False and set(images['images'])=={'base',*p.IMAGES}
paths={}
for role in p.ROLES:
 side=role.removesuffix('_aa');source=root/'binaries'/side/'qjs';assert sha(source)==images['images'][side]['binary_sha256']
 assert images['images'][side]['binary_sha256']==cfg['binary_sha256'][side]
 target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True);shutil.copy2(source,target);paths[role]=target
assert len({x.stat().st_ino for x in paths.values()})==len(p.ROLES)
work=out/'work';work.mkdir();cases=s.resolve_cases(p.CASES,s.DEFAULT_CACHE,work)
for case in cases:assert sha(case.workload)==cfg['workload_sha256']
for key in p.ENV_KEYS:os.environ.pop(key,None)
os.environ.update(TZ='UTC',LC_ALL='C')
active=out/'active/qjs';active.parent.mkdir();nxt=active.with_name('next')
def activate(path):
 nxt.unlink(missing_ok=True);os.link(path,nxt);os.replace(nxt,active)
request={'artifact_type':'t056-header-subparts','diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'base':p.BASE,'aligned':p.ALIGNED,'candidate':p.CANDIDATE,'cases':p.CASES,'blocks':p.BLOCKS,'iterations':p.COUNT,'roles':p.ROLES,'inodes':{r:x.stat().st_ino for r,x in paths.items()},'binary_sha256':cfg['binary_sha256'],'configuration':cfg,'host':host,'budgets':{'collect':p.COLLECT_SECONDS,'body':p.BODY_SECONDS,'running_job':p.JOB_SECONDS},'budget_basis':p.THROUGHPUT,'trigger':p.TRIGGER,'header_bytes':{r:v['size'] for r,v in layouts['roles'].items()},'workloads':{c.id:{'sha256':sha(c.workload),'case_id':c.case_id} for c in cases},'byte_audits':{r:v['all_equivalent'] for r,v in images['audits'].items()},'sampling':'five shared f58 block references per case, all nine images plus separate-inode byte-identical AA clones for aligned/candidate/full_a. Rotate/reverse roles and cases; N/2N order reverses; no deletion/retry'}
(out/'request.json').write_text(json.dumps(request,indent=2)+'\n')
rows={c.id:[] for c in cases};current={};raw=(out/'raw.jsonl').open('x');tool=s.counter_tool(trust_cycles=False);checks={};preflights=[]
def runner(argv,timeout):
 result=run_process(argv,p.remaining(deadline,time.monotonic(),timeout));raw.write(json.dumps({**current,'argv':argv,'result':dataclasses.asdict(result)})+'\n');raw.flush();return result
with measurement_lock('T056 header factors'):
 monitor=Monitor.from_queue()
 for block in range(p.BLOCKS):
  for ci,case in enumerate(p.order(cases,block)):
   current.update(block=block,case=case.id);row={'block':block,'samples':{},'order':p.order(p.ROLES,block,ci)}
   for role in row['order']:
    current['role']=role;activate(paths[role]);current['phase']='preflight';preflights.extend(s.launch_preflight([active],120,runner=runner))
    sizes=[('whole',None)] if case.kind=='whole' else [('N',p.COUNT),('2N',2*p.COUNT)]
    if block%2:sizes.reverse()
    values={}
    for size,n in sizes:
     current.update(phase='measured',size=size);v=s.sample(tool,s._argv(active,case,n),120,runner=runner);check=s._check_output(case,v.stdout);key=(case.id,size)
     if key in checks:assert checks[key]==check
     else:checks[key]=check
     values[size]=dataclasses.asdict(v)
    row['samples'][role]=values
   rows[case.id].append(row);(out/(case.id.replace('/','--')+'.json')).write_text(json.dumps({'case':case.id,'rows':rows[case.id]},indent=2)+'\n')
  print('BLOCK',block+1,'/',p.BLOCKS,'all images and AA retained',flush=True)
 load=monitor.snapshot();(out/'load.json').write_text(json.dumps(load,indent=2)+'\n');assert not heavy_samples(load['text'],load['anchor']),'DISTURBED: retain whole job, no decision'
 for case in cases:
  for row in rows[case.id]:
   assert set(row['samples'])==set(p.ROLES)
   for role in p.ROLES:
    for metric in ('instructions','wall_ns'):
     v=analyze.increment(row,role,metric)
     if case.kind=='internal' and metric=='wall_ns':assert v>=s.MIN_INCREMENT_NS
 assessment=analyze.analyze(rows);(out/'assessment.json').write_text(json.dumps(assessment,indent=2)+'\n');print('ASSESSMENT',assessment['verdict'],assessment['qualified_triggers'],flush=True)
 p.remaining(deadline,time.monotonic(),1)
raw.close();(out/'preflights.json').write_text(json.dumps(preflights,indent=2)+'\n')
(out/'complete.json').write_text(json.dumps({'cases':2,'images_vs_base':9,'roles':13,'blocks':5,'units':130,'measured_execs':260,'preflights':130,'seconds':time.monotonic()-start,'diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'all_outputs_match':True})+'\n')
