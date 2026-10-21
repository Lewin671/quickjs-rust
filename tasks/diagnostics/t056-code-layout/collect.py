"""Lead-only mini K=6 or code x zone diagnostic; no acceptance artifacts."""
import dataclasses,hashlib,json,os,re,shutil,subprocess,sys,time
from pathlib import Path
from tools.benchmark import screen as s
from tools.benchmark.measure_lock import measurement_lock
from tools.benchmark.process import run_process
from tools.benchmark.layout_identity import host_identity
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.performance_decision import load_unit
from tools.benchmark.pair_disturbance import Monitor,heavy_samples
import chain_common as chain
import protocol as p
import analyze
kind=sys.argv[1];case_ids,blocks,layouts=p.design(kind)
out=Path(os.environ['BENCH_OUT']);start=time.monotonic();deadline=start+p.COLLECT_SECONDS
cfg=json.loads((out/'config.json').read_text())
assert re.fullmatch(r'running [0-9TZ:-]+',(out.parent/'status').read_text().strip()),'requires lead --timing runner; owner must not submit'
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def read(path):return json.loads(Path(path).read_text())
def done(identifier):
 assert re.fullmatch(r'[0-9]{8}T[0-9]{6}Z-[a-z0-9._-]+',identifier)
 root=Path(os.environ['BENCH_QUEUE'])/identifier
 assert (root/'status').read_text().strip()=='done 0' and 'DISTURBED' not in (root/'log').read_text()
 return root/'out'
for name,expected in cfg['files'].items():assert sha(Path(__file__).with_name(name))==expected
subprocess.run(['git','merge-base','--is-ancestor',cfg['protocol_revision'],'HEAD'],check=True)
subprocess.run(['git','diff','--exit-code',p.CANDIDATE,'HEAD','--','crates','tools','scripts','.cargo','Cargo.toml','Cargo.lock'],check=True)
unit,h=load_unit(chain.PLAN);assert h==p.UNIT_SHA
policy=load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256'])
assert tuple(policy['offsets'])==p.OFFSETS and policy['cases']['broad/global_read']['wall_band']==p.GLOBAL_BAND
chain.correctness(cfg['correctness_job']);chain.pregate(cfg['pregate_job'],policy)
triage=chain.job(cfg['triage_job']);assert sha(triage/'triage/triage.json')==cfg['triage_sha256']
qualified=done(cfg['qualification_job']);assert sha(qualified/'qualification.json')==cfg['qualification_sha256']
q=read(qualified/'qualification.json');assert q['diagnostic_only'] and q['timing'] is False and q['all_outputs_match']
assert tuple(q['offsets'])==p.OFFSETS and q['host']==host_identity() and len(q['qualification'])==12
assert all(x['ng_fixtures']==255 for x in q['qualification'])
for side in ('b','c'):
 assert q['images'][side+'0']['sha256']==p.HASHES[side]
 for k,offset in enumerate(p.OFFSETS):
  image=q['images'][side+str(k)];assert image['revision']=={'b':p.BASE,'c':p.CANDIDATE}[side] and image['offset']==offset
  if k:
   audit=image['audit'];assert audit['body_drift']==audit['address_drift']==0 and audit['declared_nops_verified']
   assert audit['section_relative_shift']==offset and audit['text_section_shift']%4096==0
   assert audit['actual_uniform_shift']==audit['text_section_shift']+offset
   if k==1:assert audit['actual_uniform_shift']==16
if kind=='cross':
 zroot=done(cfg['zone_job']);assert sha(zroot/'zones.json')==cfg['zone_sha256']
 z=read(zroot/'zones.json');assert z['build_job']==cfg['qualification_job'] and z['build_sha256']==cfg['qualification_sha256'] and z['all_outputs_match'] and len(z['cells'])==8
 assert all(x['ng_fixtures']==255 for x in z['cells'])
roles={name:meta for k in layouts for name,meta in p.roles(kind,k).items()};paths={};physical={}
for name,meta in roles.items():
 source=qualified/'binaries'/meta['image']/'qjs';assert sha(source)==q['images'][meta['image']]['sha256']
 target=out/'binaries'/name/'qjs';target.parent.mkdir(parents=True)
 if meta['image'] in physical and not meta['aa']:os.link(physical[meta['image']],target)
 else:
  shutil.copy2(source,target)
  if not meta['aa']:physical[meta['image']]=target
 paths[name]=target
