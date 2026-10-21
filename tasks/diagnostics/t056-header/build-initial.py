"""Non-timing header-factor builds, original-recipe images plus layout-only probes."""
import os,subprocess,tarfile,json,hashlib,shutil,sys,re
from pathlib import Path
import parts
from tools.benchmark.build_cache import RUST_RECIPE
from tools.benchmark import screen as s
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text();ROOT=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
cache=Path('target/perf-loop/compiler-fat16-abort-v1');old=cache/('rev-'+parts.OLD)/'qjs'
parent=Path(os.environ['BENCH_QUEUE'])/'20261021T112221Z-t056-source-bisect-build';assert (parent/'status').read_text().strip()=='done 0'
parentmeta=json.loads((parent/'out/images.json').read_text());metadata={};paths={};archives={};expected=json.loads((ROOT/'parts.json').read_text())
for role,original in [('base','base'),('aligned','aligned'),('candidate','candidate'),('full_a','plus_a')]:
 source=parent/'out/binaries'/original/'qjs';assert sha(source)==parentmeta['images'][original]['binary_sha256'];target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True);shutil.copy2(source,target);paths[role]=target;metadata[role]={'binary_sha256':sha(target),'parent_role':original,'canonical_recipe':True}
archive=out/'old.tar'
with archive.open('wb') as f:subprocess.run(['git','archive',parts.OLD],stdout=f,check=True)
for role,selected in {**parts.ROLES,'aligned':(),'full_a':parts.PARTS}.items():
 source=out/('source-'+role);source.mkdir();archives[role]=source
 with tarfile.open(archive) as f:f.extractall(source,filter='data')
 hashes={}
 for path in parts.FILES:
  content=parts.source(path,selected);target=source/path
  if not content:target.unlink(missing_ok=True)
  else:target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(content)
  hashes[path]=hashlib.sha256(content).hexdigest()
 assert hashes==expected['roles'][role]['files']
 if role not in parts.ROLES:continue
 log=out/(role+'-build.log')
 with log.open('w') as err:binary=Path(subprocess.check_output([sys.executable,'-m','tools.benchmark.build_cache','build-rust','--source',str(source),'--target-dir','target/t056-header-build'],text=True,stderr=err).strip())
 target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True);shutil.copy2(binary,target);paths[role]=target;metadata[role]={'parts':selected,'files_sha256':hashes,'binary_sha256':sha(target),'canonical_recipe':True,'build_log_sha256':sha(log)}
 (out/'images-partial.json').write_text(json.dumps(metadata,indent=2)+'\n');print('BUILT',role,sha(target),flush=True)
work=out/'work';work.mkdir();cases=s.resolve_cases(['broad/global_read','broad/local_read'],s.DEFAULT_CACHE,work);checks={};audits={}
for role,binary in paths.items():
 for case in cases:
  for n in (23068672,46137344):
   q=subprocess.run([str(binary),'--raw',str(case.workload),case.case_id,str(n)],capture_output=True,text=True,check=True,timeout=120);value=s._check_output(case,q.stdout);key=case.id+':'+str(n)
   if key in checks:assert checks[key]==value
   else:checks[key]=value
 audit=out/'audits'/role
 with (out/(role+'-audit.log')).open('w') as log:subprocess.run([sys.executable,str(ROOT/'audit.py'),'--base',str(old),'--candidate',str(binary),'--output-dir',str(audit)],stdout=log,stderr=subprocess.STDOUT,check=True)
 a=json.loads((audit/'byte-audit.json').read_text());audits[role]={'all_equivalent':a['all_equivalent'],'comparison':a['comparison'],'raw_audit_sha256':sha(audit/'byte-audit.json'),'records':a['records']};print('AUDIT',role,a['all_equivalent'],flush=True)
identity={'diagnostic_only':True,'timing':False,'images':metadata,'audits':audits,'recipe':RUST_RECIPE,'parts_sha256':sha(ROOT/'parts.json'),'all_outputs_match':True,'checksums':checks,'producer_sha256':{name:sha(ROOT/name) for name in ('parts.py','bisect_partition.py','build.py','audit.py','extract-paths.py')},'parent_images_sha256':sha(parent/'out/images.json'),'toolchain':subprocess.check_output(['rustc','-vV'],text=True)}
(out/'images.json').write_text(json.dumps(identity,indent=2)+'\n');print('IMAGES_READY',flush=True)
# Extra diagnostic compiler flag, separate target and outputs. Never substitutes
# for the original-recipe images above. Source structs stay uninstrumented.
layouts={}
for role,source in archives.items():
 env={**os.environ,**RUST_RECIPE['environment'],'RUSTC_BOOTSTRAP':'1'};env['CARGO_ENCODED_RUSTFLAGS']+='\x1f-Zprint-type-sizes';env['CARGO_TARGET_DIR']=str(Path('target/t056-header-layout').resolve());env['CARGO_BUILD_TARGET']='x86_64-apple-darwin'
 log=out/(role+'-layout.log')
 with log.open('w') as stream:subprocess.run(['cargo','build',*RUST_RECIPE['cargo_args']],cwd=source,env=env,stdout=stream,stderr=subprocess.STDOUT,check=True)
 text=log.read_text();blocks=re.split(r'(?=^print-type-size type: )',text,flags=re.M);wanted=[b for b in blocks if re.match(r'print-type-size type: `(?:value::object::ObjectData|.*RcInner<value::object::ObjectData>)`:',b)]
 assert wanted,role;layouts[role]={'log_sha256':sha(log),'blocks':[b.split('\nprint-type-size type:',1)[0] for b in wanted]};print('LAYOUT',role,flush=True)
(out/'layouts.json').write_text(json.dumps({'diagnostic_only':True,'extra_flag':'-Zprint-type-sizes; never timed','roles':layouts},indent=2)+'\n');print('READY',len(paths),flush=True)
