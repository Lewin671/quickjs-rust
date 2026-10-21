"""Diagnostic effects and preregistered global-excess routing, never acceptance."""
import math,statistics
from tools.benchmark.statistics import paired_block_bootstrap
import protocol as p

def increment(row,role,metric):
 values=row['samples'][role]
 v=values['2N'][metric]-values['N'][metric] if 'N' in values else values['whole'][metric]
 if v<=0:raise ValueError(('nonpositive increment',row['block'],role,metric,v))
 return v

def effects(rows,contrasts):
 result={}
 for i,(name,weights) in enumerate(contrasts.items()):
  result[name]={}
  for metric in ('wall_ns','instructions'):
   logs={r['block']:sum(w*math.log(increment(r,role,metric)) for role,w in weights.items()) for r in rows}
   lo,hi=paired_block_bootstrap({'case':logs},samples=10000,seed=5600+i,confidence=.95)
   result[name][metric]={'median':math.exp(statistics.median(logs.values())),'lower':lo,'upper':hi}
 return result

def aa_ok(value,empty=False):
 wall=value['wall_ns'];ins=value['instructions'];med=1.10 if empty else 1.03;ci=1.25 if empty else 1.10
 return 1/med<=wall['median']<=med and wall['lower']>=1/ci and wall['upper']<=ci and 1/1.01<=ins['median']<=1.01

def route(report):
 # Applied only to a complete five-pair x six-layout mini report.
 assert len(report)==6*len(p.MINI_CASES)
 assert {(r['layout'],r['case']) for r in report}=={(k,c) for k in range(6) for c in p.MINI_CASES}
 quality=all(aa_ok(r['effects'][name],r['case']=='diagnostic/empty') for r in report for name in ('base_AA','candidate_AA'))
 global_rows=sorted((r for r in report if r['case']=='broad/global_read'),key=lambda r:r['layout'])
 assert [r['layout'] for r in global_rows]==list(range(6))
 gone=[];persistent=[]
 for r in global_rows:
  v=r['effects']['C/B'];wall=v['wall_ns'];ins=v['instructions']
  gone.append(wall['median']<=1.05 and wall['upper']<=p.GLOBAL_BAND and ins['median']<=1.0069)
  persistent.append(wall['median']>1.05 and wall['lower']>1.0 and ins['median']<=1.0069)
 if not quality:next_step='inconclusive: A/A quality failed; do not spend attempt'
 elif sum(gone)>=5:next_step='lead may consider sealed pragmatic candidate under frozen full plan; no acceptance granted'
 elif sum(persistent)>=5:next_step='persistent: do not spend attempt; preserve risk and reconsider M2 kernel isolation under its own plan'
 else:next_step='mixed or imprecise: do not spend attempt; no favorable-layout selection'
 return {'diagnostic_only':True,'acceptance':False,'attempts_consumed':0,'aa_quality':quality,'excess_disappeared_layouts':sum(gone),'persistent_layouts':sum(persistent),'per_layout_disappeared':gone,'per_layout_persistent':persistent,'next_step':next_step}
