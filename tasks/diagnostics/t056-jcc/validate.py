"""Real binary/geometry receipts plus simulated collector; no timing submission."""
import ast,contextlib,copy,datetime,hashlib,io,json,os,runpy,subprocess,sys,tempfile,types
from pathlib import Path
from unittest.mock import patch
from zoneinfo import ZoneInfo
ROOT=Path(__file__).resolve().parent;REPO=ROOT.parents[2];os.chdir(REPO);sys.path[:0]=[str(ROOT),str(REPO)]
import protocol as p,analyze,render,window,watch,chain_common as chain
from geometry import branches,summary
from tools.benchmark import screen as s
from tools.benchmark.process import ProcessResult
from tools.benchmark.layout_variant_audit import sections
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.statistics import paired_block_bootstrap
read=lambda f:json.loads(Path(f).read_text());sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
queue=Path(os.environ.get('T056_DIAGNOSTIC_QUEUE',str(REPO.parents[2]/'outputs/imac-bench')));cfg=read(ROOT/'config.json')
for f in ROOT.glob('*.py'):ast.parse(f.read_text(),filename=str(f))
for name,h in cfg['files'].items():assert sha(ROOT/name)==h
build=queue/cfg['build_job']/'out';audit=queue/cfg['audit_job']/'out'
assert sha(build/'images.json')==cfg['images_sha256'] and sha(audit/'audit.json')==cfg['audit_sha256']
b=read(build/'images.json');a=read(audit/'audit.json');assert not a['all_hot_boundaries_clear'] and not cfg['timing_ready']
assert b['producer_sha256']==sha(ROOT/'build.py')==sha(build/'build.py') and sha(audit/'audit.py')==sha(ROOT/'audit.py')
assert b['toolchain'].startswith('rustc 1.95.0') and b['all_outputs_match']
for role in p.SOURCES:
 pair=[build/'binaries'/n/'qjs' for n in (role,role+'_jcc')];text=[]
 for binary in pair:
  name=binary.parent.name;assert sha(binary)==cfg['binary_sha256'][name] and b['images'][name]['ng_fixtures']==255
  log=build/(name+'-ng.log');assert sha(log)==b['images'][name]['ng_log_sha256'] and sum(x.startswith('ok: ') for x in log.read_text().splitlines())==255
  sec=next(x for x in sections(binary) if x[2]=='__TEXT/__text');text.append(binary.read_bytes()[sec[3]:sec[3]+sec[1]])
 assert text[0]==text[1]
 r=a['images'][role+'_jcc'];assert r['text_sha256']==hashlib.sha256(text[1]).hexdigest() and r['text_byte_identical_to_unflagged']
 assert summary(branches(r['code']))==r['summary'] and r['instructions_per_iteration']==152 and r['nop_instructions']==0
 for row in r['code']:
  sec=next(x for x in sections(pair[1]) if x[0]<=int(row['pc'],16)<x[0]+x[1]);pos=sec[3]+int(row['pc'],16)-sec[0]
  assert pair[1].read_bytes()[pos:pos+row['length']].hex()==row['raw_hex']
 expected={'base':4,'aligned':5,'candidate':6,'count_flags':4}[role];assert r['summary']['affected_dynamic']==expected
cross=read(ROOT/'cross-geometry.json');assert len(cross['images'])==4
for name,r in cross['images'].items():
 code=read(ROOT/'audits'/('base_jcc.json' if name.startswith('b') else 'candidate_jcc.json'))['code'];assert branches(code,r['uniform_shift'])==r['branches'] and r['MallocNanoZone_cells']==['0','1']
original=read(ROOT/'original-geometry.json')
for role,m in original['roles'].items():
 for offset,record in m['offsets'].items():
  r=read(ROOT/record['record']);code=read(ROOT.parent/'t056-dispatch-address/loops'/(role+'-global_read.json'))['code']
  assert branches(code,r['actual_uniform_shift'] if r['actual_uniform_shift'] is not None else int(offset))==r['branches']
