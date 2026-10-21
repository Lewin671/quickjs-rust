"""Render a non-timing archive build; never submits a job."""
import base64
from pathlib import Path
ROOT=Path(__file__).resolve().parent
NAMES=('partition.py','build.py','audit.py','extract-paths.py','hunks.json')
def render():
 body='set -euo pipefail\nexport PYTHONDONTWRITEBYTECODE=1\nexport PYTHONPATH="$BENCH_OUT:$PWD${PYTHONPATH:+:$PYTHONPATH}"\npython3 - <<\'FILES\'\nfrom pathlib import Path\nimport base64,os\nout=Path(os.environ["BENCH_OUT"])\n'
 for name in NAMES:body+='(out/'+repr(name)+').write_bytes(base64.b64decode('+repr(base64.b64encode((ROOT/name).read_bytes()).decode())+'))\n'
 return body+'FILES\npython3 "$BENCH_OUT/build.py"\n'
if __name__=='__main__':print(render(),end='')
