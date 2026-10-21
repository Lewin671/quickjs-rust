"""Lead-only installed-helper run; never installs, builds or changes sudo policy."""
import dataclasses,hashlib,json,os,re,shutil,subprocess,time
from pathlib import Path
from tools.benchmark import screen as s
from tools.benchmark.layout_identity import host_identity
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.performance_decision import load_unit
from tools.benchmark.measure_lock import measurement_lock
from tools.benchmark.pair_disturbance import Monitor,heavy_samples
from tools.benchmark.process import run_process
import protocol as p,counters,chain_common as chain
out=Path(os.environ['BENCH_OUT']);start=time.monotonic();deadline=start+p.COLLECT_SECONDS
assert re.fullmatch(r'running [0-9TZ:-]+',(out.parent/'status').read_text().strip()),'lead --timing required; owner must not submit'
read=lambda f:json.loads(Path(f).read_text());sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
cfg=read(out/'config.json')
for name,h in cfg['files'].items():assert sha(out/name)==h
subprocess.run(['git','diff','--exit-code',p.CANDIDATE,'HEAD','--','crates','tools','scripts','.cargo','Cargo.toml','Cargo.lock'],check=True)
unit,h=load_unit(chain.PLAN);assert h==chain.UNIT_HASH
policy=load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256']);chain.correctness(cfg['correctness_job']);chain.pregate(cfg['pregate_job'],policy)
host=host_identity();assert host['hardware']=='iMac20,1' and 'i7-10700' in host['cpu']
helper=Path(p.HELPER)
if not helper.is_file() or sha(helper)!=cfg['helper_sha256']:
 (out/'blocked.json').write_text(json.dumps({'reason':'exact reviewed root-owned helper not installed; zero privileged launches','timing_started':False})+'\n');raise RuntimeError('user installation required; no automatic install or sudo policy edit')
assert helper.stat().st_uid==0 and not helper.stat().st_mode&0o22
queue=Path(os.environ['BENCH_QUEUE'])
def done(identifier):
 root=queue/identifier;assert (root/'status').read_text().strip()=='done 0' and 'DISTURBED' not in (root/'log').read_text();return root/'out'
qualified=done(cfg['qualification_job']);assert sha(qualified/'qualification.json')==cfg['qualification_sha256']
assert read(qualified/'qualification.json')['binary_sha256']==cfg['helper_sha256']
packed=done(cfg['groups_job']);assert sha(packed/'groups.json')==cfg['groups_sha256']
assert read(packed/'groups.json')['binary_sha256']==cfg['helper_sha256']
root=done(cfg['images_job']);images=read(root/'images.json');assert sha(root/'images.json')==cfg['images_sha256']
groups=read(out/'groups.json')['groups'];assert len(groups)==6
paths={}
for role in p.ROLES:
 name=role.removesuffix('_aa');source=root/'binaries'/name/'qjs';assert sha(source)==cfg['images'][name]
 target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True);shutil.copy2(source,target);paths[role]=target
assert len({path.stat().st_ino for path in paths.values()})==len(paths)
work=out/'work';work.mkdir();cases=s.resolve_cases(p.CASES,s.DEFAULT_CACHE,work)
for case in cases:assert sha(case.workload)==cfg['workload_sha256']
for key in p.ENV_KEYS:os.environ.pop(key,None)
os.environ.update(TZ='UTC',LC_ALL='C')
active=out/'active/qjs';active.parent.mkdir();nxt=active.with_name('next');raw=(out/'raw.jsonl').open('x');current={};rows=[];checks={}
def call(argv):
 value=run_process(argv,p.remaining(deadline,time.monotonic(),15));raw.write(json.dumps({**current,'argv':argv,'result':dataclasses.asdict(value)})+'\n');raw.flush();assert not value.timed_out and value.exit_code==0 and not value.stdout_truncated and not value.stderr_truncated;return json.loads(value.stdout)
def argv(events,target):return ['/usr/bin/sudo','-n','--',str(helper),'--events',','.join(['cycles','instructions',*events]),'--env','TZ=UTC','--env','LC_ALL=C','--',*map(str,target)]
request={'artifact_type':'t056-kpc-counters','diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'images':cfg['images'],'helper_sha256':cfg['helper_sha256'],'cases':p.CASES,'roles':p.ROLES,'groups':groups,'blocks':p.BLOCKS,'count':p.COUNT,'host':host,'rule':p.RULE,'configuration':cfg,'ownership':'root helper handles only own gated child and idle ktrace session; fixed env and uid/gid drop; no install from job','scope':'sampled intervals, no complete lifetime/all-thread guarantee'}
(out/'request.json').write_text(json.dumps(request,indent=2)+'\n')
with measurement_lock('T056 kpc sampled counter diagnostic'):
 monitor=Monitor.from_queue();current['phase']='privilege-check'
 identity=call(argv([],['/usr/bin/id','-u']));environment=call(argv([],['/usr/bin/env']))
 for result in (identity,environment):assert result['capture_complete'] and result['cleanup_ok'] and result['configuration_readback_verified'] and result['child_status']==0
 assert identity['child_stdout'].strip()==str(os.getuid()) and os.getuid()!=0
 assert set(environment['child_stdout'].splitlines())=={'TZ=UTC','LC_ALL=C'}
 (out/'privilege-check.json').write_text(json.dumps({'identity':identity,'environment':environment},indent=2)+'\n')
 for block in range(p.BLOCKS):
  for group in p.order(groups,block):
   for ci,case in enumerate(p.order(cases,block,group['id'])):
    row={'block':block,'group':group['id'],'case':case.id,'samples':{}}
    for role in p.order(p.ROLES,block,ci+group['id']):
     nxt.unlink(missing_ok=True);os.link(paths[role],nxt);os.replace(nxt,active);values={}
     for label,n in (reversed([('N',p.COUNT),('2N',2*p.COUNT)]) if block%2 else [('N',p.COUNT),('2N',2*p.COUNT)]):
      current.update(phase='measured',block=block,group=group['id'],case=case.id,role=role,size=label)
      value=call(argv(group['events'],s._argv(active,case,n)));counters.check(value,group['description']);output=s._check_output(case,value['child_stdout']);key=(case.id,label)
      if key in checks:assert checks[key]==output
      else:checks[key]=output
      values[label]=value
     row['samples'][role]=values
    rows.append(row);(out/'rows.json').write_text(json.dumps(rows,indent=2)+'\n')
   print('BLOCK',block+1,'GROUP',group['id'],'complete',flush=True)
 load=monitor.snapshot();(out/'load.json').write_text(json.dumps(load,indent=2)+'\n');assert not heavy_samples(load['text'],load['anchor']),'DISTURBED; retain whole job, no retry'
 summary=counters.summarize(rows);(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');p.remaining(deadline,time.monotonic(),1)
raw.close();(out/'complete.json').write_text(json.dumps({'diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'groups':6,'events':21,'case_role_blocks':192,'measured_execs':384,'privilege_checks':2,'seconds':time.monotonic()-start,'full_lifetime_counts':False,'all_thread_coverage':False})+'\n')
