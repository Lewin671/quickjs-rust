"""Supplemental exact-image sampling/maps; these perturbed runs never gate."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time

out = Path(sys.argv[1]); out.mkdir()
job = Path(os.environ['BENCH_QUEUE']) / '20261023T154044Z-t072-screen-attempt2/out/attempt2'
for role, directory in [('base', 'base000'), ('candidate', 'cand000')]:
    binary = json.loads((job / directory / 'identity.json').read_text())['variants'][0]['path']
    for trial in range(6):
        prefix = out / f'{role}-{trial}'
        with prefix.with_suffix('.stdout').open('w') as stdout, prefix.with_suffix('.stderr').open('w') as stderr:
            start = time.monotonic()
            child = subprocess.Popen([binary, '--raw', 'benchmarks/workloads/broad-micro.js', 'local_read', '300000000'], stdout=stdout, stderr=stderr)
            subprocess.run(['/usr/bin/sample', str(child.pid), '1', '1', '-file', str(prefix.with_suffix('.sample'))], capture_output=True, timeout=20)
            status = child.wait(timeout=30)
            print(role, trial, status, time.monotonic() - start, flush=True)
