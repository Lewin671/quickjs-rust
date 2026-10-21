"""Prospective diagnostics only: no frozen-plan acceptance or attempt record."""
BASE='f58c696e8379261ed726f0cea1051ed6d34f431a'
CANDIDATE='8f898dd96239d392c168bdd20049d073b3b47ff4'
HASHES={'b':'b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e','c':'10211f836606e0c14ce5d9a427f818adda1ff2c5b1c8c7cc615860d8b2a4b57e'}
BASE_BUILD_JOB='20261020T210104Z-t056-m1a-chain-ready'
UNIT_SHA='d184dbcc122a060a219b367ffe3f3bb02c216f542f75f51eee1925387ef23284'
OFFSETS=(0,16,64,240,512,1008)
MINI_CASES=('broad/global_read','broad/local_read','external/sunspider-1.0/string-unpack-code','diagnostic/empty')
COUNTS=dict(zip(MINI_CASES,(23068672,23068672,None,None)))
ENV_KEYS=('MallocNanoZone','MallocStackLogging','MallocStackLoggingNoCompact','T056_STACK_COOKIE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE','QJS_ARCH_PAD_MODE')
COLLECT_SECONDS=1560
BODY_SECONDS=1680
JOB_SECONDS=1800

def design(kind):
 if kind!='mini':raise ValueError('only the four-case mini is prepared')
 return MINI_CASES,5,range(6)

def roles(kind,k=0):
 assert kind=='mini';result={}
 for side in ('b','c'):
  for aa in (False,True):
   name=side+str(k)+('aa' if aa else '')
   result[name]={'side':side,'code':k,'zone':None,'aa':aa,'image':side+str(k),'environment':dict.fromkeys(ENV_KEYS)}
 return result

def comparisons(kind,k=0):
 assert kind=='mini'
 return {'C/B':{f'c{k}':1,f'b{k}':-1},'base_AA':{f'b{k}aa':1,f'b{k}':-1},'candidate_AA':{f'c{k}aa':1,f'c{k}':-1}}

def order(names,block,phase=0):
 names=list(names);offset=(block+phase)%len(names);result=names[offset:]+names[:offset]
 return list(reversed(result)) if block%2 else result

def remaining(deadline,now,requested):
 left=deadline-now
 if left<=0:raise RuntimeError('budget exhausted; retain all partial samples, no decision')
 return min(left,requested)

# Frozen policy wall band, used only as a diagnostic uncertainty guard.
GLOBAL_BAND=1.1131654120394188