# End on32 is affected; a Jcc beginning32 is not by itself; its adjacent CMP can be.
row=lambda pc,op,n:{'pc':hex(pc),'op':op,'length':n,'args':'','offset':hex(pc)}
assert summary(branches([row(30,'je',2)]))['affected_dynamic']==1
assert summary(branches([row(32,'je',2)]))['affected_dynamic']==0
assert summary(branches([row(29,'cmpl',3),row(32,'je',2)]))['pair_only_dynamic']==1
assert summary(branches([row(28,'cmpl',3),row(32,'je',2)]))['affected_dynamic']==0
policy_unit=read(chain.PLAN);policy=load_policy(Path(policy_unit['layout_gate']['policy_path']),policy_unit['layout_gate']['policy_sha256'],live=False)
with patch.dict(os.environ,{'BENCH_QUEUE':str(queue)}):chain.correctness(cfg['correctness_job']);chain.pregate(cfg['pregate_job'],policy)
assert sha(chain.PLAN)==chain.UNIT_HASH
for kind in ('build','audit','timing'):
 script=render.render(kind);subprocess.run(['bash','-n'],input=script,text=True,check=True)
 if kind=='timing':
  tree=ast.parse(script.split("<<'T056_WINDOW'\n",1)[1].rsplit('\nT056_WINDOW',1)[0]);body=next(ast.literal_eval(n.value) for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='body' for t in n.targets))
  subprocess.run(['bash','-n'],input=body,text=True,check=True)
assert cfg['throughput']['budgeted_seconds']<p.COLLECT_SECONDS<p.BODY_SECONDS<p.JOB_SECONDS==900
stamp=datetime.datetime(2026,10,21,tzinfo=datetime.timezone.utc).timestamp()
assert not watch.expired('queued',stamp+9999) and not watch.expired('running 2026-10-21T00:00:00Z',stamp+899) and watch.expired('running 2026-10-21T00:00:00Z',stamp+900)
for zone in ('Asia/Shanghai','America/Los_Angeles'):
 with patch.object(window,'HOST_ZONE',ZoneInfo(zone)):
  for day in ('2026-10-21','2026-11-01','2027-03-14'):
   t=datetime.datetime.fromisoformat(day+'T00:10').replace(tzinfo=window.HOST_ZONE).timestamp();assert window.host_local(window.deadline_for(t)+900).strftime('%H:%M')=='03:00'
   try:window.deadline_for(datetime.datetime.fromisoformat(day+'T03:00').replace(tzinfo=window.HOST_ZONE).timestamp())
   except ValueError:pass
   else:raise AssertionError('quiet window admitted')
def factor(role,mode='suppression'):
 source=role.removesuffix('_aa');value=1.5 if source in ('candidate','count_flags') or mode=='null' and source in ('candidate_jcc','count_flags_jcc') else 1.
 return value*(1.2 if mode=='AA' and role.endswith('_aa') else 1.)
