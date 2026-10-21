"""Fail-closed x86-64 replay of a whole repeated loop from real scalar snapshots.

Not a CPU emulator or dynamic branch trace: only instructions reached here are
implemented. Unknown opcode/address/control input aborts. Actual Mach-O bytes,
jump tables, opcode bytes and captured operands drive every taken successor.
"""
import re,struct,json,hashlib,collections,math
from pathlib import Path
from tools.benchmark.layout_variant_audit import sections

def operands(text):return re.split(r',\s*(?![^()]*\))',text)
def canonical_reg(name):
 name=name.lstrip('%');m=re.fullmatch(r'r(\d+)([dwb]?)',name)
 if m:return 'r'+m[1],{'':64,'d':32,'w':16,'b':8}[m[2]]
 for root,aliases in {'rax':('rax','eax','ax','al'),'rbx':('rbx','ebx','bx','bl'),'rcx':('rcx','ecx','cx','cl'),'rdx':('rdx','edx','dx','dl'),'rsi':('rsi','esi','si','sil'),'rdi':('rdi','edi','di','dil'),'rbp':('rbp','ebp','bp','bpl'),'rsp':('rsp','esp','sp','spl')}.items():
  if name in aliases:return root,(64,32,16,8)[aliases.index(name)]
 raise ValueError('unknown register '+name)

