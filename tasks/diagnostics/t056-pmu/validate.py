"""Offline evidence checks + fake xctrace orchestration; never profiles an engine."""
import ast,contextlib,datetime,hashlib,json,os,runpy,shutil,signal,struct,subprocess,sys,tempfile,zoneinfo
from pathlib import Path
from unittest.mock import patch
ROOT=Path(__file__).resolve().parent
REPO=ROOT.parents[2]
sys.path[:0]=[str(ROOT),str(REPO)]
import protocol as p,xctrace_backend as backend,chain_common as chain,render,window,watch
from tools.benchmark import screen,layout_policy,performance_decision,layout_identity,measure_lock

def read(f):return json.loads(Path(f).read_text())
def sha(f):return hashlib.sha256(Path(f).read_bytes()).hexdigest()
for file in ROOT.glob('*.py'):ast.parse(file.read_text(),filename=str(file))
config=read(ROOT/'config.json')
for name,value in config['files'].items():assert sha(ROOT/name)==value
assert sha(ROOT/'event-plan.json')==config['event_plan_sha256']
unit,h=performance_decision.load_unit(chain.PLAN);assert h==chain.UNIT_HASH
policy=layout_policy.load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256'],live=False)
# Actual sealed receipts, not mock correctness claims.
queue=Path(os.environ.get('T056_ARTIFACT_ROOT',str(REPO.parents[2]/'outputs/imac-bench')))
with patch.dict(os.environ,{'BENCH_QUEUE':str(queue)}):
 chain.correctness(config['correctness_job']);chain.pregate(config['pregate_job'],policy)
assert not subprocess.run(['git','diff','--exit-code',p.CANDIDATE,'HEAD','--','crates','tools','scripts','.cargo','Cargo.toml','Cargo.lock'],cwd=REPO,check=True).returncode

fp=read(ROOT/'fp-state.json');summary=read(ROOT/'fp-summary.json');assert len(fp['snapshots'])==24
seen=set()
for row in fp['snapshots']:
 role='candidate' if row['role']=='allocation' else row['role'];point=row['checkpoint'];seen.add((role,row['case'],point))
 assert row['backedge_iterations']==point
 expected={2:point+1,3:p.COUNT,4:(point+1)*(1 if row['case']==p.CASES[0] else 3),5:1}
 for reg,value in expected.items():
  r=row['registers'][reg];bits=struct.unpack('>Q',struct.pack('>d',value))[0]
  assert r['tag']==0 and r['variant']=='Number' and r['payload_bits']==f'{bits:016x}' and r['fp_class']=='normal'
 assert row['mxcsr']=='00001fa0' and not (row['denormal_flag'] or row['ftz'] or row['daz'])
 assert [{k:v for k,v in x.items() if k!='raw_hex'} for x in row['ops']]==summary['semantic_ops'][row['case']]
