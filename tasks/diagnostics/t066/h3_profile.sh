#!/usr/bin/env bash
# T066 repair diagnosis H3: non-timing sampled profiles of base versus HEAD on
# the two imaging cases that regress. Prints per-symbol self samples.
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
. scripts/lib.sh
cargo_bin="$(qjs_resolve_cargo)"
base_sha=9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30
candidate_sha="$(git rev-parse HEAD)"
base="$(qjs_build_revision "$PWD" "$cargo_bin" "$base_sha")"
candidate="$(qjs_build_revision "$PWD" "$cargo_bin" "$candidate_sha")"
python3 - "$base" "$candidate" "$BENCH_OUT" "$base_sha" "$candidate_sha" <<'PY'
import json, re, subprocess, sys, time
from pathlib import Path
from tools.benchmark.screen import DEFAULT_CACHE, _argv, resolve_cases

base, candidate, out = map(Path, sys.argv[1:4])
status = (out.parent / 'status').read_text()
assert 'no-timing' in status, status
workloads = out / 'workloads'
workloads.mkdir()
ids = ['external/kraken-1.1/imaging-darkroom', 'external/kraken-1.1/imaging-desaturate']
cases = {case.id: case for case in resolve_cases(ids, DEFAULT_CACHE, workloads)}
assert set(cases) == set(ids)
LINE = re.compile(r"^([ +!:|]*)(\d+) (.*)$")


def self_samples(path):
    """Return (total, {symbol: self_samples}) from a macOS sample call tree."""
    nodes, stack, in_graph = [], [], False
    for line in path.read_text(errors='replace').splitlines():
        if line.startswith('Call graph:'):
            in_graph = True
            continue
        if not in_graph:
            continue
        match = LINE.match(line)
        if not match:
            if line.strip():
                break
            continue
        prefix, count, name = match.group(1), int(match.group(2)), match.group(3)
        depth = len(prefix) // 2 - 2
        if depth < 0:
            continue
        stack = [node for node in stack if node['depth'] < depth]
        node = {'depth': depth, 'count': count, 'name': name, 'child': 0}
        if stack:
            stack[-1]['child'] += count
        nodes.append(node)
        stack.append(node)
    total = nodes[0]['count'] if nodes else 0
    symbols = {}
    for node in nodes:
        own = node['count'] - node['child']
        name = re.sub(r'\s+\(in [^)]*\)', '', node['name'])[:150]
        symbols[name] = symbols.get(name, 0) + own
    return total, symbols


report = {}
for case_id, case in cases.items():
    report[case_id] = {}
    for role, binary in [('base', base), ('candidate', candidate)]:
        directory = out / role / case_id.replace('/', '--')
        directory.mkdir(parents=True)
        stdout = directory / 'stdout.txt'
        raw = directory / 'raw.sample'
        with stdout.open('w') as handle:
            process = subprocess.Popen(_argv(binary, case, None), stdout=handle,
                                       stderr=subprocess.DEVNULL)
        sampler = subprocess.run(['sample', str(process.pid), '5', '1', '-mayDie',
                                  '-file', str(raw)], capture_output=True, text=True, timeout=60)
        status_code = process.wait(timeout=1800)
        assert status_code == 0, (case_id, role, status_code)
        assert raw.is_file() and 'Call graph:' in raw.read_text(), sampler.stderr
        total, symbols = self_samples(raw)
        report[case_id][role] = {
            'total_samples': total,
            'top': sorted(symbols.items(), key=lambda item: -item[1])[:25],
        }
(out / 'h3.json').write_text(json.dumps(report, indent=2) + '\n')
for case_id, roles in report.items():
    print('CASE', case_id, 'base total', roles['base']['total_samples'],
          'candidate total', roles['candidate']['total_samples'])
    for role in ('base', 'candidate'):
        print(' ', role, 'top self:')
        for name, count in roles[role]['top'][:12]:
            print(f'    {count:5d}  {name}')
PY
