"""Header factors, whole-loop replay and fake-collector tests; never times JS."""
import ast,contextlib,datetime,hashlib,io,json,os,runpy,subprocess,sys,tempfile,types,copy
from pathlib import Path
from unittest.mock import patch
from zoneinfo import ZoneInfo
ROOT=Path(__file__).resolve().parent;REPO=ROOT.parents[2];os.chdir(REPO);sys.path[:0]=[str(ROOT),str(REPO)]
import parts,protocol as p,analyze,render,render_build,window,watch,chain_common as chain
from walk import Walker
from evidence_io import load_images,load_mechanism
from tools.benchmark import screen as s
from tools.benchmark.process import ProcessResult
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.statistics import paired_block_bootstrap
sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
read=lambda f:json.loads(Path(f).read_text())
for f in ROOT.glob('*.py'):ast.parse(f.read_text(),filename=str(f))
manifest=read(ROOT/'parts.json');assert len(parts.ROLES)==6
for role,m in manifest['roles'].items():
 for path,h in m['files'].items():assert hashlib.sha256(parts.source(path,m['parts'])).hexdigest()==h
for path in parts.FILES:
 assert parts.source(path,())==parts.bp.blob(parts.OLD,path)
 assert parts.source(path,parts.PARTS)==parts.full(path).encode()
for shell in (render.render(),render_build.render()):
 subprocess.run(['bash','-n'],input=shell,text=True,check=True)
 if 'T056_WINDOW' in shell:
  outer=shell.split("<<'T056_WINDOW'\n",1)[1].rsplit('\nT056_WINDOW',1)[0];tree=ast.parse(outer);body=next(ast.literal_eval(n.value) for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='body' for t in n.targets));subprocess.run(['bash','-n'],input=body,text=True,check=True)
 else:body=shell
 ast.parse(body.split("<<'FILES'\n",1)[1].split('\nFILES\n',1)[0])
assert p.BLOCKS==5 and len(p.IMAGES)==9 and len(p.ROLES)==13 and len(p.CASES)==2
assert p.THROUGHPUT['scaled_seconds']*3+120<p.COLLECT_SECONDS<p.BODY_SECONDS<p.JOB_SECONDS==900
assert p.remaining(10,5,120)==5
try:p.remaining(5,5,120)
except RuntimeError:pass
else:raise AssertionError('deadline escaped')
for zone in ('Asia/Shanghai','America/Los_Angeles'):
 with patch.object(window,'HOST_ZONE',ZoneInfo(zone)):
  for date in ('2026-10-21','2026-11-01','2027-03-14'):
   now=datetime.datetime.fromisoformat(date+'T00:10').replace(tzinfo=window.HOST_ZONE).timestamp();assert window.host_local(window.deadline_for(now)+900).strftime('%H:%M')=='03:00'
   try:window.deadline_for(datetime.datetime.fromisoformat(date+'T03:30').replace(tzinfo=window.HOST_ZONE).timestamp())
   except ValueError:pass
   else:raise AssertionError('night admitted')
stamp=datetime.datetime(2026,10,21,tzinfo=datetime.timezone.utc).timestamp();assert not watch.expired('queued',stamp+9999) and not watch.expired('running 2026-10-21T00:00:00Z',stamp+899) and watch.expired('running 2026-10-21T00:00:00Z',stamp+900)
def synthetic(bad=(),aa=1.,anchor=1.43):
 factors={r:1. for r in p.ROLES};factors.update(candidate=anchor,candidate_aa=anchor*aa,full_a=1.43,full_a_aa=1.43,aligned_aa=aa)
 for r in bad:factors[r]=1.43
 return {c:[{'block':b,'samples':{r:{size:{'wall_ns':int(n*100000000*f),'instructions':n*1000000} for size,n in [('N',1),('2N',2)]} for r,f in factors.items()}} for b in range(5)] for c in p.CASES}
