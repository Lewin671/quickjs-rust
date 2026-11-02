#!/usr/bin/env python3
"""N/2N retired-instruction slopes, with event reach proving the same tier."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess


def execute(binary, path, env, instructions):
    command = [str(binary), '--raw', str(path)]
    if instructions:
        command = ['/usr/bin/time', '-l', *command]
    result = subprocess.run(command, env=env, text=True, capture_output=True, timeout=600)
    assert result.returncode == 0, (path, result.stderr)
    if instructions:
        value = int(re.search(r'(\d+)\s+instructions retired', result.stderr)[1])
    else:
        value = {key: int(count) for key, count in re.findall(r'^\s+(\w+) (\d+)$', result.stderr, re.M)}
    return result.stdout, value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ['before', 'after', 'before-counters', 'after-counters', 'output']:
        parser.add_argument('--' + flag, type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    env = {key: value for key, value in os.environ.items() if not key.startswith('QJS_')}
    records = []
    for name in ['upw0', 'upw1', 'upw4']:
        original = Path('tasks/diagnostics/T077-s3b-r4/repro') / (name + '.js')
        text = original.read_text()
        assert text.count('run(1000000);') == 1
        row = {'name': name, 'roles': {}}
        output = {}
        for n in [1000000, 2000000]:
            path = args.output / f'{name}-{n}.js'
            path.write_text(text.replace('run(1000000);', f'run({n});'))
            for role, binary, counter in [('before', args.before, args.before_counters),
                                           ('after', args.after, args.after_counters)]:
                counts = []
                for _ in range(3):
                    stdout, value = execute(binary, path, env, True)
                    counts.append(value)
                    if n in output:
                        assert stdout == output[n]
                    output[n] = stdout
                stdout, events = execute(counter, path, env, False)
                assert stdout == output[n]
                assert events['compact_direct_calls'] == n, (role, name, n, events)
                row['roles'].setdefault(role, {})[str(n)] = {
                    'source_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                    'instructions': counts, 'events': events}
        for role, samples in row['roles'].items():
            samples['instructions_per_call'] = (
                statistics.median(samples['2000000']['instructions']) -
                statistics.median(samples['1000000']['instructions'])) / 1000000
        records.append(row)
        print(name, {role: r['instructions_per_call'] for role, r in row['roles'].items()}, flush=True)
        (args.output / 'call-cost.json').write_text(json.dumps({'timing': False, 'records': records}, indent=2) + '\n')


if __name__ == '__main__':
    main()
