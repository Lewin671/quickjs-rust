"""Short installed-v3 root setup probes, ONLY in a no-timing runner job."""
import hashlib,json,os,subprocess
from pathlib import Path
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text()
tool=Path('/usr/local/libexec/qjs-kpc');expected=json.loads((out/'config-v3.json').read_text())['helper_sha256']
assert hashlib.sha256(tool.read_bytes()).hexdigest()==expected,'user must install the exact reviewed v3 digest first'
rows=[]
for events in ('cycles,instructions','cycles,instructions,IDQ.DSB_UOPS'):
 argv=['/usr/bin/sudo','-n','--',str(tool),'--events',events,'--timeout-ms','1000','--','/usr/bin/id','-u']
 p=subprocess.run(argv,text=True,capture_output=True,timeout=8);v=json.loads(p.stdout);rows.append({'argv':argv,'exit':p.returncode,'value':v,'stderr':p.stderr})
 (out/'smoke.json').write_text(json.dumps({'timing':False,'helper_sha256':expected,'rows':rows,'hardware_counts_claimed':False},indent=2)+'\n');print(json.dumps(rows[-1]),flush=True)
 if not v.get('cleanup_ok'):raise RuntimeError('cleanup failed: stop, do not reset global state or run another probe')
 assert v['capture_complete'] and v['counters_valid'] and v['configuration_readback_verified'] and v['scope']=='system_wide_user_cpu_delta'
 assert v['child_stdout'].strip()==str(os.getuid()) and v['child_status']==0
 assert all(len(s['per_cpu'])==v['cpu_count'] for s in v['snapshots'].values())