assert seen=={(r,c,n) for r in ('base','aligned','candidate') for c in p.CASES for n in (0,1,1024,p.COUNT//2)}
for image in fp['images']:assert image['headers_and_other_file_bytes_preserved'] and len(image['changed_ranges'])==2
assert len(fp['full_runs'])==6 and fp['full_runs']==summary['full_runs']
for row in fp['full_runs']:
 assert row['entry_count']==1 and row['guard_deopts']==0 and len(row['outcome_lines'])==1 and row['outcome_lines'][0].endswith(' Ran')
 role='candidate' if row['role']=='allocation' else row['role'];assert row['canonical_sha256']==p.HASHES[role]
 assert summary['canonical_executors'][row['role']]['sha256']==p.HASHES[role]
assert len({v['symbol'] for v in summary['canonical_executors'].values()})==1
for role,job,side in [('base',config['canonical_job'],'base000'),('candidate',config['canonical_job'],'cand000'),('aligned',config['aligned_job'],'cand000')]:
 assert sha(queue/job/'out/triage'/side/'qjs')==p.HASHES[role]
assert read(ROOT/'getters.json')['kpc_get_thread_counters']=={'return':-1,'errno':1}
assert read(ROOT/'access-extra.json')['rusage_cycles_trusted_by_existing_repo_probe'] is False
stack=read(ROOT/'stack-result.json');assert stack['complete']['phases']==30 and not stack['assessment_unchanged']['primary_eligible']
assert stack['results'][p.CASES[0]]['candidate_AA_failed_pads']==[256,2560,2928,3072,3584,3904,3936]

shell=render.render();subprocess.run(['bash','-n'],input=shell,text=True,check=True)
outer=shell.split("<<'T056_WINDOW'\n",1)[1].rsplit('\nT056_WINDOW\n',1)[0];tree=ast.parse(outer)
body=next(ast.literal_eval(n.value) for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='body' for t in n.targets))
subprocess.run(['bash','-n'],input=body,text=True,check=True)
ast.parse(body.split("<<'FILES'\n",1)[1].split('\nFILES\n',1)[0])
assert p.COLLECT_SECONDS<p.BODY_SECONDS<p.JOB_SECONDS<=1200
assert 32*p.BUNDLE_SECONDS+120==p.COLLECT_SECONDS
now=datetime.datetime(2026,10,21,12,tzinfo=datetime.timezone.utc).timestamp()
assert not watch.expired('queued',now) and not watch.expired('done 0',now)
assert watch.expired('running '+datetime.datetime.fromtimestamp(now-1200,datetime.timezone.utc).isoformat().replace('+00:00','Z'),now)
with patch.object(window,'HOST_ZONE',zoneinfo.ZoneInfo('America/Los_Angeles')):
 for date in ((2026,3,8),(2026,11,1)):
  t=datetime.datetime(*date,1,0,tzinfo=window.HOST_ZONE).timestamp();d=window.deadline_for(t)
  assert d>t and window.host_local(d+900).hour==3
 t=datetime.datetime(2026,10,21,3,30,tzinfo=window.HOST_ZONE).timestamp()
 try:window.deadline_for(t)
 except ValueError:pass
 else:raise AssertionError('quiet window admitted')

# Only the collector's hardware/host prerequisites and subprocess boundary are
# mocked. Real hash checks, copy/inode checks, iteration/order, XML parse,
# request/captures files, checksums and failure handling run unchanged.
real_popen=subprocess.Popen
modes=('success','blocked','no-schema','empty-table','timeout','wrong-checksum')
counts={}
with tempfile.TemporaryDirectory(prefix='t056-pmu-validation-') as directory:
 tmp=Path(directory);tool=tmp/'xctrace';tool.write_text('fake executable, never launched');tool.chmod(0o755)
 fakequeue=tmp/'queue';hashes={};roles={'base':'base000','candidate':'cand000','aligned':'cand000'}
 for job in (config['canonical_job'],config['aligned_job']):
  root=fakequeue/job;(root/'out/triage').mkdir(parents=True);(root/'status').write_text('done 0');(root/'log').write_text('fixture')
 for role,side in roles.items():
  root=fakequeue/(config['aligned_job'] if role=='aligned' else config['canonical_job'])/'out/triage'/side;root.mkdir()
  file=root/'qjs';file.write_text(role);hashes[role]=sha(file)
 def validate_mode(mode,legacy=False):
  out=tmp/(mode+('-legacy' if legacy else ''));out.mkdir();(out.parent/'status').write_text('running 2026-10-21T12:00:00Z')
  for name in ('config.json','event-plan.json'):shutil.copy2(ROOT/name,out/name)
  recorded=[];calls=[]
  class Fake:
   def __init__(self,argv,**kw):
    self.argv=list(map(str,argv));self.returncode=0;self.pid=987654;self.stdout='';self.stderr='';calls.append(self.argv)
    a=self.argv
    if a[0]==str(tool):
     if a[1:]==['help','record']:self.stdout='--no-prompt --target-stdout'
     elif a[1:]==['help','export']:self.stdout='--input' if legacy else 'export <trace>'
     elif a[1:]==['list','templates']:self.stdout='CPU Counters'
     elif a[1]=='record':
      target=Path(a[a.index('--target-stdout')+1]);n=int(a[-1]);case=a[-2];payload={'case_id':case,'iterations':n,'operations':n*(2 if case=='local_read' else 1),'checksum':n*(3 if case=='local_read' else 1)}
      target.write_text('bad checksum' if mode=='wrong-checksum' else 'QJS_BENCH_RESULT '+json.dumps(payload)+'\n');Path(a[a.index('--output')+1]).mkdir();recorded.append(a)
     elif a[1]=='export':
      target=Path(a[a.index('--output')+1]);target.write_text('<trace-toc><run number="1"><data><table schema="'+('time-profile' if mode=='no-schema' else 'cpu-counter')+'"/></data></run></trace-toc>' if '--toc' in a else '<trace-query-result>'+('' if mode=='empty-table' else '<row><value>fake</value></row>')+'</trace-query-result>')
     else:raise AssertionError(a)
    else:
     assert a[-3:]==['--raw','-e','0'];self.stdout='0\n'
   def communicate(self,timeout=None):
    if mode=='timeout' and 'record' in self.argv and self.returncode==0:raise subprocess.TimeoutExpired(self.argv,timeout)
    return self.stdout,self.stderr
   def poll(self):return self.returncode
  def popen(argv,**kw):return real_popen(argv,**kw) if argv[0]=='git' else Fake(argv,**kw)
  env={'BENCH_OUT':str(out),'BENCH_QUEUE':str(fakequeue),'T056_PMU_TEMPLATE':'CPU Counters'}
  with contextlib.ExitStack() as st:
   st.enter_context(patch.dict(os.environ,env));st.enter_context(patch.object(p,'HASHES',hashes))
   st.enter_context(patch.object(backend,'tool_path',side_effect=RuntimeError('fixture blocked')) if mode=='blocked' else patch.object(backend,'tool_path',return_value=str(tool)))
   st.enter_context(patch.object(chain,'correctness',return_value=None));st.enter_context(patch.object(chain,'pregate',return_value=None))
   st.enter_context(patch.object(layout_policy,'load_policy',return_value=policy));st.enter_context(patch.object(layout_identity,'host_identity',return_value={'hardware':'iMac20,1','cpu':'i7-10700'}))
   st.enter_context(patch.object(measure_lock,'measurement_lock',return_value=contextlib.nullcontext()))
   st.enter_context(patch.object(subprocess,'Popen',side_effect=popen));st.enter_context(patch.object(backend,'stop_process',side_effect=lambda proc:setattr(proc,'returncode',-signal.SIGTERM)))
   try:
    with contextlib.redirect_stdout(open(os.devnull,'w')):runpy.run_path(str(ROOT/'collect.py'))
   except (AssertionError,RuntimeError,screen.ScreenError):
    if mode=='success':raise
   else:assert mode=='success','failure fixture unexpectedly succeeded'
  if mode=='success':
   complete=read(out/'complete.json');assert complete['captures']==32 and not complete['hardware_attribution_validated']
   packets=read(out/'captures.json');assert len(packets)==32 and len(recorded)==32
   assert {(x['role'],x['case'],x['size'],x['block']) for x in packets}=={(r,c,n,b) for r in p.ROLES for c in p.CASES for n in ('N','2N') for b in range(2)}
   request=read(out/'request.json');assert len(set(request['inodes'].values()))==4 and not request['decision_evidence']
   assert len({a[a.index('--')+1] for a in recorded})==1
  else:assert not (out/'complete.json').exists()
  if mode=='blocked':assert read(out/'blocked.json')['pmu_samples']==0 and not recorded and not calls # no tool/engine launch
  if mode=='timeout':assert any(x.get('timed_out') for x in read(out/'commands.json'))
  return len(recorded)
 # An owner no-timing submission must fail before any prerequisite/tool call.
 owner=tmp/'owner';owner.mkdir();(owner.parent/'status').write_text('running-no-timing fixture')
 with patch.dict(os.environ,{'BENCH_OUT':str(owner)}),patch.object(backend,'tool_path') as probe:
  try:runpy.run_path(str(ROOT/'collect.py'))
  except AssertionError:pass
  else:raise AssertionError('owner no-timing launch admitted')
  assert not probe.called
 for mode in modes:counts[mode]=validate_mode(mode)
 counts['success-legacy']=validate_mode('success',legacy=True)
 # Unsafe/unrelated schema names cannot form exported XPath.
 toc=tmp/'schema.xml';toc.write_text('<trace-toc><table schema="cpu-counter&quot;bad"/><table schema="time-profile"/></trace-toc>');assert backend.schemas(toc)==[]
 # Exercise the real escalation with a fake owned pid; never signal a process.
 class Stuck:
  pid=987654
  def poll(self):return None
  def wait(self,timeout):raise subprocess.TimeoutExpired('fixture',timeout)
 with patch.object(os,'killpg') as kill:backend.stop_process(Stuck());assert [x.args[1] for x in kill.call_args_list]==[signal.SIGINT,signal.SIGTERM,signal.SIGKILL]
report={'diagnostic_only':True,'hardware_profiled':False,'pmu_samples':0,'mock_capture_counts':counts,'verified':['source seals','frozen correctness/86-case receipts','three actual canonical file hashes','24 scalar snapshots/6 logical traces','stack assessment preserved','rendered shell/embedded Python','current/legacy xctrace export','32-trace real collector control flow','owner no-timing/blocked/absent-schema/empty-table/timeout/checksum rejection','unsafe schema rejection','owned process signal escalation','20min/quiet-window/DST guards'],'production_tree':'dfff3bb6 (unchanged at 2ec4b673)'}
(ROOT/'validation.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
