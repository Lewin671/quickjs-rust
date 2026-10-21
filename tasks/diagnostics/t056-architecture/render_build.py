"""Render a NON-timing archive diagnostic payload; never submit it."""
import base64,sys
from pathlib import Path
root=Path(__file__).resolve().parent
jobs={'capture':('instrument.py','build_capture.py'),'supplement':('instrument-supplement.py','build_supplement.py'),'ablation':('ablate.py',),'padding':('pad_build.py',)}
def render(kind):
 names=jobs[kind]
 body='set -euo pipefail\nexport PYTHONDONTWRITEBYTECODE=1\nexport PYTHONPATH="$BENCH_OUT:$PWD${PYTHONPATH:+:$PYTHONPATH}"\npython3 - <<\'FILES\'\nfrom pathlib import Path\nimport base64,os\nout=Path(os.environ["BENCH_OUT"])\n'
 for name in names:
  body+='(out/'+repr(name)+').write_bytes(base64.b64decode('+repr(base64.b64encode((root/name).read_bytes()).decode())+'))\n'
 return body+'FILES\npython3 "$BENCH_OUT/'+names[-1]+'"\n'
if __name__=='__main__':
 if len(sys.argv)!=2 or sys.argv[1] not in jobs:raise SystemExit('usage: render_build.py capture|supplement|ablation|padding')
 print(render(sys.argv[1]),end='')