class Walker:
 def __init__(self,binary,assembly,entry,header,snapshot):
  self.data=Path(binary).read_bytes();self.sections=sections(binary);self.reg={r:None for r in ('rax','rbx','rcx','rdx','rsi','rdi','rbp','rsp',*[f'r{i}' for i in range(8,16)])};self.xmm={f'%xmm{i}':None for i in range(16)};self.flags={};self.rows={};self.entry=entry;self.header=header;self.snapshot=snapshot;self.memory={};self.accesses=[];self.trace=[];self.decisions=[];allrows=[]
  for line in Path(assembly).read_text().splitlines():
   m=re.match(r'^([0-9a-f]{16})\s+(\S+)\s*(.*)',line)
   if m and entry<=int(m[1],16)<entry+0x8000:allrows.append((int(m[1],16),m[2],m[3].split('##')[0].strip()))
  for row,nxt in zip(allrows,allrows[1:]):
   pc,op,args=row;self.rows[pc]={'pc':pc,'op':op,'args':args,'length':nxt[0]-pc}
  self.reg.update(r9=snapshot['bank'],r11=snapshot['program'],rbp=snapshot['rbp'],rsp=snapshot['rsp'],rbx=snapshot['backedge_iterations'],rdi=0)
  def seed(address,data):self.memory.update({address+i:b for i,b in enumerate(data)})
  for r in snapshot['registers']:seed(snapshot['bank']+16*r['index'],bytes.fromhex(r['raw_hex']))
  for i,op in enumerate(snapshot['ops']):seed(snapshot['ops_base']+16*i,bytes.fromhex(op['raw_hex']))
  seed(snapshot['program']+8,struct.pack('<Q',snapshot['ops_base']));seed(snapshot['program']+16,struct.pack('<Q',snapshot['ops_count']))
  self.memory.update({snapshot['rbp']-48+i:None for i in range(8)})
  spans=[(snapshot['bank'],128),(snapshot['ops_base'],16*snapshot['ops_count']),(snapshot['program']+8,16),(snapshot['rbp']-48,8)]
  assert all(max(a,c)>=min(a+b,c+d) for i,(a,b) in enumerate(spans) for c,d in spans[i+1:]), 'captured regions overlap'
 def at(self,pc,n):
  section=next(s for s in self.sections if s[0]<=pc and pc+n<=s[0]+s[1]);start=section[3]+pc-section[0];return self.data[start:start+n]
 def regget(self,name):
  root,width=canonical_reg(name);v=self.reg[root]
  return None if v is None else v&((1<<width)-1)
 def regset(self,name,value):
  root,width=canonical_reg(name)
  if value is None:self.reg[root]=None;return
  mask=(1<<width)-1
  self.reg[root]=value&mask if width>=32 else ((self.reg[root] or 0)&~mask)|(value&mask)
 def address(self,text,row):
  m=re.fullmatch(r'(-?(?:0x[0-9a-f]+|\d+))?\(([^)]*)\)',text);assert m,('bad address',text)
  displacement=int(m[1] or '0',0);parts=m[2].split(',');base=parts[0]
  value=row['pc']+row['length'] if base=='%rip' else self.regget(base) if base else 0
  assert value is not None,('unknown base',text)
  if len(parts)>1 and parts[1]:
   index=self.regget(parts[1]);assert index is not None;value+=index*int(parts[2] if len(parts)>2 else '1')
  return (value+displacement)&((1<<64)-1)
 def region(self,addr,n):
  s=self.snapshot
  for name,start,size in [('bank',s['bank'],128),('ops',s['ops_base'],16*s['ops_count']),('program',s['program']+8,16),('stack_scratch_pointer',s['rbp']-48,8)]:
   if start<=addr and addr+n<=start+size:return name,addr-start
  if any(x[0]<=addr and addr+n<=x[0]+x[1] for x in self.sections):return 'image_constant_or_jump_table',hex(addr)
  raise AssertionError(('unclassified access, cannot claim no ObjectData touch',hex(addr),n))
 def read(self,text,n,row):
  if text.startswith('$'):return int(text[1:],0)
  if text.startswith('%'):return self.xmm[text] if text.startswith('%xmm') else self.regget(text)
  addr=self.address(text,row);region,offset=self.region(addr,n)
  data=[self.memory.get(addr+i) for i in range(n)] if region!='image_constant_or_jump_table' else list(self.at(addr,n))
  self.accesses.append({'pc':hex(row['pc']),'access':'read','bytes':n,'address':hex(addr),'region':region,'region_offset':offset})
  return None if any(b is None for b in data) else int.from_bytes(bytes(data),'little')
 def write(self,text,n,value,row):
  if text.startswith('%'):
   if text.startswith('%xmm'):self.xmm[text]=value
   else:self.regset(text,value)
   return
  addr=self.address(text,row);region,offset=self.region(addr,n);assert region=='bank',('unexpected non-bank write',text,region)
  data=[None]*n if value is None else list((value&((1<<(8*n))-1)).to_bytes(n,'little'));self.memory.update({addr+i:b for i,b in enumerate(data)})
  self.accesses.append({'pc':hex(row['pc']),'access':'write','bytes':n,'address':hex(addr),'region':region,'region_offset':offset})
 def condition(self,op):
  z=self.flags['ZF'];c=self.flags['CF'];return {'je':z,'jne':not z,'ja':not c and not z,'jae':not c,'jb':c,'jbe':c or z}[op]
 def run(self):
  pc=self.header;initial=self.reg['rbx']
  for _ in range(1000):
   if self.trace and pc==self.header and self.reg['rbx']==initial+1 and self.reg['rdi']==0:break
   row=self.rows[pc];op=row['op'];args=operands(row['args']);nxt=pc+row['length'];raw=self.at(pc,row['length']);self.trace.append({**row,'pc':hex(pc),'offset':hex(pc-self.entry),'raw_hex':raw.hex()})
   if op=='leaq':self.write(args[1],8,self.address(args[0],row),row)
   elif op in ('movb','movl','movq','movsd','movapd','movzbl','movslq'):
    if op=='movapd':assert all(x.startswith('%xmm') for x in args), 'only register movapd supported'
    n={'movb':1,'movl':4,'movq':8,'movsd':8,'movapd':8,'movzbl':1,'movslq':4}[op];value=self.read(args[0],n,row)
    if op=='movslq':assert value is not None;value=value-(1<<32) if value&(1<<31) else value
    self.write(args[1],4 if op=='movzbl' else 8 if op=='movslq' else n,value,row)
   elif op.startswith(('cmp','test')):
    n={'b':1,'l':4,'q':8}[op[-1]];left=self.read(args[0],n,row);right=self.read(args[1],n,row);assert left is not None and right is not None
    self.flags={'ZF':right==left,'CF':right<left} if op.startswith('cmp') else {'ZF':right&left==0,'CF':False}
   elif op in ('orb','xorl','shll','shlq','addq'):
    n={'orb':1,'xorl':4,'shll':4,'shlq':8,'addq':8}[op];left=self.read(args[0],n,row);right=self.read(args[1],n,row);assert left is not None and right is not None
    value=(left|right) if op=='orb' else (left^right) if op=='xorl' else (right<<left) if op.startswith('shl') else right+left;value&=(1<<(8*n))-1;self.write(args[1],n,value,row);self.flags={'ZF':value==0,'CF':(right+left >= 1<<(8*n)) if op=='addq' else bool((right>>(8*n-left))&1) if op.startswith('shl') and 0<left<=8*n else False}
   elif op in ('ucomisd','addsd'):
    left=self.read(args[0],8,row);right=self.read(args[1],8,row);assert left is not None and right is not None
    left,right=[struct.unpack('<d',x.to_bytes(8,'little'))[0] for x in (left,right)]
    assert math.isfinite(left) and math.isfinite(right),'only captured finite operands are supported'
    if op=='ucomisd':self.flags={'ZF':right==left,'CF':right<left}
    else:self.write(args[1],8,int.from_bytes(struct.pack('<d',right+left),'little'),row)
   elif op=='seta':self.write(args[0],1,int(self.condition('ja')),row)
   elif op in ('jmp','jmpq'):
    nxt=self.regget(args[0][1:]) if args[0].startswith('*') else int(args[0],16);assert nxt is not None;self.decisions.append({'pc':hex(pc),'op':op,'taken':True,'target':hex(nxt)})
   elif op in ('je','jne','ja','jae','jb','jbe'):
    take=self.condition(op)
    if take:nxt=int(args[0],16)
    self.decisions.append({'pc':hex(pc),'op':op,'taken':take,'target':hex(nxt)})
   else:raise AssertionError(('unknown reached instruction',row))
   pc=nxt
  else:raise AssertionError('did not complete one loop')
  assert self.reg['rsp']==self.snapshot['rsp'] and self.reg['rbp']==self.snapshot['rbp']
  partial=[]
  for w in (x for x in self.accesses if x['access']=='write'):
   for r in (x for x in self.accesses if x['access']=='read'):
    wa,ra=int(w['address'],16),int(r['address'],16)
    if max(wa,ra)<min(wa+w['bytes'],ra+r['bytes']) and (wa,w['bytes'])!=(ra,r['bytes']):partial.append((w,r))
  assert not partial,'actual overlapping unequal-width accesses found'
  summary=[{'access':a,'region':r,'bytes':b,'per_iteration':n} for (a,r,b),n in sorted(collections.Counter((x['access'],x['region'],x['bytes']) for x in self.accesses).items())]
  return {'partial_overlap_store_load_pairs':partial,'memory_summary':summary,'instructions_per_iteration':len(self.trace),'trace':self.trace,'accesses':self.accesses,'branches':self.decisions,'no_calls':all(not r['op'].startswith('call') for r in self.trace),'no_architectural_ObjectData_or_RefCell_access':True,'caveat':'operand-driven replay of real binary and real snapshots, not hardware branch tracing; image addresses are unslid Mach-O VM addresses; other data addresses are captured process addresses. Unknown memory/control/op aborts.'}