bootstrap=lambda *args,**kw:paired_block_bootstrap(*args,**{**kw,'samples':100})
simulations={}
with patch.object(analyze,'paired_block_bootstrap',side_effect=bootstrap):
 for mode in ('suppression','null','AA'):
  data={c:[{'block':block,'samples':{r:({size:{'wall_ns':int(n*100000000*factor(r,mode)),'instructions':n*1000000} for size,n in [('N',1),('2N',2)]} if p.COUNTS[c] else {'whole':{'wall_ns':int(100000000*factor(r,mode)),'instructions':1000000}}) for r in p.ROLES}} for block in range(5)] for c in p.CASES}
  result=analyze.analyze(data);assert result['eligible']==(mode!='AA') and result['suppression_threshold']==(mode!='null')
 for mode in ('success','actual-blocked','actual-audit','wrong-output','timeout','disturbed','expired','bad-hash','owner'):
  with tempfile.TemporaryDirectory(prefix='.simulation-',dir=ROOT) as temp:
   temp=Path(temp);out=temp/'out';out.mkdir();(temp/'status').write_text('no-timing fixture' if mode=='owner' else 'running 2026-10-21T00:00:00Z')
   localcfg=copy.deepcopy(cfg);fakeq=temp/'queue';fakeq.mkdir()
   for identifier in (cfg['build_job'],cfg['correctness_job'],cfg['pregate_job']):(fakeq/identifier).symlink_to(queue/identifier,target_is_directory=True)
   # ONLY a synthetic receipt lets the mock collector exercise a future qualified
   # path. It is never written to the real queue, protocol or raw audit evidence.
   root=fakeq/cfg['audit_job'];(root/'out').mkdir(parents=True);(root/'status').write_text('done 0');(root/'log').write_text('SIMULATED zero-boundary receipt, not actual image qualification')
   synthetic=copy.deepcopy(a);synthetic.update(simulation_only=True)
   if mode!='actual-audit':
    synthetic['all_hot_boundaries_clear']=True
    for v in synthetic['images'].values():v['summary']['affected_dynamic']=0
   (root/'out/audit.json').write_text(json.dumps(synthetic));localcfg['audit_sha256']=sha(root/'out/audit.json');localcfg['timing_ready']=mode!='actual-blocked'
   if mode=='bad-hash':localcfg['images_sha256']='0'*64
   (out/'config.json').write_text(json.dumps(localcfg));calls=[]
   def fake(argv,timeout):
    assert 0<timeout<=120 and all(k not in os.environ for k in p.ENV_KEYS);calls.append(argv);cmd=argv[2:] if argv[:2]==['/usr/bin/time','-l'] else argv
    role=next(r for r in p.ROLES if (out/'binaries'/r/'qjs').stat().st_ino==Path(cmd[0]).stat().st_ino)
    if cmd[-2:]==['-e','0']:n=1;stdout='0\n';duration=1000
    else:
     n=1 if len(cmd)==3 else 1 if int(cmd[-1]) in p.COUNTS.values() else 2
     stdout='0\n' if len(cmd)==3 and cmd[-1].endswith('empty.js') else b['images']['base']['outputs']['external/sunspider-1.0/string-unpack-code:None'] if len(cmd)==3 else s.RESULT_PREFIX+json.dumps({'operations':int(cmd[-1]),'checksum':int(cmd[-1])})+'\n'
     duration=int(n*100000000*factor(role))+1000000
     if mode=='wrong-output' and role=='candidate':stdout='WRONG\n'
    return ProcessResult('SIMULATED',0,duration,duration,0,mode=='timeout',stdout,str(n*1000000+1000000)+' instructions retired\n',False,False)
   monitor=types.SimpleNamespace(snapshot=lambda:{'text':'00:00:00 250 intruder\n' if mode=='disturbed' else '','anchor':stamp})
   with contextlib.ExitStack() as st:
    st.enter_context(patch.dict(os.environ,{'BENCH_OUT':str(out),'BENCH_QUEUE':str(fakeq),'T056_INITIAL_STACK_PAD':'stale'}));st.enter_context(patch('tools.benchmark.layout_identity.host_identity',return_value={'hardware':'iMac20,1','cpu':'i7-10700'}));st.enter_context(patch('tools.benchmark.layout_policy.load_policy',return_value=policy));st.enter_context(patch('tools.benchmark.measure_lock.measurement_lock',return_value=contextlib.nullcontext()));st.enter_context(patch('tools.benchmark.screen.counter_tool',return_value=s.counter_tool(system='Darwin',trust_cycles=False)));st.enter_context(patch('tools.benchmark.process.run_process',side_effect=fake));st.enter_context(patch('tools.benchmark.pair_disturbance.Monitor.from_queue',return_value=monitor))
    if mode=='expired':st.enter_context(patch.object(p,'remaining',side_effect=RuntimeError('deadline fixture')))
    try:
     with contextlib.redirect_stdout(io.StringIO()):runpy.run_path(str(ROOT/'collect.py'))
    except (AssertionError,RuntimeError,s.ScreenError):
     if mode=='success':raise
    else:assert mode=='success'
   if mode=='success':assert read(out/'complete.json')['units']==360 and len(calls)==960 and read(out/'assessment.json')['eligible']
   else:assert not (out/'complete.json').exists() and not (out/'assessment.json').exists()
   if mode in ('actual-blocked','actual-audit','bad-hash','owner','expired'):assert not calls
   simulations[mode]=len(calls)
(ROOT/'validation.json').write_text(json.dumps({'diagnostic_only':True,'timing_submitted':False,'actual_qualification':'blocked, four __text pairs identical, boundary counts4/5/6/4','original_geometry_records':30,'real_NG_checks':8*255,'real_all8_workload_checks':True,'real_receipts_and_binary_hashes':True,'mock_calls':simulations,'success_fixture':'synthetic zero-boundary receipt in isolated temporary queue; never actual qualification','checks':['raw bytes/PC/geometry of four flagged 152 paths','all30 original K6 geometry records; crossing/end/adjacency edge cases','matched recipe ratios/interaction/AA/suppression/null routing','actual blocked config stops before any process','full360units/600measured+360preflight simulated','hash/output/timeout/load/deadline/owner failures','shell render/15min watchdog/DST/quiet-window']},indent=2)+'\n')
print('VALIDATED; real timing remains blocked',simulations)