bootstrap=lambda *a,**k:paired_block_bootstrap(*a,**{**k,'samples':100})
with patch.object(analyze,'paired_block_bootstrap',side_effect=bootstrap):
 one=('count','count_flags','count_order');a=analyze.analyze(synthetic(one));assert a['qualified_triggers']==['count'] and a['minimal_observed_bad_subsets']==['count']
 assert analyze.analyze(synthetic(('count_flags',)))['minimal_observed_bad_subsets']==['count_flags']
 assert analyze.analyze(synthetic())['minimal_observed_bad_subsets']==['full_a']
 assert not analyze.analyze(synthetic((*one,'flags_order')))['qualified_triggers']
 assert not analyze.analyze(synthetic(one,aa=1.2))['qualified_triggers']
 assert not analyze.analyze(synthetic(one,anchor=1.1))['qualified_triggers']
 bad=synthetic();bad[p.CASES[0]].pop()
 try:analyze.analyze(bad)
 except AssertionError:pass
 else:raise AssertionError('partial block accepted')
 cfg=read(ROOT/'config.json');queue=Path(os.environ.get('T056_DIAGNOSTIC_QUEUE','/Users/qingyingliu/Code/co-work-space/outputs/imac-bench'))
 for name,h in cfg['files'].items():assert sha(ROOT/name)==h
 unit=read(chain.PLAN);policy=load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256'],live=False)
 with patch.dict(os.environ,{'BENCH_QUEUE':str(queue)}):chain.correctness(cfg['correctness_job']);chain.pregate(cfg['pregate_job'],policy)
 layouts=read(ROOT/'layouts.json');summary=read(ROOT/'layouts-summary.json');assert sha(ROOT/'layouts.json')==cfg['layouts_sha256'] and sha(queue/cfg['build_job']/'out/layouts.json')==cfg['layouts_sha256']
 assert layouts['qualifier_sha256']==sha(ROOT/'qualify.py')
 assert len({v['separate_fresh_target'] for v in layouts['roles'].values()})==8
 for role,m in layouts['roles'].items():
  assert m['size']==(112 if role in ('count_flags','full_a') else 120)
  assert any(b['type']=='value::object::ObjectData' for b in m['blocks'])
  assert any(b['type'].startswith('std::rc::RcInner<') and b['size']==summary[role]['Rc_request_bytes'] for b in m['blocks'])
 images=load_images(ROOT);assert sha(ROOT/'images.json')==cfg['images_record_sha256'] and sha(queue/cfg['build_job']/'out/images.json')==cfg['images_sha256'];assert images==read(queue/cfg['build_job']/'out/images.json')
 for role,m in images['images'].items():
  assert sha(queue/cfg['build_job']/'out/binaries'/role/'qjs')==m['binary_sha256']==cfg['binary_sha256'][role]
  if role in parts.ROLES:assert m['files_sha256']==manifest['roles'][role]['files']
  audit=images['audits'][role];eq=[]
  for name,path in audit['records']['candidate']['paths'].items():
   normal=lambda rows:[(r['normalized_hex'],[v.get('referent8') for v in r['relocations']]) for r in rows]
   equal=normal(path)==normal(audit['records']['base']['paths'][name]);eq.append(equal);assert equal==audit['comparison'][name]['equivalent_modulo_addresses']
  assert all(eq)==audit['all_equivalent']
 # Re-run all24 whole-iteration replays from real operand captures, independent
 # of the six old path lists. Also fail on unknown opcode/address/NaN input.
 mechanism=load_mechanism(ROOT);assert all(sha(ROOT/n)==h for n,h in mechanism['loop_files_sha256'].items());assert all(sha(ROOT/n)==h for n,h in mechanism['producer_sha256'].items());fp=read(ROOT.parent/'t056-pmu/fp-state.json');prior=queue/'20261021T112221Z-t056-source-bisect-build/out'
 for role,fprole in [('base','base'),('aligned','aligned'),('candidate','allocation')]:
  binary=prior/'binaries'/role/'qjs';asm=prior/'audits'/role/'final-path.asm';assert sha(asm)==mechanism['input_assembly_sha256'][role]
  for case in p.CASES:
   expected=mechanism['loops'][role][case];entry=int(expected['trace'][0]['pc'],16)-int(expected['trace'][0]['offset'],16);header=int(expected['trace'][0]['pc'],16)
   for snap in [r for r in fp['snapshots'] if r['role']==fprole and r['case']==case]:
    walker=Walker(binary,asm,entry,header,snap);observed=walker.run();assert observed['trace']==expected['trace'] and observed['instructions_per_iteration']==(152 if case==p.CASES[0] else 197) and not observed['partial_overlap_store_load_pairs']
   walker=Walker(binary,asm,entry,header,snap);walker.rows[header]['op']='unknown'
   try:walker.run()
   except AssertionError:pass
   else:raise AssertionError('unknown opcode accepted')
   walker=Walker(binary,asm,entry,header,snap);walker.reg['rax']=0
   try:walker.read('(%rax)',8,walker.rows[header])
   except AssertionError:pass
   else:raise AssertionError('unclassified memory accepted')
   nan=copy.deepcopy(snap);raw=bytearray.fromhex(nan['registers'][2]['raw_hex']);raw[8:16]=bytes.fromhex('000000000000f87f');nan['registers'][2]['raw_hex']=raw.hex()
   try:Walker(binary,asm,entry,header,nan).run()
   except AssertionError:pass
   else:raise AssertionError('NaN replay admitted')
 simulations={}
 for mode in ('trigger','wrong-output','timeout','disturbed','expired','bad-hash','owner'):
  with tempfile.TemporaryDirectory(prefix='.simulation-',dir=ROOT) as directory:
   out=Path(directory)/'out';out.mkdir();(out.parent/'status').write_text('running-no-timing fixture' if mode=='owner' else 'running 2026-10-21T00:00:00Z');localcfg=dict(cfg)
   if mode=='bad-hash':localcfg['images_sha256']='0'*64
   (out/'config.json').write_text(json.dumps(localcfg));(out/'parts.json').write_bytes((ROOT/'parts.json').read_bytes());calls=[]
   def fake_process(argv,timeout):
    assert 0<timeout<=120 and all(k not in os.environ for k in p.ENV_KEYS);calls.append(argv);cmd=argv[2:] if argv[:2]==['/usr/bin/time','-l'] else argv;role=next(r for r in p.ROLES if (out/'binaries'/r/'qjs').stat().st_ino==Path(cmd[0]).stat().st_ino)
    factor=1.43 if role in ('candidate','candidate_aa','full_a','full_a_aa','count','count_flags','count_order') else 1.
    if cmd[-2:]==['-e','0']:n=1;stdout='0\n';duration=1000
    else:n=int(cmd[-1]);stdout=s.RESULT_PREFIX+json.dumps({'operations':n,'checksum':n})+'\n';duration=int(10*n*factor)+1000000
    if mode=='wrong-output' and cmd[-2]!='-e' and role=='candidate':stdout=s.RESULT_PREFIX+'{"operations":0,"checksum":0}\n'
    return ProcessResult('SIMULATED',0,duration,duration,0,mode=='timeout',stdout,str(100*n+1000000)+' instructions retired\n',False,False)
   monitor=types.SimpleNamespace(snapshot=lambda:{'text':'00:00:00 250 intruder\n' if mode=='disturbed' else '','anchor':stamp})
   with contextlib.ExitStack() as st:
    st.enter_context(patch.dict(os.environ,{'BENCH_OUT':str(out),'BENCH_QUEUE':str(queue),'T056_INITIAL_STACK_PAD':'stale'}));st.enter_context(patch('tools.benchmark.layout_identity.host_identity',return_value={'hardware':'iMac20,1','cpu':'i7-10700'}));st.enter_context(patch('tools.benchmark.layout_policy.load_policy',return_value=policy));st.enter_context(patch('tools.benchmark.measure_lock.measurement_lock',return_value=contextlib.nullcontext()));st.enter_context(patch('tools.benchmark.screen.counter_tool',return_value=s.counter_tool(system='Darwin',trust_cycles=False)));st.enter_context(patch('tools.benchmark.process.run_process',side_effect=fake_process));st.enter_context(patch('tools.benchmark.pair_disturbance.Monitor.from_queue',return_value=monitor))
    if mode=='expired':st.enter_context(patch.object(p,'remaining',side_effect=RuntimeError('fixture deadline')))
    try:
     with contextlib.redirect_stdout(io.StringIO()):runpy.run_path(str(ROOT/'collect.py'))
    except (AssertionError,RuntimeError,s.ScreenError):
     if mode=='trigger':raise
    else:assert mode=='trigger','failure fixture completed'
   if mode=='trigger':
    complete=read(out/'complete.json');assert complete['units']==130 and complete['measured_execs']==260 and len(calls)==390;assert read(out/'assessment.json')['qualified_triggers']==['count'];assert len(set(read(out/'request.json')['inodes'].values()))==13
   else:assert not (out/'complete.json').exists() and not (out/'assessment.json').exists()
   if mode in ('owner','bad-hash','expired'):assert not calls
   simulations[mode]=len(calls)
report={'diagnostic_only':True,'hardware_profiled':False,'timing_submitted':False,'production_tree':'dfff3bb6 unchanged at 3df764e4','mock_process_calls':simulations,'whole_loop_replays':24,'global_instructions_per_iteration':152,'local_instructions_per_iteration':197,'no_architectural_ObjectData_access_or_partial_overlap_in_repeated_loop':True,'verified':['factor source hashes and old/full reconstruction','rendered shell and embedded Python','real images/receipts/source and byte audits','eight fresh independent type-layout probes','whole-path table-driven replay, memory widths and unknown-input rejection','factorial singleton/pair/triple patterns, AA/anchor/incomplete rejection','five blocks/13 roles/130 units/260 measured+130 preflight','output/timeout/disturbance/hash/deadline/owner rejection','15min/DST/quiet window']}
(ROOT/'validation.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
