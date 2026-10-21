"""Lead-only system-wide counting diagnostic with idle controls, no acceptance data."""
BASE='f58c696e8379261ed726f0cea1051ed6d34f431a'
ALIGNED='8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a'
CANDIDATE='dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'
IMAGES=('base','aligned','count','flags','count_flags','candidate')
ROLES=(*IMAGES,'base_aa','candidate_aa')
CASES=('broad/global_read','broad/local_read')
COUNT=11534336
BLOCKS=1
COLLECT_SECONDS=720
JOB_SECONDS=900
HELPER='/usr/local/libexec/qjs-kpc'
ENV_KEYS=('MallocNanoZone','MallocStackLogging','MallocStackLoggingNoCompact','QJS_TL_TRACE','QJS_ARCH_PAD_MODE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE','T056_INITIAL_STACK_PAD','T056_STACK_COOKIE','T056_CAPTURE_GRID')
RULE='All-CPU USER-only counters include background and helper polling; no PID attribution. Require exact config/readback/CPU shapes, monotonic raw snapshots and summed per-CPU deltas, successful nonroot child and cleanup, idle/work snapshot span ratio0.95..1.05. Report duration-normalized idle fraction for every event; cycles/instructions<=5% is descriptive qualification, never a presumed fact. Preserve raw/idle/adjusted N2N and negative rare increments; no clamping, retries, full-lifetime or causal claims. AA CPI10% guard descriptive only.'
def order(values,block,phase=0):
 values=list(values);n=(block+phase)%len(values);r=values[n:]+values[:n]
 return r[::-1] if block%2 else r

def remaining(deadline,now,limit):
 if now>=deadline:raise RuntimeError('15-minute PMU deadline exhausted, retain partial evidence')
 return min(deadline-now,limit)
