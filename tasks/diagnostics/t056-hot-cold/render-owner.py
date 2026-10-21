"""Print reproducible non-timing owner payloads; never submit or execute them."""
import argparse,base64,json
from pathlib import Path
ROOT=Path(__file__).resolve().parent

def render(kind):
    body='set -euo pipefail\ntest "$(git rev-parse HEAD)" = 5dd3a810cac6a88f6a1f86c0ef46dc68dfac47ee\nexport PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$BENCH_OUT:$PWD"\npython3 - <<\'OWNER_FILES\'\nfrom pathlib import Path\nimport base64,os\nout=Path(os.environ["BENCH_OUT"])\n'
    if kind=='qualification':
        files={'qualify-producer.py':(ROOT/'qualify.py').read_bytes(),'event-runner.py':(ROOT/'events.py').read_bytes()}
    else:
        # Preserve the exact original protocol, including the hash derived
        # by build.py from its fixed exact86 receipt rather than a placeholder.
        proof=json.loads((ROOT/'panel/producer-proof.json').read_text())
        files={'build.py':(ROOT/'panel/build.py').read_bytes(),'protocol.py':proof['executed_protocol_source'].encode()}
    for name,data in files.items():body+=f'(out/{name!r}).write_bytes(base64.b64decode({base64.b64encode(data).decode()!r}))\n'
    body+='OWNER_FILES\n'
    if kind=='qualification':
        body+='python3 "$BENCH_OUT/qualify-producer.py" "$(git rev-parse HEAD)"\nmkdir "$BENCH_OUT/events"\ncp "$BENCH_OUT/../status" "$BENCH_OUT/status"\nBENCH_OUT="$BENCH_OUT/events" python3 "$BENCH_OUT/event-runner.py"\n'
    else:body+='python3 "$BENCH_OUT/build.py"\n'
    return body

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('kind',choices=('qualification','layouts'));print(render(p.parse_args().kind),end='')
