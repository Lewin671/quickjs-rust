"""Three-factor header diagnostic; no acceptance or attempt consumption."""
BASE='f58c696e8379261ed726f0cea1051ed6d34f431a'
ALIGNED='8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a'
CANDIDATE='dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'
PARTS=('count','flags','order')
VARIANTS=('count','flags','order','count_flags','count_order','flags_order')
IMAGES=('aligned','candidate','full_a',*VARIANTS)
AA_NAMES=('aligned','candidate','full_a')
ROLES=('base',*IMAGES,*(r+'_aa' for r in AA_NAMES))
CASES=('broad/global_read','broad/local_read')
BLOCKS=5
COUNT=23068672
COLLECT_SECONDS=720
BODY_SECONDS=840
JOB_SECONDS=900
THROUGHPUT={'job':'20261021T114227Z-t056-source-bisect','units':195,'seconds':148.1509197880514,'planned_units':130,'internal_vs_prior_mix_factor':1.125,'scaled_seconds':148.1509197880514*130/195*1.125,'reserve_factor':3,'setup_analysis_seconds':120}
TRIGGER={'single_part_sufficient':1.30,'complement_restores_max':1.05,'metric':'five paired-block global wall N/2N increments relative to f58','AA':'global/local aligned,candidate,full_a median [1/1.03,1.03], CI [1/1.10,1.10], instructions median [1/1.01,1.01]','anchors':'aligned/base <=1.05; candidate/base and full_a/base >=1.30','interpretation':'report all factorial cell medians/CIs and minimal observed bad subsets. Singleton >=1.30 with complement <=1.05 is bidirectional match; no singletons may indicate combination interaction. No deletion/retry, no architectural fix or acceptance decision.'}
ENV_KEYS=('MallocNanoZone','MallocStackLogging','MallocStackLoggingNoCompact','QJS_TL_TRACE','QJS_ARCH_PAD_MODE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE','T056_INITIAL_STACK_PAD','T056_STACK_COOKIE','T056_CAPTURE_GRID')
def order(names,block,phase=0):
 names=list(names);n=(block+phase)%len(names);result=names[n:]+names[:n]
 return result[::-1] if block%2 else result

def remaining(deadline,now,requested):
 if deadline<=now:raise RuntimeError('header diagnostic deadline exhausted; retain partial data; no retry/decision')
 return min(deadline-now,requested)
