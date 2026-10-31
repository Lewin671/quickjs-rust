"""Same-input raytrace/gaussian diagnostic counters and canonical instructions."""
from pathlib import Path
import argparse,dataclasses,hashlib,json,os,subprocess,sys
sys.path.insert(0,str(Path(__file__).resolve().parents[2]))
from tools.benchmark.screen import counter_tool,sample
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--base-probe',type=Path,required=True)
p.add_argument('--candidate-probe',type=Path,required=True)
p.add_argument('--candidate',type=Path,required=True)
p.add_argument('--revision',required=True)
p.add_argument('--output',type=Path,required=True)
a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
old=json.loads(Path('tasks/diagnostics/T077-s2-r3-profile-summary.json').read_text())
roles={'base':(Path(old['images']['base']['path']),a.base_probe),'candidate':(a.candidate,a.candidate_probe)}
tool=counter_tool();rows=[]
for key in ['QJS_DISABLE_IC','QJS_VERIFY_IC']: os.environ.pop(key,None)
os.environ['TZ']='UTC'
for record in old['records']:
 if record['case'] not in ['external/sunspider-1.0/3d-raytrace','external/jetstream3-js-subset/gaussian-blur']:continue
 args=record['argv']['base'][1:];source=next(Path(x) for x in args if x.endswith('.js'))
 assert hashlib.sha256(source.read_bytes()).hexdigest()==record['workload_sha256']
 row={'case':record['case'],'argv_without_binary':args,'source_sha256':record['workload_sha256'],'roles':{}}
 expected=None
 for role,(canonical,probe) in roles.items():
  result=subprocess.run([str(probe),*args],capture_output=True,text=True,timeout=600)
  assert result.returncode==0,(role,record['case'],result.stderr[-2000:])
  if expected is None:expected=result.stdout
  assert result.stdout==expected
  counters={}
  for line in result.stderr.splitlines():
   bits=line.split()
   if len(bits)==2 and bits[1].isdigit():counters[bits[0]]=int(bits[1])
  assert any(k.startswith('r4_') for k in counters)
  measurements=[dataclasses.asdict(sample(tool,[str(canonical),*args],120.0)) for _ in range(3)]
  assert all(m['stdout']==expected for m in measurements)
  row['roles'][role]={'canonical_sha256':hashlib.sha256(canonical.read_bytes()).hexdigest(),'probe_sha256':hashlib.sha256(probe.read_bytes()).hexdigest(),'counters':counters,'unprofiled_canonical_counts':measurements,'stdout_sha256':hashlib.sha256(expected.encode()).hexdigest()}
  (a.output/(record['case'].replace('/','--')+'-'+role+'.stderr')).write_text(result.stderr)
 rows.append(row);print('COUNTED',record['case'],flush=True)
(a.output/'counters.json').write_text(json.dumps({'revision':a.revision,'base':'dcee3c56','note':'diagnostic-only archived counter builds; no timing claim; dynamic instructions use separate canonical images; same original argv as five-case pass','records':rows},indent=2)+'\n')
