"""Non-timing event snapshot at the end of intrinsic installation, archive only."""
import os,shutil,subprocess,json,hashlib,re
from pathlib import Path
from tools.benchmark.build_cache import RUST_RECIPE
out=Path(os.environ['BENCH_OUT']);previous=Path(os.environ['BENCH_QUEUE'])/'20261021T092319Z-t056-startup-event-attribution'
assert (previous/'status').read_text().strip()=='done 0'
source=out/'source';shutil.copytree(previous/'out/source-allocation',source)
p=source/'crates/qjs-runtime/src/builtins.rs';s=p.read_text();key='    generator::install_generator(env, global_this, object_prototype);';assert s.count(key)==1
s=s.replace(key,key+'\n    for (name,value) in crate::counters().unwrap().entries() { eprintln!("BOOTSTRAP {} {}",name,value); }');p.write_text(s)
env={**os.environ,**RUST_RECIPE['environment'],'CARGO_BUILD_TARGET':'x86_64-apple-darwin','CARGO_TARGET_DIR':str(Path.cwd()/'target/t056-stack-counters-build')};args=list(RUST_RECIPE['cargo_args']);args[args.index('agents')]='agents,perf-counters'
subprocess.run(['cargo','build',*args],cwd=source,env=env,check=True)
binary=Path(env['CARGO_TARGET_DIR'])/'x86_64-apple-darwin/release/qjs';result=subprocess.run([str(binary),'--raw','-e','0;'],text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True)
(out/'stderr.txt').write_text(result.stderr);(out/'stdout.txt').write_text(result.stdout)
boot={m[1]:int(m[2]) for line in result.stderr.splitlines() if (m:=re.fullmatch(r'BOOTSTRAP (\w+) (\d+)',line))};final={m[1]:int(m[2]) for line in result.stderr.splitlines() if (m:=re.fullmatch(r'\s+(\w+) (\d+)',line))}
(out/'bootstrap.json').write_text(json.dumps({'revision':'dfff3bb6bf3db01fb52a7612ed1144900b5f08c5','diagnostic_only':True,'event_counts_only':True,'image_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'at_end_initialize_builtins':boot,'final':final},indent=2)+'\n')
