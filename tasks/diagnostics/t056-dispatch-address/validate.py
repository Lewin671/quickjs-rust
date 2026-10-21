"""Exact original/probe bytes + actual 152/197 paths, fake timed collector."""
import ast,contextlib,datetime,hashlib,io,json,os,runpy,subprocess,sys,tempfile,types
from pathlib import Path
from unittest.mock import patch
from zoneinfo import ZoneInfo
ROOT=Path(__file__).resolve().parent;REPO=ROOT.parents[2];os.chdir(REPO);sys.path[:0]=[str(ROOT),str(REPO)]
import protocol as p,analyze,render,window,watch,chain_common as chain
from tools.benchmark import screen as s
from tools.benchmark.process import ProcessResult
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.layout_variant_audit import sections
from tools.benchmark.statistics import paired_block_bootstrap
read=lambda f:json.loads(Path(f).read_text());sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
for f in ROOT.glob('*.py'):ast.parse(f.read_text(),filename=str(f))
from decode import decode
from walk import Walker
cfg=read(ROOT/'config.json');queue=Path(os.environ.get('T056_DIAGNOSTIC_QUEUE',str(REPO.parents[2]/'outputs/imac-bench')))
for name,h in cfg['files'].items():assert sha(ROOT/name)==h
rawimages=queue/cfg['build_job']/'out/probe-images.json';images=read(rawimages);assert sha(rawimages)==cfg['images_sha256'];unit=read(chain.PLAN);policy=load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256'],live=False)
with patch.dict(os.environ,{'BENCH_QUEUE':str(queue)}):chain.correctness(cfg['correctness_job']);chain.pregate(cfg['pregate_job'],policy)
# Executed producer differs from current source in its clarified docstring only.
executed=queue/cfg['build_job']/'out/probe_build.py';assert sha(executed)==images['producer_sha256']
a,b=ast.parse(executed.read_text()),ast.parse((ROOT/'probe_build.py').read_text());assert ast.dump(ast.Module(body=a.body[1:],type_ignores=[]))==ast.dump(ast.Module(body=b.body[1:],type_ignores=[]))
inputs=read(ROOT/'inputs.json');assert inputs==read(queue/cfg['build_job']/'out/inputs.json')
for role,m in images['images'].items():
 binary=queue/cfg['build_job']/'out/binaries'/role/'qjs';assert sha(binary)==cfg['binary_sha256'][role]==m['binary_sha256']
 if m['kind']=='original':continue
 parent=queue/cfg['build_job']/'out/binaries'/m['source']/'qjs';data=parent.read_bytes();modified=binary.read_bytes();assert sections(parent)==sections(binary) and len(data)==len(modified);sec=next(x for x in sections(parent) if x[2]=='__TEXT/__text');cursor=0
 for patch_ in m['patches']:
  start=sec[3]+int(patch_['address'],16)-sec[0];blob=bytes.fromhex(patch_['bytes']);assert data[cursor:start]==modified[cursor:start] and modified[start:start+len(blob)]==blob;cursor=start+len(blob)
 assert data[cursor:]==modified[cursor:]
 for record in inputs['roles'][m['source']]['paths'].values():
  for row in record:
   start=sec[3]+row['pc']-sec[0];assert modified[start:start+row['length']]==data[start:start+row['length']]
comparison=read(ROOT/'comparison.json');assert comparison['global_PC_deltas_from_count']['count_flags']=={'0':96,'-6':24,'-12':20,'-3':12}
for role,cases in comparison['roles'].items():
 for case,meta in cases.items():
  record=read(ROOT/meta['loop_record']);assert record['instructions_per_iteration']==(152 if case==p.CASES[0] else 197) and record['all_six_capture_paths_equal']
  assert len(record['memory'])==(55 if case==p.CASES[0] else 72)
  for row in record['code']:
   pc=int(row['pc'],16);assert row['offset32']==pc%32 and row['offset64']==pc%64 and int(row['window32'],16)==pc//32*32 and int(row['window64'],16)==pc//64*64
rawcapture=read(queue/cfg['capture_job']/'out/captures.json');rawprobe=read(queue/cfg['probe_capture_job']/'out/captures.json');assert sha(queue/cfg['probe_capture_job']/'out/captures.json')==cfg['probe_capture_sha256']
assert len(rawcapture['snapshots'])==60 and len(rawprobe['snapshots'])==108
for stem,raw,job in [('captures',rawcapture,cfg['capture_job']),('probe-captures',rawprobe,cfg['probe_capture_job'])]:
 index=read(ROOT/(stem+'.json'));assert index['raw_job_sha256']==sha(queue/job/'out/captures.json') and index['types']==raw['types'] and index['images']==raw['images']
 for role,path in index['snapshot_files'].items():assert read(ROOT/path)['snapshots']==[r for r in raw['snapshots'] if r['role']==role]
