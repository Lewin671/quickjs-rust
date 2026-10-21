"""Non-timing compiler-policy copies; never modify canonical cache or sources."""
import hashlib,json,os,shutil,subprocess,tarfile
from pathlib import Path
from tools.benchmark.build_cache import RUST_RECIPE
from tools.benchmark import screen as s
from tools.benchmark.layout_variant_audit import sections

out=Path(os.environ['BENCH_OUT']);queue=Path(os.environ['BENCH_QUEUE'])
assert 'no-timing' in (out.parent/'status').read_text()
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
FLAG='-Cllvm-args=-x86-branches-within-32B-boundaries'
revs={'base':'f58c696e8379261ed726f0cea1051ed6d34f431a','aligned':'8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a','candidate':'dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'}
parent=queue/'20261021T120731Z-t056-header-layout-qualification'
assert (parent/'status').read_text().strip()=='done 0'
originals=json.loads((parent/'out/images.json').read_text())
cpu=subprocess.check_output(['sysctl',*[f'machdep.cpu.{x}' for x in ('family','model','stepping','signature','microcode_version','brand_string')]],text=True)
(out/'cpu.txt').write_text(cpu)
toolchain=subprocess.check_output(['rustc','-vV'],text=True)
helptext=subprocess.run(['rustc','-Cllvm-args=--help-hidden'],text=True,capture_output=True,check=True).stdout
assert 'x86-branches-within-32B-boundaries' in helptext
(out/'llvm-option.txt').write_text('\n'.join(x for x in helptext.splitlines() if any(y in x for y in ('x86-align-branch','x86-branches-within')))+'\n')
work=out/'work';work.mkdir();cases=s.resolve_cases(['broad/global_read','broad/local_read','broad/branch_arithmetic','external/sunspider-1.0/string-unpack-code','sentinel/heterogeneous_property_read'],s.DEFAULT_CACHE,work)
counts={'broad/global_read':23068672,'broad/local_read':23068672,'broad/branch_arithmetic':13182098,'sentinel/heterogeneous_property_read':2883584}
metadata={};checks={}
for role in (*revs,'count_flags'):
 original=parent/'out/binaries'/role/'qjs'
 assert sha(original)==originals['images'][role]['binary_sha256']
 source=out/('source-'+role);source.mkdir();archive=out/(role+'.tar')
 with archive.open('wb') as f:subprocess.run(['git','archive',revs.get(role,revs['aligned'])],stdout=f,check=True)
 with tarfile.open(archive) as f:f.extractall(source,filter='data')
 source_record={'revision':revs.get(role,revs['aligned'])}
 if role=='count_flags':
  old=queue/'20261021T115238Z-t056-header-subparts-build/out'
  part=json.loads((old/'parts.json').read_text())['roles'][role]['files']
  for path,expected in part.items():
   src=old/'source-count_flags'/path;content=src.read_bytes() if src.exists() else b''
   assert hashlib.sha256(content).hexdigest()==expected
   dst=source/path
   if content:dst.parent.mkdir(parents=True,exist_ok=True);dst.write_bytes(content)
   else:dst.unlink(missing_ok=True)
  source_record['header_files_sha256']=part
 env={**os.environ,**RUST_RECIPE['environment']}
 for key in ('RUSTFLAGS','RUSTC_BOOTSTRAP'):env.pop(key,None)
 env['CARGO_ENCODED_RUSTFLAGS']+='\x1f'+FLAG
 env['CARGO_TARGET_DIR']=str((out/('target-'+role)).resolve())
 env['CARGO_BUILD_TARGET']='x86_64-apple-darwin'
 log=out/(role+'-build.log')
 with log.open('w') as f:subprocess.run(['cargo','build',*RUST_RECIPE['cargo_args']],cwd=source,env=env,stdout=f,stderr=subprocess.STDOUT,check=True)
 built=Path(env['CARGO_TARGET_DIR'])/'x86_64-apple-darwin/release/qjs'
 for flagged,src in ((False,original),(True,built)):
  name=role+('_jcc' if flagged else '');dst=out/'binaries'/name/'qjs';dst.parent.mkdir(parents=True);shutil.copy2(src,dst)
  log=out/(name+'-ng.log');testenv={**os.environ,'QJS_CLI_BIN':str(dst),'TZ':'UTC','LC_ALL':'C'}
  for key in ('MallocNanoZone','QJS_TL_TRACE','QJS_ARCH_PAD_MODE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE','T056_INITIAL_STACK_PAD','T056_STACK_COOKIE','T056_CAPTURE_GRID'):testenv.pop(key,None)
  with log.open('w') as f:subprocess.run(['./scripts/compare-qjs.sh'],env=testenv,stdout=f,stderr=subprocess.STDOUT,check=True)
  assert sum(x.startswith('ok: ') for x in log.read_text().splitlines())==255
  values={}
  for case in cases:
   for n in ([None] if case.kind=='whole' else [counts[case.id],2*counts[case.id]]):
    q=subprocess.run(s._argv(dst,case,n),env=testenv,text=True,capture_output=True,check=True,timeout=120);key=case.id+':'+str(n);value=s._check_output(case,q.stdout)
    if key in checks:assert checks[key]==value,(name,key,value,checks[key])
    else:checks[key]=value
    values[key]=value
  empty=subprocess.check_output([str(dst),'--raw','-e','0'],env=testenv,text=True,timeout=30);assert empty.strip()=='0'
  metadata[name]={'binary_sha256':sha(dst),'bytes':dst.stat().st_size,'text_bytes':next(x[1] for x in sections(dst) if x[2]=='__TEXT/__text'),'source':source_record,'mitigated':flagged,'ng_fixtures':255,'ng_log_sha256':sha(log),'outputs':values,'empty':empty}
  (out/'images-partial.json').write_text(json.dumps(metadata,indent=2)+'\n');print('QUALIFIED',name,sha(dst),flush=True)
(out/'images.json').write_text(json.dumps({'timing':False,'diagnostic_only':True,'attempts_consumed':0,'flag':FLAG,'original_recipe':RUST_RECIPE,'toolchain':toolchain,'cpu':cpu,'all_outputs_match':True,'parent_job':parent.name,'parent_images_sha256':sha(parent/'out/images.json'),'producer_sha256':sha(__file__),'workloads':{c.id:{'sha256':sha(c.workload),'case_id':c.case_id,'kind':c.kind} for c in cases},'images':metadata},indent=2)+'\n')
print('READY',len(metadata),flush=True)
