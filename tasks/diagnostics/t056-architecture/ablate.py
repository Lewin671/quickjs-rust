"""Non-timing instruction attribution. All source edits stay in private archives."""
import os,subprocess,tarfile,json,hashlib,shutil,statistics
from pathlib import Path
out=Path(os.environ['BENCH_OUT']);prefix='crates/qjs-runtime/src/';old='8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a';new='dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'
cache=Path('target/perf-loop/compiler-fat16-abort-v1')
reference=cache/('rev-'+old)/'qjs';full=cache/('rev-'+new)/'qjs'
assert hashlib.sha256(reference.read_bytes()).hexdigest()=='b0b84489a3f6fd673b06dc4bdc71c8f176387a231ca19fe1da5ea48ca4ee0fd5'
assert hashlib.sha256(full.read_bytes()).hexdigest()=='3466c37a62f2fb6d764d0123212749ff810138ca05fcad336ce1afa05bdb1e3f'
roles={'full':full.resolve()};metadata={}
for role in ['header','adapter','write_index']:
 source=out/('source-'+role);source.mkdir()
 with (out/(role+'.tar')).open('wb') as f:subprocess.run(['git','archive',old],stdout=f,check=True)
 with tarfile.open(out/(role+'.tar')) as f:f.extractall(source,filter='data')
 files={'header':['value/object.rs','value/object/flags.rs','value/object/class_fields.rs'],'adapter':['bytecode/typed_loop/execute.rs'],'write_index':['property/layout.rs','value/object/storage.rs']}[role]
 for name in files:
  p=source/prefix/name;p.parent.mkdir(exist_ok=True,parents=True);p.write_bytes(subprocess.check_output(['git','show',new+':'+prefix+name]))
 p=source/prefix/'value/object.rs';s=p.read_text()
 if role=='header':
  assert s.count('storage.lookup_for_write(key)')==2;s=s.replace('storage.lookup_for_write(key)','storage.lookup(key)')
 if role=='write_index':
  assert s.count('match storage.lookup(key) {')==1 and s.count('let slot = storage.lookup(key)?;')==1
  s=s.replace('match storage.lookup(key) {','match storage.lookup_for_write(key) {').replace('let slot = storage.lookup(key)?;','let slot = storage.lookup_for_write(key)?;');files.append('value/object.rs')
 p.write_text(s)
 binary=Path(subprocess.check_output(['python3','-m','tools.benchmark.build_cache','build-rust','--source',str(source),'--target-dir','target/t056-arch-ablation-build'],text=True).strip());target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True);shutil.copy2(binary,target);roles[role]=target
 metadata[role]={'base':old,'changes_from':new,'files':{name:hashlib.sha256((source/prefix/name).read_bytes()).hexdigest() for name in files},'binary_sha256':hashlib.sha256(target.read_bytes()).hexdigest()}
 print('BUILT',role,metadata[role]['binary_sha256'],flush=True)
for role,binary in roles.items():
 args=['python3','-m','tools.benchmark.screen','--base',str(reference.resolve()),'--candidate',str(binary.resolve()),'--pairs','3','--symbols','20','--json',str(out/(role+'.json'))]
 for case in ['external/sunspider-1.0/string-unpack-code','broad/string_slice','external/sunspider-1.0/math-spectral-norm','broad/global_read']:args.extend(['--case',case])
 with (out/(role+'.stdout')).open('w'),(out/(role+'.stderr')).open('w') as err:
  subprocess.run(args,stdout=open(out/(role+'.stdout'),'w'),stderr=err,check=True)
 r=json.loads((out/(role+'.json')).read_text());print(role,{c['id']:statistics.median(c['ratios']['instructions']) for c in r['cases']},flush=True)
(out/'ablation-identity.json').write_text(json.dumps({'scope':'instructions only, no acceptance; source changes include downstream LLVM codegen and padding effects','roles':metadata,'reference':old,'full':new},indent=2)+'\n')
