"""Verify process-local ASLR behavior against mappings, not flag success alone."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time

out = Path(sys.argv[1]); out.mkdir()
launcher = str(Path(sys.argv[2]).resolve())
job = Path(os.environ['BENCH_QUEUE']) / '20261023T154044Z-t072-screen-attempt2/out/attempt2'
binary = json.loads((job / 'cand000/identity.json').read_text())['variants'][0]['path']
for mode in ['on', 'off']:
    for trial in range(3):
        child = subprocess.Popen([launcher, mode, '-1', binary, '--raw', 'benchmarks/workloads/broad-micro.js', 'local_read', '500000000'], stdout=subprocess.DEVNULL)
        time.sleep(0.05)
        result = subprocess.run(['/usr/bin/vmmap', '-w', str(child.pid)], capture_output=True, text=True, timeout=20)
        (out / f'{mode}-{trial}.txt').write_text(result.stdout + result.stderr)
        print(mode, trial, 'vmmap', result.returncode, 'child', child.wait(timeout=30), flush=True)
