"""Source trigger isolation only; no acceptance decision or attempt consumed."""
BASE='f58c696e8379261ed726f0cea1051ed6d34f431a'
ALIGNED='8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a'
CANDIDATE='dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'
VARIANTS=tuple('plus_'+g for g in 'abcd')+tuple('minus_'+g for g in 'abcd')
IMAGES=('aligned','candidate',*VARIANTS)
ROLES=('base',*IMAGES,'aligned_aa','candidate_aa')
CASES=('broad/global_read','broad/local_read','diagnostic/empty')
BLOCKS=5
COUNT=23068672
COLLECT_SECONDS=1200
BODY_SECONDS=1380
JOB_SECONDS=1500
THROUGHPUT={'job':'20261021T101153Z-t056-stack-phase-probe','units':1440,'seconds':1049.2299277540296,'planned_units':195,'scaled_seconds':1049.2299277540296*195/1440,'reserve_factor':3,'setup_analysis_seconds':120}
TRIGGER={'add_to_aligned_C_over_base_min':1.30,'remove_from_candidate_C_over_base_max':1.05,'metric':'median of five paired-block global_read N/2N wall increments, relative to f58','AA':'median within [1/1.03,1.03], 95% CI within [1/1.10,1.10], instruction median within [1/1.01,1.01]','anchor':'aligned/base <=1.05 and candidate/base >=1.30','interpretation':'bidirectional group trigger, not a hardware mechanism proof; CI reported without changing thresholds; failure/no group/multiple groups retained, no follow-up timing here'}
ENV_KEYS=('MallocNanoZone','MallocStackLogging','MallocStackLoggingNoCompact','QJS_TL_TRACE','QJS_ARCH_PAD_MODE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE','T056_INITIAL_STACK_PAD','T056_STACK_COOKIE','T056_CAPTURE_GRID')
def order(names,block,phase=0):
 names=list(names);n=(block+phase)%len(names);result=names[n:]+names[:n]
 return result[::-1] if block%2 else result

def remaining(deadline,now,requested):
 if deadline<=now:raise RuntimeError('bisect deadline exhausted; incomplete, retain raw data, no retry/decision')
 return min(deadline-now,requested)
