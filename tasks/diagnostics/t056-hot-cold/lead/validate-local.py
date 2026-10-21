"""Validate prepared commands without running/submitting any job."""
import ast,datetime,importlib.util,json,os,re,subprocess,sys,tempfile
from pathlib import Path
from zoneinfo import ZoneInfo
root=Path(__file__).resolve().parent
repo=Path(os.environ.get('T056_VERIFY_REPO',root.parents[3]))
os.chdir(repo)
sys.path[:0]=[str(root),str(Path.cwd())]
import common,render,window
checks=[]
for p in root.glob('*.py'):ast.parse(p.read_text(),filename=str(p))
checks.append('Python modules parse')
try: common.fast_family()
except ValueError: pass
else: raise AssertionError('failed static prerequisite accepted')
guard=json.loads((root.parent/'fast-family.json').read_text())
assert guard['candidate_image_sha256']==common.CANDIDATE_HASH and guard['fast_family_static_pass']==common.STATIC_FAST_FAMILY_PASS
checks.append('failed static fast-family receipt bound to exact image rejects timing')
jobs={line.split('=',1)[0]:line.split('=',1)[1] for line in (root/'jobs.env').read_text().splitlines() if line and not line.startswith('#')}
assert jobs['CAND']==common.CANDIDATE
previous={v for k,v in jobs.items() if k.startswith('PREVIOUS_')}
for key in ['CORRECT','PREGATE','TRIAGE']:
 assert not jobs[key] or (re.fullmatch(r'[0-9]{8}T[0-9]{6}Z-[a-z0-9._-]+',jobs[key]) and jobs[key] not in previous)
checks.append('jobs.env selects current candidate; no historical blocked receipt selected')

for p in root.glob('*.sh'):
 subprocess.run(['bash','-n',str(p)],check=True)
 lines=p.read_text().splitlines()
 for i,line in enumerate(lines):
  m=re.search(r"<<'([A-Z_]+)'",line)
  if m and line.lstrip().startswith('python3'):
   end=lines.index(m[1],i+1);ast.parse('\n'.join(lines[i+1:end]),filename=p.name+':'+m[1])
checks.append('four shell scripts and embedded Python parse')
for n in render.JOBS:
 s=render.render(n);subprocess.run(['bash','-n'],input=s,text=True,check=True)
 ast.parse(s.split("<<'T056_WINDOW'\n",1)[1].rsplit('\nT056_WINDOW',1)[0])
checks.append('three rendered commands parse; rendering does not execute jobs')
zone=window.HOST_ZONE
for name in ['Asia/Shanghai','America/Los_Angeles']:
 window.HOST_ZONE=ZoneInfo(name)
 for date in ['2026-10-21','2026-11-01','2027-03-14']:
  for clock in ['00:10','01:15','05:01','12:00','23:00']:
   now=datetime.datetime.fromisoformat(date+'T'+clock).replace(tzinfo=window.HOST_ZONE).timestamp()
   deadline=window.deadline_for(now);assert deadline>now and window.host_local(deadline+900).strftime('%H:%M')=='03:00'
  for clock in ['03:00','04:59']:
   now=datetime.datetime.fromisoformat(date+'T'+clock).replace(tzinfo=window.HOST_ZONE).timestamp()
   try:window.deadline_for(now)
   except ValueError:pass
   else:raise AssertionError('night window accepted')
window.HOST_ZONE=zone
checks.append('system-zone window checks: Shanghai, LA and both DST days; no tzset API')
with tempfile.TemporaryDirectory(prefix='guard-',dir=root) as temporary:
 old=os.environ.get('BENCH_QUEUE');os.environ['BENCH_QUEUE']=temporary
 identifier='20261020T000000Z-test';p=Path(temporary)/identifier;p.mkdir();(p/'out').mkdir()
 (p/'out/binding.json').write_text(json.dumps({'candidate':common.CANDIDATE,'unit_sha256':common.UNIT_HASH,'helper_sha256':common.sha(common.__file__)}))
 (p/'status').write_text('done 0\n');(p/'log').write_text('clean')
 assert common.job(identifier)==p/'out'
 for status,log in [('done 3','clean'),('done 0','DISTURBED'),('done 130 (cancelled)','clean')]:
  (p/'status').write_text(status);(p/'log').write_text(log)
  try:common.job(identifier)
  except ValueError:pass
  else:raise AssertionError('invalid prerequisite accepted')
 (p/'status').write_text('done 0\n');(p/'log').write_text('clean')
 binding={'candidate':common.CANDIDATE,'unit_sha256':common.UNIT_HASH,'helper_sha256':common.sha(common.__file__)}
 for key in binding:
  bad={**binding,key:'0'*40 if key=='candidate' else '0'*64};(p/'out/binding.json').write_text(json.dumps(bad))
  try:common.job(identifier)
  except ValueError:pass
  else:raise AssertionError('stale candidate/unit/helper prerequisite accepted')
 if old is None:os.environ.pop('BENCH_QUEUE',None)
 else:os.environ['BENCH_QUEUE']=old
checks.append('prerequisites accept done0; reject failure, cancellation, disturbance')
from tools.benchmark.performance_decision import load_unit
from tools.benchmark.quick_triage import inventory,POLICY,POLICY_SHA
from tools.benchmark.layout_policy import load_policy
u,h=load_unit(common.PLAN);assert h==common.UNIT_HASH and u['fast_gate']['max_attempts']==2 and u['migration']['current_stage']==1
policy=load_policy(Path(POLICY),POLICY_SHA,live=False);ids=inventory(list(common.FOCUS),policy)
assert len(ids)==28 and set(common.FOCUS)<=set(ids)
checks.append('frozen unit unchanged; T061 inventory is 28 cases retaining all fixed controls')
for paths in [['crates','.cargo','Cargo.toml','Cargo.lock','tools','scripts']]:
 subprocess.run(['git','diff','--exit-code',common.CANDIDATE,'HEAD','--',*paths],check=True)
subprocess.run(['git','diff','--exit-code',common.CANDIDATE,'--','crates','tools','scripts','.cargo','Cargo.toml','Cargo.lock'],check=True)
checks.append('build/tool inputs equal exact candidate; preparation docs may differ')
report={'scope':'static/local checks only; no prepared job was executed or submitted by this validator',
        'candidate':common.CANDIDATE,'binary_sha256':common.CANDIDATE_HASH,'unit_sha256':h,'triage_case_ids':ids,'checks':checks}
destination=Path(os.environ.get('T056_PRAGMATIC_LEAD_VALIDATION',repo/'target/t056-hot-cold-lead-validation.json'));destination.parent.mkdir(parents=True,exist_ok=True);destination.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
