"""Non-timing instruction totals and event attribution in isolated archives."""
import os,subprocess,json,tarfile,hashlib,shutil,statistics,re
from pathlib import Path
from tools.benchmark.build_cache import RUST_RECIPE
from tools.benchmark.screen import counter_tool,sample,resolve_cases,DEFAULT_CACHE
out=Path(os.environ['BENCH_OUT']);work=out/'work';work.mkdir()
roles={'base':'f58c696e8379261ed726f0cea1051ed6d34f431a','aligned':'8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a','allocation':'dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'}
cases=resolve_cases(['external/sunspider-1.0/string-unpack-code'],DEFAULT_CACHE,work)
commands={'empty':['--raw','-e','0;'],'unpack':['--raw',str(cases[0].workload)],'cli_only':['--help']}
report={'diagnostic_only':True,'timing':False,'canonical_instructions':{},'counter_builds':{}}
for role,rev in roles.items():
 binary=Path('target/perf-loop/compiler-fat16-abort-v1')/('rev-'+rev)/'qjs'
 report['canonical_instructions'][role]={case:[sample(counter_tool(trust_cycles=False),[str(binary),*args],120).instructions for _ in range(3)] for case,args in commands.items()}
 source=out/('source-'+role);source.mkdir()
 archive=out/(role+'.tar')
 with archive.open('wb') as f:subprocess.run(['git','archive',rev],stdout=f,check=True)
 with tarfile.open(archive) as f:f.extractall(source,filter='data')
 diag=source/'crates/qjs-runtime/src/diagnostics.rs';s=diag.read_text().replace('declare_counters! {','declare_counters! {\n    diag_layout_key_compares,\n    diag_layout_index_builds,\n    diag_layout_index_fields,',1);diag.write_text(s)
 layout=source/'crates/qjs-runtime/src/property/layout.rs'
 if layout.exists():
  s=layout.read_text().replace('fn text_matches(text: &str, key: &str) -> bool {','fn text_matches(text: &str, key: &str) -> bool {\n    crate::diagnostics::count!(diag_layout_key_compares);',1)
  old='fn build_index(fields: &[Field]) -> Box<NameMap<Rc<str>, u32>> {';assert old in s;s=s.replace(old,old+'\n    crate::diagnostics::count!(diag_layout_index_builds);\n    crate::diagnostics::count!(diag_layout_index_fields, fields.len());',1);layout.write_text(s)
 env={**os.environ,**RUST_RECIPE['environment'],'CARGO_BUILD_TARGET':'x86_64-apple-darwin','CARGO_TARGET_DIR':str(Path.cwd()/'target/t056-stack-counters-build')}
 args=list(RUST_RECIPE['cargo_args']);args[args.index('agents')]='agents,perf-counters'
 subprocess.run(['cargo','build',*args],cwd=source,env=env,check=True)
 binary=Path(env['CARGO_TARGET_DIR'])/'x86_64-apple-darwin/release/qjs';target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True);shutil.copy2(binary,target)
 counters={}
 for case,args in commands.items():
  result=subprocess.run([str(target),*args],text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True,timeout=120)
  (out/(role+'-'+case+'.stdout')).write_text(result.stdout);(out/(role+'-'+case+'.stderr')).write_text(result.stderr)
  counters[case]={m[1]:int(m[2]) for line in result.stderr.splitlines() if (m:=re.fullmatch(r'\s+(\w+) (\d+)',line))}
 report['counter_builds'][role]={'revision':rev,'image_sha256':hashlib.sha256(target.read_bytes()).hexdigest(),'counters':counters,'caveat':'extra counters and feature change codegen; event counts only, never canonical instruction/wall attribution'}
 (out/'startup.json').write_text(json.dumps(report,indent=2)+'\n')
 print('COUNTERS',role,{case:{k:v for k,v in c.items() if v and (k.startswith(('property_','diag_')))} for case,c in counters.items()},flush=True)
