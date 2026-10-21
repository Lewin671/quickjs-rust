"""Build/qualify diagnostic-only host allocation perturbation images; never time."""
import os,subprocess,tarfile,json,hashlib,shutil
from pathlib import Path
from tools.benchmark.screen import resolve_cases,DEFAULT_CACHE
out=Path(os.environ['BENCH_OUT']);work=out/'work';work.mkdir()
versions={'aligned':'8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a','allocation':'dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'}
meta={'diagnostic_only':True,'runtime_source_unchanged':True,'pad_count':64,'pad_modes':{'pad0128':128,'pad0144':144,'pad1536':1536,'pada128':128},'images':{},'qualification':[]}
for side,revision in versions.items():
 source=out/('source-'+side);source.mkdir()
 with (out/(side+'.tar')).open('wb') as f:subprocess.run(['git','archive',revision],stdout=f,check=True)
 with tarfile.open(out/(side+'.tar')) as f:f.extractall(source,filter='data')
 path=source/'crates/qjs-cli/src/main.rs';s=path.read_text();a=s.index('fn main() -> ExitCode {');b=s.index('\n#[derive(Debug)]',a);old=s[a:b]
 assert '    match run() {' in old and old.rstrip().endswith('}')
 block=old.replace('fn main() -> ExitCode {','fn main() -> ExitCode {\n    let unrelated = arch_unrelated_allocations();',1).replace('    match run() {','    let result = match run() {',1)
 end=block.rfind('\n}')
 block=block[:end].rstrip()+';\n    std::hint::black_box(&unrelated);\n    result\n}\n'
 block+='''
// Isolated diagnostic host only; no engine representation or operation changes.
fn arch_unrelated_allocations() -> (std::ffi::OsString, Vec<Box<[u8]>>) {
    let mode=std::env::var_os("QJS_ARCH_PAD_MODE").unwrap_or_else(|| "pad0128".into());
    let size=match mode.as_encoded_bytes() {
      b"pad0128"|b"pada128"=>128,b"pad0144"=>144,b"pad1536"=>1536,
      _=>{eprintln!("invalid QJS_ARCH_PAD_MODE");std::process::exit(2)}
    };
    let mut blocks=Vec::with_capacity(64);
    for _ in 0..64 {blocks.push(vec![0_u8;size].into_boxed_slice());}
    std::hint::black_box(&blocks);
    (mode,blocks)
}
'''
 path.write_text(s[:a]+block+s[b:])
 original_runtime=subprocess.check_output(['git','ls-tree','-r',revision,'--','crates/qjs-runtime'],text=True)
 # Only the CLI main file was touched; keep complete source and patch for audit.
 (out/(side+'-main-original.rs')).write_text(s)
 binary=Path(subprocess.check_output(['python3','-m','tools.benchmark.build_cache','build-rust','--source',str(source),'--target-dir','target/t056-architecture-perturb-build'],text=True).strip())
 target=out/'binaries'/side/'qjs';target.parent.mkdir(parents=True);shutil.copy2(binary,target)
 meta['images'][side]={'revision':revision,'sha256':hashlib.sha256(target.read_bytes()).hexdigest(),'host_source_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'runtime_tree_sha256':hashlib.sha256(original_runtime.encode()).hexdigest()}
 print('BUILT PERTURBATION',side,meta['images'][side]['sha256'],flush=True)
 for mode in meta['pad_modes']:
  env={**os.environ,'QJS_ARCH_PAD_MODE':mode,'QJS_CLI_BIN':str(target),'QJS_CHECK_SKIP_TEST262':'0'}
  if mode!='pada128':
   log=out/(side+'-'+mode+'-ng.log')
   with log.open('w') as f:subprocess.run(['./scripts/compare-qjs.sh'],env=env,stdout=f,stderr=subprocess.STDOUT,check=True)
   assert sum(line.startswith('ok: ') for line in log.read_text().splitlines())==255
  outputs={}
  for case in resolve_cases(['broad/global_read','broad/local_read','broad/branch_arithmetic','sentinel/heterogeneous_property_read','external/sunspider-1.0/string-unpack-code'],DEFAULT_CACHE,work):
   argv=[str(target),'--raw',str(case.workload)]
   if case.kind=='internal':argv.extend([case.case_id,'1000'])
   output=subprocess.check_output(argv,env=env,text=True,timeout=90);outputs[case.id]=output
  assert subprocess.check_output([str(target),'--raw','-e','0;'],env=env,text=True).strip()=='0'
  meta['qualification'].append({'side':side,'mode':mode,'ng_fixtures':255 if mode!='pada128' else None,'outputs':outputs})
  print('QUALIFIED',side,mode,flush=True)
 # Per-side stdout must match for all factors; canonical output contract is pinned by existing cases.
 group=[r['outputs'] for r in meta['qualification'] if r['side']==side];assert all(r==group[0] for r in group)
assert all(r['outputs']==meta['qualification'][0]['outputs'] for r in meta['qualification'])
meta['all_outputs_match']=True
(out/'pad-build.json').write_text(json.dumps(meta,indent=2)+'\n')
