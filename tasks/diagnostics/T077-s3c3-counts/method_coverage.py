#!/usr/bin/env python3
"""Pair coverage of public method controls with an archived S3b source/build."""
from pathlib import Path
import subprocess,os,json,re,hashlib,argparse
parser=argparse.ArgumentParser(description='S3b/current method-path coverage; no timing')
for key in ['baseline-binary','baseline-source','candidate-binary','output']:
 parser.add_argument('--'+key,type=Path,required=True)
args=parser.parse_args()
ROOT=Path.cwd();out=args.output;out.mkdir(exist_ok=False,parents=True)
llvm=Path(subprocess.check_output(['rustc','--print','sysroot'],text=True).strip())/'lib/rustlib/aarch64-apple-darwin/bin'
roles={'s3b':(args.baseline_binary,args.baseline_source),'candidate':(args.candidate_binary,ROOT)}
work=out/'programs';work.mkdir()
from tools.benchmark import screen
from tools.benchmark.public_correctness import invocations
from tools.benchmark.schema import load_manifest
internal={f'{lane}/{c.id}':c for lane,path in screen.INTERNAL_MANIFESTS.items() for c in load_manifest(path).cases}
cases=screen.resolve_cases(['external/jetstream3-js-subset/hash-map','broad/method_call'],screen.DEFAULT_CACHE,work)
env={k:v for k,v in os.environ.items() if not k.startswith(('QJS_','LLVM_PROFILE_FILE'))};env['TZ']='UTC'
rows=[]
for case in cases:
 count,expected=invocations(case,internal)[0];args=[] if count is None else [case.case_id,str(count)]
 row={'id':case.id,'source_sha256':hashlib.sha256(case.workload.read_bytes()).hexdigest(),'args':args,'roles':{}}
 for label,(binary,srcroot) in roles.items():
  raw=out/(label+'-'+case.id.replace('/','--')+'.profraw');prof=raw.with_suffix('.profdata')
  r=subprocess.run([str(binary),'--raw',str(case.workload),*args],env={**env,'LLVM_PROFILE_FILE':str(raw)},capture_output=True,text=True,timeout=600)
  assert r.returncode==0 and r.stdout==expected,(case.id,label,r.stderr)
  subprocess.run([str(llvm/'llvm-profdata'),'merge','-sparse',str(raw),'-o',str(prof)],check=True,capture_output=True)
  cov=json.loads(subprocess.check_output([str(llvm/'llvm-cov'),'export',str(binary),'-instr-profile='+str(prof)],text=True))['data'][0]
  fs=[]
  for f in cov['functions']:
   filename=f['filenames'][0]
   if '/crates/qjs-runtime/src/' not in filename:continue
   rel=filename.split('/crates/qjs-runtime/src/')[1]
   if not (rel.startswith(('context/','function/','bytecode/compact_fn/','bytecode/typed_loop/')) or rel in ['bytecode/frame_bank.rs','bytecode/named_property_cache.rs','bytecode/vm_numeric_leaf.rs','bytecode/vm_this_property_leaf.rs']):continue
   if f['name'].startswith('_RNC'):continue
   text=(srcroot/'crates/qjs-runtime/src'/rel).read_text().splitlines();line=f['regions'][0][0]
   signature=' '.join(text[max(0,line-1):min(line+2,len(text))]).strip()
   hitlines={}
   if f['count']:
    for region in f['regions']:
     if region[5]==0 and region[7]==0 and region[4]:hitlines[str(region[0])]=max(region[4],hitlines.get(str(region[0]),0))
   if f['count']: fs.append({'symbol':f['name'],'path':rel,'line':line,'source':signature,'count':f['count'],'hit_lines':hitlines})
  row['roles'][label]={'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'functions':fs}
  print(case.id,label,len(fs),flush=True)
  raw.unlink();prof.unlink()
 rows.append(row)
 (out/'coverage.json').write_text(json.dumps({'measurement':'coverage only; never timing','records':rows},separators=(',',':'))+'\n')
