"""Frozen diagnostic contrasts; no acceptance or attempt decision."""
CASE_IDS=('broad/global_read','broad/local_read','broad/branch_arithmetic','sentinel/heterogeneous_property_read','external/sunspider-1.0/string-unpack-code','diagnostic/empty')
COUNTS=dict(zip(CASE_IDS,(23068672,23068672,13182098,2883584,None,None)))
BLOCKS=15
COLLECT_SECONDS=1260
BODY_SECONDS=1380
JOB_SECONDS=1500
CANDIDATE='dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'
UNIT_SHA='d184dbcc122a060a219b367ffe3f3bb02c216f542f75f51eee1925387ef23284'
NATURAL_HASH={'b':'b0b84489a3f6fd673b06dc4bdc71c8f176387a231ca19fe1da5ea48ca4ee0fd5','c':'3466c37a62f2fb6d764d0123212749ff810138ca05fcad336ce1afa05bdb1e3f'}
ENV_KEYS=('MallocNanoZone','MallocStackLogging','MallocStackLoggingNoCompact','T056_STACK_COOKIE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE','QJS_ARCH_PAD_MODE')
def roles():
 result={}
 for side in ('b','c'):
  for factor,mode in [('nat',None),('aa',None),('pad128','pad0128'),('pad144','pad0144'),('pad1536','pad1536'),('padaa','pada128')]:
   env=dict.fromkeys(ENV_KEYS);env['QJS_ARCH_PAD_MODE']=mode
   result[side+'_'+factor]={'side':side,'factor':factor,'image':side+('_nat' if mode is None else '_pad'),'environment':env}
 return result

def comparisons():
 result={}
 for side in ('b','c'):
  for name,top,bottom in [('natural_AA','aa','nat'),('padding_AA','padaa','pad128'),('pad144/128','pad144','pad128'),('pad1536/128','pad1536','pad128'),('diagnostic/canonical','pad128','nat')]:
   result[side+'_'+name]={side+'_'+top:1,side+'_'+bottom:-1}
 for factor in ('nat','pad128','pad144','pad1536'):
  result['C/B_'+factor]={'c_'+factor:1,'b_'+factor:-1}
 for factor in ('pad144','pad1536'):
  result['interaction_'+factor]={'c_'+factor:1,'c_pad128':-1,'b_'+factor:-1,'b_pad128':1}
 return result

def order(names,block,phase=0):
 names=list(names);offset=(block+phase)%len(names);result=names[offset:]+names[:offset]
 return list(reversed(result)) if block%2 else result

def remaining(deadline,now,requested):
 left=deadline-now
 if left<=0:raise RuntimeError('budget exhausted; retain partial evidence, whole job incomplete')
 return min(left,requested)
