set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
python3 - <<'PY'
import os,subprocess,tarfile,re,json
from pathlib import Path
out=Path(os.environ['BENCH_OUT']);data={}
for role,rev in [('base','f58c696e8379261ed726f0cea1051ed6d34f431a'),('candidate','8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a')]:
 source=out/role;source.mkdir()
 archive=out/(role+'.tar')
 with archive.open('wb') as f:subprocess.run(['git','archive',rev],stdout=f,check=True)
 with tarfile.open(archive) as f:f.extractall(source,filter='data')
 env={**os.environ,'RUSTC_BOOTSTRAP':'1','CARGO_INCREMENTAL':'0','CARGO_ENCODED_RUSTFLAGS':'-Csymbol-mangling-version=v0\x1f-Cllvm-args=-align-all-functions=4\x1f-Zprint-type-sizes'}
 cmd=['cargo','build','--locked','--release','--manifest-path',str(source/'Cargo.toml'),'--target','x86_64-apple-darwin','--target-dir',str(Path('target/m1a-allocation-layout-build').resolve()),'-p','qjs-cli','--features','agents','--config=profile.release.opt-level=3','--config=profile.release.lto="fat"','--config=profile.release.codegen-units=16','--config=profile.release.panic="abort"']
 with (out/(role+'.sizes.log')).open('w') as f:subprocess.run(cmd,env=env,stdout=f,stderr=subprocess.STDOUT,check=True)
 rows={}
 for line in (out/(role+'.sizes.log')).read_text().splitlines():
  m=re.match(r'print-type-size type: `(.*)`: (\d+) bytes, alignment: (\d+) bytes',line)
  if m:rows[m[1]]={'size':int(m[2]),'align':int(m[3])}
 data[role]=rows
 print(role,'types',len(rows),flush=True)
(out/'type-sizes.json').write_text(json.dumps(data,indent=2)+'\n')
diff={k:{'base':data['base'].get(k),'candidate':data['candidate'].get(k)} for k in set(data['base'])|set(data['candidate']) if data['base'].get(k)!=data['candidate'].get(k)}
(out/'type-changes.json').write_text(json.dumps(diff,indent=2,sort_keys=True)+'\n')
for k,v in sorted(diff.items()):
 if any(x in k for x in ['ObjectData','PropertyStorage','ObjectLiteralShape','StringData','RcInner','LayoutNode','SharedSlot']):print(k,v)
PY
