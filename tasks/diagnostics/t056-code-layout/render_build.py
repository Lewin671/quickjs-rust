"""Print non-timing qualification payload; never submit."""
from pathlib import Path
import base64
root=Path(__file__).resolve().parent
s='set -euo pipefail\nexport PYTHONDONTWRITEBYTECODE=1\nexport PYTHONPATH="$BENCH_OUT:$PWD${PYTHONPATH:+:$PYTHONPATH}"\npython3 - <<\'FILES\'\nimport base64,os\nfrom pathlib import Path\nout=Path(os.environ["BENCH_OUT"])\n'
for name in ('build.py','protocol.py'):
 s+='(out/'+repr(name)+').write_bytes(base64.b64decode('+repr(base64.b64encode((root/name).read_bytes()).decode())+'))\n'
print(s+'FILES\npython3 "$BENCH_OUT/build.py"',end='\n')
