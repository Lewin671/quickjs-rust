"""Non-timing KPEP group packing and encoded-register audit against the crosswalk."""
import hashlib,json,os,subprocess
from pathlib import Path
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text()
root=Path(os.environ['BENCH_QUEUE'])/'20261021T170020Z-t056-kpc-v3-qualified';assert (root/'status').read_text().strip()=='done 0'
report=json.loads((root/'out/qualification.json').read_text());binary=root/'out/qjs-kpc';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();assert sha(binary)==report['binary_sha256']
inventory=json.loads((out/'event-inventory.json').read_text());groups=[];current=[];checks=[]
def describe(names):
 q=subprocess.run([str(binary),'--describe','--events',','.join(['cycles','instructions',*names])],text=True,capture_output=True,timeout=10)
 checks.append({'events':names,'status':q.returncode})
 if q.returncode:return None
 d=json.loads(q.stdout);assert d['classes']==2 and d['counter_count']==4 and d['config'][:2]==['0x41003c','0x4100c0']
 for n in names:
  official=next(x['intel'] for x in inventory['events'] if x['name']==n);mapping=next(e['index'] for e in d['events'] if e['name']==n);value=int(d['config'][mapping],0)
  want=int(official['EventCode'],0)|(int(official['UMask'],0)<<8)|(1<<16)|(1<<22)|(int(official['CounterMask'])<<24)|(int(official['EdgeDetect'])<<18)|(int(official['Invert'])<<23)
  assert value==want,(n,hex(value),hex(want))
 return d
for row in inventory['events']:
 if not row['supported']:continue
 candidate=current+[row['name']]
 if len(candidate)<=2 and describe(candidate):current=candidate;continue
 assert current;groups.append({'id':len(groups),'events':current,'description':describe(current)});current=[row['name']];assert describe(current)
if current:groups.append({'id':len(groups),'events':current,'description':describe(current)})
assert sum(len(g['events']) for g in groups)==sum(r['supported'] for r in inventory['events'])
(out/'groups.json').write_text(json.dumps({'diagnostic_only':True,'timing':False,'privileged':False,'hardware_access_verified':False,'binary_sha256':sha(binary),'qualification_job':root.name,'qualification_sha256':sha(root/'out/qualification.json'),'inventory_sha256':sha(out/'event-inventory.json'),'groups':groups,'group_packing_checks':checks,'unsupported':[r for r in inventory['events'] if not r['supported']]},indent=2)+'\n');print('GROUPS',len(groups),'EVENTS',sum(len(g['events']) for g in groups),flush=True)
