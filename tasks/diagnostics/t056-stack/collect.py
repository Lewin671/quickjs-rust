"""Lead-only initial-stack phase probe; no acceptance, no attempt consumed."""
import os,json,hashlib,re,subprocess,shutil,dataclasses,time
from pathlib import Path
from tools.benchmark import screen as s
from tools.benchmark.process import run_process
from tools.benchmark.measure_lock import measurement_lock
from tools.benchmark.pair_disturbance import Monitor,heavy_samples
from tools.benchmark.layout_identity import host_identity
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.performance_decision import load_unit
import protocol as p,phase,analyze,chain_common as chain
out=Path(os.environ['BENCH_OUT']);start=time.monotonic();deadline=start+p.COLLECT_SECONDS
assert re.fullmatch(r'running [0-9TZ:-]+',(out.parent/'status').read_text().strip()),'lead --timing job required; owner must not submit'
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
qroot=done(cfg['snapshot_job']);assert sha(qroot/'snapshots.json')==cfg['snapshot_sha256'];q=read(qroot/'snapshots.json')
root=done(cfg['canonical_job']);paths={};snapshots={};roles=p.roles()
for side,version in [('b','base'),('c','allocation')]:
 canonical=root/'triage'/('base000' if side=='b' else 'cand000')/'qjs';assert sha(canonical)==p.HASHES[side]
 meta=q['images'][version];assert meta['canonical_sha256']==p.HASHES[side] and meta['mapped_sections_unchanged'] and meta['caller_and_prologue_bytes_identical']
 snap=qroot/(version+'-snapshot');assert sha(snap)==meta['snapshot_sha256'];snapshots[side]=snap
 for role in [side,side+'aa']:
  target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True);shutil.copy2(canonical,target);paths[role]=target
work=out/'work';work.mkdir();cases=s.resolve_cases(p.CASE_IDS[:-1],s.DEFAULT_CACHE,work);empty=work/'empty.js';empty.write_text('0;\n');cases.append(dataclasses.replace(cases[0],id='diagnostic/empty',kind='whole',workload=empty,case_id=None,expected_stdout=None))
os.environ['TZ']='UTC';os.environ['LC_ALL']='C'
active=out/'active/qjs';active.parent.mkdir();nxt=active.with_name('next')
def activate(path):
 nxt.unlink(missing_ok=True);os.link(path,nxt);os.replace(nxt,active)
def environment(pad):
 env=dict(os.environ)
 for key in (*p.ENV_KEYS,'T056_CAPTURE_GRID'):env.pop(key,None)
 env.update(TZ='UTC',LC_ALL='C',T056_INITIAL_STACK_PAD='x'*pad)
 return env
calibration=[]
def capture(side,case,pad,n):
 activate(snapshots[side]);env=environment(pad);row=phase.take(active,['--raw',str(case.workload),case.case_id,str(n)],env)
 assert row['ops_count']==(4 if case.id=='broad/global_read' else 5)
 row.update(side=side,case=case.id,pad_bytes=pad,iterations=n);calibration.append(row);return row
seeds=[capture(side,cases[0],0,p.COUNT) for side in ('b','c')];grid=p.grid(seeds);assert len(grid)<=30 and all(x%16==0 for x in grid)
for side in ('b','c'):
 for case in cases[:-1]:
  for pad in grid:
   for n in (p.COUNT,2*p.COUNT):capture(side,case,pad,n)
# Same geometry must be stable across N/2N, with deliberate stack movement and
# unchanged hot heap page offsets. Otherwise no canonical timing starts.
for side in ('b','c'):
 for case in cases[:-1]:
  group=[r for r in calibration if r['side']==side and r['case']==case.id];seed=next(r for r in group if r['pad_bytes']==0)
  for r in group:
   assert (r['stack_slot']-seed['stack_slot'])%4096==(-r['pad_bytes'])%4096
  assert len({r['bank']%4096 for r in group})==1 if side=='c' else True
  assert any(analyze.overlap(r,8) for r in group) and any(not analyze.overlap(r,8) for r in group),'grid misses predicted hit or clean phase'
