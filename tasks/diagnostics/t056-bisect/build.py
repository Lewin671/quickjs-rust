"""Non-timing source ablations on archives only. No canonical cache mutation."""
import os,subprocess,tarfile,json,hashlib,shutil,sys
from pathlib import Path
import partition
from tools.benchmark.build_cache import RUST_RECIPE
from tools.benchmark import screen as s
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text()
ROOT=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
rows=partition.inventory();groups=partition.verify(rows);expected=json.loads((ROOT/'hunks.json').read_text());assert rows==expected['files'] and groups==expected['groups']
cache=Path('target/perf-loop/compiler-fat16-abort-v1');old=cache/('rev-'+partition.OLD)/'qjs';new=cache/('rev-'+partition.NEW)/'qjs'
canonical={'aligned':(old,'b0b84489a3f6fd673b06dc4bdc71c8f176387a231ca19fe1da5ea48ca4ee0fd5'),'candidate':(new,'3466c37a62f2fb6d764d0123212749ff810138ca05fcad336ce1afa05bdb1e3f'),'base':(cache/'rev-f58c696e8379261ed726f0cea1051ed6d34f431a/qjs','b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e')}
metadata={};paths={};snapshot=out/'old.tar'
with snapshot.open('wb') as f:subprocess.run(['git','archive',partition.OLD],stdout=f,check=True)
for role,(source,expected_hash) in canonical.items():
 assert sha(source)==expected_hash;target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True);shutil.copy2(source,target);paths[role]=target
 metadata[role]={'canonical':True,'revision':partition.OLD if role=='aligned' else partition.NEW if role=='candidate' else 'f58c696e8379261ed726f0cea1051ed6d34f431a','binary_sha256':sha(target)}
for role,selected in partition.ROLES.items():
 source=out/('source-'+role);source.mkdir()
 with tarfile.open(snapshot) as f:f.extractall(source,filter='data')
 changed={}
 for row in rows:
  if not row['release_input']:continue
  content=partition.materialize(row,selected);path=source/row['path']
  if not content and row['path'].endswith('/flags.rs'):path.unlink(missing_ok=True)
  else:path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(content)
  changed[row['path']]=partition.digest(content)
 log=out/(role+'-build.log')
 with log.open('w') as err:
  binary=Path(subprocess.check_output([sys.executable,'-m','tools.benchmark.build_cache','build-rust','--source',str(source),'--target-dir','target/t056-bisect-build'],text=True,stderr=err).strip())
 target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True);shutil.copy2(binary,target);paths[role]=target
 metadata[role]={'canonical':False,'base':partition.OLD,'selected_groups':selected,'files_sha256':changed,'binary_sha256':sha(target),'build_log_sha256':sha(log),'recipe':RUST_RECIPE}
 (out/'images-partial.json').write_text(json.dumps(metadata,indent=2)+'\n');print('BUILT',role,metadata[role]['binary_sha256'],flush=True)
# Canonical recipe, no binary relink/patch/resign and no padding search.
work=out/'work';work.mkdir();cases=s.resolve_cases(['broad/global_read','broad/local_read'],s.DEFAULT_CACHE,work);checks={};audits={}
semantic='let x={};for(let i=0;i<20;i++)x["f"+i]=i; x.f19=21; x["4294967294"]=2; delete x["4294967294"]; Object.preventExtensions(x); let y={a:1}; Object.freeze(y); if(x.f19!==21||Object.isExtensible(x)||!Object.isFrozen(y)||Object.keys(x).length!==20)throw 1; 1;'
for role,binary in paths.items():
 for case in cases:
  for n in (23068672,46137344):
   result=subprocess.run([str(binary),'--raw',str(case.workload),case.case_id,str(n)],capture_output=True,text=True,check=True,timeout=120);check=s._check_output(case,result.stdout);key=case.id+':'+str(n)
   if key in checks:assert check==checks[key]
   else:checks[key]=check
 q=subprocess.run([str(binary),'--raw','-e',semantic],capture_output=True,text=True,check=True,timeout=30);assert q.stdout.strip()=='1'
 audit=out/'audits'/role
 with (out/(role+'-audit.log')).open('w') as log:subprocess.run([sys.executable,str(ROOT/'audit.py'),'--base',str(old),'--candidate',str(binary),'--output-dir',str(audit)],stdout=log,stderr=subprocess.STDOUT,check=True)
 full=json.loads((audit/'byte-audit.json').read_text());audits[role]={'all_equivalent':full['all_equivalent'],'comparison':full['comparison'],'raw_audit_sha256':sha(audit/'byte-audit.json'),'placement':{k:{n:v for n,v in r.items() if n not in ('paths','branches')} for k,r in full['records'].items()},'candidate_paths':{name:[{k:v for k,v in row.items() if k in ('offset','raw_hex','normalized_hex','relocations')} for row in path] for name,path in full['records']['candidate']['paths'].items()}}
 print('AUDIT',role,full['all_equivalent'],flush=True)
identity={'diagnostic_only':True,'timing':False,'images':metadata,'audits':audits,'hunks_sha256':sha(ROOT/'hunks.json'),'producer_sha256':{name:sha(ROOT/name) for name in ('build.py','partition.py','audit.py','extract-paths.py')},'recipe':RUST_RECIPE,'toolchain':{tool:subprocess.check_output([tool,flag],text=True).strip() for tool,flag in [('rustc','-vV'),('cargo','-V')]},'all_outputs_match':True,'checksums':checks,'attempts_consumed':0}
(out/'images.json').write_text(json.dumps(identity,indent=2)+'\n');print('READY',len(metadata),{r:a['all_equivalent'] for r,a in audits.items()},flush=True)