for role in ('count','flags','count_flags','aligned','candidate'):
 groups=[r for r in rawcapture['snapshots'] if r['role']==role];assert len(groups)==12
 for row in groups:
  assert row['bank']%4096==0 and row['program']%4096==(0 if row['case']==p.CASES[0] else 0x600)
  if row is groups[0] or row is next(r for r in groups if r['case']==p.CASES[1]):
   meta=inputs['roles'][role];asm=(queue/('20261021T112221Z-t056-source-bisect-build' if role in ('aligned','candidate') else '20261021T115238Z-t056-header-subparts-build')/'out/audits'/role/'final-path.asm');binary=queue/cfg['base_job']/'out/binaries'/role/'qjs'
   actual=Walker(binary,asm,meta['entry'],meta['header'],decode(row)).run();expected=read(ROOT/comparison['roles'][role][row['case']]['loop_record']);assert actual['trace']==[{k:v for k,v in x.items() if k in ('pc','op','args','length','offset','raw_hex')} for x in expected['code']]
  if row['case']==p.CASES[0]:assert any(c['reg']==5 and bytes.fromhex(c['cell_raw_hex'])[32:40]==__import__('struct').pack('<d',1.) for c in row['received_cells'])
# All forced targets validated; sham address anomalies remain in the evidence.
for row in rawprobe['snapshots']:
 if row['role'].endswith('phase600'):assert row['ops_base']%4096==0x600
 if row['role'].endswith('phase900'):assert row['ops_base']%4096==0x900
assert {hex(r['ops_base']%4096) for r in rawprobe['snapshots'] if r['role']=='flags_sham'}=={'0x600','0x900','0x180'}
script=render.render();subprocess.run(['bash','-n'],input=script,text=True,check=True);outer=script.split("<<'T056_WINDOW'\n",1)[1].rsplit('\nT056_WINDOW',1)[0];tree=ast.parse(outer);body=next(ast.literal_eval(n.value) for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='body' for t in n.targets));subprocess.run(['bash','-n'],input=body,text=True,check=True);ast.parse(body.split("<<'FILES'\n",1)[1].split('\nFILES',1)[0]);assert p.THROUGHPUT['scaled_seconds']*3+120<p.COLLECT_SECONDS<p.BODY_SECONDS<p.JOB_SECONDS==900
stamp=datetime.datetime(2026,10,21,tzinfo=datetime.timezone.utc).timestamp();assert not watch.expired('queued',stamp+9999) and not watch.expired('running 2026-10-21T00:00:00Z',stamp+899) and watch.expired('running 2026-10-21T00:00:00Z',stamp+900)
for zone in ('Asia/Shanghai','America/Los_Angeles'):
 with patch.object(window,'HOST_ZONE',ZoneInfo(zone)):
  for day in ('2026-10-21','2026-11-01','2027-03-14'):
   t=datetime.datetime.fromisoformat(day+'T00:10').replace(tzinfo=window.HOST_ZONE).timestamp();assert window.host_local(window.deadline_for(t)+900).strftime('%H:%M')=='03:00'
   try:window.deadline_for(datetime.datetime.fromisoformat(day+'T03:00').replace(tzinfo=window.HOST_ZONE).timestamp())
   except ValueError:pass
   else:raise AssertionError('quiet window admitted')
def factors(mode):
 f={r:1. for r in p.ROLES};f.update(candidate=1.43,count_flags=1.57,count_flags_sham=1.57,count_flags_sham_aa=1.57,count_flags_phase900=1.57)
 if mode in ('causal','guard','aa'):f.update(count_phase900=1.5,flags_phase900=1.5)
 if mode=='none':f['count_flags_phase600']=1.57
 if mode=='guard':f['count_phase600']=1.2
 if mode=='aa':f['count_sham_aa']=1.2
 return f
