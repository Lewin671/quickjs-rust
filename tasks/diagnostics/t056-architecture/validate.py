"""Local/static and fake-process checks only; never launches an engine or job."""
import ast,contextlib,datetime,io,json,os,re,runpy,subprocess,sys,tempfile
from pathlib import Path
from unittest.mock import patch
from zoneinfo import ZoneInfo
ROOT=Path(__file__).resolve().parent
REPO=Path(os.environ.get('T056_ARCH_REPO',ROOT.parents[2]));assert (REPO/'Cargo.toml').exists(), 'set T056_ARCH_REPO to the engine checkout'
os.chdir(REPO);sys.path[:0]=[str(ROOT),str(REPO)]
import protocol as p,render,window,watch,chain_common as chain
checks=[]
cfg=json.loads((ROOT/'probe-config.json').read_text())
assert chain.sha(ROOT/'collect.py')==cfg['collector_sha256'] and chain.sha(ROOT/'protocol.py')==cfg['protocol_sha256']
assert chain.sha(ROOT/'chain_common.py')=='24f2e1551c457b6e5d37f70229d3ec2f85e49cc4a3da563ad4fb31a6f411e7d5'
for f in ROOT.glob('*.py'):ast.parse(f.read_text(),filename=str(f))
script=render.render();subprocess.run(['bash','-n'],input=script,text=True,check=True);ast.parse(script.split("<<'T056_WINDOW'\n",1)[1].rsplit('\nT056_WINDOW',1)[0]);checks.append('all modules and rendered shell/embedded Python parse; renderer only prints')
assert p.BLOCKS==15 and len(p.roles())==12 and len(p.CASE_IDS)==6
assert 15*12*6==1080
assert 802*1.4+120 < p.COLLECT_SECONDS < p.BODY_SECONDS < p.JOB_SECONDS==1500
for weights in p.comparisons().values():assert sum(weights.values())==0 and set(weights)<=set(p.roles())
for block in range(15):assert set(p.order(p.roles(),block))==set(p.roles())
for side in ('b','c'):
 r=p.roles();assert r[side+'_pad128']['image']==r[side+'_pad144']['image']==r[side+'_pad1536']['image']==r[side+'_padaa']['image']
 assert {len(r[side+'_'+f]['environment']['QJS_ARCH_PAD_MODE']) for f in ('pad128','pad144','pad1536','padaa')}=={7}
 assert set(r[side+'_nat']['environment'])==set(p.ENV_KEYS) and all(v is None for v in r[side+'_nat']['environment'].values())
try:p.remaining(1,1,120)
except RuntimeError:pass
else:raise AssertionError('expired sample accepted')
assert p.remaining(5,1,120)==4
checks.append('15 blocks, six cases, twelve roles, 1080 units; projection fits; contrasts, resets, same-image factors')
oldzone=window.HOST_ZONE
for z in ('Asia/Shanghai','America/Los_Angeles'):
 window.HOST_ZONE=ZoneInfo(z)
 for date in ('2026-10-21','2026-11-01','2027-03-14'):
  for clock in ('00:10','05:01','12:00','23:00'):
   now=datetime.datetime.fromisoformat(date+'T'+clock).replace(tzinfo=window.HOST_ZONE).timestamp();deadline=window.deadline_for(now);assert deadline>now and window.host_local(deadline+900).strftime('%H:%M')=='03:00'
  for clock in ('03:00','04:59'):
   try:window.deadline_for(datetime.datetime.fromisoformat(date+'T'+clock).replace(tzinfo=window.HOST_ZONE).timestamp())
   except ValueError:pass
   else:raise AssertionError('night window accepted')
window.HOST_ZONE=oldzone
stamp=datetime.datetime(2026,10,21,tzinfo=datetime.timezone.utc).timestamp()
assert not watch.expired('queued',stamp+99999) and not watch.expired('done 0',stamp+99999)
assert not watch.expired('running 2026-10-21T00:00:00Z',stamp+1499)
assert watch.expired('running 2026-10-21T00:00:00Z',stamp+1500)
ast.parse(watch.stop_script());assert "'bench-'+sys.argv[1]" in watch.stop_script()
checks.append('Shanghai/LA/DST quiet windows; total running-job watchdog includes quieting, excludes queue wait; remote own-tree stop payload parses')
# Run the actual collector with real sealed artifact guards and files, but a
# simulated x86 host and fake subprocess/counters. No JS executable is started.
from tools.benchmark import screen as s
from tools.benchmark.process import ProcessResult
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.layout_identity import host_identity
unit=json.loads(chain.PLAN.read_text());policy=load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256'],live=False)
original_load=load_policy;original_sysconf=os.sysconf
case_map={}
def fake_cases(ids,cache,work):
 result=[]
 for i,name in enumerate(ids):
  internal=not name.startswith('external/');f=work/(str(i)+'.js');f.write_text('// simulation only; never executed\n')
  case=s.CaseSpec(name,'internal' if internal else 'whole',f,name.rsplit('/',1)[-1] if internal else None,None if internal else 'FAKE_EXTERNAL_OK\n');result.append(case);case_map[str(f)]=case
 return result
