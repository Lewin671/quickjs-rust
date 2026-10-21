"""Local simulations and guard/routing checks; never submits or runs an engine."""
import ast,contextlib,datetime,io,json,os,runpy,subprocess,sys,tempfile,types
from pathlib import Path
from unittest.mock import patch
from zoneinfo import ZoneInfo
ROOT=Path(__file__).resolve().parent;REPO=ROOT.parents[2];os.chdir(REPO);sys.path[:0]=[str(ROOT),str(REPO)]
import protocol as p,render,window,watch,chain_common as chain,analyze
from tools.benchmark import screen as s
from tools.benchmark.process import ProcessResult
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.statistics import paired_block_bootstrap
cfg=json.loads((ROOT/'config.json').read_text());queue=Path(os.environ.get('T056_DIAGNOSTIC_QUEUE','/Users/qingyingliu/Code/co-work-space/outputs/imac-bench'))
for name,expected in cfg['files'].items():assert chain.sha(ROOT/name)==expected
for path in ROOT.glob('*.py'):ast.parse(path.read_text(),filename=str(path))
for kind in ('mini','cross'):
 script=render.render(kind);subprocess.run(['bash','-n'],input=script,text=True,check=True)
 ast.parse(script.split("<<'T056_WINDOW'\n",1)[1].rsplit('\nT056_WINDOW',1)[0])
 ids,blocks,layouts=p.design(kind);assert sum(len(p.roles(kind,k))*blocks*len(ids) for k in layouts)=={'mini':1080,'cross':960}[kind]
 for k in layouts:
  roles=p.roles(kind,k)
  for weights in p.comparisons(kind,k).values():assert sum(weights.values())==0 and set(weights)<=set(roles)
  for block in range(blocks):assert set(p.order(roles,block))==set(roles)
 assert 401.0238/540*{'mini':1080,'cross':960}[kind]*1.4+180<p.COLLECT_SECONDS<p.BODY_SECONDS<p.JOB_SECONDS==1800
assert p.OFFSETS==(0,16,64,240,512,1008)
for role,meta in p.roles('cross').items():
 assert set(meta['environment'])==set(p.ENV_KEYS) and meta['environment']['MallocNanoZone'] in ('0','1')
 assert all(value is None for key,value in meta['environment'].items() if key!='MallocNanoZone')
try:p.remaining(1,1,120)
except RuntimeError:pass
else:raise AssertionError('expired budget accepted')
assert p.remaining(5,1,120)==4
oldzone=window.HOST_ZONE
for zone in ('Asia/Shanghai','America/Los_Angeles'):
 window.HOST_ZONE=ZoneInfo(zone)
 for date in ('2026-10-21','2026-11-01','2027-03-14'):
  for clock in ('00:10','05:01','12:00','23:00'):
   now=datetime.datetime.fromisoformat(date+'T'+clock).replace(tzinfo=window.HOST_ZONE).timestamp();deadline=window.deadline_for(now);assert deadline>now and window.host_local(deadline+900).strftime('%H:%M')=='03:00'
  for clock in ('03:00','04:59'):
   try:window.deadline_for(datetime.datetime.fromisoformat(date+'T'+clock).replace(tzinfo=window.HOST_ZONE).timestamp())
   except ValueError:pass
   else:raise AssertionError('night accepted')
window.HOST_ZONE=oldzone
stamp=datetime.datetime(2026,10,21,tzinfo=datetime.timezone.utc).timestamp()
assert not watch.expired('queued',stamp+99999) and not watch.expired('running 2026-10-21T00:00:00Z',stamp+1799) and watch.expired('running 2026-10-21T00:00:00Z',stamp+1800)
ast.parse(watch.stop_script())

def effect(ratio=1):return {'wall_ns':{'median':ratio,'lower':ratio*.995,'upper':ratio*1.005},'instructions':{'median':1,'lower':.999,'upper':1.001}}
def routing_rows(gone):
 return [{'layout':k,'case':case,'effects':{'C/B':effect(1.01 if k<gone else 1.427),'base_AA':effect(),'candidate_AA':effect()}} for k in range(6) for case in p.MINI_CASES]
assert 'may consider' in analyze.route(routing_rows(5))['next_step']
assert 'persistent' in analyze.route(routing_rows(1))['next_step']
assert 'mixed' in analyze.route(routing_rows(4))['next_step']
r=routing_rows(6);r[0]['effects']['base_AA']=effect(1.1);assert 'inconclusive' in analyze.route(r)['next_step']
r=routing_rows(6)
for x in r:x['effects']['C/B']['instructions']['median']=1.02
assert 'may consider' not in analyze.route(r)['next_step']
try:analyze.route(r[:-1])
except AssertionError:pass
else:raise AssertionError('partial panel routed')
r=routing_rows(6);r[-1]=r[-2]
try:analyze.route(r)
except AssertionError:pass
else:raise AssertionError('duplicate case substituted for missing control')
try:analyze.increment({'block':0,'samples':{'x':{'N':{'wall_ns':2},'2N':{'wall_ns':1}}}},'x','wall_ns')
except ValueError:pass
else:raise AssertionError('negative increment accepted')

