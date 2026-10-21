"""Print a lead-only diagnostic job; never execute or submit it."""
import base64,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parent

def render(kind):
 if kind!='mini':raise ValueError('only the four-case mini is prepared')
 body='set -euo pipefail\nexport PYTHONDONTWRITEBYTECODE=1\nexport PYTHONPATH="$BENCH_OUT:$PWD${PYTHONPATH:+:$PYTHONPATH}"\npython3 - <<\'FILES\'\nfrom pathlib import Path\nimport base64,os\nout=Path(os.environ["BENCH_OUT"])\n'
 for name in ('collect.py','protocol.py','analyze.py','chain_common.py','window.py','config.json'):
  body+='(out/'+repr(name)+').write_bytes(base64.b64decode('+repr(base64.b64encode((ROOT/name).read_bytes()).decode())+'))\n'
 body+='FILES\npython3 "$BENCH_OUT/collect.py" '+kind+' "$@"\n'
 return 'set -euo pipefail\npython3 - "$@" <<\'T056_WINDOW\'\n'+(ROOT/'window.py').read_text()+'\nbody = '+repr(body)+'\ntry:\n    status=run(body,35*60,sys.argv[1:])\nexcept ValueError as error:\n    print("error:",error,file=sys.stderr);status=124\nsys.exit(status)\nT056_WINDOW\n'
if __name__=='__main__':
 if len(sys.argv)!=2:raise SystemExit('usage: render.py mini')
 print(render(sys.argv[1]),end='')
