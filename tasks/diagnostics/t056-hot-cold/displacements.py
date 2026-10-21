"""Attribute emitted relative-PC drift to actual changed instruction encodings."""
import argparse,difflib,hashlib,json,re
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('audit',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
records={}
for role,tag in [('parent','base-path'),('candidate','final-path')]:
    report=json.loads((a.audit/(tag+'.hot-paths.json')).read_text());b=report['bodies']['WideLoopFrame'];entry=int(b['entry'],16);end=entry+b['extent']
    allrows=[]
    for line in (a.audit/(tag+'.asm')).read_text().splitlines():
        m=re.match(r'^([0-9a-f]{16})\s+(\S+)\s*(.*)',line)
        if m:allrows.append((int(m[1],16),m[2],m[3].split('##')[0].strip()))
    rows=[{'offset':hex(pc-entry),'op':op,'args':args,'bytes':allrows[i+1][0]-pc} for i,(pc,op,args) in enumerate(allrows[:-1]) if entry<=pc<end]
    records[role]={'binary_sha256':report['sha256'],'entry':hex(entry),'rows':rows}
# Normalize numeric operands solely to align candidates. Output retains actual
# operands/lengths; this is not an equivalence test or hardware-cause inference.
key=lambda r:(r['op'],re.sub(r'0x[0-9a-f]+','HEX',r['args']))
old=records['parent']['rows'];new=records['candidate']['rows'];matches=[]
for block in difflib.SequenceMatcher(None,list(map(key,old)),list(map(key,new)),autojunk=False).get_matching_blocks():
    for x,y in zip(old[block.a:block.a+block.size],new[block.b:block.b+block.size]):
        if x['bytes']!=y['bytes'] and '(' in x['args'] and x['op']==y['op']:
            matches.append({'parent':x,'candidate':y,'length_delta':y['bytes']-x['bytes']})
r={'scope':'aligned emitted memory instructions in WideLoopFrame execute; real operands and lengths, no CPU latency claim','images':{n:{k:v for k,v in r.items() if k!='rows'} for n,r in records.items()},'memory_encoding_changes':matches,'all_negative_three_byte_changes':[r for r in matches if r['length_delta']==-3], 'source_assembly_sha256':{n:hashlib.sha256((a.audit/(n+'.asm')).read_bytes()).hexdigest() for n in ('base-path','final-path')}}
a.output.write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(matches,indent=2))
