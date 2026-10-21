"""Qualify original-recipe images from initial job; fresh per-role layout probes."""
import os,json,hashlib,shutil,re,subprocess
from pathlib import Path
from tools.benchmark.build_cache import RUST_RECIPE
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text();ROOT=Path(__file__).resolve().parent
old=Path(os.environ['BENCH_QUEUE'])/'20261021T115238Z-t056-header-subparts-build';assert (old/'status').read_text().strip()=='done 1';assert 'AssertionError: flags' in (old/'log').read_text()
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
identity=json.loads((old/'out/images.json').read_text());assert identity['all_outputs_match'] and len(identity['images'])==10 and all(a['all_equivalent'] for a in identity['audits'].values())
assert identity['producer_sha256']['build.py']==sha(ROOT/'build-initial.py')
for role,m in identity['images'].items():
 source=old/'out/binaries'/role/'qjs';assert sha(source)==m['binary_sha256'];target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True);shutil.copy2(source,target)
shutil.copy2(old/'out/images.json',out/'images.json');manifest=json.loads((ROOT/'parts.json').read_text());layouts={}
for role,m in manifest['roles'].items():
 source=old/'out'/('source-'+role)
 for name,h in m['files'].items():
  path=source/name;assert (sha(path) if path.exists() else hashlib.sha256(b'').hexdigest())==h
 env={**os.environ,**RUST_RECIPE['environment'],'RUSTC_BOOTSTRAP':'1'};env['CARGO_ENCODED_RUSTFLAGS']+='\x1f-Zprint-type-sizes';env['CARGO_TARGET_DIR']=str((out/'layout-targets'/role).resolve());env['CARGO_BUILD_TARGET']='x86_64-apple-darwin';log=out/(role+'-layout.log')
 with log.open('w') as stream:subprocess.run(['cargo','build',*RUST_RECIPE['cargo_args']],cwd=source,env=env,stdout=stream,stderr=subprocess.STDOUT,check=True)
 blocks=re.split(r'(?=^print-type-size type: )',log.read_text(),flags=re.M);wanted=[]
 for block in blocks:
  match=re.match(r'print-type-size type: `((?:qjs_runtime::)?value::object::ObjectData|std::rc::RcInner<(?:qjs_runtime::)?value::object::ObjectData>)`: (\d+) bytes, alignment: (\d+) bytes',block)
  if match:wanted.append({'type':match[1],'size':int(match[2]),'align':int(match[3]),'text':block})
 object_sizes={x['size'] for x in wanted if not x['type'].startswith('std::')};assert len(object_sizes)==1,(role,wanted);assert any(x['type']=='value::object::ObjectData' for x in wanted),'runtime crate must freshly emit layout, CLI-only cached output is insufficient'
 layouts[role]={'size':object_sizes.pop(),'log_sha256':sha(log),'separate_fresh_target':str(Path('layout-targets')/role),'blocks':wanted};print('LAYOUT',role,layouts[role]['size'],flush=True)
 (out/'layouts-partial.json').write_text(json.dumps(layouts,indent=2)+'\n')
(out/'layouts.json').write_text(json.dumps({'diagnostic_only':True,'extra_flag':'-Zprint-type-sizes; fresh isolated target per role, never timed','initial_job':'20261021T115238Z-t056-header-subparts-build','initial_images_sha256':sha(out/'images.json'),'roles':layouts,'qualifier_sha256':sha(ROOT/'qualify.py')},indent=2)+'\n');print('READY',len(identity['images']),flush=True)
