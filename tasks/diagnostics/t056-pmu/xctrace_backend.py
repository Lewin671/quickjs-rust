"""Headless xctrace command/export adapter. No PMU/security setting mutation."""
import os,re,shutil,subprocess,time,signal,json,xml.etree.ElementTree as ET
from pathlib import Path

def tool_path():
 candidates=[os.environ.get('T056_XCTRACE_BIN')]
 candidates.extend(str(p) for p in Path('/Applications').glob('Xcode*.app/Contents/Developer/usr/bin/xctrace'))
 candidates.extend(str(p) for p in (Path.home()/'Applications').glob('Xcode*.app/Contents/Developer/usr/bin/xctrace'))
 try:
  r=subprocess.run(['xcrun','--find','xctrace'],capture_output=True,text=True,timeout=10)
  if r.returncode==0:candidates.append(r.stdout.strip())
 except (OSError,subprocess.TimeoutExpired):pass
 for candidate in candidates:
  if candidate and Path(candidate).is_file() and os.access(candidate,os.X_OK):return str(Path(candidate).resolve())
 raise RuntimeError('PMU unavailable: xctrace is not installed/located; no installation or sudo/security change attempted')

def inspect(tool,runner):
 record=runner([tool,'help','record'],20);export=runner([tool,'help','export'],20);templates=runner([tool,'list','templates'],20)
 if any(r['returncode'] for r in (record,export,templates)):raise RuntimeError('xctrace capability command failed')
 if '--no-prompt' not in record['stdout']+record['stderr'] or '--target-stdout' not in record['stdout']+record['stderr']:raise RuntimeError('unsupported headless xctrace CLI')
 template=os.environ.get('T056_PMU_TEMPLATE','CPU Counters')
 if Path(template).is_file():template=str(Path(template).resolve())
 elif template!='CPU Counters' or 'CPU Counters' not in templates['stdout']+templates['stderr']:raise RuntimeError('requested CPU Counters template unavailable; no fallback profiler selected')
 return {'tool':tool,'record_help':record,'export_help':export,'templates':templates,'template':template,'legacy_export_input':'--input' in export['stdout']+export['stderr']}

def export_argv(meta,trace):return [meta['tool'],'export',*(['--input',str(trace)] if meta['legacy_export_input'] else [str(trace)])]

def record_argv(meta,trace,target_stdout,argv,seconds):
 return [meta['tool'],'record','--no-prompt','--template',meta['template'],'--time-limit',str(seconds)+'s','--output',str(trace),'--target-stdout',str(target_stdout),'--launch','--',*map(str,argv)]

def schemas(toc):
 root=ET.fromstring(Path(toc).read_text());result=[]
 for node in root.iter('table'):
  name=node.attrib.get('schema','')
  if re.fullmatch(r'[A-Za-z0-9_.-]+',name) and re.search(r'counter|pmc|pmu|core-event',name,re.I) and name not in result:result.append(name)
 return result

def stop_process(proc):
 if proc.poll() is not None:return
 for signum,seconds in ((signal.SIGINT,2),(signal.SIGTERM,.3),(signal.SIGKILL,1)):
  try:os.killpg(proc.pid,signum)
  except ProcessLookupError:break
  try:proc.wait(timeout=seconds);break
  except subprocess.TimeoutExpired:pass
