import argparse,hashlib,json,os,re,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--source',required=True);p.add_argument('--output',required=True);args=p.parse_args()
binary=Path(args.binary).resolve();inputs=json.loads(Path('target/s3b-development-inputs.json').read_text());rows=[]
env={k:v for k,v in os.environ.items() if not k.startswith('QJS_')}
for case in inputs['cases']:
 assert hashlib.sha256(Path(case['path']).read_bytes()).hexdigest()==case['sha256']
 repeats=[]
 for repeat in range(2):
  result=subprocess.run([str(binary),'--raw',case['path']],env=env,capture_output=True,text=True,timeout=600)
  assert result.returncode==0,(case['id'],result.returncode,result.stderr[:1500])
  assert result.stdout==case['expected'],(case['id'],result.stdout)
  counters={key:int(value) for key,value in re.findall(r'^  ([a-z0-9_]+) (\d+)$',result.stderr,re.M)}
  assert counters,(case['id'],result.stderr[:500])
  repeats.append(counters)
 assert repeats[0]==repeats[1],(case['id'],[(key,repeats[0][key],repeats[1][key]) for key in repeats[0] if repeats[0][key]!=repeats[1][key]])
 digest=hashlib.sha256(json.dumps(repeats[0],sort_keys=True,separators=(',',':')).encode()).hexdigest()
 rows.append(case|{'stdout':case['expected'],'stdout_sha256':hashlib.sha256(case['expected'].encode()).hexdigest(),'repeats':2,'repeats_equal':True,'repeat_counter_sha256':[digest,digest],'counters':repeats[0]})
 Path(args.output).write_text(json.dumps({'source':args.source,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'features':'perf-counters','manifest_sha256':inputs['manifest_sha256'],'timing':False,'cases':rows},indent=2)+'\n')
 print(case['id'],'2 equal full maps',flush=True)
