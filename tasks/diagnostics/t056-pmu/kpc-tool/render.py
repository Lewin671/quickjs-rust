"""Render the lead-only job, never submit/install/elevate."""
import base64,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parent

def render(kind="timing"):
 body='set -euo pipefail\nexport PYTHONDONTWRITEBYTECODE=1\nexport PYTHONPATH="$BENCH_OUT:$PWD${PYTHONPATH:+:$PYTHONPATH}"\npython3 - <<\'FILES\'\nfrom pathlib import Path\nimport base64,sys,os\nout=Path(os.environ["BENCH_OUT"])\n'
 for n in (('smoke.py','config-v3.json') if kind=='smoke' else ('collect.py','protocol.py','counters.py','recovery.py','chain_common.py','window.py','config.json','groups.json')):
  body+='(out/'+repr(n)+').write_bytes(base64.b64decode('+repr(base64.b64encode((ROOT/({'config.json':'config-v3-retry1.json','groups.json':'groups-v3.json'}.get(n,n))).read_bytes()).decode())+'))\n'
 body+='FILES\npython3 "$BENCH_OUT/'+('smoke.py' if kind=='smoke' else 'collect.py')+'"\n'
 if kind=='smoke':return body
 return 'set -euo pipefail\npython3 - <<\'T056_WINDOW\'\n'+(ROOT/'window.py').read_text()+'\nbody='+repr(body)+'\ntry:\n    status=run(body,20*60,[])\nexcept ValueError as error:\n    print("error:",error,file=sys.stderr);status=124\nsys.exit(status)\nT056_WINDOW\n'
if __name__=='__main__':print(render(sys.argv[1] if len(sys.argv)>1 else "timing"),end='')
