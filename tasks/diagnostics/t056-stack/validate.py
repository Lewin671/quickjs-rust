"""Local guards and actual collector over fake processes; no JS/timing job."""
import ast,contextlib,datetime,io,json,os,runpy,subprocess,sys,tempfile,types,hashlib
from pathlib import Path
from unittest.mock import patch
from zoneinfo import ZoneInfo
ROOT=Path(__file__).resolve().parent;REPO=ROOT.parents[2];os.chdir(REPO);sys.path[:0]=[str(ROOT),str(REPO)]
import protocol as p,phase,analyze,render,window,watch,chain_common as chain
from tools.benchmark import screen as s
from tools.benchmark.process import ProcessResult
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.statistics import paired_block_bootstrap
cfg=json.loads((ROOT/'config.json').read_text());queue=Path(os.environ.get('T056_DIAGNOSTIC_QUEUE','/Users/qingyingliu/Code/co-work-space/outputs/imac-bench'))
for name,h in cfg['files'].items():assert chain.sha(ROOT/name)==h
for f in ROOT.glob('*.py'):ast.parse(f.read_text(),filename=str(f))
for name in ('numeric-audit-8f.json','numeric-audit-dfff.json'):
 audit=json.loads((ROOT/name).read_text());assert audit['all_equivalent']
 for record in audit['records'].values():
  update=record['paths']['Update'];assert len(update)==18
  assert any(ref.get('referent8')=='000000000000f03f' for row in update for ref in row['relocations'])
  for rows in record['paths'].values():
   for row in rows:
    if '%rbp' in row['args'] or '%rsp' in row['args']:assert row['op']=='movq' and row['args']=='-0x30(%rbp), %r8'
    assert not row['op'].startswith(('push','pop','call'))

script=render.render();subprocess.run(['bash','-n'],input=script,text=True,check=True);ast.parse(script.split("<<'T056_WINDOW'\n",1)[1].rsplit('\nT056_WINDOW',1)[0])
assert 600.242272/960*1440*1.2+120<p.COLLECT_SECONDS<p.BODY_SECONDS<p.JOB_SECONDS==1500
assert p.BLOCKS==4 and len(p.roles())==4
for slot in range(0,4096,16):
 seeds=[{'stack_slot':slot,'bank':0},{'stack_slot':(slot-16)%4096,'bank':1024}];grid=p.grid(seeds);assert set(range(0,4096,256))<=set(grid) and len(grid)<=30
 for seed in seeds:
  rows=[{'stack_slot':(seed['stack_slot']-pad)%4096,'bank':seed['bank']} for pad in grid]
  assert any(analyze.overlap(r,8) for r in rows) and any(not analyze.overlap(r,8) for r in rows)
assert analyze.overlap({'stack_slot':0,'bank':0},4) and not analyze.overlap({'stack_slot':16,'bank':0},8)
assert p.remaining(5,1,120)==4
try:p.remaining(1,1,120)
except RuntimeError:pass
else:raise AssertionError('expired budget accepted')
oldzone=window.HOST_ZONE
for zone in ('Asia/Shanghai','America/Los_Angeles'):
 window.HOST_ZONE=ZoneInfo(zone)
 for date in ('2026-10-21','2026-11-01','2027-03-14'):
  for clock in ('00:10','05:01','12:00'):
   now=datetime.datetime.fromisoformat(date+'T'+clock).replace(tzinfo=window.HOST_ZONE).timestamp();assert window.host_local(window.deadline_for(now)+900).strftime('%H:%M')=='03:00'
  try:window.deadline_for(datetime.datetime.fromisoformat(date+'T03:00').replace(tzinfo=window.HOST_ZONE).timestamp())
  except ValueError:pass
  else:raise AssertionError('night window accepted')
window.HOST_ZONE=oldzone
stamp=datetime.datetime(2026,10,21,tzinfo=datetime.timezone.utc).timestamp();assert not watch.expired('queued',stamp+99999) and not watch.expired('running 2026-10-21T00:00:00Z',stamp+1499) and watch.expired('running 2026-10-21T00:00:00Z',stamp+1500)
unit=json.loads(chain.PLAN.read_text());policy=load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256'],live=False);original_load=load_policy
q=json.loads((queue/cfg['snapshot_job']/'out/snapshots.json').read_text());case_map={}
def fake_cases(ids,cache,work):
 result=[]
 for i,name in enumerate(ids):
  file=work/(str(i)+'.js');file.write_text('// fake only\n');case=s.CaseSpec(name,'internal',file,name.rsplit('/',1)[-1]);case_map[str(file)]=case;result.append(case)
 return result

def fake_phase(active,argv,env):
 side='b' if hashlib.sha256(Path(active).read_bytes()).hexdigest()==q['images']['base']['snapshot_sha256'] else 'c';global_case=argv[2]=='global_read';pad=len(env['T056_INITIAL_STACK_PAD']);slot=0xf00+(0 if global_case else 16);slot=(slot-pad)%4096
 row={'ops_count':4 if global_case else 5,'ops_base':0x600000000c00 if side=='b' else 0x600000000900,'bank':0x7f0000000400 if side=='b' else 0x7f0000000000,'program':0x7f1000000200 if side=='b' else 0x7f1000000000,'stack_slot':0x7fff00000000+slot,'stack_slot_value':0x7fff00001000,'env_bytes':sum(len(k+'='+v)+1 for k,v in env.items()),'env_entry_bytes':{k:len(k+'='+v)+1 for k,v in env.items()},'env_count':len(env),'argv_bytes':sum(len(x)+1 for x in argv),'env_order_sha256':hashlib.sha256(repr(env).encode()).hexdigest()};row['rbp']=row['stack_slot']+48;row['rsp']=row['rbp']-(0x1a0 if side=='b' else 0x190);return row

