"""Render the prospective architecture robustness job; never execute or submit it."""
import base64
from pathlib import Path
ROOT=Path(__file__).resolve().parent

def render():
 body='set -euo pipefail\nexport PYTHONDONTWRITEBYTECODE=1\nexport PYTHONPATH="$BENCH_OUT:$PWD${PYTHONPATH:+:$PYTHONPATH}"\npython3 - <<\'FILES\'\nfrom pathlib import Path\nimport base64,os\nout=Path(os.environ["BENCH_OUT"])\n'
 for name in ('collect.py','protocol.py','chain_common.py','probe-config.json'):
  encoded=base64.b64encode((ROOT/name).read_bytes()).decode()
  body+='with (out/'+repr(name)+').open("xb") as stream: stream.write(base64.b64decode('+repr(encoded)+'))\n'
 body+='FILES\npython3 "$BENCH_OUT/collect.py"\n'
 wrapper=(ROOT/'window.py').read_text()
 return 'set -euo pipefail\npython3 - <<\'T056_WINDOW\'\n'+wrapper+'\nbody = '+repr(body)+'\ntry:\n    status=run(body,30*60,[])\nexcept ValueError as error:\n    print("error:",error,file=sys.stderr);status=124\nsys.exit(status)\nT056_WINDOW\n'
if __name__=='__main__':print(render(),end='')
