"""Canonical-recipe, non-timing address capture on isolated diagnostic archives."""
import os,subprocess,tarfile,json,hashlib,shutil,gzip
from pathlib import Path
from tools.benchmark.screen import resolve_cases,DEFAULT_CACHE
from tools.benchmark.build_cache import RUST_RECIPE
out=Path(os.environ['BENCH_OUT']);work=out/'work';work.mkdir()
roles={'base':('f58c696e8379261ed726f0cea1051ed6d34f431a',112),'aligned':('8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a',120),'allocation':('dfff3bb6bf3db01fb52a7612ed1144900b5f08c5',112)}
identity={'diagnostic_only':True,'timing':False,'struct_layout_fields_unchanged':True,'trace_position':'after typed loop, after wide run, before recycler','caveat':'new code and post-print allocations can perturb subsequent entries; these are not canonical-image addresses or latency evidence','roles':{},'runs':[]}
for role,(revision,obj_size) in roles.items():
 source=out/('source-'+role);source.mkdir()
 archive=out/(role+'.tar')
 with archive.open('wb') as f:subprocess.run(['git','archive',revision],stdout=f,check=True)
 with tarfile.open(archive) as f:f.extractall(source,filter='data')
 subprocess.run(['python3',str(out/'instrument-supplement.py'),str(source)],check=True)
 with (source/'crates/qjs-runtime/src/value/object.rs').open('a') as f:f.write('\nconst _:()=assert!(std::mem::size_of::<ObjectData>()=='+str(obj_size)+');\n')
 with (source/'crates/qjs-runtime/src/bytecode/typed_loop/mod.rs').open('a') as f:f.write('\nconst _:()={ assert!(std::mem::size_of::<RegisterFile>()==8);assert!(std::mem::size_of::<TypedLoopScratch>()==80); };\n')
 build=Path(subprocess.check_output(['python3','-m','tools.benchmark.build_cache','build-rust','--source',str(source),'--target-dir','target/t056-arch-capture-build'],text=True).strip())
 binary=out/'binaries'/role/'qjs';binary.parent.mkdir(parents=True);shutil.copy2(build,binary)
 identity['roles'][role]={'revision':revision,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'recipe':RUST_RECIPE,'object_data_bytes':obj_size,'register_owner_bytes':8,'scratch_bytes':80}
 print('BUILT',role,identity['roles'][role]['binary_sha256'],flush=True)
 active=out/'active/qjs';active.parent.mkdir(exist_ok=True);shutil.copy2(binary,active)
 cases=resolve_cases(['broad/global_read','broad/local_read','external/sunspider-1.0/string-unpack-code'],DEFAULT_CACHE,work)
 for case in cases:
  # Use the actual dfff triage N and 2N to expose phase/input differences too.
  ns=[1000] if case.kind=='internal' else [None]
  for n in ns:
   for repeat in range(1):
    tag=role+'-'+case.id.replace('/','--')+'-'+str(n)+'-'+str(repeat)
    path=out/(tag+'.stderr.gz')
    with (out/(tag+'.stdout')).open('wb') as so,path.open('wb') as zipped:
     compressor=subprocess.Popen(['gzip','-c'],stdin=subprocess.PIPE,stdout=zipped)
     argv=[str(active),'--raw',str(case.workload)]
     if case.kind=='internal':argv.extend([case.case_id,str(n)])
     try:result=subprocess.run(argv,stdout=so,stderr=compressor.stdin,timeout=180)
     finally:compressor.stdin.close();assert compressor.wait(timeout=30)==0
    assert result.returncode==0,(tag,result.returncode)
    identity['runs'].append({'role':role,'case':case.id,'iterations':n,'repeat':repeat,'stderr':path.name,'stderr_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'stdout':tag+'.stdout','argv':argv})
    print('CAPTURED',tag,'gzip_bytes',path.stat().st_size,flush=True)
 (out/'capture-identity.json').write_text(json.dumps(identity,indent=2)+'\n')
subprocess.run(['/usr/sbin/sysctl','hw.l1dcachesize','hw.cachelinesize','machdep.cpu.family','machdep.cpu.model','machdep.cpu.stepping'],stdout=(out/'host-caches.txt').open('w'),check=True)
