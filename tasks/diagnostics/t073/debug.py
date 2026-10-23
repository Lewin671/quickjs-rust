"""Retained rejected LLDB pilot; non-interactive debug permission is unavailable.

Use build_probe.py for the supplemental logger fallback. This script preserves
the failed launch and stops immediately when no address breakpoint ran.
"""
import json
import os
from pathlib import Path
import subprocess
import sys

out = Path(sys.argv[1]); out.mkdir()
job = Path(os.environ['BENCH_QUEUE']) / '20261023T154044Z-t072-screen-attempt2/out/attempt2'
for repeat in range(int(sys.argv[2])):
    for role, directory in [('base', 'base000'), ('candidate', 'cand000')]:
        binary = json.loads((job / directory / 'identity.json').read_text())['variants'][0]['path']
        prefix = out / f'{role}-{repeat}'
        env = os.environ | {'T073_STDOUT': str(prefix.with_suffix('.stdout')), 'T073_STDERR': str(prefix.with_suffix('.stderr'))}
        result = subprocess.run(['xcrun', 'lldb', '--no-lldbinit', '--batch', '-o',
            'command script import ' + str(Path('tasks/diagnostics/t073/debug_addresses.py').resolve()),
            '-o', 't073 ' + binary + ' 46137344'], env=env, capture_output=True, text=True, timeout=60)
        prefix.with_suffix('.lldb').write_text(result.stdout + result.stderr)
        print(role, repeat, result.returncode, result.stdout[-1200:], result.stderr[-200:], flush=True)
        if 'T073_ADDRESS' not in result.stdout:
            raise RuntimeError('address breakpoint did not run; keep failed pilot')
