"""Validate child-scoped sampled counter JSON; preserve negative rare-event deltas."""
import statistics
import protocol as p

def check(value,description):
 assert value['capture_complete'] and value['cleanup_ok'] and value['child_status']==0 and not value['timed_out']
 assert value['counter_backend']=='kpep_programmable_user'
 assert value['counters_valid'] and value['configuration_readback_verified'] and value['encoding_crosswalk_applied']
 assert value['scope']=='sum_of_observed_thread_intervals' and value['all_threads_covered'] is False and value['full_lifetime_counts'] is False
 assert value['config']==description['config']
 expected=[(e['name'],e['index']) for e in description['events']];assert [(e['name'],e['index']) for e in value['events']]==expected
 assert all(isinstance(e['count'],int) and e['count']>=0 for e in value['events'])
 assert value['cycles']>0 and value['instructions']>0
 assert value['threads'] and all(t['samples']>=10 for t in value['threads'])
 assert sum(t['observed_span_ns'] for t in value['threads'])/value['child_window_ns']>=.70
 return {e['name']:e['count'] for e in value['events']}

def delta(samples):
 a,b=samples['N'],samples['2N'];left={e['name']:e['count'] for e in a['events']};right={e['name']:e['count'] for e in b['events']};assert set(left)==set(right)
 counts={n:right[n]-left[n] for n in left};instructions=counts['INST_RETIRED.ANY'];cycles=counts['CPU_CLK_UNHALTED.THREAD'];valid=instructions>0 and cycles>0
 return {'counts':counts,'positive_fixed_increment':valid,'observed_window_CPI':cycles/instructions if valid else None,'events_per_instruction':{n:v/instructions if valid and v>=0 else None for n,v in counts.items()},'negative_events':[n for n,v in counts.items() if v<0],'scope':'N/2N difference of sampled intervals, not full child totals'}

def summarize(rows):
 result=[]
 for row in rows:
  effects={r:delta(row['samples'][r]) for r in p.ROLES};aa={}
  for source in ('base','candidate'):
   x,y=effects[source+'_aa']['observed_window_CPI'],effects[source]['observed_window_CPI'];aa[source]=x/y if x and y else None
  result.append({'block':row['block'],'group':row['group'],'case':row['case'],'roles':effects,'AA_CPI':aa})
 aa_values=[v for row in result for v in row['AA_CPI'].values() if v is not None]
 return {'diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'hardware_full_lifetime_validated':False,'all_thread_coverage_validated':False,'scope':'sampled user-space intervals only; no inferred DSB residency or JCC causality','rows':result,'AA_CPI_median':statistics.median(aa_values) if aa_values else None,'AA_CPI_descriptive_guard':len(aa_values)==len(result)*2 and all(1/1.10<=v<=1.10 for v in aa_values),'rule':p.RULE}
