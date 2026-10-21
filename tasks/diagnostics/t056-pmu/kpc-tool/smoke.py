"""Short installed-v2 root setup probes, ONLY in a no-timing runner job."""
import hashlib,json,os,subprocess
from pathlib import Path
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text()
tool=Path('/usr/local/libexec/qjs-kpc');expected=json.loads((out/'config-v2.json').read_text())['helper_sha256']
assert hashlib.sha256(tool.read_bytes()).hexdigest()==expected,'user must install the exact reviewed v2 digest first'
rows=[]
for fixed,events in ((False,'cycles,instructions'),(False,'cycles,instructions,IDQ.DSB_UOPS'),(True,'cycles,instructions')):
 argv=['/usr/bin/sudo','-n','--',str(tool),*(['--fixed-diagnostic'] if fixed else []),'--events',events,'--timeout-ms','1000','--','/usr/bin/id','-u']
 p=subprocess.run(argv,text=True,capture_output=True,timeout=5);v=json.loads(p.stdout);rows.append({'argv':argv,'exit':p.returncode,'value':v,'stderr':p.stderr})
 (out/'smoke.json').write_text(json.dumps({'timing':False,'helper_sha256':expected,'rows':rows,'hardware_counts_claimed':False},indent=2)+'\n');print(json.dumps(rows[-1]),flush=True)
 if not v.get('cleanup_ok'):raise RuntimeError('cleanup failed: stop, do not reset global state or run another probe')
 if v['capture_complete']:assert v['child_stdout'].strip()==str(os.getuid()) and v['child_status']==0
