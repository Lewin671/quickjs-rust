"""Prospective diagnostics only: no frozen-plan acceptance or attempt record."""
BASE='f58c696e8379261ed726f0cea1051ed6d34f431a'
CANDIDATE='dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'
HASHES={'b':'b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e','c':'3466c37a62f2fb6d764d0123212749ff810138ca05fcad336ce1afa05bdb1e3f'}
BASE_BUILD_JOB='20261020T210104Z-t056-m1a-chain-ready'
UNIT_SHA='d184dbcc122a060a219b367ffe3f3bb02c216f542f75f51eee1925387ef23284'
OFFSETS=(0,16,64,240,512,1008)
MINI_CASES=('broad/global_read','broad/local_read','broad/branch_arithmetic','external/sunspider-1.0/string-unpack-code','sentinel/heterogeneous_property_read','broad/array_read','broad/captured_read','external/sunspider-1.0/bitops-bitwise-and','diagnostic/empty')
CROSS_CASES=('broad/global_read','broad/local_read','external/sunspider-1.0/string-unpack-code','diagnostic/empty')
COUNTS=dict(zip(MINI_CASES,(23068672,23068672,13182098,None,2883584,1153433,8388608,None,None)))
ENV_KEYS=('MallocNanoZone','MallocStackLogging','MallocStackLoggingNoCompact','T056_STACK_COOKIE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE','QJS_ARCH_PAD_MODE')
COLLECT_SECONDS=1560
BODY_SECONDS=1680
JOB_SECONDS=1800

def design(kind):
 if kind=='mini':return MINI_CASES,5,range(6)
 if kind=='cross':return CROSS_CASES,15,range(1)
 raise ValueError('unknown diagnostic')

def roles(kind,k=0):
 result={}
 for side in ('b','c'):
  cells=[(str(k),k,None)] if kind=='mini' else [(f'{code}z{zone}',code,str(zone)) for code in (0,1) for zone in (0,1)]
  for label,code,zone in cells:
   for aa in (False,True):
    name=side+label+('aa' if aa else '');env=dict.fromkeys(ENV_KEYS);env['MallocNanoZone']=zone
    result[name]={'side':side,'code':code,'zone':zone,'aa':aa,'image':side+str(code),'environment':env}
 return result

def comparisons(kind,k=0):
 if kind=='mini':
  return {'C/B':{f'c{k}':1,f'b{k}':-1},'base_AA':{f'b{k}aa':1,f'b{k}':-1},'candidate_AA':{f'c{k}aa':1,f'c{k}':-1}}
 result={}
 for side in ('b','c'):
  for code in (0,1):
   for zone in (0,1):result[f'{side}{code}z{zone}_AA']={f'{side}{code}z{zone}aa':1,f'{side}{code}z{zone}':-1}
   result[f'{side}_zone_at_code{code}']={f'{side}{code}z1':1,f'{side}{code}z0':-1}
  for zone in (0,1):result[f'{side}_code_at_zone{zone}']={f'{side}1z{zone}':1,f'{side}0z{zone}':-1}
  result[f'{side}_code_x_zone']={f'{side}1z1':1,f'{side}1z0':-1,f'{side}0z1':-1,f'{side}0z0':1}
 for code in (0,1):
  for zone in (0,1):result[f'C/B_{code}z{zone}']={f'c{code}z{zone}':1,f'b{code}z{zone}':-1}
 return result

def order(names,block,phase=0):
 names=list(names);offset=(block+phase)%len(names);result=names[offset:]+names[:offset]
 return list(reversed(result)) if block%2 else result

def remaining(deadline,now,requested):
 left=deadline-now
 if left<=0:raise RuntimeError('budget exhausted; retain all partial samples, no decision')
 return min(left,requested)

# Frozen policy wall band, used only as a diagnostic uncertainty guard.
GLOBAL_BAND=1.1131654120394188
