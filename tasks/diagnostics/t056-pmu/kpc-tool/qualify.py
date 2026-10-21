"""Build/static/nonroot checks on iMac, no counters or installed files changed."""
import hashlib,json,os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parent;out=Path(os.environ['BENCH_OUT'])
assert 'no-timing' in (out.parent/'status').read_text()
assert os.geteuid()!=0
sources=['main.c','security.c','events.c','aggregate.c','trace.c','process.c','json.c']
flags=['-std=c11','-D_DARWIN_C_SOURCE','-O2','-mmacosx-version-min=12.0','-fstack-protector-strong','-fPIE','-Wall','-Wextra','-Werror','-Wformat=2','-Wshadow','-Wconversion','-Wcast-qual','-Wstrict-prototypes']
def run(argv):
 p=subprocess.run(argv,text=True,capture_output=True,timeout=60);return {'argv':list(map(str,argv)),'status':p.returncode,'stdout':p.stdout,'stderr':p.stderr}
binary=out/'qjs-kpc';build=run(['/usr/bin/clang',*flags,*[str(root/s) for s in sources],'-Wl,-pie','-o',str(binary)])
(out/'build.json').write_text(json.dumps(build,indent=2)+'\n');assert build['status']==0,build
unit=out/'unit';u=run(['/usr/bin/clang',*flags,str(root/'unit.c'),str(root/'security.c'),str(root/'aggregate.c'),'-o',str(unit)]);assert u['status']==0,u
mock=out/'drop-unit';definitions=['-D'+x+'=test_'+x for x in ('setgroups','setgid','setuid','getuid','geteuid','getgid','getegid')];m=run(['/usr/bin/clang',*flags,*definitions,str(root/'drop-unit.c'),str(root/'security.c'),'-o',str(mock)]);assert m['status']==0,m
trace=out/'trace-unit';m=run(['/usr/bin/clang',*flags,'-Dsysctl=test_sysctl','-Dsysctlbyname=test_sysctlbyname','-Dgetpid=test_getpid',str(root/'trace-unit.c'),str(root/'trace.c'),str(root/'aggregate.c'),'-o',str(trace)]);assert m['status']==0,m
json_unit=out/'json-unit';m=run(['/usr/bin/clang',*flags,str(root/'json-unit.c'),str(root/'json.c'),str(root/'aggregate.c'),'-o',str(json_unit)]);assert m['status']==0,m
checks={'json-unit':run([str(json_unit)]),'trace-unit':run([str(trace)]),'drop-unit':run([str(mock)]),'unit':run([str(unit)]),'help':run([str(binary),'--help']),'list':run([str(binary),'--list-events']),'unprivileged':run([str(binary),'--events','cycles,instructions','--','/usr/bin/id','-u']),'bad-env':run([str(binary),'--events','cycles,instructions','--env','DYLD_INSERT_LIBRARIES=/tmp/evil','--','/usr/bin/id'])}
for name in ('unit','json-unit','trace-unit','drop-unit','help','list'):assert checks[name]['status']==0,checks[name]
for name in ('unprivileged','bad-env'):assert checks[name]['status']!=0 and not json.loads(checks[name]['stdout'])['capture_complete']
values=[json.loads(line) for line in checks['json-unit']['stdout'].splitlines()];assert values[0]['cycles']==50 and values[0]['instructions']==100 and values[0]['counter_backend']=='kpep_programmable_user'
assert values[1]['failed_step']=='setup.config_equal' and values[1]['readback'][0]=='0x23' and values[1]['cleanup_failed_step']=='cleanup.kperf_reset' and values[1]['cycles'] is None
events=json.loads(checks['list']['stdout']);(out/'catalog.json').write_text(json.dumps(events,indent=2)+'\n')
checks['list']={'status':0,'events':len(events['events'])}
checks['fixed_plus_four']=run([str(binary),'--describe','--fixed-diagnostic','--events','cycles,instructions,IDQ.DSB_UOPS,IDQ.MITE_UOPS,IDQ.MS_UOPS,DSB2MITE_SWITCHES.PENALTY_CYCLES']);assert checks['fixed_plus_four']['status']==0
description=json.loads(checks['fixed_plus_four']['stdout']);assert description['classes']==3 and description['counter_count']==7 and description['config'][0]=='0x22'
checks['programmable']=run([str(binary),'--describe','--events','cycles,instructions,IDQ.DSB_UOPS,IDQ.MITE_UOPS']);assert checks['programmable']['status']==0
description=json.loads(checks['programmable']['stdout']);assert description['classes']==2 and description['counter_count']==4 and len(description['config'])==4 and description['config'][:2]==['0x41003c','0x4100c0']
checks['static']={}
for source in sources:
 check=run(['/usr/bin/clang',*flags,'--analyze','-Xanalyzer','-analyzer-output=text',str(root/source)]);checks['static'][source]=check;assert check['status']==0 and 'warning:' not in check['stderr'],check
checks['libraries']=run(['/usr/bin/otool','-L',str(binary)])
checks['load_commands']=run(['/usr/bin/otool','-l',str(binary)])
assert 'LC_RPATH' not in checks['load_commands']['stdout']
checks['load_commands']={'status':checks['load_commands']['status'],'LC_RPATH':False}
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
report={'diagnostic_only':True,'timing':False,'privileged_run':False,'install_performed':False,'hardware_counter_evidence':False,'binary_sha256':sha(binary),'source_sha256':{p.name:sha(p) for p in root.iterdir() if p.suffix in ('.c','.h','.py')},'clang':run(['/usr/bin/clang','--version']),'build':build,'checks':checks,'cpu':run(['/usr/sbin/sysctl','machdep.cpu.family','machdep.cpu.model','machdep.cpu.stepping','machdep.cpu.microcode_version'])}
(out/'qualification.json').write_text(json.dumps(report,indent=2)+'\n');print('QUALIFIED NONROOT',sha(binary),flush=True)