bootstrap=lambda *a,**k:paired_block_bootstrap(*a,**{**k,'samples':100})
with patch.object(analyze,'paired_block_bootstrap',side_effect=bootstrap):
 for mode in ('causal','source','none','guard','aa'):
  f=factors(mode);data={c:[{'block':b,'samples':{r:{size:{'wall_ns':int(n*100000000*f[r]),'instructions':n*1000000} for size,n in [('N',1),('2N',2)]} for r in p.ROLES}} for b in range(5)] for c in p.CASES};report=analyze.analyze(data)
  assert report['eligible']==(mode in ('causal','source','none'))
  if mode=='causal':assert 'causal contribution' in report['verdict']
  if mode=='source':assert 'source-dependent' in report['verdict']
  if mode=='none':assert 'does not explain' in report['verdict']
 simulations={}
 for mode in ('success','wrong-output','timeout','disturbed','expired','bad-hash','owner'):
  with tempfile.TemporaryDirectory(prefix='.simulation-',dir=ROOT) as temp:
   out=Path(temp)/'out';out.mkdir();(out.parent/'status').write_text('no-timing fixture' if mode=='owner' else 'running 2026-10-21T00:00:00Z');localcfg=dict(cfg)
   if mode=='bad-hash':localcfg['images_sha256']='0'*64
   (out/'config.json').write_text(json.dumps(localcfg));calls=[]
   def fake(argv,timeout):
    assert 0<timeout<=120 and all(k not in os.environ for k in p.ENV_KEYS);calls.append(argv);cmd=argv[2:] if argv[:2]==['/usr/bin/time','-l'] else argv;role=next(r for r in p.ROLES if (out/'binaries'/r/'qjs').stat().st_ino==Path(cmd[0]).stat().st_ino)
    if cmd[-2:]==['-e','0']:n=1;stdout='0\n';duration=1000
    else:n=int(cmd[-1]);stdout=s.RESULT_PREFIX+json.dumps({'operations':n,'checksum':n})+'\n';duration=int(10*n*factors('causal')[role])+1000000
    if mode=='wrong-output' and cmd[-2]!='-e' and role=='candidate':stdout=s.RESULT_PREFIX+'{"operations":0,"checksum":0}\n'
    return ProcessResult('SIMULATED',0,duration,duration,0,mode=='timeout',stdout,str(100*n+1000000)+' instructions retired\n',False,False)
   monitor=types.SimpleNamespace(snapshot=lambda:{'text':'00:00:00 250 intruder\n' if mode=='disturbed' else '','anchor':stamp})
   with contextlib.ExitStack() as st:
    st.enter_context(patch.dict(os.environ,{'BENCH_OUT':str(out),'BENCH_QUEUE':str(queue),'T056_INITIAL_STACK_PAD':'stale'}));st.enter_context(patch('tools.benchmark.layout_identity.host_identity',return_value={'hardware':'iMac20,1','cpu':'i7-10700'}));st.enter_context(patch('tools.benchmark.layout_policy.load_policy',return_value=policy));st.enter_context(patch('tools.benchmark.measure_lock.measurement_lock',return_value=contextlib.nullcontext()));st.enter_context(patch('tools.benchmark.screen.counter_tool',return_value=s.counter_tool(system='Darwin',trust_cycles=False)));st.enter_context(patch('tools.benchmark.process.run_process',side_effect=fake));st.enter_context(patch('tools.benchmark.pair_disturbance.Monitor.from_queue',return_value=monitor))
    if mode=='expired':st.enter_context(patch.object(p,'remaining',side_effect=RuntimeError('deadline fixture')))
    try:
     with contextlib.redirect_stdout(io.StringIO()):runpy.run_path(str(ROOT/'collect.py'))
    except (AssertionError,RuntimeError,s.ScreenError):
     if mode=='success':raise
    else:assert mode=='success'
   if mode=='success':assert read(out/'complete.json')['units']==170 and len(calls)==510 and read(out/'assessment.json')['eligible']
   else:assert not (out/'complete.json').exists() and not (out/'assessment.json').exists()
   if mode in ('bad-hash','owner','expired'):assert not calls
   simulations[mode]=len(calls)
(ROOT/'validation.json').write_text(json.dumps({'diagnostic_only':True,'timing_submitted':False,'producer_comment_only_change_verified':True,'real_original_capture_rows':60,'real_probe_capture_rows':108,'all152_repeat_bytes_and_PCs_preserved_for_probes':True,'retained_flags_sham_phase_outliers':['900','180'],'mock_process_calls':simulations,'checks':['all binary/capture/receipt seals','every patch and byte outside patches','all loop PCs/32B/64B windows and data operands','causal/source-dependent/null/confounded/AA result routing','five blocks/17 roles/170 units/340 measured+170 preflight','output/timeout/disturbance/deadline/hash/owner rejection','rendered shell/embedded Python and15min/DST/quiet-window guards']},indent=2)+'\n');print('VALIDATED',simulations)
