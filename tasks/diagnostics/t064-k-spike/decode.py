"""Static paths for numeric operands; never executes the image or infers cycles.

Conditional branches follow explicit finite-small-number/true-boolean choices.
All chosen branches and original instruction bytes' locations are retained.
"""
import argparse,json,re,struct,subprocess
from pathlib import Path
from tools.benchmark.layout_variant_audit import inspect,sections
from tools.benchmark.layout_tables import text_symbols
from tools.benchmark.layout_symbols import parse_symbol
p=argparse.ArgumentParser();p.add_argument('binary',type=Path);p.add_argument('output',type=Path);p.add_argument('--tag',default='image');a=p.parse_args();a.output.mkdir(exist_ok=True,parents=True)
meta=inspect(a.binary,a.output,a.tag);symbols=text_symbols(a.binary)
variants=re.findall(r'^    ([A-Z]\w*)(?: \{|,)',subprocess.check_output(['git','show','06e199fec9e9e51742cdae7633c5dc758c326752:crates/qjs-runtime/src/bytecode/typed_loop/mod.rs'],text=True).split('enum TypedOp {')[1].split('\n}')[0],re.M)
segments=sections(a.binary);text=next(x for x in segments if x[2]=='__TEXT/__text');data=a.binary.read_bytes()
def table(pc,count):return [pc+x for x in struct.unpack_from('<'+'i'*count,data,text[3]+pc-text[0])]
allrows=[]
for line in (a.output/(a.tag+'.asm')).read_text().splitlines():
 m=re.match(r'^([0-9a-f]{16})\s+(\S+)\s*(.*)',line)
 if m:allrows.append([int(m[1],16),m[2],m[3].split('##')[0].strip()])
