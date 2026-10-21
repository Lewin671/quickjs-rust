"""Prospective d130 data/launch diagnosis; no acceptance or attempt decisions."""
CASE_IDS=('broad/local_read','sentinel/heterogeneous_property_read','external/sunspider-1.0/string-unpack-code')
BLOCKS=15
COLLECT_SECONDS=1260
BODY_SECONDS=1380
JOB_SECONDS=1500
CANDIDATE='d130c77b7b2e216a3f8e6cf68892ee28dd632c22'
UNIT_SHA='d184dbcc122a060a219b367ffe3f3bb02c216f542f75f51eee1925387ef23284'
NATURAL_HASH={'b':'b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e','c':'66ffb46f31c226076b6b4fde1405c84dcef789ed382613d8c67a2f10bdd7136e'}
ENV_KEYS=('MallocNanoZone','T056_STACK_COOKIE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE')
def roles():
 result={}
 for side in ('b','c'):
  for factor,zone,cookie,bank,image in [('nat',None,None,None,'nat'),('z0','0',None,None,'nat'),('z1','1',None,None,'nat'),('env128',None,'x'*128,None,'nat'),('aa',None,None,None,'nat'),('bank00',None,None,'page00','bank'),('bank64',None,None,'page64','bank'),('bankaa',None,None,'page00','bank')]:
   result[side+'_'+factor]={'side':side,'factor':factor,'image':side+'_'+image,'environment':dict(zip(ENV_KEYS,(zone,cookie,bank,None)))}
 return result

def active_roles(case):
 return {n:r for n,r in roles().items() if case=='broad/local_read' or r['image'].endswith('_nat')}

def comparisons(case):
 result={}
 for side in ('b','c'):
  for factor in ('z0','z1','env128','aa'):
   result[side+'_'+factor+'/nat']={side+'_'+factor:1,side+'_nat':-1}
  result[side+'_z0/z1']={side+'_z0':1,side+'_z1':-1}
 for factor in ('nat','z0','z1','env128'):
  result['C/B_'+factor]={'c_'+factor:1,'b_'+factor:-1}
 for factor in ('z0','z1','env128'):
  result['interaction_'+factor]={'c_'+factor:1,'c_nat':-1,'b_'+factor:-1,'b_nat':1}
 if case=='broad/local_read':
  for side in ('b','c'):
   result[side+'_bank00/nat']={side+'_bank00':1,side+'_nat':-1}
   result[side+'_bank64/bank00']={side+'_bank64':1,side+'_bank00':-1}
   result[side+'_bank_AA']={side+'_bankaa':1,side+'_bank00':-1}
  for factor in ('bank00','bank64'):
   result['C/B_'+factor]={'c_'+factor:1,'b_'+factor:-1}
  result['bank_interaction']={'c_bank64':1,'c_bank00':-1,'b_bank64':-1,'b_bank00':1}
 return result

def order(names,block,phase=0):
 names=list(names);offset=(block+phase)%len(names);result=names[offset:]+names[:offset]
 return list(reversed(result)) if block%2 else result

def remaining(deadline,now,requested):
 left=deadline-now
 if left<=0:raise RuntimeError('diagnostic budget exhausted; whole job incomplete; retain all pairs')
 return min(left,requested)
