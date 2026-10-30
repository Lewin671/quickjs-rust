"""Collect construction counters and plain-image retired instructions on iMac."""
from pathlib import Path
import argparse
import hashlib
import json
import os
import re
import subprocess

p=argparse.ArgumentParser()
p.add_argument('--inputs',type=Path,required=True)
p.add_argument('--base-probe',type=Path,required=True)
p.add_argument('--candidate-probe',type=Path,required=True)
p.add_argument('--output',type=Path,required=True)
p.add_argument('--external-extras-only',action='store_true',help='seven public instruction regressions, separate from the frozen 24-case construction cohort')
a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
root=Path.cwd();records=[]
roles={'base':(a.inputs/'base000/qjs',a.base_probe), 'candidate':(a.inputs/'cand000/qjs',a.candidate_probe)}
cases=[]
for source in sorted((a.inputs/'work').glob('development--*.js')):
 cases.append(('development/'+source.stem.removeprefix('development--').replace('--','/'),source,[]))
for name in ['sunspider-1.0--access-binary-trees','kraken-1.1--json-parse-financial','jetstream3-js-subset--raytrace-public-class-fields','jetstream3-js-subset--cdjs']:
 cases.append(('external/'+name.replace('--','/'),a.inputs/'work'/(name+'.js'),[]))
for name in ['string_slice','top_level_function_call']:
 cases.append(('broad/'+name,root/'benchmarks/workloads/broad-micro.js',[name,'1000000']))
for name in ['constructor','literal','class','json']:
 for n in [1000,2000]:cases.append((f'control/{name}/{n}',root/'tasks/diagnostics/T077-s2-construction-micro.js',[name,str(n)]))
if a.external_extras_only:
 cases=[('external/'+name.replace('--','/'),a.inputs/'work'/(name+'.js'),[]) for name in ['sunspider-1.0--access-nbody','sunspider-1.0--access-nsieve','sunspider-1.0--date-format-xparb','sunspider-1.0--3d-morph','sunspider-1.0--3d-raytrace','jetstream3-js-subset--gaussian-blur','kraken-1.1--json-stringify-tinderbox']]
for case,source,args in cases:
 assert source.is_file(),source
 row={'case':case,'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'args':args,'roles':{}}
 for role,(plain,probe) in roles.items():
  result=subprocess.run([str(probe.resolve()),str(source.resolve()),*args],capture_output=True,text=True,timeout=600,env={**os.environ,'TZ':'UTC'})
  assert result.returncode==0,(case,role,result.stderr[-2000:])
  counters={}
  for line in result.stderr.splitlines():
   bits=line.split()
   if len(bits)==2 and bits[1].isdigit():counters[bits[0]]=int(bits[1])
   if line.startswith(('R2_ALLOC ','QJS_ALLOC_COUNTERS ')):
    counters.update({'allocator_'+k:int(v) for k,v in re.findall(r'(\w+)=(\d+)',line)})
  assert 'r2_objects_created' in counters,'missing probe'
  instructions=[]
  for repeat in range(3):
   run=subprocess.run(['/usr/bin/time','-l',str(plain.resolve()),str(source.resolve()),*args],capture_output=True,text=True,timeout=600,env={**os.environ,'TZ':'UTC'})
   assert run.returncode==0 and run.stdout==result.stdout,(case,role,'stdout mismatch',run.stdout[-1000:],result.stdout[-1000:])
   retired=re.search(r'(\d+)\s+instructions retired',run.stderr);assert retired,run.stderr
   instructions.append(int(retired.group(1)))
   (a.output/(case.replace('/','--')+'-'+role+f'-{repeat}.rusage')).write_text(run.stderr)
  obj=counters['r2_objects_created']
  row['roles'][role]={'counters':counters,'per_object':{k:v/obj for k,v in counters.items() if obj and k.startswith(('r2_','property_','allocator_'))},'instructions':instructions,'stdout':result.stdout,'plain_sha256':hashlib.sha256(plain.read_bytes()).hexdigest(),'probe_sha256':hashlib.sha256(probe.read_bytes()).hexdigest()}
 records.append(row)
 print('COLLECTED',case,{r:{'objects':v['counters']['r2_objects_created'],'instructions':v['instructions']} for r,v in row['roles'].items()},flush=True)
 (a.output/'work.json').write_text(json.dumps({'records':records,'measurement':'non-timing counts; whole-process totals normalized by actual object allocations'},indent=2)+'\n')
print('COMPLETE',len(records),flush=True)
