#!/usr/bin/env python3
"""Canonical frozen-image diagnostic RSS/wall replay; standard development RSS lane."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import statistics
import subprocess
from tools.benchmark.triage_build import build

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--candidate',required=True)
p.add_argument('--output',type=Path,required=True)
p.add_argument('--development',action='store_true')
a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
roles={}
refs={'base':'1e6e2a2b','candidate':a.candidate}
if not a.development:refs={'pre':'d38f30a7',**refs}
for role,ref in refs.items():
 revision=subprocess.check_output(['git','rev-parse',ref],text=True).strip()
 folder=a.output/role;folder.mkdir()
 roles[role]=build(revision,folder/'qjs')
report={'images':roles,'timing_decision':False,'records':[]}
if a.development:
 from tools.benchmark.resource_process import run_process_wait4
 from tools.benchmark.screen import DEFAULT_CACHE,_argv,_check_output,resolve_cases
 work=a.output/'work';work.mkdir()
 cases=resolve_cases(['development'],DEFAULT_CACHE,work)
 assert len(cases)==10
 for case in cases:
  values={role:[] for role in roles}
  for pair in range(15):
   order=list(roles)
   if pair%2:order.reverse()
   for role in order:
    r=run_process_wait4(_argv(Path(roles[role]['path']),case,None),1800)
    assert r.exit_code==0 and not r.timed_out,(case.id,role,r.stderr[-300:])
    _check_output(case,r.stdout)
    values[role].append(r.peak_rss_bytes)
  med={role:statistics.median(v) for role,v in values.items()}
  report['records'].append({'case':case.id,'rss_samples':values,'rss_medians':med,'ratio':med['candidate']/med['base']})
  (a.output/'rss.json').write_text(json.dumps(report,indent=2)+'\n')
 report['geomean']=math.exp(sum(math.log(r['ratio']) for r in report['records'])/10)
 report['pass']=report['geomean']<=1.05 and all(r['ratio']<=1.10 for r in report['records'])
else:
 old=Path('tasks/diagnostics/t077-s2-followup');new=Path(__file__).parent
 programs=[old/(n+'.js') for n in ['mem1','mem2','mem3','mem4','mem5','mem6','mem7','rep','casc1','casc2','ch','deepcls']]
 programs += [new/(n+'.js') for n in ['owned-chain','owned-fan','mx2','owned-two-chain','class4000','class8000']]
 for program in programs:
  samples={role:[] for role in roles}
  for pair in range(3):
   order=list(roles)
   if pair%2:order.reverse()
   for role in order:
    r=subprocess.run(['/usr/bin/time','-l',roles[role]['path'],str(program)],capture_output=True,text=True,timeout=240)
    assert r.returncode==0,(program,role,r.returncode,r.stderr[-500:])
    rss=re.search(r'(\d+)\s+maximum resident set size',r.stderr)
    wall=re.search(r'([\d.]+)\s+real\s',r.stderr)
    assert rss and wall,(program,r.stderr)
    samples[role].append({'rss_bytes':int(rss[1]),'wall_s':float(wall[1]),'stdout':r.stdout})
  med={role:{k:statistics.median(v[k] for v in runs) for k in ['rss_bytes','wall_s']} for role,runs in samples.items()}
  row={'case':program.stem,'sha256':hashlib.sha256(program.read_bytes()).hexdigest(),'samples':samples,'medians':med}
  if program.stem.startswith('mem') or program.stem in ['owned-chain','owned-fan','mx2','owned-two-chain']:
   limit=1.1 if program.stem.startswith('mem') else 1.5
   row.update(rss_ratio=med['candidate']['rss_bytes']/med['pre']['rss_bytes'],rss_limit=limit)
   row['pass']=row['rss_ratio']<=limit
  report['records'].append(row)
  (a.output/'resources.json').write_text(json.dumps(report,indent=2)+'\n')
  print(program.stem,med,flush=True)
 # This formerly aborted on native recursion. Do not compare failed base timing.
 program=new/'function-deep.js'
 for role,binary in [('candidate',roles['candidate']['path']),('quickjs-ng','third_party/quickjs-ng/build/qjs')]:
  args=[binary,str(program)] if role=='candidate' else [binary,'--script',str(program)]
  r=subprocess.run(args,capture_output=True,text=True,timeout=120)
  assert r.returncode==0 and 'function-deep true' in r.stdout,(role,r.returncode,r.stderr)
 report['function_deep_pass']=True
 report['pass']=all(r.get('pass',True) for r in report['records'])
(a.output/'verdict.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS',report['pass'])
raise SystemExit(0 if report['pass'] else 3)
