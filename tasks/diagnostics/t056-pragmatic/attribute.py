"""Non-timing intrinsic installation event attribution in an isolated archive."""
import hashlib,json,os,re,shutil,subprocess,tarfile
from pathlib import Path
from tools.benchmark.build_cache import RUST_RECIPE
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text()
source=out/'source';source.mkdir()
with (out/'source.tar').open('wb') as stream:subprocess.run(['git','archive','HEAD'],stdout=stream,check=True)
with tarfile.open(out/'source.tar') as archive:archive.extractall(source,filter='data')
for path in (out/'input').rglob('*.rs'):
 target=source/path.relative_to(out/'input');target.write_bytes(path.read_bytes())
p=source/'crates/qjs-runtime/src/builtins.rs';s=p.read_text()
def stage(match):
 text=match[0];name=re.search(r'(?:crate::)?(\w+)::(\w+)',text);label=name[1]+'.'+name[2]
 return text+'\n    { let c=crate::counters().unwrap(); eprintln!("BOOT '+label+' {} {} {}", c.property_shapes_created,c.property_shape_fields_copied,c.property_add_probes); }'
s,n=re.subn(r'(?m)^    (?:let object_prototype = )?(?:crate::function|\w+)::\w+\([^;]*?\);',stage,s);assert n>=30;p.write_text(s)
env={**os.environ,**RUST_RECIPE['environment'],'CARGO_BUILD_TARGET':'x86_64-apple-darwin','CARGO_TARGET_DIR':str(Path.cwd()/'target/t056-pragmatic-events')}
args=list(RUST_RECIPE['cargo_args']);args[args.index('agents')]='agents,perf-counters'
subprocess.run(['cargo','build',*args],cwd=source,env=env,check=True)
binary=Path(env['CARGO_TARGET_DIR'])/'x86_64-apple-darwin/release/qjs'
p=subprocess.run([str(binary),'--raw','-e','0;'],text=True,capture_output=True,timeout=30,check=True)
(out/'stderr.txt').write_text(p.stderr);rows=[];previous=[0,0,0]
for line in p.stderr.splitlines():
 m=re.fullmatch(r'BOOT (\S+) (\d+) (\d+) (\d+)',line)
 if not m:continue
 values=list(map(int,m.groups()[1:]));rows.append({'stage':m[1],'shapes':values[0]-previous[0],'fields':values[1]-previous[1],'probes':values[2]-previous[2]});previous=values
(out/'attribution.json').write_text(json.dumps({'diagnostic_only':True,'timing':False,'instrumented_events_only':True,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'stages':rows},indent=2)+'\n')
print(json.dumps(sorted(rows,key=lambda r:r['fields'],reverse=True)[:12]),flush=True)
