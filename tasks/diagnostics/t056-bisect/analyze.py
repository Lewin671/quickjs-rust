"""Five-block contrasts and the preregistered bidirectional source trigger."""
import math,statistics
import protocol as p
from tools.benchmark.statistics import paired_block_bootstrap

def increment(row,role,metric):
 samples=row['samples'][role];v=samples['2N'][metric]-samples['N'][metric] if 'N' in samples else samples['whole'][metric]
 if v<=0:raise ValueError('nonpositive increment')
 return v

def effects(rows,numerator,denominator):
 result={}
 for metric in ('wall_ns','instructions'):
  logs={r['block']:math.log(increment(r,numerator,metric))-math.log(increment(r,denominator,metric)) for r in rows}
  lo,hi=paired_block_bootstrap({'case':logs},samples=10000,seed=5658,confidence=.95)
  result[metric]={'median':math.exp(statistics.median(logs.values())),'lower':lo,'upper':hi}
 return result

def aa_ok(e):
 w=e['wall_ns'];i=e['instructions']
 return 1/1.03<=w['median']<=1.03 and w['lower']>=1/1.10 and w['upper']<=1.10 and 1/1.01<=i['median']<=1.01

def analyze(rows):
 assert set(rows)==set(p.CASES)
 report={}
 for case,data in rows.items():
  assert len(data)==p.BLOCKS and {r['block'] for r in data}==set(range(p.BLOCKS))
  report[case]={'C/B':{r:effects(data,r,'base') for r in p.IMAGES},'AA':{r:effects(data,r+'_aa',r) for r in ('aligned','candidate')},'direct':{**{r:effects(data,r,'aligned') for r in p.VARIANTS if r.startswith('plus_')},**{r:effects(data,r,'candidate') for r in p.VARIANTS if r.startswith('minus_')}}}
 global_=report[p.CASES[0]];valid=all(aa_ok(v) for c in p.CASES[:-1] for v in report[c]['AA'].values())
 anchors=global_['C/B']['aligned']['wall_ns']['median']<=1.05 and global_['C/B']['candidate']['wall_ns']['median']>=1.30
 groups={g:{'addition_reproduces':global_['C/B']['plus_'+g]['wall_ns']['median']>=1.30,'removal_restores':global_['C/B']['minus_'+g]['wall_ns']['median']<=1.05,'addition_C_over_base':global_['C/B']['plus_'+g],'removal_C_over_base':global_['C/B']['minus_'+g]} for g in 'abcd'}
 triggers=[g for g,v in groups.items() if v['addition_reproduces'] and v['removal_restores']]
 return {'diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'effects':report,'groups':groups,'AA_eligible':valid,'anchors_reproduced':anchors,'threshold_matches':triggers,'qualified_triggers':triggers if valid and anchors else [],'verdict':'bidirectional trigger(s) identified; mechanism unresolved' if triggers and valid and anchors else 'inconclusive; no qualified bidirectional trigger','rule':p.TRIGGER,'limits':'one canonical build per source combination; downstream codegen/placement included. No favorable image/phase selected, no pair removed or retried. Five shared f58 block references are not independent per-image experiments. Empty is diagnostic startup control; its A/A is reported but not precision-gated.'}
