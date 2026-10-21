"""Read-only extra discovery and the repository's existing clock-validity smoke."""
import glob,os,json,subprocess
from pathlib import Path
from tools.benchmark.cycle_trust import cycles_trusted
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text()
r={'read_only':True,'timing':False,'xctrace_candidates':glob.glob('/Applications/Xcode*.app/Contents/Developer/usr/bin/xctrace')+glob.glob(str(Path.home()/'Applications/Xcode*.app/Contents/Developer/usr/bin/xctrace')),'rusage_cycles_trusted_by_existing_repo_probe':cycles_trusted()}
for name,argv in [('sudo_list',['sudo','-n','-l']),('powermetrics_help',['/usr/bin/powermetrics','--help'])]:
 p=subprocess.run(argv,text=True,capture_output=True,timeout=15);r[name]={'exit':p.returncode,'stdout':p.stdout,'stderr':p.stderr}
(out/'extra.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))
