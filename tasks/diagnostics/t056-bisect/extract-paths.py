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
variants=re.findall(r'^    ([A-Z]\w*)(?: \{|,)',subprocess.check_output(['git','show','d130c77b:crates/qjs-runtime/src/bytecode/typed_loop/mod.rs'],text=True).split('enum TypedOp {')[1].split('\n}')[0],re.M)
text=next(x for x in sections(a.binary) if x[2]=='__TEXT/__text');data=a.binary.read_bytes()
def table(pc,count):return [pc+x for x in struct.unpack_from('<'+'i'*count,data,text[3]+pc-text[0])]
allrows=[]
for line in (a.output/(a.tag+'.asm')).read_text().splitlines():
 m=re.match(r'^([0-9a-f]{16})\s+(\S+)\s*(.*)',line)
 if m:allrows.append([int(m[1],16),m[2],m[3].split('##')[0].strip()])
result={'sha256':meta['sha256'],'assumptions':'finite small numeric operands, true Exit/JumpIfFalsy condition, Increment, no residency exit; static selected paths only','bodies':{}}
for frame in ['WideLoopFrame','vm::Vm']:
 n=next(n for n in symbols.names if (s:=parse_symbol(n)) and 'typed_loop::execute::' in s.canonical and frame in s.canonical and symbols.sizes[n]>10000)
 entry=symbols.addresses[n];end=entry+symbols.sizes[n];rows={}
 for i,(pc,op,args) in enumerate(allrows[:-1]):
  if entry<=pc<end:rows[pc]={'pc':pc,'offset':hex(pc-entry),'length':allrows[i+1][0]-pc,'op':op,'args':args}
 seq=list(rows.values());ind=next(i for i,r in enumerate(seq) if r['op'].startswith('jmp') and '*' in r['args'])
 lea=next(r for r in reversed(seq[:ind]) if r['op']=='leaq' and '(%rip)' in r['args'])
 def rip(r):return r['pc']+r['length']+int(re.search(r'(-?0x[0-9a-f]+)\(%rip\)',r['args'])[1],16)
 targets=dict(zip(variants,table(rip(lea),len(variants))))
 firstjmp=next(r for r in seq[:ind] if r['op'].startswith('jmp') and re.fullmatch('0x[0-9a-f]+',r['args']));header=int(firstjmp['args'],16)
 def path(start,select=None,kind='Binary'):
  pc=start;seen=[];decisions=[]
  for _ in range(1000):
   if pc==header and seen:break
   r=rows[pc];seen.append(r);nextpc=pc+r['length']
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
    if kind=='Jump' and r['op']=='jb' and int(r['args'],16)==header:take=True
    if kind in ['Exit','JumpIfFalsy'] and r['op']=='jne' and int(r['args'],16)==header:take=True
    if take:nextpc=int(r['args'],16)
    decisions.append([r['offset'],'taken' if take else 'fallthrough',hex(nextpc-entry)])
   elif r['op'].startswith(('ret','call')):raise ValueError(('selected path unexpectedly calls/exits',kind,r))
   pc=nextpc
  else:raise ValueError('path did not return to dispatch')
  return {'instructions':len(seen),'decisions':decisions,'stack_operands':[r for r in seen if re.search(r'%[rs]bp|%rsp',r['args'])], 'rows':seen}
 paths={name:path(targets['Binary'],op) for name,op in [('Add',0),('StrictEq',10),('BitwiseAnd',13),('Lt',16)]}
 paths.update({name:path(targets[name],kind=name) for name in ['Update','Exit','JumpIfFalsy','Jump']})
 paths['JumpIfFalsyFalse']=path(targets['JumpIfFalsy'],kind='JumpIfFalsyFalse')
 reserve=next(r['args'] for r in seq[:ind] if r['op']=='subq' and r['args'].endswith('%rsp'))
 result['bodies'][frame]={'symbol':n,'entry':hex(entry),'extent':end-entry,'stack_reserve':reserve,'dispatch':hex(header-entry),'paths':paths}
f=a.output/(a.tag+'.hot-paths.json');f.write_text(json.dumps(result,indent=2)+'\n')
for frame,b in result['bodies'].items():
 print(frame,b['stack_reserve'],[(n,p['instructions'],[(r['offset'],r['op'],r['args']) for r in p['stack_operands']]) for n,p in b['paths'].items()])
