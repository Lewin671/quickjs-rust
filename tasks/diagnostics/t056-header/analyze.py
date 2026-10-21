"""Five shared paired-block contrasts; prospective header factorial rules."""
import math,statistics
import protocol as p
from tools.benchmark.statistics import paired_block_bootstrap

def increment(row,role,metric):
 s=row['samples'][role];v=s['2N'][metric]-s['N'][metric]
 if v<=0:raise ValueError('nonpositive increment')
 return v

def effects(rows,numerator,denominator):
 result={}
 for metric in ('wall_ns','instructions'):
  logs={r['block']:math.log(increment(r,numerator,metric))-math.log(increment(r,denominator,metric)) for r in rows}
  lo,hi=paired_block_bootstrap({'case':logs},samples=10000,seed=5659,confidence=.95)
  result[metric]={'median':math.exp(statistics.median(logs.values())),'lower':lo,'upper':hi}
 return result

def aa_ok(e):
 w=e['wall_ns'];i=e['instructions']
 return 1/1.03<=w['median']<=1.03 and w['lower']>=1/1.10 and w['upper']<=1.10 and 1/1.01<=i['median']<=1.01

def analyze(rows):
 assert set(rows)==set(p.CASES);report={}
 for case,data in rows.items():
  assert len(data)==p.BLOCKS and {r['block'] for r in data}==set(range(p.BLOCKS))
  report[case]={'C/B':{r:effects(data,r,'base') for r in p.IMAGES},'AA':{r:effects(data,r+'_aa',r) for r in p.AA_NAMES},'vs_aligned':{r:effects(data,r,'aligned') for r in (*p.VARIANTS,'full_a')}}
 g=report[p.CASES[0]]['C/B'];med=lambda r:g[r]['wall_ns']['median'];valid=all(aa_ok(v) for c in p.CASES for v in report[c]['AA'].values());anchors=med('aligned')<=1.05 and med('candidate')>=1.30 and med('full_a')>=1.30
 matches=[part for part in p.PARTS if med(part)>=1.30 and med('_'.join(x for x in p.PARTS if x!=part))<=1.05]
 cells={r:frozenset(r.split('_')) for r in p.VARIANTS};cells['full_a']=frozenset(p.PARTS);bad=[r for r in cells if med(r)>=1.30];minimal=[r for r in bad if not any(cells[s]<cells[r] for s in bad)]
 return {'diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'effects':report,'AA_eligible':valid,'anchors_reproduced':anchors,'threshold_matches':matches,'qualified_triggers':matches if valid and anchors else [],'minimal_observed_bad_subsets':minimal,'verdict':'header factor pattern available; mechanism and fix pending lead review' if valid and anchors else 'inconclusive; retain all factorial cells','rule':p.TRIGGER,'limits':'source type/order factors include downstream code generation and heap layout; instruction fraction does not establish time attribution; shared five f58 references, no row deletion or retry'}
