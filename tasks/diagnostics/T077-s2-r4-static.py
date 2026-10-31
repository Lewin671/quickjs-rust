"""Address-attributed static instructions for the frozen S2 dispatch functions."""
import argparse, bisect, hashlib, json, re, subprocess, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from tools.benchmark.layout_tables import text_symbols
from tools.benchmark.layout_symbols import parse_symbol
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary', type=Path, required=True)
p.add_argument('--revision', required=True)
p.add_argument('--output', type=Path, required=True)
a=p.parse_args(); symbols=text_symbols(a.binary)
ranges=sorted((symbols.address_of(n),symbols.size_of(n),n) for n in symbols.names if symbols.size_of(n))
starts=[r[0] for r in ranges]; counts={n:0 for _,_,n in ranges}
proc=subprocess.Popen(['otool','-tvV',str(a.binary)],stdout=subprocess.PIPE,text=True)
for line in proc.stdout:
 m=re.match(r'^([0-9a-fA-F]{16})\s+[a-z]',line)
 if not m: continue
 addr=int(m[1],16);i=bisect.bisect_right(starts,addr)-1
 if i>=0:
  start,size,name=ranges[i]
  if addr<start+size: counts[name]+=1
assert proc.wait()==0
rows=[]
for start,size,name in ranges:
 parsed=parse_symbol(name)
 if not parsed: continue
 canonical=parsed.canonical
 if canonical=='qjs_runtime::bytecode::compact_fn::wide::activation::run' or ('qjs_runtime::bytecode::typed_loop::execute::' in canonical and any(token in canonical for token in ['::run::<','::execute::<'])):
  rows.append({'raw':name,'canonical':canonical,'address':start,'extent_bytes':size,'decoded_instructions':counts[name]})
assert rows
result={'revision':a.revision,'binary_sha256':hashlib.sha256(a.binary.read_bytes()).hexdigest(),'method':'otool decoded addresses within unique nm extents; static counts, not dynamic retired instructions; aliases omitted','functions':rows}
a.output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
