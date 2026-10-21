"""System-wide user counters and matched idle controls; no PID attribution."""
import statistics
import protocol as p

def windows(value):
 shots=value['snapshots']
 span=lambda a,b:((shots[b]['begin_ns']+shots[b]['end_ns'])-(shots[a]['begin_ns']+shots[a]['end_ns']))/2
 return span('before','after'),span('idle_before','idle_after')

def contamination(value):
 work,idle=windows(value)
 return {e['name']:e['idle_count']*work/idle/e['count'] if e['count'] else None for e in value['events']}

def structure(value,description):
 assert value['version']=='qjs-kpc-3' and value['counter_backend']=='kpep_programmable_user'
 assert value['configuration_readback_verified'] and value['encoding_crosswalk_applied']
 assert value['scope']=='system_wide_user_cpu_delta' and value['pid_filtered'] is False and value['full_lifetime_counts'] is False
 assert value['config']==value['readback']==description['config'] and value['counter_count']==4
 expected=[(e['name'],e['index']) for e in description['events']];assert [(e['name'],e['index']) for e in value['events']]==expected
 ncpu=value['cpu_count'];assert isinstance(ncpu,int) and 0<ncpu<=256
 for shot in value['snapshots'].values():
  assert shot['end_ns']>=shot['begin_ns']>0 and 0<=shot['current_cpu']<ncpu
  assert len(shot['per_cpu'])==ncpu and all(len(row)==4 for row in shot['per_cpu'])
  assert all(isinstance(n,int) and 0<=n<2**64 for row in shot['per_cpu'] for n in row)
 previous_end=0
 for name in ('before','after','idle_before','idle_after'):
  shot=value['snapshots'][name];assert shot['begin_ns']>=previous_end;previous_end=shot['end_ns']
 work,idle=windows(value);assert work>0 and .95<=idle/work<=1.05

def discontinuities(value):
 issues=[];shots=value['snapshots'];count=value['counter_count']
 cycle_slots={event['index'] for event in value['events'] if event['name']=='CPU_CLK_UNHALTED.THREAD'}
 for left,right in (('before','after'),('idle_before','idle_after')):
  ns=shots[right]['end_ns']-shots[left]['begin_ns'];sums=[0]*count
  for cpu,(a,b) in enumerate(zip(shots[left]['per_cpu'],shots[right]['per_cpu'])):
   for index,(x,y) in enumerate(zip(a,b)):
    delta=y-x;sums[index]+=delta
    # Deliberately loose bounds for this fixed Comet Lake diagnostic. These
    # admit stalls/noise; they reject 2^48-scale discontinuities, never repair.
    bound=(p.MAX_CYCLE_PER_NS if index in cycle_slots else p.MAX_EVENT_PER_NS)*ns+p.COUNTER_RATE_SLACK
    if delta<0 or delta>bound:issues.append({'window':left+'..'+right,'cpu':cpu,'index':index,'before':x,'after':y,'delta':delta,'bound':bound,'reason':'counter_decrease' if delta<0 else 'counter_rate_bound'})
  for index,total in enumerate(sums):
   if total>=2**64:issues.append({'window':left+'..'+right,'index':index,'sum':total,'reason':'sum_overflow'})
 return issues

def totals(value):
 ncpu=value['cpu_count']
 for e in value['events']:
  for prefix,left,right in (('', 'before','after'),('idle_', 'idle_before','idle_after')):
   observed=e[prefix+'per_cpu'];assert len(observed)==ncpu and all(isinstance(n,int) and n>=0 for n in observed)
   assert observed==[value['snapshots'][right]['per_cpu'][c][e['index']]-value['snapshots'][left]['per_cpu'][c][e['index']] for c in range(ncpu)]
   assert sum(observed)==e[prefix+'count']<2**64
 counts={event['name']:event['count'] for event in value['events']}
 assert value['cycles']==counts['CPU_CLK_UNHALTED.THREAD']>0 and value['instructions']==counts['INST_RETIRED.ANY']>0

def check(value,description):
 assert value['capture_complete'] and value['cleanup_ok'] and value['child_status']==0 and not value['timed_out']
 assert value['counters_valid']
 structure(value,description);totals(value);assert not discontinuities(value)
 return contamination(value)

def delta(samples):
 a,b=samples['N'],samples['2N'];left={e['name']:e['count'] for e in a['events']};right={e['name']:e['count'] for e in b['events']};assert set(left)==set(right)
 counts={n:right[n]-left[n] for n in left};instructions=counts['INST_RETIRED.ANY'];cycles=counts['CPU_CLK_UNHALTED.THREAD'];valid=instructions>0 and cycles>0
 fractions={label:contamination(v) for label,v in samples.items()}
 controls={}
 for label,v in samples.items():
  work,idle=windows(v);controls[label]={e['name']:e['idle_count']*work/idle for e in v['events']}
 idle_increment={n:controls['2N'][n]-controls['N'][n] for n in counts}
 net={n:counts[n]-idle_increment[n] for n in counts};net_valid=net['INST_RETIRED.ANY']>0 and net['CPU_CLK_UNHALTED.THREAD']>0
 quiet=all(fractions[label][n] is not None and fractions[label][n]<=.05 for label in ('N','2N') for n in ('CPU_CLK_UNHALTED.THREAD','INST_RETIRED.ANY'))
 return {'counts':counts,'positive_cycle_instruction_increment':valid,'system_window_CPI':cycles/instructions if valid else None,'events_per_instruction':{n:v/instructions if valid and v>=0 else None for n,v in counts.items()},'negative_events':[n for n,v in counts.items() if v<0],'idle_fraction':fractions,'cycle_instruction_idle_at_most_5pct':quiet,'duration_scaled_idle_increment':idle_increment,'idle_adjusted_counts':net,'idle_adjusted_CPI':net['CPU_CLK_UNHALTED.THREAD']/net['INST_RETIRED.ANY'] if net_valid else None,'scope':'N/2N system-wide USER-only difference; idle subtraction descriptive, not child attribution'}

def summarize(rows):
 result=[]
 for row in rows:
  effects={r:delta(row['samples'][r]) for r in p.ROLES};aa={}
  for source in ('base','candidate'):
   x,y=effects[source+'_aa']['system_window_CPI'],effects[source]['system_window_CPI'];aa[source]=x/y if x and y else None
  result.append({'block':row['block'],'group':row['group'],'case':row['case'],'roles':effects,'AA_CPI':aa})
 aa_values=[v for row in result for v in row['AA_CPI'].values() if v is not None]
 return {'diagnostic_only':True,'decision_evidence':False,'attempts_consumed':0,'pid_attribution':False,'scope':'system-wide USER-only, background and polling helper included; no inferred DSB residency or JCC causality','rows':result,'all_cycle_instruction_idle_at_most_5pct':all(effect['cycle_instruction_idle_at_most_5pct'] for row in result for effect in row['roles'].values()),'AA_CPI_median':statistics.median(aa_values) if aa_values else None,'AA_CPI_descriptive_guard':len(aa_values)==len(result)*2 and all(1/1.10<=v<=1.10 for v in aa_values),'rule':p.RULE}
