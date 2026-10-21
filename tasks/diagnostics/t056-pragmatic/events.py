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
diag=source/'crates/qjs-runtime/src/diagnostics.rs';text=diag.read_text().replace('declare_counters! {','declare_counters! {\n    diag_layout_key_compares,\n    diag_layout_index_builds,\n    diag_layout_index_fields,',1);diag.write_text(text)
layout=source/'crates/qjs-runtime/src/property/layout.rs';text=layout.read_text().replace('fn text_matches(text: &str, key: &str) -> bool {','fn text_matches(text: &str, key: &str) -> bool {\n    crate::diagnostics::count!(diag_layout_key_compares);',1).replace('fn build_index(fields: &[Field]) -> Box<NameMap<Rc<str>, u32>> {','fn build_index(fields: &[Field]) -> Box<NameMap<Rc<str>, u32>> {\n    crate::diagnostics::count!(diag_layout_index_builds);\n    crate::diagnostics::count!(diag_layout_index_fields, fields.len());',1);layout.write_text(text)
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

from tools.benchmark.screen import resolve_cases,DEFAULT_CACHE
work=out/'event-work';work.mkdir();case=resolve_cases(['external/sunspider-1.0/string-unpack-code'],DEFAULT_CACHE,work)[0]
rows={}
for name,args in {'empty':['--raw','-e','0;'],'unpack':['--raw',str(case.workload)]}.items():
 result=subprocess.run([str(binary),*args],text=True,capture_output=True,timeout=120,check=True)
 (out/(name+'-events.stderr')).write_text(result.stderr)
 rows[name]={m[1]:int(m[2]) for line in result.stderr.splitlines() if (m:=re.fullmatch(r'\s+(\w+) (\d+)',line))}
(out/'events.json').write_text(json.dumps({'diagnostic_only':True,'timing':False,'candidate':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'events':rows},indent=2)+'\n')
print('FINAL EVENTS',json.dumps({n:{k:v for k,v in row.items() if k.startswith(('diag_','property_shape'))} for n,row in rows.items()}),flush=True)
