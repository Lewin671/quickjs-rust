"""Compare actual local_read scalar instruction bytes modulo checked relocations."""
import json,re,struct,hashlib
from pathlib import Path
from tools.benchmark.layout_variant_audit import sections
import argparse,subprocess,sys
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--base',type=Path,required=True)
parser.add_argument('--candidate',type=Path,required=True)
parser.add_argument('--output-dir',type=Path,required=True)
args=parser.parse_args();p=out=args.output_dir;out.mkdir(parents=True,exist_ok=True)
sources={'base':args.base.resolve(),'candidate':args.candidate.resolve()}
for role,binary in sources.items():
    tag='base-path' if role=='base' else 'final-path'
    with (out/(tag+'.log')).open('w') as log:
        subprocess.run([sys.executable,str(Path(__file__).with_name('extract-paths.py')),str(binary),str(out),'--tag',tag],stdout=log,stderr=subprocess.STDOUT,check=True)
expected={'base':'b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e','candidate':'66ffb46f31c226076b6b4fde1405c84dcef789ed382613d8c67a2f10bdd7136e'}
records={}
for role,binary in sources.items():
 data=binary.read_bytes();assert hashlib.sha256(data).hexdigest()==expected[role]
 seg=sections(binary);text=next(x for x in seg if x[2]=='__TEXT/__text')
 def at(pc,size):
  s=next(x for x in seg if x[0]<=pc and pc+size<=x[0]+x[1]);return data[s[3]+pc-s[0]:s[3]+pc-s[0]+size]
 report=json.loads((p/('base-path.hot-paths.json' if role=='base' else 'final-path.hot-paths.json')).read_text())['bodies']['WideLoopFrame'];entry=int(report['entry'],16)
 allrows=[]
 for line in (p/('base-path.asm' if role=='base' else 'final-path.asm')).read_text().splitlines():
  m=re.match(r'^([0-9a-f]{16})\s+(\S+)\s*(.*)',line)
  if m:allrows.append((int(m[1],16),m[2],m[3].split('##')[0].strip()))
 rows={pc:{'pc':pc,'length':allrows[i+1][0]-pc,'op':op,'args':args} for i,(pc,op,args) in enumerate(allrows[:-1]) if entry<=pc<entry+report['extent']}
 header=entry+int(report['dispatch'],16)
 dispatch=[];pc=header
 while True:
  r=rows[pc];dispatch.append(r);pc+=r['length']
  if r['op'].startswith('jmp') and '*' in r['args']:break
 # The preceding reset/increment block executes on each exhausted op stream.
 backstart=next(pc for pc,r in rows.items() if r['op']=='movl' and r['args']=='$0x0, %edi' and pc<header)
 backedge=[];pc=backstart
 while pc<header:backedge.append(rows[pc]);pc+=rows[pc]['length']
 paths={k:report['paths'][k]['rows'] for k in ['Lt','Exit','Add','Update']};paths.update(Dispatch=dispatch,Backedge=backedge)
 table_lea=next(r for r in reversed(dispatch) if r['op']=='leaq' and '(%rip)' in r['args'])
 table=table_lea['pc']+table_lea['length']+int(re.search(r'(-?0x[0-9a-f]+)\(%rip\)',table_lea['args'])[1],16)
 extracted={};branches=[]
 for name,path in paths.items():
  encoded=[]
  for i,r in enumerate(path):
   pc=r['pc'];raw=at(pc,r['length']);norm=bytearray(raw);relocs=[]
   if r['op'].startswith('j') and re.fullmatch('0x[0-9a-f]+',r['args']):
    width=1 if raw[0]==0xeb or 0x70<=raw[0]<=0x7f else 4
    target=pc+len(raw)+int.from_bytes(raw[-width:],'little',signed=True);assert target==int(r['args'],16)
    relocs.append({'kind':'branch','target':hex(target),'width':width});norm[-width:]=b'\0'*width
   m=re.search(r'(-?0x[0-9a-f]+)\(%rip\)',r['args'])
   if m:
    displacement=int(m[1],16);needle=struct.pack('<i',displacement);assert raw.count(needle)==1,(r,raw.hex())
    pos=raw.index(needle);target=pc+len(raw)+displacement;norm[pos:pos+4]=b'\0'*4
    relocation={'kind':'rip','target':hex(target),'position':pos}
    # LEA addresses a relocation table; numeric loads must name equal constants.
    if r['op']!='leaq':relocation['referent8']=at(target,8).hex()
    relocs.append(relocation)
   encoded.append({**r,'offset':hex(pc-entry),'raw_hex':raw.hex(),'normalized_hex':norm.hex(),'relocations':relocs})
   if r['op'].startswith('j'):
    prev=path[i-1] if i else None;fused=prev['pc'] if prev and prev['op'].startswith(('cmp','test')) and prev['pc']+prev['length']==pc else pc
    branches.append({'path':name,'offset':hex(pc-entry),'address':hex(pc),'op':r['op'],'length':len(raw),'mod32':pc%32,'end_mod32':(pc+len(raw))%32,'branch_crosses_or_ends32':pc//32!=(pc+len(raw))//32,'cmp_test_branch_crosses_or_ends32':fused//32!=(pc+len(raw))//32})
  extracted[name]=encoded
 records[role]={'sha256':expected[role],'entry':hex(entry),'header':hex(header),'header_mod64':header%64,'header_mod32':header%32,'jump_table':hex(table),'jump_table_mod64':table%64,'paths':extracted,'branches':branches}
compare={}
for name,left in records['base']['paths'].items():
 right=records['candidate']['paths'][name]
 equivalent=len(left)==len(right) and all(a['normalized_hex']==b['normalized_hex'] and [r.get('referent8') for r in a['relocations']]==[r.get('referent8') for r in b['relocations']] for a,b in zip(left,right))
 compare[name]={'equivalent_modulo_addresses':equivalent,'instructions_base':len(left),'instructions_candidate':len(right),'raw_bytes_base':sum(r['length'] for r in left),'raw_bytes_candidate':sum(r['length'] for r in right)}
 if not equivalent:
  print('DIFF',name)
  for a,b in zip(left,right):
   if a['normalized_hex']!=b['normalized_hex']:print(a,b)
result={'scope':'local_read repeated hot path, finite numeric operands; not whole functions or loop exit teardown','records':records,'comparison':compare,'all_equivalent':all(r['equivalent_modulo_addresses'] for r in compare.values())}
(out/'byte-audit.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'comparison':compare,'all_equivalent':result['all_equivalent'],'placement':{k:{f:v for f,v in r.items() if f not in ['paths','branches']} for k,r in records.items()}},indent=2))
for k,r in records.items():print(k,'32B affected candidates',[b for b in r['branches'] if b['branch_crosses_or_ends32'] or b['cmp_test_branch_crosses_or_ends32']])
