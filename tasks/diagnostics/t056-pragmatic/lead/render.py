#!/usr/bin/env python3
"""Print a self-contained remote job body; never submit or execute a job."""
import argparse
import base64
from pathlib import Path

ROOT = Path(__file__).resolve().parent
JOBS = {'01': ('01-correctness.sh', 50 * 60), '02': ('02-pre-gate.sh', 10 * 60),
        '03': ('03-triage.sh', 45 * 60)}


def render(number):
    filename, headroom = JOBS[number]
    payload = base64.b64encode((ROOT / 'common.py').read_bytes()).decode()
    bootstrap = '''set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
export PYTHONPATH="$BENCH_OUT:$PWD${PYTHONPATH:+:$PYTHONPATH}"
python3 - <<'CHAINLIB'
import base64,os
from pathlib import Path
p=Path(os.environ['BENCH_OUT'])/'common.py'
with p.open('xb') as stream: stream.write(base64.b64decode(%r))
CHAINLIB
''' % payload
    body = bootstrap + (ROOT / filename).read_text()
    wrapper = (ROOT / 'window.py').read_text()
    return "set -euo pipefail\npython3 - \"$@\" <<'T056_WINDOW'\n" + wrapper + '\nbody = ' + repr(body) + '\ntry:\n    code = run(body, ' + str(headroom) + ', sys.argv[1:])\nexcept ValueError as error:\n    print("error:", error, file=sys.stderr)\n    code = 124\nsys.exit(code)\nT056_WINDOW\n'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('job', choices=JOBS)
    print(render(parser.parse_args().job), end='')
