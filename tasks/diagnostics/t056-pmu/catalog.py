"""Read local event catalog and read-only PMU getters; never configure counters."""
import os,json,ctypes,ctypes.util,plistlib,subprocess,errno
from pathlib import Path
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text()
path=Path('/usr/share/kpep/cometlake.plist');data=plistlib.loads(path.read_bytes());(out/'cometlake-catalog.json').write_text(json.dumps(data,indent=2)+'\n')
lib=ctypes.CDLL('/System/Library/PrivateFrameworks/kperf.framework/kperf',use_errno=True);r={}
for name,args in [('kpc_get_thread_counters',(0,32,(ctypes.c_uint64*32)())),('kpc_force_all_ctrs_get',(ctypes.pointer(ctypes.c_int()),))]:
 fn=getattr(lib,name);ctypes.set_errno(0);ret=fn(*args);r[name]={'return':ret,'errno':ctypes.get_errno()}
libc=ctypes.CDLL(ctypes.util.find_library('proc'),use_errno=True);buffer=(ctypes.c_ubyte*4096)();libc.proc_pid_rusage.argtypes=[ctypes.c_int,ctypes.c_int,ctypes.c_void_p];ctypes.set_errno(0);ret=libc.proc_pid_rusage(os.getpid(),4,ctypes.byref(buffer));r['self_rusage_v4']={'return':ret,'errno':ctypes.get_errno()}
(out/'rusage-self.bin').write_bytes(bytes(buffer));(out/'read-only-getters.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))
for cmd in [['/usr/bin/csrutil','status'],['/usr/bin/find','/Applications','-maxdepth','3','-name','xctrace']]:
 q=subprocess.run(cmd,text=True,capture_output=True);print(cmd,q.returncode,q.stdout,q.stderr)
