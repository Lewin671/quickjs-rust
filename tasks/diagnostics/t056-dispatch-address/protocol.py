"""Ops-buffer phase confirmation only; no acceptance or attempt consumption."""
BASE='f58c696e8379261ed726f0cea1051ed6d34f431a'
ALIGNED='8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a'
CANDIDATE='dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'
SOURCES=('count','flags','count_flags')
IMAGES=(*SOURCES,'aligned','candidate',*(s+'_'+m for s in SOURCES for m in ('sham','phase600','phase900')))
AA_NAMES=('count_sham','count_flags_sham')
ROLES=('base',*IMAGES,*(r+'_aa' for r in AA_NAMES))
CASES=('broad/global_read','broad/local_read')
BLOCKS=5
COUNT=23068672
COLLECT_SECONDS=720
BODY_SECONDS=840
JOB_SECONDS=900
THROUGHPUT={'reference_job':'20261021T114227Z-t056-source-bisect','reference_units':195,'reference_seconds':148.1509197880514,'units':170,'internal_mix_factor':1.125,'scaled_seconds':148.1509197880514*170/195*1.125,'reserve_factor':3,'setup_analysis_seconds':120}
RULE={'neutrality':'sham/original AND same-phase-relocation/sham wall median/95% CI within [1/1.05,1.05]; instruction median within [1/1.01,1.01] on global/local','AA':'count_sham and count_flags_sham median [1/1.03,1.03], CI [1/1.10,1.10], instruction median [1/1.01,1.01]','anchors':'count/flags <=1.05 and count_flags/candidate >=1.30 vs f58 on global','restore':'count_flags_phase600/base <=1.05, phase600/phase900 upper95% <1/1.10','reproduce':'at least one fast count/flags phase900/base >=1.30, phase900/phase600 lower95% >1.10','interpretation':'after neutrality/AA/anchors, restore+reproduce supports ops-phase causality. Restore alone is source-dependent sensitivity. Neutrality failure => relocation/setup/domain confounded; keep raw results. Neither effect with neutral controls => sampled ops phase does not explain. Never infer PMU event, best phase or production fix from this job.'}
ENV_KEYS=('MallocNanoZone','MallocStackLogging','MallocStackLoggingNoCompact','QJS_TL_TRACE','QJS_ARCH_PAD_MODE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE','T056_INITIAL_STACK_PAD','T056_STACK_COOKIE','T056_CAPTURE_GRID')
def order(names,block,phase=0):
 names=list(names);n=(block+phase)%len(names);result=names[n:]+names[:n]
 return result[::-1] if block%2 else result

def remaining(deadline,now,requested):
 if deadline<=now:raise RuntimeError('15-minute ops confirmation deadline exhausted; retain partial data, no retry')
 return min(deadline-now,requested)