def fake_process(argv,timeout):
 assert 0<timeout<=120
 cmd=argv[2:] if argv[:2]==['/usr/bin/time','-l'] else argv
 if cmd[-2:]==['-e','0']:n=1;stdout='0\n';duration=1000
 else:
  case=case_map.get(cmd[2]);n=int(cmd[-1]) if case else 10000;stdout=s.RESULT_PREFIX+json.dumps({'operations':n,'checksum':0})+'\n' if case else '0\n';duration=20000*n+1000000
 env={k:os.environ.get(k) for k in p.ENV_KEYS};return ProcessResult('SIMULATED',0,duration,duration,0,False,stdout,str(100*n+1000000)+' instructions retired\nENV '+json.dumps(env)+'\n',False,False)
with tempfile.TemporaryDirectory(prefix='.simulation-',dir=ROOT) as temp:
 out=Path(temp)/'out';out.mkdir();(out.parent/'status').write_text('running 2026-10-21T00:00:00Z\n');(out/'config.json').write_text(json.dumps(cfg))
 for name in cfg['files']:(out/name).write_bytes((ROOT/name).read_bytes())
 with contextlib.ExitStack() as stack:
  stack.enter_context(patch.dict(os.environ,{'BENCH_OUT':str(out),'BENCH_QUEUE':str(queue)},clear=False))
  stack.enter_context(patch('tools.benchmark.layout_identity.host_identity',return_value=policy['host']))
  stack.enter_context(patch('tools.benchmark.layout_policy.load_policy',side_effect=lambda *a,**k:original_load(*a,**{**k,'live':False})))
  stack.enter_context(patch('tools.benchmark.screen.resolve_cases',side_effect=fake_cases))
  stack.enter_context(patch('tools.benchmark.screen.counter_tool',return_value=s.counter_tool(system='Darwin',trust_cycles=False)))
  stack.enter_context(patch('tools.benchmark.process.run_process',side_effect=fake_process))
  stack.enter_context(patch.object(phase,'take',side_effect=fake_phase))
  stack.enter_context(patch('tools.benchmark.pair_disturbance.Monitor.from_queue',return_value=types.SimpleNamespace(snapshot=lambda:{'text':'','anchor':stamp})))
  stack.enter_context(patch.object(analyze,'paired_block_bootstrap',side_effect=lambda *a,**k:paired_block_bootstrap(*a,**{**k,'samples':100})))
  stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
  runpy.run_path(str(ROOT/'collect.py'),run_name='__main__')
 complete=json.loads((out/'complete.json').read_text());assert complete['units']<=1440 and complete['blocks']==4
 req=json.loads((out/'request.json').read_text());assert req['inodes']['b']!=req['inodes']['baa'] and req['inodes']['c']!=req['inodes']['caa']
 assert json.loads((out/'assessment.json').read_text())['verdict'].startswith('flat:')
 effects=json.loads((out/'effects.json').read_text());assert len(effects)==len(req['grid'])*3
 assert all(abs(v['median']-1)<1e-12 for r in effects for e in r['effects'].values() for v in e.values())
 for line in (out/'raw.jsonl').read_text().splitlines():
  r=json.loads(line);env=json.loads(r['result']['stderr'].split('ENV ',1)[1]);assert len(env['T056_INITIAL_STACK_PAD'])==r['pad_bytes'] and all(v is None for k,v in env.items() if k!='T056_INITIAL_STACK_PAD')
 calibration=json.loads((out/'calibration.json').read_text());costs=json.loads((out/'phase-costs.json').read_text())
 predicted={x['pad_bytes'] for x in calibration if x['side']=='c' and x['case']=='broad/global_read' and analyze.overlap(x,8)}
 altered=[dict(x,wall_ns=x['wall_ns']*1.4) if x['side']=='c' and x['case']=='broad/global_read' and x['pad_bytes'] in predicted else x for x in costs]
 assert analyze.assess(altered,calibration,effects)['verdict'].startswith('supports')
 altered_cal=[dict(x,program=x['program']+16) if i==0 and x['side']=='c' else x for i,x in enumerate(calibration)]
 # Force one explicit candidate-global heap anomaly; it must prevent rejection.
 idx=next(i for i,x in enumerate(calibration) if x['side']=='c' and x['case']=='broad/global_read');altered_cal=list(map(dict,calibration));altered_cal[idx]['program']+=16
 assert analyze.assess(costs,altered_cal,effects)['verdict'].startswith('inconclusive')
 report={'scope':'fake processes/counters/phase only, no engine executable or timing submission','complete':complete,'geometry_all_256_aligned_seed_pairs_tested':True,'actual_collector_completed':True,'environment_AA_inodes_raw_effects_verified':True,'quiet_window_DST_and_25min_watchdog':True}
 Path('target/t056-stack-validation.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