heap_variation=[{'side':side,'case':case.id,'low12_histograms':{key:{str(v):sum(r[key]%4096==v for r in calibration if r['side']==side and r['case']==case.id) for v in {r[key]%4096 for r in calibration if r['side']==side and r['case']==case.id}} for key in ('bank','program','ops_base')}} for side in ('b','c') for case in cases[:-1]]
(out/'heap-variation.json').write_text(json.dumps(heap_variation,indent=2)+'\n')
(out/'calibration.json').write_text(json.dumps(calibration,indent=2)+'\n')
request={'artifact_type':'t056-stack-phase-diagnostic','diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'base':p.BASE,'candidate':p.CANDIDATE,'grid':grid,'grid_rule':'16 coarse phases + two address-only anchors with -64,-32,-16,0,16,32,64; one shared grid, sealed before canonical costs','cases':p.CASE_IDS,'blocks':p.BLOCKS,'iterations':p.COUNT,'roles':roles,'inodes':{r:x.stat().st_ino for r,x in paths.items()},'binary_sha256':{r:sha(x) for r,x in paths.items()},'configuration':cfg,'host':host_identity(),'budgets':{'collect':p.COLLECT_SECONDS,'body':p.BODY_SECONDS,'running_job':p.JOB_SECONDS},'budget_basis':{'reference_job':'20261021T085751Z-t056-code-layout-cross','reference_units':960,'reference_seconds':600.242272,'max_units':1440,'reserve_factor':1.2,'setup_analysis_seconds':120},'calibration_sha256':sha(out/'calibration.json'),'only_factor':'unknown environment entry with fixed name/count, varying string length; no new engine allocation, no bytecode/code/zone change; absolute heap ASLR is not frozen'}
(out/'request.json').write_text(json.dumps(request,indent=2)+'\n')
rows={(pad,case.id):[] for pad in grid for case in cases};current={};raw=(out/'raw.jsonl').open('x');tool=s.counter_tool(trust_cycles=False);checks={};preflights=[]
def runner(argv,timeout):
 result=run_process(argv,p.remaining(deadline,time.monotonic(),timeout));raw.write(json.dumps({**current,'argv':argv,'result':dataclasses.asdict(result)})+'\n');raw.flush();return result
with measurement_lock('T056 initial stack phase diagnostic'):
 monitor=Monitor.from_queue()
 for block in range(p.BLOCKS):
  for pi,pad in enumerate(p.order(grid,block)):
   env=environment(pad)
   for case in p.order(cases,block,pi):
    current.update(block=block,pad_bytes=pad,case=case.id);row={'block':block,'samples':{},'order':p.order(roles,block,pi)}
    for role in row['order']:
     current['role']=role;activate(paths[role])
     for key in (*p.ENV_KEYS,'T056_CAPTURE_GRID'):os.environ.pop(key,None)
     os.environ['T056_INITIAL_STACK_PAD']='x'*pad
     current['phase']='preflight';preflights.extend(s.launch_preflight([active],120,runner=runner))
     sizes=[('whole',None)] if case.kind=='whole' else [('N',p.COUNT),('2N',2*p.COUNT)]
     if block%2:sizes.reverse()
     values={}
     for size,n in sizes:
      current.update(phase='measured',size=size);v=s.sample(tool,s._argv(active,case,n),120,runner=runner);check=s._check_output(case,v.stdout);key=(case.id,size)
      if key in checks:assert checks[key]==check
      else:checks[key]=check
      values[size]=dataclasses.asdict(v)
     row['samples'][role]=values
    rows[pad,case.id].append(row)
    (out/f'pad{pad}-{case.id.replace("/","--")}.json').write_text(json.dumps({'pad_bytes':pad,'case':case.id,'rows':rows[pad,case.id]},indent=2)+'\n')
  print('BLOCK',block+1,'/',p.BLOCKS,'all phases/cases/AA retained',flush=True)
 load=monitor.snapshot();(out/'load.json').write_text(json.dumps(load,indent=2)+'\n');assert not heavy_samples(load['text'],load['anchor']),'DISTURBED: no pair deletion or decision'
 effects=[]
 for pad in grid:
  for case in cases:
   rr=rows[pad,case.id];assert len(rr)==p.BLOCKS
   for row in rr:
    for role in roles:
     for metric in ('instructions','wall_ns'):
      v=analyze.increment(row,role,metric)
      if case.kind=='internal' and metric=='wall_ns':assert v>=s.MIN_INCREMENT_NS
   effects.append({'pad_bytes':pad,'case':case.id,'effects':{name:analyze.effects(rr,w) for name,w in {'C/B':{'c':1,'b':-1},'base_AA':{'baa':1,'b':-1},'candidate_AA':{'caa':1,'c':-1}}.items()}})
 (out/'effects.json').write_text(json.dumps(effects,indent=2)+'\n')
 # Retain phase-relative side costs for preregistered collision/clean contrasts;
 # no best offset or favorable run is chosen by this producer.
 costs=[{'case':case.id,'side':side,'pad_bytes':pad,'block':r['block'],'wall_ns':analyze.increment(r,side,'wall_ns'),'instructions':analyze.increment(r,side,'instructions')} for case in cases for side in ('b','c') for pad in grid for r in rows[pad,case.id]]
 (out/'phase-costs.json').write_text(json.dumps(costs,indent=2)+'\n')
 (out/'assessment.json').write_text(json.dumps(analyze.assess(costs,calibration,effects),indent=2)+'\n')
 p.remaining(deadline,time.monotonic(),1)
raw.close();(out/'preflights.json').write_text(json.dumps(preflights,indent=2)+'\n')
(out/'complete.json').write_text(json.dumps({'cases':3,'phases':len(grid),'blocks':p.BLOCKS,'units':len(grid)*3*4*p.BLOCKS,'seconds':time.monotonic()-start,'diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'all_outputs_match':True})+'\n')
