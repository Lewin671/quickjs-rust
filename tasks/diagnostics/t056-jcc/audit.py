"""Non-timing operand replay of flagged binaries using original scalar inputs.

Captured bank/ops are replay inputs, not addresses observed in flagged children.
Unknown instructions fail closed. Compiler flags do not change Rust data layout.
"""
import hashlib,json,os,re,subprocess
from pathlib import Path
from tools.benchmark.layout_tables import text_symbols
from tools.benchmark.layout_symbols import parse_symbol
from tools.benchmark.layout_variant_audit import sections
from geometry import branches,summary
from walk import Walker
from decode import decode
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text()
root=Path(os.environ['BENCH_QUEUE'])/'20261021T140336Z-t056-jcc-build'
assert (root/'status').read_text().strip()=='done 0'
meta=json.loads((root/'out/images.json').read_text());snapshots=json.loads((out/'captures.json').read_text());results={}
for role in ('base_jcc','aligned_jcc','candidate_jcc','count_flags_jcc'):
 binary=root/'out/binaries'/role/'qjs';assert hashlib.sha256(binary.read_bytes()).hexdigest()==meta['images'][role]['binary_sha256']
 symbols=text_symbols(binary)
 name=next(n for n in symbols.names if (s:=parse_symbol(n)) and 'typed_loop::execute::' in s.canonical and 'WideLoopFrame' in s.canonical and symbols.sizes[n]>10000)
 entry=symbols.addresses[name];asm=out/(role+'.asm');asm.write_text(subprocess.check_output(['otool','-tv',str(binary)],text=True))
 rows=[]
 for line in asm.read_text().splitlines():
  m=re.match(r'^([0-9a-f]{16})\s+(\S+)\s*(.*)',line)
  if m and entry<=int(m[1],16)<entry+symbols.sizes[name]:rows.append((int(m[1],16),m[2],m[3].split('##')[0].strip()))
 first=next(r for r in rows if r[1].startswith('jmp') and re.fullmatch('0x[0-9a-f]+',r[2]));header=int(first[2],16)
 original=role.removesuffix('_jcc');original='aligned' if original=='base' else original
 snap=decode(next(r for r in snapshots['snapshots'] if r['role']==original and r['case']=='broad/global_read'))
 walker=Walker(binary,asm,entry,header,snap);r=walker.run();code=r['trace'];b=branches(code);stats=summary(b)
 # All architectural scalar operations must stay the original ordered path.
 expected=json.loads((out/(original+'-global_read.json')).read_text())['code']
 actual=[x for x in code if not x['op'].startswith('nop')]
 assert [x['op'] for x in actual]==[x['op'] for x in expected],(role,len(actual),len(expected))
 original_binary=root/'out/binaries'/role.removesuffix('_jcc')/'qjs'
 def text_bytes(path):
  sec=next(x for x in sections(path) if x[2]=='__TEXT/__text');return path.read_bytes()[sec[3]:sec[3]+sec[1]]
 original_text=text_bytes(original_binary);flagged_text=text_bytes(binary)
 results[role]={'text_byte_identical_to_unflagged':flagged_text==original_text,'text_sha256':hashlib.sha256(flagged_text).hexdigest(),'binary_sha256':meta['images'][role]['binary_sha256'],'entry':hex(entry),'header':hex(header),'function_bytes':symbols.sizes[name],'instructions_per_iteration':len(code),'nop_instructions':len(code)-len(actual),'original_scalar_operations':len(actual),'summary':stats,'branches':b,'code':code,'replay_input_role':original,'replay_scope':'bank/ops decoded from unflagged scalar snapshot, not flagged process addresses'}
 (out/(role+'-audit.json')).write_text(json.dumps(results[role],indent=2)+'\n');print('AUDIT',role,stats,'instructions',len(code),flush=True)
(out/'audit.json').write_text(json.dumps({'diagnostic_only':True,'timing':False,'all_hot_boundaries_clear':all(x['summary']['affected_dynamic']==0 for x in results.values()),'timing_ready':all(x['summary']['affected_dynamic']==0 for x in results.values()),'qualification_rule':'NG/output pass alone is insufficient; no lead timing until all four hot boundary counts are zero','images':results},indent=2)+'\n')
