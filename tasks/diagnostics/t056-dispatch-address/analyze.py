"""Preregistered matched-phase/sham guards before interpreting address effects."""
import math,statistics
import protocol as p
from tools.benchmark.statistics import paired_block_bootstrap

def increment(row,role,metric):
 s=row['samples'][role];v=s['2N'][metric]-s['N'][metric]
 if v<=0:raise ValueError('nonpositive increment')
 return v

def effect(rows,n,d):
 out={}
 for metric in ('wall_ns','instructions'):
  logs={r['block']:math.log(increment(r,n,metric))-math.log(increment(r,d,metric)) for r in rows};lo,hi=paired_block_bootstrap({'case':logs},samples=10000,seed=5660,confidence=.95);out[metric]={'median':math.exp(statistics.median(logs.values())),'lower':lo,'upper':hi}
 return out

def neutral(e,band=.05):
 w=e['wall_ns'];i=e['instructions'];return 1/(1+band)<=w['median']<=1+band and w['lower']>=1/(1+band) and w['upper']<=1+band and 1/1.01<=i['median']<=1.01

def aa(e):
 w=e['wall_ns'];return 1/1.03<=w['median']<=1.03 and w['lower']>=1/1.10 and w['upper']<=1.10 and 1/1.01<=e['instructions']['median']<=1.01

def analyze(rows):
 assert set(rows)==set(p.CASES);reports={};guards=[]
 for case,data in rows.items():
  assert len(data)==5 and {r['block'] for r in data}==set(range(5))
  pairs={s:{'sham/original':effect(data,s+'_sham',s),'same_phase/sham':effect(data,s+('_phase900' if s=='count_flags' else '_phase600'),s+'_sham'),'600/900':effect(data,s+'_phase600',s+'_phase900'),'900/600':effect(data,s+'_phase900',s+'_phase600')} for s in p.SOURCES};aas={r:effect(data,r+'_aa',r) for r in p.AA_NAMES};guards.extend(neutral(e) for v in pairs.values() for k,e in v.items() if k in ('sham/original','same_phase/sham'));guards.extend(aa(e) for e in aas.values());reports[case]={'C/B':{r:effect(data,r,'base') for r in p.IMAGES},'contrasts':pairs,'AA':aas}
 g=reports[p.CASES[0]];median=lambda r:g['C/B'][r]['wall_ns']['median'];anchors=median('count')<=1.05 and median('flags')<=1.05 and median('count_flags')>=1.30 and median('candidate')>=1.30
 restore=median('count_flags_phase600')<=1.05 and g['contrasts']['count_flags']['600/900']['wall_ns']['upper']<1/1.10
 reproduce=[s for s in ('count','flags') if median(s+'_phase900')>=1.30 and g['contrasts'][s]['900/600']['wall_ns']['lower']>1.10]
 eligible=all(guards) and anchors
 verdict='inconclusive: neutrality/AA/anchor failed; no selection or retry'
 if eligible:verdict='supports ops-phase causal contribution across code families' if restore and reproduce else 'source-dependent ops-phase sensitivity; not universal phase rule' if restore or reproduce else 'neutral controls, no threshold phase effect; sampled ops phase does not explain regression'
 return {'diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'effects':reports,'neutrality_and_AA':all(guards),'anchors_reproduced':anchors,'eligible':eligible,'slow_restore_threshold':restore,'fast_reproduce_threshold':reproduce,'verdict':verdict,'rule':p.RULE,'limitations':'one page per child is a different allocation domain. Same-phase and sham neutrality are required. Sham original allocation phases can vary; all rows retained, no costs filtered by address. Code PCs/bytes remain fixed within each source; no PMU event attribution or acceptance.'}
