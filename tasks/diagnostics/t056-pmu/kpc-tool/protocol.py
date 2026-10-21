"""Lead-only sampled-window PMU diagnostic, not full-lifetime or acceptance data."""
BASE='f58c696e8379261ed726f0cea1051ed6d34f431a'
ALIGNED='8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a'
CANDIDATE='dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'
IMAGES=('base','aligned','count','flags','count_flags','candidate')
ROLES=(*IMAGES,'base_aa','candidate_aa')
CASES=('broad/global_read','broad/local_read')
COUNT=23068672
BLOCKS=1
COLLECT_SECONDS=720
JOB_SECONDS=900
HELPER='/usr/local/libexec/qjs-kpc'
ENV_KEYS=('MallocNanoZone','MallocStackLogging','MallocStackLoggingNoCompact','QJS_TL_TRACE','QJS_ARCH_PAD_MODE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE','T056_INITIAL_STACK_PAD','T056_STACK_COOKIE','T056_CAPTURE_GRID')
RULE='Sum observed-thread intervals only. Require completed child/config readback/cleanup, valid nonempty samples, each observed thread>=10 samples, aggregate observed span/child window>=0.70. No whole-lifetime/all-thread coverage claim. N/2N counts retained; nonpositive instruction/cycle increments or negative rare-event increments are uninterpretable, never clamped. AA CPI median within10% is descriptive guard, not acceptance or event attribution.'
def order(values,block,phase=0):
 values=list(values);n=(block+phase)%len(values);r=values[n:]+values[:n]
 return r[::-1] if block%2 else r

def remaining(deadline,now,limit):
 if now>=deadline:raise RuntimeError('15-minute PMU deadline exhausted, retain partial evidence')
 return min(deadline-now,limit)
