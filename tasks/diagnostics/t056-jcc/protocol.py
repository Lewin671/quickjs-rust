"""One lead-only matched-recipe diagnostic; never consumes an acceptance attempt."""
BASE='f58c696e8379261ed726f0cea1051ed6d34f431a'
ALIGNED='8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a'
CANDIDATE='dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'
SOURCES=('base','aligned','candidate','count_flags')
IMAGES=tuple(x+s for x in SOURCES for s in ('','_jcc'))
AA_NAMES=('base','base_jcc','candidate','candidate_jcc')
ROLES=(*IMAGES,*(x+'_aa' for x in AA_NAMES))
CASES=('broad/global_read','broad/local_read','broad/branch_arithmetic','external/sunspider-1.0/string-unpack-code','sentinel/heterogeneous_property_read','diagnostic/empty')
COUNTS=dict(zip(CASES,(23068672,23068672,13182098,None,2883584,None)))
BLOCKS=5
COLLECT_SECONDS=720
BODY_SECONDS=840
JOB_SECONDS=900
ENV_KEYS=('MallocNanoZone','MallocStackLogging','MallocStackLoggingNoCompact','QJS_TL_TRACE','QJS_ARCH_PAD_MODE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE','T056_INITIAL_STACK_PAD','T056_STACK_COOKIE','T056_CAPTURE_GRID')
RULE={'AA':'all four base/candidate +/-flag clones, all six cases: wall median [1/1.03,1.03], 95%CI [1/1.10,1.10], instruction median [1/1.01,1.01]',
 'anchors':'unflagged aligned/base <=1.05; candidate/base and count_flags/base >=1.30 on global_read',
 'suppression':'flagged candidate/base_jcc and count_flags/base_jcc upper95% <=1.10; each slow flag/original upper95% <=1/1.15; aligned_jcc/base_jcc upper95% <=1.10',
 'interpretation':'AA+anchors precede suppression inference. Suppression supports sensitivity to this general compiler layout transformation, not JCC erratum attribution or production acceptance. Preserve every row; no retries, offsets or case selection. Other five cases expose collateral changes; report both matched-recipe and frozen-original-base ratios.'}
def order(names,block,phase=0):
 names=list(names);n=(block+phase)%len(names);r=names[n:]+names[:n]
 return r[::-1] if block%2 else r
def remaining(deadline,now,requested):
 if now>=deadline:raise RuntimeError('15-minute JCC diagnostic deadline exhausted; retain partial evidence, no retry')
 return min(deadline-now,requested)
