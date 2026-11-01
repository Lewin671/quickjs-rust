#!/usr/bin/env python3
"""Deterministic ARM instruction attribution only; no local timing verdict."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--base', required=True, type=Path)
p.add_argument('--candidate', required=True, type=Path)
p.add_argument('--candidate-revision', required=True)
p.add_argument('--output', required=True, type=Path)
p.add_argument('--cases', nargs='+')
p.add_argument('--repeats', type=int, default=3)
a = p.parse_args()
assert a.repeats >= 3
root = Path(__file__).resolve().parent
cases = [root / 'repro' / (name + '.js') for name in a.cases] if a.cases else sorted((root / 'repro').glob('*.js'))
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
binaries = {'base': a.base.resolve(), 'candidate': a.candidate.resolve()}
request = {'base_revision': '3cc12c6f', 'candidate_revision': a.candidate_revision,
           'binaries': {k: {'path': str(v), 'sha256': digest(v)} for k, v in binaries.items()},
           'reproducers': {f.name: digest(f) for f in cases}, 'repeats': a.repeats,
           'metric': '/usr/bin/time -l instructions retired only; wall/cycles/RSS not used',
           'producer_sha256': digest(Path(__file__))}
a.output.parent.mkdir(parents=True, exist_ok=True)
assert not a.output.exists(), a.output
rows = []
env = {k: v for k, v in os.environ.items() if not k.startswith('QJS_')}
env['TZ'] = 'UTC'
for case in cases:
    modes = {}
    for name, binary in binaries.items():
        samples, stdout = [], None
        for _ in range(a.repeats):
            r = subprocess.run(['/usr/bin/time', '-l', str(binary), '--raw', str(case)],
                               env=env, capture_output=True, text=True, timeout=120)
            assert r.returncode == 0, (case, name, r.stderr)
            match = re.search(r'^\s*(\d+)\s+instructions retired\s*$', r.stderr, re.M)
            assert match, r.stderr
            samples.append(int(match[1]))
            if stdout is None:
                stdout = r.stdout
            assert r.stdout == stdout, (case, name, r.stdout, stdout)
        modes[name] = {'instructions': samples, 'median': statistics.median(samples), 'stdout': stdout}
    assert modes['base']['stdout'] == modes['candidate']['stdout'], (case, modes)
    row = {'case': case.stem, 'modes': modes,
           'ratio': modes['candidate']['median'] / modes['base']['median']}
    rows.append(row)
    print(case.stem, f"{row['ratio']:.6f}", flush=True)
    a.output.write_text(json.dumps({'request': request, 'cases': rows}, indent=2) + '\n')
assert all(digest(v) == request['binaries'][k]['sha256'] for k, v in binaries.items())
assert all(digest(f) == request['reproducers'][f.name] for f in cases)
