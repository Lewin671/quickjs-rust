"""Bounded lead-only PMU trace capture. Current iMac is tool/permission blocked."""
import os,time,json,hashlib,subprocess,shutil,re,xml.etree.ElementTree as ET
from pathlib import Path
from tools.benchmark.screen import resolve_cases,DEFAULT_CACHE,_check_output,launch_preflight
from tools.benchmark.layout_identity import host_identity
from tools.benchmark.measure_lock import measurement_lock
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.performance_decision import load_unit
import protocol as p,xctrace_backend as backend,chain_common as chain
out=Path(os.environ['BENCH_OUT']);start=time.monotonic();deadline=start+p.COLLECT_SECONDS
assert re.fullmatch(r'running [0-9TZ:-]+',(out.parent/'status').read_text().strip()),'lead --timing job required; owner must not record PMU'
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def read(path):return json.loads(Path(path).read_text())
def done(identifier):
 assert re.fullmatch(r'[0-9]{8}T[0-9]{6}Z-[a-z0-9._-]+',identifier)
 root=Path(os.environ['BENCH_QUEUE'])/identifier
 assert (root/'status').read_text().strip()=='done 0' and 'DISTURBED' not in (root/'log').read_text()
 return root/'out'
cfg=read(out/'config.json')
for name,h in cfg['files'].items():assert sha(Path(__file__).with_name(name))==h
assert sha(Path(__file__).with_name('event-plan.json'))==cfg['event_plan_sha256']
unit,h=load_unit(chain.PLAN);assert h==chain.UNIT_HASH
policy=load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256'])
chain.correctness(cfg['correctness_job']);chain.pregate(cfg['pregate_job'],policy)
subprocess.run(['git','diff','--exit-code',p.CANDIDATE,'HEAD','--','crates','tools','scripts','.cargo','Cargo.toml','Cargo.lock'],check=True)
host=host_identity();assert host['hardware']=='iMac20,1' and 'i7-10700' in host['cpu']
for key in p.ENV_RESET:os.environ.pop(key,None)
os.environ['TZ']='UTC';os.environ['LC_ALL']='C';commands=[]
def run(argv,timeout):
 left=deadline-time.monotonic()
 if left<=0 or timeout<=0:raise RuntimeError('20-minute PMU budget exhausted; incomplete; retain raw traces')
 proc=subprocess.Popen(list(map(str,argv)),stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True);began=time.monotonic()
 try:stdout,stderr=proc.communicate(timeout=min(left,timeout))
 except subprocess.TimeoutExpired:
  backend.stop_process(proc);stdout,stderr=proc.communicate();row={'argv':list(map(str,argv)),'returncode':proc.returncode,'stdout':stdout,'stderr':stderr,'timed_out':True,'seconds':time.monotonic()-began};commands.append(row);(out/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');raise RuntimeError('PMU command timed out; retained partial trace; no retry')
 row={'argv':list(map(str,argv)),'returncode':proc.returncode,'stdout':stdout,'stderr':stderr,'seconds':time.monotonic()-began};commands.append(row);(out/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');return row
try:meta=backend.inspect(backend.tool_path(),run)
except Exception as error:
 (out/'blocked.json').write_text(json.dumps({'diagnostic_only':True,'status':'blocked','reason':str(error),'pmu_samples':0,'security_changes':False,'attempts_consumed':0},indent=2)+'\n');raise
# No trace is recorded before the capability/identity request is sealed.
meta['tool_sha256']=sha(meta['tool']);meta['template_sha256']=sha(meta['template']) if Path(meta['template']).is_file() else None
base=done(cfg['canonical_job']);aligned=done(cfg['aligned_job']);sources={'base':base/'triage/base000/qjs','candidate':base/'triage/cand000/qjs','aligned':aligned/'triage/cand000/qjs'};paths={}
for role in p.ROLES:
 side='base' if role=='baseaa' else role;source=sources[side];assert sha(source)==p.HASHES[side]
 target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True);shutil.copy2(source,target);paths[role]=target
active=out/'active/qjs';active.parent.mkdir();work=out/'work';work.mkdir();cases=resolve_cases(p.CASES,DEFAULT_CACHE,work)
request={'artifact_type':'t056-pmu-diagnostic','diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'base':p.BASE,'aligned':p.ALIGNED,'candidate':p.CANDIDATE,'hashes':p.HASHES,'cases':p.CASES,'blocks':p.BLOCKS,'N':p.COUNT,'samples':32,'backend':meta,'host':host,'configuration':cfg,'inodes':{r:x.stat().st_ino for r,x in paths.items()},'budgets':{'collect':p.COLLECT_SECONDS,'body':p.BODY_SECONDS,'job':p.JOB_SECONDS,'bundle':p.BUNDLE_SECONDS},'budget_basis':'32 bundles x 30s=960s +120s setup/reserve; xctrace throughput unavailable on this host, completion not guaranteed','requested_events_file_sha256':cfg['event_plan_sha256'],'analysis_caveat':'preserve raw PMU tables, event encoding/units, sampling/coverage/PID attribution; no normalization of unknown schema, no absent event represented by zero; capture completion is not validated hardware attribution'}
(out/'request.json').write_text(json.dumps(request,indent=2)+'\n');packets=[];checks={}
with measurement_lock('T056 lead PMU attribution'):
 for block in range(p.BLOCKS):
  for ci,case in enumerate(p.order(cases,block)):
   for role in p.order(p.ROLES,block,ci):
    active.unlink(missing_ok=True);os.link(paths[role],active)
    # Only a correctness warmup; these subprocess costs are never counter data.
    q=run([active,'--raw','-e','0'],10);assert q['returncode']==0 and q['stdout'].strip()=='0'
    sizes=[('N',p.COUNT),('2N',2*p.COUNT)]
    if block%2:sizes.reverse()
    for size,n in sizes:
     bundle_deadline=min(deadline,time.monotonic()+p.BUNDLE_SECONDS);tag=f'{block}-{ci}-{role}-{size}';trace=out/(tag+'.trace');stdout_file=out/(tag+'.stdout');argv=[active,'--raw',case.workload,case.case_id,str(n)]
     r=run(backend.record_argv(meta,trace,stdout_file,argv,p.TRACE_SECONDS),min(20,bundle_deadline-time.monotonic()))
     assert r['returncode']==0,('PMU recording rejected; no security setting changed',r)
     check=_check_output(case,stdout_file.read_text());key=(case.id,size)
     if key in checks:assert checks[key]==check
     else:checks[key]=check
     toc=out/(tag+'-toc.xml');q=run([*backend.export_argv(meta,trace),'--toc','--output',toc],min(5,bundle_deadline-time.monotonic()));assert q['returncode']==0
     names=backend.schemas(toc);assert names,'no counter schema exposed; raw trace retained; no PMU values fabricated'
     tables=[]
     for index,name in enumerate(names):
      left=bundle_deadline-time.monotonic()
      if left<=0:raise RuntimeError('trace/export bundle budget exhausted; retain partial data')
      target=out/(tag+f'-counter{index}.xml');xpath='/trace-toc/run[@number="1"]/data/table[@schema="'+name+'"]';q=run([*backend.export_argv(meta,trace),'--xpath',xpath,'--output',target],left);assert q['returncode']==0
      tree=ET.parse(target);tables.append({'schema':name,'path':target.name,'sha256':sha(target),'rows':sum(1 for _ in tree.iter('row'))})
     assert any(t['rows'] for t in tables),'counter tables empty; not hardware evidence'
     packets.append({'block':block,'case':case.id,'role':role,'size':size,'iterations':n,'trace':trace.name,'stdout_sha256':sha(stdout_file),'toc_sha256':sha(toc),'tables':tables})
     (out/'captures.json').write_text(json.dumps(packets,indent=2)+'\n');print('CAPTURED',tag,names,flush=True)
assert len(packets)==32 and time.monotonic()<deadline
(out/'complete.json').write_text(json.dumps({'diagnostic_only':True,'capture_complete':True,'hardware_attribution_validated':False,'captures':len(packets),'seconds':time.monotonic()-start,'attempts_consumed':0,'security_changes':False})+'\n')
