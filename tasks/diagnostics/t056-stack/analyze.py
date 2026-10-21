"""Address predicates and descriptive statistics, never a hardware-cause proof."""
import math,statistics
from tools.benchmark.statistics import paired_block_bootstrap

def granules(address,size,g):return {(address+i)%4096//g for i in range(size)}
def overlap(row,g):
 slot=granules(row['stack_slot'],8,g);bank=row['bank']
 writes=[(0,2),(8,8),(32,1),(40,8),(64,2),(72,8)]
 return [{'bank_offset':offset,'bytes':size} for offset,size in writes if slot&granules(bank+offset,size,g)]
def increment(row,role,metric):
 s=row['samples'][role];v=s['2N'][metric]-s['N'][metric] if 'N' in s else s['whole'][metric]
 if v<=0:raise ValueError('nonpositive internal increment')
 return v

def effects(rows,weights):
 result={}
 for metric in ('wall_ns','instructions'):
  logs={r['block']:sum(w*math.log(increment(r,role,metric)) for role,w in weights.items()) for r in rows}
  lo,hi=paired_block_bootstrap({'case':logs},samples=10000,seed=5656,confidence=.95)
  result[metric]={'median':math.exp(statistics.median(logs.values())),'lower':lo,'upper':hi}
 return result

def assess(costs,calibration,phase_effects):
 reports=[]
 for side in ('b','c'):
  for case in ('broad/global_read','broad/local_read'):
   rr=[r for r in calibration if r['side']==side and r['case']==case]
   pads=sorted({r['pad_bytes'] for r in rr});labels={}
   for pad in pads:
    flags={bool(overlap(r,8)) for r in rr if r['pad_bytes']==pad};labels[pad]=next(iter(flags)) if len(flags)==1 else None
   hit=[x for x,v in labels.items() if v is True];clean=[x for x,v in labels.items() if v is False]
   selected=[r for r in costs if r['side']==side and r['case']==case];blocks=sorted({r['block'] for r in selected});table={(r['block'],r['pad_bytes']):r for r in selected}
   contrast={}
   if hit and clean:
    for metric in ('wall_ns','instructions'):
     logs={b:statistics.fmean(math.log(table[b,p][metric]) for p in hit)-statistics.fmean(math.log(table[b,p][metric]) for p in clean) for b in blocks}
     lo,hi=paired_block_bootstrap({'case':logs},samples=10000,seed=5657,confidence=.95)
     contrast[metric]={'median':math.exp(statistics.median(logs.values())),'lower':lo,'upper':hi}
   spans={}
   for metric in ('wall_ns','instructions'):
    medians=[statistics.median(table[b,p][metric] for b in blocks) for p in pads];spans[metric]=max(medians)/min(medians)
   stable={key:len({r[key]%4096 for r in rr})==1 for key in ('bank','program','ops_base')}
   reports.append({'side':side,'case':case,'collision_pads':hit,'clean_pads':clean,'ambiguous_pads':[x for x,v in labels.items() if v is None],'collision_vs_clean':contrast,'phase_max_min_median':spans,'hot_heap_offsets_stable':stable})
 primary=next(r for r in reports if r['side']=='c' and r['case']=='broad/global_read')
 aa=[]
 for row in phase_effects:
  if row['case']=='broad/global_read':
   for name in ('base_AA','candidate_AA'):
    v=row['effects'][name];w=v['wall_ns'];i=v['instructions'];aa.append(1/1.03<=w['median']<=1.03 and w['lower']>=1/1.10 and w['upper']<=1.10 and 1/1.01<=i['median']<=1.01)
 eligible=all(aa) and all(primary['hot_heap_offsets_stable'].values()) and not primary['ambiguous_pads'] and bool(primary['collision_pads']) and bool(primary['clean_pads']) and primary['phase_max_min_median']['instructions']<=1.01
 c=primary['collision_vs_clean'].get('wall_ns',{});span=primary['phase_max_min_median']['wall_ns']
 if eligible and span<=1.05 and c['lower']>=1/1.05 and c['upper']<=1.05:verdict='flat: reject stack-bank alias explanation under sampled conditions; old unrecorded timing phase remains unverified'
 elif eligible and span>=1.10 and c['median']>=1.10 and c['lower']>1.03:verdict='supports stack-phase sensitivity; not proof of a particular hardware mechanism or old natural regression'
 else:verdict='inconclusive; no favorable phase, retry or acceptance selected'
 return {'diagnostic_only':True,'attempts_consumed':0,'primary_eligible':eligible,'verdict':verdict,'groups':reports}
