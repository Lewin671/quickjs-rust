"""Real nonroot tool receipts, encoding groups, and fake collector; never sudo."""
import ast,contextlib,copy,datetime,hashlib,io,json,os,runpy,stat,subprocess,sys,tempfile,types
from pathlib import Path
from unittest.mock import patch
ROOT=Path(__file__).resolve().parent;REPO=ROOT.parents[3];os.chdir(REPO);sys.path[:0]=[str(ROOT),str(REPO)]
import protocol as p,counters,render,chain_common as chain
from tools.benchmark import screen as s
from tools.benchmark.process import ProcessResult
from tools.benchmark.layout_policy import load_policy
read=lambda f:json.loads(Path(f).read_text());sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
queue=REPO.parents[2]/'outputs/imac-bench';cfg=read(ROOT/'config-v3.json');source=queue/cfg['qualification_job']/'out';report=read(source/'qualification.json');groups=read(ROOT/'groups-v3.json')['groups']
assert not report['privileged_run'] and not report['install_performed'] and not report['hardware_counter_evidence']
assert sha(source/'qjs-kpc')==cfg['helper_sha256'] and sha(source/'qualification.json')==cfg['qualification_sha256']
for name,h in report['source_sha256'].items():
 if name.endswith(('.c','.h')) or name=='qualify.py':assert sha(ROOT/('groups-v3.json' if name=='groups.json' else name))==h
for name,h in cfg['files'].items():assert sha(ROOT/('groups-v3.json' if name=='groups.json' else name))==h
assert all(v['status']==0 and not v['stderr'] for v in report['checks']['static'].values())
assert report['checks']['unit']['status']==report['checks']['drop-unit']['status']==0
assert len(groups)==11 and sum(len(g['events']) for g in groups)==21
inventory=read(ROOT/'event-inventory.json');assert sum(not r['supported'] for r in inventory['events'])==18
for g in groups:
 assert len(g['events'])<=2 and g['description']['counter_count']==4 and g['description']['classes']==2
 for e in g['description']['events']:
  if e['name'] in ('CPU_CLK_UNHALTED.THREAD','INST_RETIRED.ANY'):continue
  rule=next(r['intel'] for r in inventory['events'] if r['name']==e['name']);value=int(g['description']['config'][e['index']],0)
  assert value&0xffff==int(rule['EventCode'],0)|(int(rule['UMask'],0)<<8)
  assert value>>24==int(rule['CounterMask']) and bool(value&(1<<18))==bool(int(rule['EdgeDetect']))
  assert value&(1<<16) and not value&((1<<17)|(1<<20)|(1<<21))
for f in ROOT.glob('*.py'):ast.parse(f.read_text(),filename=str(f))
script=render.render();subprocess.run(['bash','-n'],input=script,text=True,check=True);tree=ast.parse(script.split("<<'T056_WINDOW'\n",1)[1].rsplit('\nT056_WINDOW',1)[0]);body=next(ast.literal_eval(n.value) for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='body' for t in n.targets));subprocess.run(['bash','-n'],input=body,text=True,check=True)
subprocess.run(['sh','-n',str(ROOT/'install-user.sh')],check=True)
assert cfg['throughput']['budgeted_seconds']<p.COLLECT_SECONDS<840<890<p.JOB_SECONDS==900
# Root exec boundary and forbidden shell APIs are checked in addition to clang analysis.
code=(ROOT/'process.c').read_text();assert code.index('drop_identity(uid,gid)')<code.index('execve(argv[0]') and 'close_extra()' in code
assert all(not arg.endswith('/aggregate.c') for arg in report['build']['argv'])
for name in ('main.c','events.c','count.c','process.c','json.c','security.c'):
 assert not any(token in (ROOT/name).read_text() for token in ('KERN_KDEBUG','kperf_sample_', 'kperf_timer_', 'kperf_action_', 'kperf_reset('))
for f in ROOT.glob('*.c'):
 if f.name not in ('unit.c','drop-unit.c'):assert not any(n in f.read_text() for n in ('system(', 'popen(', 'posix_spawn(', 'execvp(', 'execvpe('))
