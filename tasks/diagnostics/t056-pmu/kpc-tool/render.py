"""Render the lead-only job, never submit/install/elevate."""
import base64
from pathlib import Path
ROOT=Path(__file__).resolve().parent

def render():
 body='set -euo pipefail\nexport PYTHONDONTWRITEBYTECODE=1\nexport PYTHONPATH="$BENCH_OUT:$PWD${PYTHONPATH:+:$PYTHONPATH}"\npython3 - <<\'FILES\'\nfrom pathlib import Path\nimport base64,os\nout=Path(os.environ["BENCH_OUT"])\n'
 for n in ('collect.py','protocol.py','counters.py','chain_common.py','window.py','config.json','groups.json'):
  body+='(out/'+repr(n)+').write_bytes(base64.b64decode('+repr(base64.b64encode((ROOT/n).read_bytes()).decode())+'))\n'
 body+='FILES\npython3 "$BENCH_OUT/collect.py"\n'
 return 'set -euo pipefail\npython3 - <<\'T056_WINDOW\'\n'+(ROOT/'window.py').read_text()+'\nbody='+repr(body)+'\ntry:\n    status=run(body,20*60,[])\nexcept ValueError as error:\n    print("error:",error,file=sys.stderr);status=124\nsys.exit(status)\nT056_WINDOW\n'
if __name__=='__main__':print(render(),end='')