os.environ['TZ']='UTC';os.environ['LC_ALL']='C'
request={'artifact_type':'t056-code-layout-diagnostic','diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'kind':kind,'base':p.BASE,'candidate':p.CANDIDATE,'cases':case_ids,'blocks':blocks,'layouts':list(layouts),'offsets':p.OFFSETS,'roles':roles,'binary_sha256':{n:sha(x) for n,x in paths.items()},'inodes':{n:x.stat().st_ino for n,x in paths.items()},'active_path':str(out/'active/qjs'),'fixed_counts':p.COUNTS,'config':cfg,'host':q['host'],'budget':{'collect':p.COLLECT_SECONDS,'body':p.BODY_SECONDS,'running_job':p.JOB_SECONDS},'budget_basis':{'reference_units':540,'reference_seconds':401.0238,'mini_units':1080,'cross_units':960,'reserve_factor':1.4,'setup_analysis_seconds':180},'caveat':'code variants have audited identical bodies modulo relocations; zone changes environment only, not source; neither proves a specific cache cause or acceptance'}
(out/'request.json').write_text(json.dumps(request,indent=2)+'\n')
work=out/'work';work.mkdir();cases=s.resolve_cases(case_ids[:-1],s.DEFAULT_CACHE,work)
empty=work/'empty.js';empty.write_text('0;\n');cases.append(dataclasses.replace(cases[-1],id='diagnostic/empty',kind='whole',workload=empty,case_id=None,expected_stdout=None))
active=out/'active/qjs';active.parent.mkdir();temporary=active.with_name('next');current={};raw=(out/'raw.jsonl').open('x');tool=s.counter_tool(trust_cycles=False)
rows={(k,c.id):[] for k in layouts for c in cases};checks={};preflights=[]
def runner(argv,timeout):
 result=run_process(argv,p.remaining(deadline,time.monotonic(),timeout))
 raw.write(json.dumps({**current,'argv':argv,'environment_overrides':roles[current['role']]['environment'],'result':dataclasses.asdict(result)})+'\n');raw.flush();return result

def activate(name):
 current['role']=name
 for key,value in roles[name]['environment'].items():
  if value is None:os.environ.pop(key,None)
  else:os.environ[key]=value
 temporary.unlink(missing_ok=True);os.link(paths[name],temporary);os.replace(temporary,active)
 current['phase']='preflight';preflights.extend(s.launch_preflight([active],120,runner=runner))

def take(case,n):return s.sample(tool,s._argv(active,case,n),120,runner=runner)
with measurement_lock('T056 code-layout '+kind+' diagnostic'):
 monitor=Monitor.from_queue()
 for block in range(blocks):
  for k in p.order(layouts,block):
   for ci,case in enumerate(p.order(cases,block,k)):
    current.update(block=block,layout=k,case=case.id);count=p.COUNTS[case.id]
    row={'block':block,'samples':{},'started_at':time.time(),'order':p.order(p.roles(kind,k),block,ci+k)}
    for name in row['order']:
     activate(name);current['phase']='measured';sizes=[('N',count),('2N',2*count)] if count is not None else [('whole',None)]
     if block%2:sizes.reverse()
     values={}
     for size,n in sizes:
      current['size']=size;value=take(case,n);check=s._check_output(case,value.stdout);key=(case.id,size)
      if key in checks:assert checks[key]==check,('stdout mismatch',case.id,name,size)
      else:checks[key]=check
      values[size]=dataclasses.asdict(value)
     row['samples'][name]=values
    row['ended_at']=time.time();rows[k,case.id].append(row)
    (out/f'layout{k}-{case.id.replace("/","--")}.json').write_text(json.dumps({'layout':k,'case':case.id,'iterations':count,'rows':rows[k,case.id]},indent=2)+'\n')
  print('BLOCK',block+1,'/',blocks,'complete, all layouts/cases/roles retained',flush=True)
 load=monitor.snapshot();(out/'load.json').write_text(json.dumps(load,indent=2)+'\n')
 assert not heavy_samples(load['text'],load['anchor']),'DISTURBED: whole diagnostic invalid; retain pairs without deletion/retry'
 report=[]
 for k in layouts:
  for case in cases:
   data=rows[k,case.id];assert len(data)==blocks
   for row in data:
    for role in row['samples']:
     for metric in ('wall_ns','instructions'):
      inc=analyze.increment(row,role,metric)
      if p.COUNTS[case.id] is not None and metric=='wall_ns':assert inc>=s.MIN_INCREMENT_NS
   report.append({'layout':k,'case':case.id,'effects':analyze.effects(data,p.comparisons(kind,k))})
 (out/'effects.json').write_text(json.dumps(report,indent=2)+'\n')
 if kind=='mini':(out/'routing.json').write_text(json.dumps(analyze.route(report),indent=2)+'\n')
 p.remaining(deadline,time.monotonic(),1)
raw.close();(out/'preflights.json').write_text(json.dumps(preflights,indent=2)+'\n')
(out/'complete.json').write_text(json.dumps({'kind':kind,'layouts':len(layouts),'cases':len(cases),'blocks':blocks,'case_role_blocks':sum(len(p.roles(kind,k))*blocks*len(cases) for k in layouts),'seconds':time.monotonic()-start,'diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'all_outputs_match':True})+'\n')
print('COMPLETE diagnostic only; no acceptance/attempt consumed',flush=True)
