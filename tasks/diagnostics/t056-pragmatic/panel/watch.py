"""Lead-side watchdog for this job only, including runner quieting (not queue wait)."""
import datetime,json,re,subprocess,sys,time,inspect
import window
from pathlib import Path
from protocol import JOB_SECONDS
import os
RUNNER=os.environ.get('T056_BENCH_RUNNER','/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh')
HOST='qingyingliu@192.168.7.165'
def expired(status,now):
 if status.startswith('done '):return False
 if status=='queued':return False
 match=re.fullmatch(r'running ([0-9TZ:-]+)',status)
 if not match:raise ValueError('unknown runner status: '+status)
 start=datetime.datetime.fromisoformat(match[1].replace('Z','+00:00')).timestamp()
 return now-start>=JOB_SECONDS

def stop_script():
 return 'import os,signal,subprocess,time,sys\n'+inspect.getsource(window.tree)+'\n'+inspect.getsource(window.signal_owned)+'\n'+inspect.getsource(window.stop_tree)+"\nresult=subprocess.run(['tmux','display-message','-p','-t','bench-'+sys.argv[1],'#{pane_pid}'],text=True,capture_output=True)\nif result.returncode==0: stop_tree(int(result.stdout.strip()))\n"

def main(identifier):
 if not re.fullmatch(r'[0-9]{8}T[0-9]{6}Z-t056-pragmatic-mini',identifier):raise ValueError('watchdog accepts only the named pragmatic mini job')
 remote='Code/co-work-space/tmp/bench-queue/'+identifier+'/status'
 while True:
  status=subprocess.check_output(['ssh','-o','BatchMode=yes','-o','ConnectTimeout=10',HOST,'cat '+remote],text=True,timeout=15).strip()
  if status.startswith('done '):
   print(identifier,status,flush=True);return 0 if status=='done 0' else 1
  if expired(status,time.time()):
   print('30-minute running-job budget exhausted, including runner quieting; cancel this job only; retain partial evidence',flush=True)
   # Stop the current job's pane process and descendants before closing tmux;
   # the body has its own session, so killing tmux alone is insufficient.
   remote_code=stop_script()
   subprocess.run(['ssh','-o','BatchMode=yes',HOST,'python3 - '+identifier],input=remote_code,text=True,check=True)
   subprocess.run([RUNNER,'cancel',identifier],check=True)
   return 124
  time.sleep(2)
if __name__=='__main__':
 if len(sys.argv)!=2:raise SystemExit('usage: watch.py JOB_ID')
 raise SystemExit(main(sys.argv[1]))
