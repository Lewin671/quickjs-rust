"""Evidence/guard tests and real collector over fake processes. Never times JS."""
import ast,contextlib,datetime,hashlib,io,json,os,runpy,subprocess,sys,tempfile,types
from pathlib import Path
from unittest.mock import patch
from zoneinfo import ZoneInfo
ROOT=Path(__file__).resolve().parent;REPO=ROOT.parents[2];os.chdir(REPO);sys.path[:0]=[str(ROOT),str(REPO)]
import partition,protocol as p,analyze,render,render_build,window,watch,chain_common as chain
from tools.benchmark import screen as s
from tools.benchmark.process import ProcessResult
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.statistics import paired_block_bootstrap
sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
read=lambda f:json.loads(Path(f).read_text())
for file in ROOT.glob('*.py'):ast.parse(file.read_text(),filename=str(file))
inventory=partition.inventory();assert inventory==read(ROOT/'hunks.json')['files'];groups=partition.verify(inventory)
assert {g:len(h) for g,h in groups.items()}=={'a':22,'b':6,'c':2,'d':11}
assert len(inventory)==21 and sum(r['release_input'] for r in inventory)==9
try:partition.classify('crates/unexpected.rs',[])
except ValueError:pass
else:raise AssertionError('new code escaped hunk inventory')
for shell in (render.render(),render_build.render()):
 subprocess.run(['bash','-n'],input=shell,text=True,check=True)
 if 'T056_WINDOW' in shell:
  outer=shell.split("<<'T056_WINDOW'\n",1)[1].rsplit('\nT056_WINDOW',1)[0];tree=ast.parse(outer)
  body=next(ast.literal_eval(n.value) for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='body' for t in n.targets))
  subprocess.run(['bash','-n'],input=body,text=True,check=True)
 else:body=shell
 ast.parse(body.split("<<'FILES'\n",1)[1].split('\nFILES\n',1)[0])
assert p.BLOCKS==5 and len(p.IMAGES)==10 and len(p.ROLES)==13
assert p.THROUGHPUT['scaled_seconds']*3+120<p.COLLECT_SECONDS<p.BODY_SECONDS<p.JOB_SECONDS==1500
assert p.remaining(10,5,120)==5
try:p.remaining(5,5,120)
except RuntimeError:pass
else:raise AssertionError('deadline escaped')
for zone in ('Asia/Shanghai','America/Los_Angeles'):
 with patch.object(window,'HOST_ZONE',ZoneInfo(zone)):
  for date in ('2026-10-21','2026-11-01','2027-03-14'):
   now=datetime.datetime.fromisoformat(date+'T00:10').replace(tzinfo=window.HOST_ZONE).timestamp()
   assert window.host_local(window.deadline_for(now)+900).strftime('%H:%M')=='03:00'
   try:window.deadline_for(datetime.datetime.fromisoformat(date+'T03:30').replace(tzinfo=window.HOST_ZONE).timestamp())
   except ValueError:pass
   else:raise AssertionError('night admitted')
stamp=datetime.datetime(2026,10,21,tzinfo=datetime.timezone.utc).timestamp()
assert not watch.expired('queued',stamp+9999) and not watch.expired('running 2026-10-21T00:00:00Z',stamp+1499) and watch.expired('running 2026-10-21T00:00:00Z',stamp+1500)
# Fast deterministic statistical fixtures; final collector still uses 10,000 draws.
def synthetic(trigger=(),oneway=False,aa=1.,anchor=1.43):
 factors={r:1. for r in p.ROLES};factors.update(candidate=anchor,candidate_aa=anchor*aa,aligned_aa=aa)
 for g in 'abcd':factors['minus_'+g]=1.43
 for g in trigger:factors['plus_'+g]=1.43;factors['minus_'+g]=1.43 if oneway else 1.
 rows={}
 for case in p.CASES:
  rows[case]=[]
  for block in range(p.BLOCKS):
   samples={r:{'whole':{'wall_ns':int(100000000*f),'instructions':1000000}} for r,f in factors.items()}
   rows[case].append({'block':block,'samples':samples})
 return rows
