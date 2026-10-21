"""Render unsigned-copy non-timing address capture only; never submits."""
import base64
from pathlib import Path
ROOT=Path(__file__).resolve().parent
def render():
 text='set -euo pipefail\nexport PYTHONDONTWRITEBYTECODE=1\nexport PYTHONPATH="$BENCH_OUT:$PWD${PYTHONPATH:+:$PYTHONPATH}"\npython3 - <<\'FILES\'\nfrom pathlib import Path\nimport os,base64\nout=Path(os.environ["BENCH_OUT"])\n'
 for name in ('capture.py','inputs.json'):text+='(out/'+repr(name)+').write_bytes(base64.b64decode('+repr(base64.b64encode((ROOT/name).read_bytes()).decode())+'))\n'
 return text+'FILES\npython3 "$BENCH_OUT/capture.py"\n'
if __name__=='__main__':print(render(),end='')
