#!/usr/bin/env python3
"""Diagnostic-only operation/allocation counters on frozen public triage argv."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tarfile

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--base', required=True)
p.add_argument('--candidate', required=True)
p.add_argument('--triage', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=False)
from tools.benchmark.build_cache import RUST_RECIPE
roles = {}
for role, ref in [('base', a.base), ('candidate', a.candidate)]:
    revision = subprocess.check_output(['git', 'rev-parse', ref], text=True).strip()
    source = a.output / ('source-' + role)
    source.mkdir()
    archive = subprocess.Popen(['git', 'archive', revision], stdout=subprocess.PIPE)
    with tarfile.open(fileobj=archive.stdout, mode='r|') as tree:
        tree.extractall(source, filter='data')
    if archive.wait():
        raise RuntimeError('git archive failed')
    args = list(RUST_RECIPE['cargo_args'])
    args[args.index('--features') + 1] = 'agents,perf-counters,alloc-counters'
    env = dict(os.environ, **RUST_RECIPE['environment'])
    target = a.output / ('build-' + role)
    with (a.output / ('build-' + role + '.log')).open('w') as log:
        subprocess.run(['cargo', 'build', *args, '--manifest-path', str(source / 'Cargo.toml'),
                        '--target-dir', str(target)], env=env, stdout=log, stderr=log, check=True)
    binary = target / 'release/qjs'
    roles[role] = {'revision': revision, 'path': str(binary),
                   'sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}
cases = ['development/diff/compare', 'development/lodash/transform',
         'sentinel/string_key_map_churn', 'broad/property_dynamic_read']
records = []
for case in cases:
    raw = a.triage / 'variant0' / (case.replace('/', '--') + '.jsonl')
    rows = [json.loads(line) for line in raw.read_text().splitlines()]
    measured = max(rows, key=lambda row: int(row['argv'][-1])) if case.startswith(('broad/', 'sentinel/')) else rows[-1]
    argv = measured['argv'][1:]
    outputs = {}
    record = {'case': case, 'argv_without_binary': argv, 'roles': {}}
    for role, build in roles.items():
        run = subprocess.run([build['path'], *argv], capture_output=True, text=True,
                             timeout=300, check=True)
        outputs[role] = run.stdout
        counters = {}
        in_runtime = False
        for line in run.stderr.splitlines():
            if line == 'QJS_PERF_COUNTERS':
                in_runtime = True
                continue
            if line.startswith('QJS_ALLOC_COUNTERS '):
                for token in line.split()[1:]:
                    key, value = token.split('=')
                    counters['alloc_' + key] = int(value)
            elif in_runtime:
                tokens = line.split()
                if len(tokens) == 2 and tokens[1].isdigit():
                    counters[tokens[0]] = int(tokens[1])
        assert counters, (case, role, run.stderr[:1000])
        record['roles'][role] = counters
    assert outputs['base'] == outputs['candidate'], case
    records.append(record)
result = {'diagnostic_only': True, 'timing_claim': False, 'builds': roles,
          'triage_argv_source': str(a.triage), 'records': records}
(a.output / 'counters.json').write_text(json.dumps(result, indent=2) + '\n')
print(a.output / 'counters.json')
