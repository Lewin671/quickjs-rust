"""Paired five-block recipe contrasts; descriptive diagnostic, never a gate."""
import math,statistics
from tools.benchmark.statistics import paired_block_bootstrap
import protocol as p
def increment(row,role,metric):
 s=row['samples'][role];v=s['whole'][metric] if 'whole' in s else s['2N'][metric]-s['N'][metric]
 if v<=0:raise ValueError('nonpositive sample/increment')
 return v
def effect(rows,terms):
 result={}
 for metric in ('wall_ns','instructions'):
  logs={r['block']:sum(k*math.log(increment(r,n,metric)) for n,k in terms.items()) for r in rows}
  lo,hi=paired_block_bootstrap({'case':logs},samples=10000,seed=5661,confidence=.95)
  result[metric]={'median':math.exp(statistics.median(logs.values())),'lower':lo,'upper':hi}
 return result
def aa(e):
 w=e['wall_ns'];return 1/1.03<=w['median']<=1.03 and w['lower']>=1/1.10 and w['upper']<=1.10 and 1/1.01<=e['instructions']['median']<=1.01
def analyze(rows):
 assert set(rows)==set(p.CASES);reports={};guards=[]
 for case,data in rows.items():
  assert len(data)==p.BLOCKS and {r['block'] for r in data}==set(range(p.BLOCKS))
  assert all(set(r['samples'])==set(p.ROLES) for r in data)
  ratio=lambda n,d:effect(data,{n:1,d:-1})
  report={'against_original_base':{n:ratio(n,'base') for n in p.IMAGES if n!='base'},'against_flagged_base':{n:ratio(n+'_jcc','base_jcc') for n in p.SOURCES if n!='base'},'within_source':{n:ratio(n+'_jcc',n) for n in p.SOURCES},'AA':{n:ratio(n+'_aa',n) for n in p.AA_NAMES},'recipe_interaction':{n:effect(data,{n+'_jcc':1,'base_jcc':-1,n:-1,'base':1}) for n in p.SOURCES if n!='base'}}
  reports[case]=report;guards.extend(aa(e) for e in report['AA'].values())
 g=reports[p.CASES[0]];original=g['against_original_base'];anchors=original['aligned']['wall_ns']['median']<=1.05 and all(original[n]['wall_ns']['median']>=1.30 for n in ('candidate','count_flags'))
 suppressed=all(g['against_flagged_base'][n]['wall_ns']['upper']<=1.10 for n in ('aligned','candidate','count_flags')) and all(g['within_source'][n]['wall_ns']['upper']<=1/1.15 for n in ('candidate','count_flags'))
 eligible=all(guards) and anchors
 verdict='inconclusive: AA or anchors failed; retain all effects, no retry'
 if eligible:verdict='general compiler transformation suppresses observed global separation; hardware cause unproven' if suppressed else 'no preregistered suppression; this flag is not a demonstrated repair'
 return {'diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'eligible':eligible,'AA_pass':all(guards),'anchors_reproduced':anchors,'suppression_threshold':suppressed,'verdict':verdict,'effects':reports,'rule':p.RULE,'limits':'flag changes layout throughout binary; padding/branch widening may change retired instructions. No trusted cycles/DSB/MITE events; host A5 absent from Intel JCC affected table. Frozen plan still binds original unflagged base/recipe.'}