result={'sha256':meta['sha256'],'assumptions':'finite small numeric operands, true Boolean Exit/JumpIfFalsy, false Boolean JumpIfFalsyFalse, Increment, forward Jump/no residency exit; static selected paths only','sections':segments,'bodies':{}}
for frame in ['WideLoopFrame','vm::Vm']:
 candidates=[n for n in symbols.names if (s:=parse_symbol(n)) and 'typed_loop::execute::execute::' in s.canonical and frame in s.canonical and symbols.sizes[n]>2000]
 if not candidates:
  candidates=[n for n in symbols.names if (s:=parse_symbol(n)) and 'typed_loop::execute::run::' in s.canonical and frame in s.canonical and symbols.sizes[n]>10000]
 assert len(candidates)==1,[(parse_symbol(n).canonical,symbols.sizes[n]) for n in candidates]
 n=candidates[0]
 entry=symbols.addresses[n];end=entry+symbols.sizes[n];rows={}
 for i,(pc,op,args) in enumerate(allrows[:-1]):
  if entry<=pc<end:rows[pc]={'pc':pc,'offset':hex(pc-entry),'length':allrows[i+1][0]-pc,'op':op,'args':args}
 seq=list(rows.values());ind=next(i for i,r in enumerate(seq) if r['op'].startswith('jmp') and '*' in r['args'])
 lea=next(r for r in reversed(seq[:ind]) if r['op']=='leaq' and '(%rip)' in r['args'])
 def rip(r):return r['pc']+r['length']+int(re.search(r'(-?0x[0-9a-f]+)\(%rip\)',r['args'])[1],16)
 targets=dict(zip(variants,table(rip(lea),len(variants))))
 assert all(t in rows for t in targets.values()),targets
 headers=[r['pc'] for r in seq[:ind] if r['op']=='cmpq' and re.match(r'0x10\(%\w+\), %\w+$',r['args'])]
 assert len(headers)==1,headers
 header=headers[0]
 def path(start,select=None,kind='Binary'):
  pc=start;seen=[];decisions=[];boolean_zf=None
  for _ in range(1000):
   if pc==header and seen:break
   r=rows[pc];seen.append(r);nextpc=pc+r['length']
   if kind in ['Exit','JumpIfFalsy','JumpIfFalsyFalse']:
    if r['op']=='testl': boolean_zf=False  # Typed::Boolean has tag 1.
    elif r['op']=='cmpl' and r['args'].startswith('$0x1,'): boolean_zf=True
    elif r['op']=='testb' and r['args'].startswith('$0x1,'): boolean_zf=kind=='JumpIfFalsyFalse'
   if r['op'].startswith('jmp'):
    if '*' in r['args']:
     assert select is not None,(kind,r)
     ld=next(x for x in reversed(seen[:-1]) if x['op']=='leaq' and '(%rip)' in x['args'])
     nextpc=table(rip(ld),20)[select];decisions.append([r['offset'],'numeric BinaryOp table index '+str(select),hex(nextpc-entry)])
    else:nextpc=int(r['args'],16)
   elif r['op'].startswith('j'):
    take=False
    if kind=='Binary' and len(decisions)==0 and r['op']=='je':take=True
    if kind=='Update' and r['op']=='je' and len(decisions)<2:take=True
    if kind=='Jump' and r['op'] in ['jb','jae','jbe'] and int(r['args'],16)==header:take=True
    if kind in ['Exit','JumpIfFalsy','JumpIfFalsyFalse']:
     assert r['op'] in ['je','jne'] and boolean_zf is not None,(kind,r)
     take=boolean_zf if r['op']=='je' else not boolean_zf
    if take:nextpc=int(r['args'],16)
    decisions.append([r['offset'],'taken' if take else 'fallthrough',hex(nextpc-entry)])
   elif r['op'].startswith(('ret','call')):raise ValueError(('selected path unexpectedly calls/exits',kind,r))
   pc=nextpc
  else:raise ValueError('path did not return to dispatch')
  return {'instructions':len(seen),'decisions':decisions,'stack_operands':[r for r in seen if re.search(r'%[rs]bp|%rsp',r['args'])], 'rows':seen}
 paths={name:path(targets['Binary'],op) for name,op in [('Add',0),('StrictEq',10),('BitwiseAnd',13),('Lt',16)]}
 paths.update({name:path(targets[name],kind=name) for name in ['Update','Exit','JumpIfFalsy','Jump']})
 paths['JumpIfFalsyFalse']=path(targets['JumpIfFalsy'],kind='JumpIfFalsyFalse')
 assert any(r['op']=='addsd' for r in paths['Add']['rows']), 'Add path did not perform addition'
 paths['Dispatch']={'rows':seq[seq.index(rows[header]):ind+1]}
 paths['Prologue']={'rows':seq[:seq.index(rows[header])]}
 def raw(pc,n):
  seg=next(x for x in segments if x[0]<=pc and pc+n<=x[0]+x[1]);off=seg[3]+pc-seg[0];return data[off:off+n]
 for v in paths.values():
  for r in v['rows']:
   r['raw_hex']=raw(r['pc'],r['length']).hex()
   rip_match=re.search(r'(-?0x[0-9a-f]+)\(%rip\)',r['args'])
   if rip_match:
    target=rip(r);r['rip_target_offset']=hex(target-entry);r['referent8']=raw(target,8).hex()
    width=16 if r['op'] in ['movapd','movaps','movdqa','movdqu','andpd','orpd','xorpd','andps','orps','xorps'] else 8
    r['referent_hex']=raw(target,width).hex()
    seg=next(x for x in segments if x[0]<=target< x[0]+x[1])
    r['referent_section']=seg[2];r['referent_section_offset']=hex(target-seg[0])
 tables={'dispatch':{'offset':hex(rip(lea)-entry),'targets':{k:hex(v-entry) for k,v in targets.items()},'raw_hex':raw(rip(lea),4*len(variants)).hex()}}
 for r in paths['Add']['rows']:
  if r['op']=='leaq' and '(%rip)' in r['args']:
   t=rip(r);tables['binary']={'offset':hex(t-entry),'targets':[hex(x-entry) for x in table(t,20)],'raw_hex':raw(t,80).hex()}
 (a.output/(a.tag+'.'+frame.replace('::','-')+'.bin')).write_bytes(raw(entry,end-entry))
 reserve=next(r['args'] for r in seq[:ind] if r['op']=='subq' and r['args'].endswith('%rsp'))
 service_addresses={a for n,a in symbols.addresses.items() if (p:=parse_symbol(n)) and 'typed_loop::execute::object_service::' in p.canonical}
 service_calls=[r['offset'] for r in seq if r['op'].startswith('call') and re.fullmatch('0x[0-9a-f]+',r['args']) and int(r['args'],16) in service_addresses]
 result['bodies'][frame]={'service_calls':service_calls,'symbol':n,'entry':hex(entry),'extent':end-entry,'stack_reserve':reserve,'tables':tables,'dispatch':hex(header-entry),'paths':paths}
f=a.output/(a.tag+'.hot-paths.json');f.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({f:{'extent':b['extent'],'dispatch':b['dispatch'],'instructions':{n:len(p['rows']) for n,p in b['paths'].items()}} for f,b in result['bodies'].items()},indent=2))