unit=json.loads(chain.PLAN.read_text());policy=load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256'],live=False)
qualification=json.loads((queue/cfg['qualification_job']/'out/qualification.json').read_text());original_sysconf=os.sysconf;original_load=load_policy
case_map={};calls=[]
def fake_cases(ids,cache,work):
 cases=[]
 for i,name in enumerate(ids):
  internal=not name.startswith('external/');path=work/(str(i)+'.js');path.write_text('// simulated, never executed\n')
  case=s.CaseSpec(name,'internal' if internal else 'whole',path,name.rsplit('/',1)[-1] if internal else None,None if internal else 'FAKE_OK\n');case_map[str(path)]=case;cases.append(case)
 return cases

def fake_process(argv,timeout):
 assert 0<timeout<=120
 cmd=argv[2:] if argv[:2]==['/usr/bin/time','-l'] else argv
 if cmd[-2:]==['-e','0']:n=1;stdout='0\n';duration=1000
 else:
  case=case_map.get(cmd[2]);n=int(cmd[-1]) if case and case.kind=='internal' else 10000
  stdout=s.RESULT_PREFIX+json.dumps({'operations':n,'checksum':0})+'\n' if case and case.kind=='internal' else case.expected_stdout if case else '0\n';duration=20_000*n+1_000_000
 env={k:os.environ.get(k) for k in p.ENV_KEYS};calls.append(env)
 return ProcessResult('SIMULATED',0,duration,duration,0,False,stdout,str(100*n+1_000_000)+' instructions retired\nENV '+json.dumps(env)+'\n',False,False)
reports=[]
for kind in ('mini','cross'):
 with tempfile.TemporaryDirectory(prefix='.simulation-',dir=ROOT) as temp:
  out=Path(temp)/'out';out.mkdir();(out.parent/'status').write_text('running 2026-10-21T00:00:00Z\n');(out/'config.json').write_text(json.dumps(cfg))
  for name in cfg['files']:(out/name).write_bytes((ROOT/name).read_bytes())
  snapshot={'anchor':stamp,'text':'','sha256':chain.hashlib.sha256(b'').hexdigest(),'sealed_at':stamp+1}
  with contextlib.ExitStack() as stack:
   stack.enter_context(patch.dict(os.environ,{'BENCH_OUT':str(out),'BENCH_QUEUE':str(queue)},clear=False))
   stack.enter_context(patch.object(sys,'argv',['collect.py',kind]))
   stack.enter_context(patch('tools.benchmark.layout_identity.host_identity',return_value=qualification['host']))
   stack.enter_context(patch('tools.benchmark.layout_policy.load_policy',side_effect=lambda *a,**k:original_load(*a,**{**k,'live':False})))
   stack.enter_context(patch('tools.benchmark.screen.resolve_cases',side_effect=fake_cases))
   stack.enter_context(patch('tools.benchmark.screen.counter_tool',return_value=s.counter_tool(system='Darwin',trust_cycles=False)))
   stack.enter_context(patch('tools.benchmark.process.run_process',side_effect=fake_process))
   stack.enter_context(patch('tools.benchmark.pair_disturbance.Monitor.from_queue',return_value=types.SimpleNamespace(snapshot=lambda:snapshot)))
   # Statistics implementation is separately covered by full tests; reduce
   # resample count only in simulation, all fake effect rows equal exactly1.
   stack.enter_context(patch.object(analyze,'paired_block_bootstrap',side_effect=lambda *a,**k:paired_block_bootstrap(*a,**{**k,'samples':100})))
   stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
   runpy.run_path(str(ROOT/'collect.py'),run_name='__main__')
  complete=json.loads((out/'complete.json').read_text());assert complete['case_role_blocks']=={'mini':1080,'cross':960}[kind]
  req=json.loads((out/'request.json').read_text());effects=json.loads((out/'effects.json').read_text())
  assert all(abs(v['median']-1)<1e-12 for row in effects for e in row['effects'].values() for v in e.values())
  for name,meta in req['roles'].items():
   if meta['aa']:assert req['inodes'][name]!=req['inodes'][name[:-2]]
  if kind=='cross':
   for side in ('b','c'):
    for code in (0,1):assert req['inodes'][f'{side}{code}z0']==req['inodes'][f'{side}{code}z1']
  raw=[json.loads(l) for l in (out/'raw.jsonl').read_text().splitlines()]
  for row in raw:assert json.loads(row['result']['stderr'].split('ENV ',1)[1])==req['roles'][row['role']]['environment']
  if kind=='mini':assert json.loads((out/'routing.json').read_text())['excess_disappeared_layouts']==6
  reports.append(complete)
subprocess.run(['git','diff','--exit-code',p.CANDIDATE,'HEAD','--','crates','tools','scripts','.cargo','Cargo.toml','Cargo.lock'],check=True)
report={'scope':'FAKE processes/counters and local guards only; no engine executable or timing job','matrices':reports,'routing_boundary_tests':True,'environment_resets_and_inode_AA':True,'quiet_window_DST_and_30min_cap':True,'rendered_syntax':True,'real_qualified_image_hashes_and_prerequisites':True}
destination=Path(os.environ.get('T056_CODE_VALIDATION',REPO/'target/t056-code-layout-validation.json'));destination.parent.mkdir(parents=True,exist_ok=True);destination.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
