from pathlib import Path
import json,statistics,dataclasses,sys
from tools.benchmark.screen import sample,counter_tool
from tools.benchmark.measure_lock import busy_processes,wait_for_builds
p=Path(sys.argv[1]).resolve();cfg=json.loads((p/'spec.json').read_text());rows={};tool=counter_tool()
keys=sys.argv[2:] or list(cfg)
for key in keys:
 spec=cfg[key];N=spec['N'];ks=spec.get('K',[4,8]);forms=[(c,k) for c in [4,8] for k in ks] if spec['kind']=='matrix' else [(c,0) for c in [4,8]]
 rows[key]={}
 for C,K in forms:
  f=p/(f'{key}-{C}-{K}.js' if K else f'{key}-{C}.js');cell=rows[key][f'{C},{K}']={r:[] for r in ['r','n']}
  def run(r):return dataclasses.asdict(sample(tool,[str(p/r/'qjs'),'--raw' if r=='r' else '--script',str(f)],120))
  for r in ['r','n']:run(r)
  for pair in range(5):
   busy=wait_for_builds(60)
   if busy:raise RuntimeError('competing builds remain: '+str(busy))
   for r in (['r','n'] if pair%2==0 else ['n','r']):cell[r].append(run(r))
   if busy_processes():raise RuntimeError('competing build appeared; discard case and rerun')
 results={}
 for r in ['r','n']:
  results[r]={}
  for metric in ['cycles','instructions']:
   med=lambda c,k:statistics.median(s[metric] for s in rows[key][f'{c},{k}'][r])
   if spec['kind']=='call':results[r][metric]={'D':(med(8,0)-med(4,0))/(4*N)}
   else:
    d1=(med(8,ks[0])-med(4,ks[0]))/(4*N);d2=(med(8,ks[1])-med(4,ks[1]))/(4*N);b=(d2-d1)/(ks[1]-ks[0]);results[r][metric]={'A':d1-ks[0]*b,'B':b,'D1':d1,'D2':d2}
 record={'spec':spec,'samples':rows[key],'prices':results};(p/f'{key}-result.json').write_text(json.dumps(record,indent=2));print(key,results,flush=True)