policy_unit=read(chain.PLAN);policy=load_policy(Path(policy_unit['layout_gate']['policy_path']),policy_unit['layout_gate']['policy_sha256'],live=False)
with patch.dict(os.environ,{'BENCH_QUEUE':str(queue)}):chain.correctness(cfg['correctness_job']);chain.pregate(cfg['pregate_job'],policy)
assert sha(chain.PLAN)==chain.UNIT_HASH
simulations={}
for mode in ('success','uninstalled','permission','wrong-uid','bad-output','bad-counters','bad-idle','bad-percpu','scope','expired','timeout','disturbed'):
 with tempfile.TemporaryDirectory(prefix='.simulation-',dir=ROOT) as temp:
  out=Path(temp)/'out';out.mkdir();(out.parent/'status').write_text('running 2026-10-21T00:00:00Z');(out/'config.json').write_text(json.dumps(cfg))
  for name in cfg['files']:(out/name).write_bytes((ROOT/('groups-v3.json' if name=='groups.json' else name)).read_bytes())
  helper=source/'qjs-kpc' if mode!='uninstalled' else Path(temp)/'missing';realstat=Path.stat;calls=[]
  def patched_stat(self,*args,**kw):
   v=realstat(self,*args,**kw)
   if self==helper:
    fields=list(v);fields[4]=0;fields[0]=stat.S_IFREG|0o755;return os.stat_result(fields)
   return v
  def fake(argv,timeout):
   assert 0<timeout<=26;calls.append(argv)
   if argv[0]!='/usr/bin/sudo':
    n=int(argv[-1]);return ProcessResult(started_at='SIMULATED',timer_started_ns=0,timer_finished_ns=100000000,duration_ns=100000000,exit_code=0,timed_out=False,stdout=s.RESULT_PREFIX+json.dumps({'operations':n,'checksum':n})+'\n',stderr='',stdout_truncated=False,stderr_truncated=False)
   separator=max(i for i,x in enumerate(argv) if x=='--');target=argv[separator+1:]
   if target[0]=='/usr/bin/id':value={'counter_backend':'kpep_programmable_user','capture_complete':True,'cleanup_ok':True,'configuration_readback_verified':True,'child_status':0,'child_stdout':str(os.getuid()+int(mode=='wrong-uid'))+'\n'}
   elif target[0]=='/usr/bin/env':value={'counter_backend':'kpep_programmable_user','capture_complete':True,'cleanup_ok':True,'configuration_readback_verified':True,'child_status':0,'child_stdout':'TZ=UTC\nLC_ALL=C\n'}
   else:
    role=next(r for r in p.ROLES if (out/'binaries'/r/'qjs').stat().st_ino==Path(target[0]).stat().st_ino);n=int(target[-1]);scale=1 if n==p.COUNT else 2
    names=argv[argv.index('--events')+1].split(',')[2:];g=next(g for g in groups if g['events']==names);slow=1.45 if role.removesuffix('_aa') in ('count_flags','candidate') else 1.
    events=[]
    for e in g['description']['events']:
     count=scale*100000000+10000 if e['name']=='INST_RETIRED.ANY' else int(scale*100000000*slow)+10000 if e['name']=='CPU_CLK_UNHALTED.THREAD' else scale*1000+10
     events.append({**e,'count':count})
    vectors=[0]*4;idle=[100]*4
    for event in events:vectors[event['index']]=event['count'];event.update(idle_count=100,per_cpu=[event['count'],0],idle_per_cpu=[100,0])
    def shot(t,v):return {'begin_ns':t,'end_ns':t+10,'current_cpu':0,'per_cpu':[v,[0]*4]}
    value={'version':'qjs-kpc-3','counter_backend':'kpep_programmable_user','capture_complete':True,'cleanup_ok':True,'child_status':0,'timed_out':False,'counters_valid':mode!='bad-counters','configuration_readback_verified':True,'encoding_crosswalk_applied':True,'scope':'system_wide_user_cpu_delta' if mode!='scope' else 'pid_scoped','pid_filtered':False,'full_lifetime_counts':False,'config':g['description']['config'],'readback':g['description']['config'],'events':events,'cycles':events[0]['count'],'instructions':events[1]['count'],'cpu_count':2,'counter_count':4,'snapshots':{'before':shot(1000000,[0]*4),'after':shot(101000000,vectors),'idle_before':shot(102000000,[0]*4),'idle_after':shot(202000000 if mode!='bad-idle' else 302000000,idle)},'child_window_ns':100000000,'idle_window_ns':100000000,'elapsed_ns':400000000,'child_stdout':s.RESULT_PREFIX+json.dumps({'operations':n,'checksum':n})+'\n'}
    if mode=='bad-percpu':value['events'][0]['per_cpu'][0]+=1
    if mode=='bad-output' and role=='candidate':value['child_stdout']='WRONG\n'
   return ProcessResult(started_at='SIMULATED',timer_started_ns=0,timer_finished_ns=100000000,duration_ns=100000000,exit_code=1 if mode=='permission' else 0,timed_out=mode=='timeout',stdout=json.dumps(value),stderr='',stdout_truncated=False,stderr_truncated=False)
  monitor=types.SimpleNamespace(snapshot=lambda:{'text':'00:00:00 250 intruder\n' if mode=='disturbed' else '','anchor':datetime.datetime(2026,10,21,tzinfo=datetime.timezone.utc).timestamp()})
  with contextlib.ExitStack() as st:
   st.enter_context(patch.dict(os.environ,{'BENCH_OUT':str(out),'BENCH_QUEUE':str(queue)}));st.enter_context(patch.object(p,'HELPER',str(helper)));st.enter_context(patch.object(Path,'stat',patched_stat));st.enter_context(patch('tools.benchmark.layout_identity.host_identity',return_value={'hardware':'iMac20,1','cpu':'i7-10700'}));st.enter_context(patch('tools.benchmark.layout_policy.load_policy',return_value=policy));st.enter_context(patch('tools.benchmark.measure_lock.measurement_lock',return_value=contextlib.nullcontext()));st.enter_context(patch('tools.benchmark.process.run_process',side_effect=fake));st.enter_context(patch('tools.benchmark.pair_disturbance.Monitor.from_queue',return_value=monitor))
   if mode=='expired':st.enter_context(patch.object(p,'remaining',side_effect=RuntimeError('deadline fixture')))
   try:
    with contextlib.redirect_stdout(io.StringIO()):runpy.run_path(str(ROOT/'collect.py'))
   except (AssertionError,RuntimeError,s.ScreenError):
    if mode=='success':raise
   else:assert mode=='success',mode
  if mode=='success':
   assert len(calls)==362 and read(out/'complete.json')['measured_execs']==352
   rows=read(out/'rows.json');assert len(rows)==22
   noisy=copy.deepcopy(rows[0]['samples']['base'])
   for sample in noisy.values():
    for event in sample['events']:event['idle_count']=event['count']*2
   assert not counters.delta(noisy)['cycle_instruction_idle_at_most_5pct']
   assert all(v==2 for v in counters.delta(noisy)['idle_fraction']['N'].values())
   broken=copy.deepcopy(rows[0]['samples']['base']);event=next(e for e in broken['2N']['events'] if e['name'] not in ('INST_RETIRED.ANY','CPU_CLK_UNHALTED.THREAD'));event['count']=0;delta=counters.delta(broken);assert delta['negative_events'] and any(v is None for v in delta['events_per_instruction'].values())
  else:assert not (out/'complete.json').exists()
  if mode in ('uninstalled','expired'):assert not calls
  simulations[mode]=len(calls)
(ROOT/'validation-v3.json').write_text(json.dumps({'diagnostic_only':True,'privileged_execution_performed':False,'installation_performed':False,'hardware_evidence':False,'nonroot_tool_receipt':cfg['qualification_job'],'crosswalk_events':21,'version':'qjs-kpc-3','groups':11,'mock_calls':simulations,'scope':'354 fake helper calls and8 nonprivileged warmups exercise real collector. Actual counting backend is compile/static/API-mock qualified; root all-CPU getter awaits user install/smoke. System-wide, not child/PID attribution.','checks':['C/H/qualifier and binary seals','clang warnings-as-errors/static analyzer/unit and mocked credentials receipts','all21 programmable event encodings including CMask/edge','no shell API; drop before only execve','render/deadline/budget/install shell syntax','N2N/AA/all rows/negative events retained','missing-install/permission/UID/output/counter/idle/percpu/scope/timeout/load/deadline rejection']},indent=2)+'\n');print('VALIDATED, no real root execution',simulations)