bootstrap=lambda *a,**k:paired_block_bootstrap(*a,**{**k,'samples':100})
with patch.object(analyze,'paired_block_bootstrap',side_effect=bootstrap):
 assert analyze.analyze(synthetic())['qualified_triggers']==[]
 assert analyze.analyze(synthetic(('a',)))['qualified_triggers']==['a']
 assert analyze.analyze(synthetic(('b',),oneway=True))['qualified_triggers']==[]
 assert analyze.analyze(synthetic(('a','d')))['qualified_triggers']==['a','d']
 aa=analyze.analyze(synthetic(('c',),aa=1.2));assert aa['threshold_matches']==['c'] and not aa['qualified_triggers']
 assert not analyze.analyze(synthetic(('c',),anchor=1.1))['qualified_triggers']
 bad=synthetic();bad[p.CASES[0]].pop()
 try:analyze.analyze(bad)
 except AssertionError:pass
 else:raise AssertionError('partial blocks admitted')
 cfg=read(ROOT/'config.json');queue=Path(os.environ.get('T056_DIAGNOSTIC_QUEUE','/Users/qingyingliu/Code/co-work-space/outputs/imac-bench'))
 for name,h in cfg['files'].items():assert sha(ROOT/name)==h
 unit=read(chain.PLAN);policy=load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256'],live=False)
 with patch.dict(os.environ,{'BENCH_QUEUE':str(queue)}):chain.correctness(cfg['correctness_job']);chain.pregate(cfg['pregate_job'],policy)
 images=read(ROOT/'images.json');assert sha(ROOT/'images.json')==cfg['images_sha256'] and sha(queue/cfg['build_job']/'out/images.json')==cfg['images_sha256']
 for role,m in images['images'].items():
  assert sha(queue/cfg['build_job']/'out/binaries'/role/'qjs')==m['binary_sha256']==cfg['binary_sha256'][role]
  if role in partition.ROLES:
   assert m['selected_groups']==partition.ROLES[role]
   assert m['files_sha256']=={r['path']:partition.digest(partition.materialize(r,partition.ROLES[role])) for r in inventory if r['release_input']}
  a=images['audits'][role]
  assert any(v.get('referent8')=='000000000000f03f' for row in a['candidate_paths']['Update'] for v in row['relocations']), 'not the Increment path'
  # Fail visibly if an audit claim disagrees with its actual bytes/constants.
  baseline=images['audits']['aligned']['candidate_paths']
  eq=True
  for name,path in a['candidate_paths'].items():
   norm=lambda rows:[(r['normalized_hex'],[v.get('referent8') for v in r['relocations']]) for r in rows]
   equal=norm(path)==norm(baseline[name]);assert equal==a['comparison'][name]['equivalent_modulo_addresses'];eq=eq and equal
  assert eq==a['all_equivalent']
 # Exercise all five blocks and thirteen real copied image files; no execution.
 simulations={}
 for mode in ('trigger','wrong-output','timeout','disturbed','expired','bad-hash','owner'):
  with tempfile.TemporaryDirectory(prefix='.simulation-',dir=ROOT) as directory:
   out=Path(directory)/'out';out.mkdir();(out.parent/'status').write_text('running-no-timing fixture' if mode=='owner' else 'running 2026-10-21T00:00:00Z')
   localcfg=dict(cfg)
   if mode=='bad-hash':localcfg['images_sha256']='0'*64
   (out/'config.json').write_text(json.dumps(localcfg));(out/'hunks.json').write_bytes((ROOT/'hunks.json').read_bytes());calls=[]
   def fake_process(argv,timeout):
    assert 0<timeout<=120 and all(k not in os.environ for k in p.ENV_KEYS)
    calls.append(argv);cmd=argv[2:] if argv[:2]==['/usr/bin/time','-l'] else argv
    role=next(r for r in p.ROLES if (out/'binaries'/r/'qjs').stat().st_ino==Path(cmd[0]).stat().st_ino)
    factor=1.43 if role in ('candidate','candidate_aa','plus_a','minus_b','minus_c','minus_d') else 1.
    if cmd[-2:]==['-e','0']:n=1;stdout='0\n';duration=1000
    elif len(cmd)>3:
     n=int(cmd[-1]);stdout=s.RESULT_PREFIX+json.dumps({'operations':n,'checksum':n})+'\n';duration=int(10*n*factor)+1000000
    else:n=10000;stdout='0\n';duration=int(1000000*factor)
    if mode=='wrong-output' and len(cmd)>3 and cmd[-2]!='-e':stdout=s.RESULT_PREFIX+'{"operations":0,"checksum":0}\n' if role=='candidate' else stdout
    return ProcessResult('SIMULATED',0,duration,duration,0,mode=='timeout',stdout,str(100*n+1000000)+' instructions retired\n',False,False)
   monitor=types.SimpleNamespace(snapshot=lambda:{'text':'00:00:00 250 intruder\n' if mode=='disturbed' else '','anchor':stamp})
   with contextlib.ExitStack() as st:
    st.enter_context(patch.dict(os.environ,{'BENCH_OUT':str(out),'BENCH_QUEUE':str(queue),'T056_INITIAL_STACK_PAD':'stale'}))
    st.enter_context(patch('tools.benchmark.layout_identity.host_identity',return_value={'hardware':'iMac20,1','cpu':'i7-10700'}))
    st.enter_context(patch('tools.benchmark.layout_policy.load_policy',return_value=policy))
    st.enter_context(patch('tools.benchmark.measure_lock.measurement_lock',return_value=contextlib.nullcontext()))
    st.enter_context(patch('tools.benchmark.screen.counter_tool',return_value=s.counter_tool(system='Darwin',trust_cycles=False)))
    st.enter_context(patch('tools.benchmark.process.run_process',side_effect=fake_process))
    st.enter_context(patch('tools.benchmark.pair_disturbance.Monitor.from_queue',return_value=monitor))
    if mode=='expired':st.enter_context(patch.object(p,'remaining',side_effect=RuntimeError('fixture deadline')))
    try:
     with contextlib.redirect_stdout(io.StringIO()):runpy.run_path(str(ROOT/'collect.py'))
    except (AssertionError,RuntimeError,s.ScreenError):
     if mode=='trigger':raise
    else:assert mode=='trigger','failure fixture completed'
   if mode=='trigger':
    complete=read(out/'complete.json');assert complete['units']==195 and complete['measured_execs']==325
    assert read(out/'assessment.json')['qualified_triggers']==['a'] and len(calls)==520
    req=read(out/'request.json');assert len(set(req['inodes'].values()))==13
    assert len({r[2] for r in calls if r[:2]==['/usr/bin/time','-l']})==1
    for case in p.CASES:
     rows=read(out/(case.replace('/','--')+'.json'))['rows'];assert len(rows)==5 and all(set(x['samples'])==set(p.ROLES) for x in rows)
   else:assert not (out/'complete.json').exists() and not (out/'assessment.json').exists()
   if mode in ('owner','bad-hash','expired'):assert not calls
   simulations[mode]=len(calls)
report={'diagnostic_only':True,'hardware_profiled':False,'timing_submitted':False,'production_tree':'dfff3bb6 unchanged at 802aa51a','hunks':{k:len(v) for k,v in groups.items()},'mock_process_calls':simulations,'verified':['all source hunk reconstruction and partition completeness','exact images/receipts/source hashes','normalized bytes and numeric constants','lead-only guard','five blocks/13 roles/195 units/325 measured+195 preflight','positive and no/one-way/multiple trigger','AA/anchor/incomplete rejection','output/timeout/disturbance/hash/deadline rejection','environment/inodes/one active path','25min/DST/quiet window','rendered shell and embedded Python'],'actual_byte_audit_equivalence':{r:a['all_equivalent'] for r,a in images['audits'].items()}}
(ROOT/'validation.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