calls=[]
def fake_process(argv,timeout):
 assert 0 < timeout <= 120
 command=argv[2:] if argv[:2]==['/usr/bin/time','-l'] else argv
 if command[-2:]==['-e','0']:n=1;stdout='0\n';duration=1000
 else:
  case=case_map.get(command[2]);n=int(command[-1]) if case is not None and case.kind=='internal' else 10000
  stdout=s.RESULT_PREFIX+json.dumps({'operations':n,'checksum':0})+'\n' if case is not None and case.kind=='internal' else case.expected_stdout if case is not None else '0\n'
  duration=20_000*n+1_000_000
 env={k:os.environ.get(k) for k in p.ENV_KEYS};calls.append(env)
 stderr=str(100*n+1_000_000)+' instructions retired\nENV '+json.dumps(env)+'\n'
 return ProcessResult('SIMULATED',0,duration,duration,0,False,stdout,stderr,False,False)
with tempfile.TemporaryDirectory(prefix='.simulation-',dir=ROOT) as temp:
 out=Path(temp)/'out';out.mkdir();(out.parent/'status').write_text('running 2026-10-21T00:00:00Z\n');cfg=json.loads((ROOT/'probe-config.json').read_text());(out/'probe-config.json').write_text(json.dumps(cfg))
 for name in ('protocol.py','chain_common.py'): (out/name).write_bytes((ROOT/name).read_bytes())
 captured=io.StringIO()
 with contextlib.ExitStack() as stack:
  stack.enter_context(patch.dict(os.environ,{'BENCH_OUT':str(out),'BENCH_QUEUE':'/Users/qingyingliu/Code/co-work-space/outputs/imac-bench'},clear=False))
  stack.enter_context(patch('tools.benchmark.layout_identity.host_identity',return_value={**policy['host'],'hardware':'iMac20,1','cpu':'Intel(R) Core(TM) i7-10700 CPU @ 2.90GHz'}))
  stack.enter_context(patch('tools.benchmark.layout_policy.load_policy',side_effect=lambda *a,**k:original_load(*a,**{**k,'live':False})))
  stack.enter_context(patch('os.sysconf',side_effect=lambda name:4096 if name=='SC_PAGE_SIZE' else original_sysconf(name)))
  stack.enter_context(patch('tools.benchmark.screen.resolve_cases',side_effect=fake_cases))
  stack.enter_context(patch('tools.benchmark.screen.counter_tool',return_value=s.counter_tool(system='Darwin',trust_cycles=False)))
  stack.enter_context(patch('tools.benchmark.process.run_process',side_effect=fake_process))
  stack.enter_context(contextlib.redirect_stdout(captured))
  runpy.run_path(str(ROOT/'collect.py'),run_name='__main__')
 complete=json.loads((out/'complete.json').read_text());assert complete['blocks']==15 and complete['case_role_blocks']==1080
 request=json.loads((out/'request.json').read_text());allrows=[]
 for name in p.CASE_IDS:
  r=json.loads((out/(name.replace('/','--')+'.json')).read_text());assert len(r['rows'])==15
  for row in r['rows']:assert set(row['samples'])==set(p.roles())
  allrows.extend(r['rows'])
 for side in ('b','c'):
  assert request['role_inodes'][side+'_pad128']==request['role_inodes'][side+'_pad144']==request['role_inodes'][side+'_pad1536']!=request['role_inodes'][side+'_padaa']
  assert request['role_inodes'][side+'_nat']!=request['role_inodes'][side+'_aa']
 raw=[json.loads(x) for x in (out/'raw.jsonl').read_text().splitlines()]
 for item in raw:
  effective=json.loads(item['result']['stderr'].split('ENV ',1)[1]);assert effective==request['roles'][item['role']]['environment']
 effects=json.loads((out/'effects.json').read_text());assert len(effects)==6
 assert all(abs(v['median']-1)<1e-12 for c in effects for m in c['effects'].values() for v in m.values())
checks.append('actual collector completes simulated 15-block matrix with fake process/counters; env effective values reset, raw logs/AA/same inode/output/ratios verified; temporary fake artifacts removed')
oldqueue=os.environ.get('BENCH_QUEUE');os.environ['BENCH_QUEUE']='/Users/qingyingliu/Code/co-work-space/outputs/imac-bench'
for old in ['20261021T013452Z-t056-placement-code-probe','20261020T233507Z-t056-nbody-final-correctness']:
 try:chain.job(old)
 except ValueError:pass
 else:raise AssertionError('old/failed prerequisite accepted')
if oldqueue is None:os.environ.pop('BENCH_QUEUE',None)
else:os.environ['BENCH_QUEUE']=oldqueue
checks.append('old and failed prerequisites rejected; frozen receipt helper unchanged')
subprocess.run(['git','diff','--exit-code',p.CANDIDATE,'HEAD','--','crates','tools','scripts','.cargo','Cargo.toml','Cargo.lock'],check=True)
report={'scope':'static/local and FAKE-PROCESS SIMULATION only; no engine, timing or job submission','candidate':p.CANDIDATE,'cases':list(p.CASE_IDS),'blocks':p.BLOCKS,'case_role_units':1080,'checks':checks}
destination=Path(os.environ.get('T056_ARCH_VALIDATION',REPO/'target/t056-architecture-validation.json'));destination.parent.mkdir(parents=True,exist_ok=True)
destination.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
