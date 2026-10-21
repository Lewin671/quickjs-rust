"""Stack-phase diagnosis only; no acceptance or attempt record."""
BASE='f58c696e8379261ed726f0cea1051ed6d34f431a'
CANDIDATE='dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'
HASHES={'b':'b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e','c':'3466c37a62f2fb6d764d0123212749ff810138ca05fcad336ce1afa05bdb1e3f'}
CASE_IDS=('broad/global_read','broad/local_read','diagnostic/empty')
BLOCKS=4
COUNT=23068672
COLLECT_SECONDS=1260
BODY_SECONDS=1380
JOB_SECONDS=1500
ENV_KEYS=('MallocNanoZone','MallocStackLogging','MallocStackLoggingNoCompact','T056_STACK_COOKIE','QJS_ARCH_PAD_MODE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE','T056_INITIAL_STACK_PAD')

def grid(seeds):
 # Geometry only, before canonical timing. Both sides use the identical list.
 anchors=[(r['stack_slot']-r['bank'])%4096 for r in seeds]
 return sorted(set(range(0,4096,256))|{(a+d)%4096 for a in anchors for d in (-64,-32,-16,0,16,32,64)})

def roles():
 return {side+('aa' if aa else ''):{'side':side,'aa':aa} for side in ('b','c') for aa in (False,True)}

def order(names,block,phase=0):
 names=list(names);offset=(block+phase)%len(names);result=names[offset:]+names[:offset]
 return list(reversed(result)) if block%2 else result

def remaining(deadline,now,requested):
 left=deadline-now
 if left<=0:raise RuntimeError('25-minute probe budget exhausted; retain partial samples; no conclusion')
 return min(left,requested)
