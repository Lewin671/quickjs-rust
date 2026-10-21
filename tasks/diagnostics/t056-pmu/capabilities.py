"""Read-only iMac tool/permission inventory; never programs PMU or records a task."""
import os,json,subprocess,ctypes,glob,platform,hashlib
from pathlib import Path
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text()
report={'timing':False,'read_only':True,'platform':platform.platform(),'commands':{},'libraries':{}}
for name,argv in {'xcode':['xcode-select','-p'],'xctrace':['xcrun','--find','xctrace'],'templates':['xcrun','xctrace','list','templates'],'sudo_noninteractive':['sudo','-n','/usr/bin/id','-u'],'security':['/usr/bin/csrutil','status'],'cpu':['/usr/sbin/sysctl','machdep.cpu.brand_string','machdep.cpu.family','machdep.cpu.model','machdep.cpu.stepping']}.items():
 try:
  r=subprocess.run(argv,capture_output=True,text=True,timeout=25);report['commands'][name]={'argv':argv,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr}
 except (OSError,subprocess.TimeoutExpired) as e:report['commands'][name]={'argv':argv,'error':str(e)}
for name,path in [('kperf','/System/Library/PrivateFrameworks/kperf.framework/kperf'),('kperfdata','/System/Library/PrivateFrameworks/kperfdata.framework/kperfdata')]:
 try:
  lib=ctypes.CDLL(path);row={'loaded':True,'symbols':{s:hasattr(lib,s) for s in (['kpc_pmu_version','kpc_get_counting','kpc_get_thread_counting','kpc_get_counter_count','kpc_get_config_count','kpc_get_thread_counters','kpc_set_config','kpc_set_counting','kpc_set_thread_counting','kpc_force_all_ctrs_set','kpc_force_all_ctrs_get'] if name=='kperf' else ['kpep_db_create','kpep_db_free','kpep_db_events_count','kpep_db_events','kpep_config_create','kpep_config_add_event','kpep_config_kpc','kpep_config_kpc_map'])}}
  if name=='kperf':
   for symbol in ['kpc_pmu_version','kpc_get_counting','kpc_get_thread_counting']:
    fn=getattr(lib,symbol);fn.restype=ctypes.c_uint32;row[symbol]=fn()
   for symbol in ['kpc_get_counter_count','kpc_get_config_count']:
    fn=getattr(lib,symbol);fn.argtypes=[ctypes.c_uint32];fn.restype=ctypes.c_uint32;row[symbol]={str(mask):fn(mask) for mask in (1,2,3,4)}
  report['libraries'][name]=row
 except OSError as e:report['libraries'][name]={'loaded':False,'error':str(e)}
report['kpep_files']=glob.glob('/usr/share/kpep/*')
(out/'capabilities.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
