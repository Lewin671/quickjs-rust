#!/usr/bin/env python3
"""Instruction/event census only; never retains timing, cycles or held-out input."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def run(binary, program, arguments, env, instructions=False):
    command = [str(binary), '--raw', str(program), *arguments]
    if instructions:
        command = ['/usr/bin/time', '-l', *command]
    result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=600)
    assert result.returncode == 0, (program, result.returncode, result.stderr[-1500:])
    if instructions:
        count = int(re.search(r'(\d+)\s+instructions retired', result.stderr)[1])
    else:
        count = {key: int(value) for key, value in re.findall(
            r'^\s+(\w+) (\d+)$', result.stderr, re.M)}
        assert count
    return result.stdout, count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ['before', 'after', 'before-counters', 'after-counters', 'output']:
        parser.add_argument('--' + flag, required=True, type=Path)
    parser.add_argument('--programs', type=Path, default=Path('target/s3c-counts/programs'))
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    env = {key: value for key, value in os.environ.items() if not key.startswith('QJS_')}
    census = json.load(open('tasks/diagnostics/T077-s3c-counts.json'))
    roles = {'before': (args.before, args.before_counters),
             'after': (args.after, args.after_counters)}
    args.output.mkdir(parents=True, exist_ok=True)
    identity = {role: {'instructions': sha(pair[0]), 'counters': sha(pair[1])}
                for role, pair in roles.items()}
    seal = args.output / 'identity.json'
    if seal.exists():
        assert json.load(open(seal)) == identity, 'use a fresh output for different binaries'
    else:
        seal.write_text(json.dumps(identity, indent=2) + '\n')
    records = []
    for source in census['records']:
        ident = source['id']
        parts = ident.split('/')
        if parts[0] == 'repro':
            program = Path('tasks/diagnostics/T077-s3b-r4/repro') / (parts[1] + '.js')
        elif parts[0] == 'diagnostic':
            program = args.programs / ('audio-beat-detection-fixed-clock.js'
                       if parts[1] == 'audio-fixed-clock' else 'crypto-aes-fixed-clock.js')
        elif parts[0] == 'development':
            program = args.programs / ('--'.join(parts) + '.js')
        elif parts[0] == 'external':
            program = args.programs / ('--'.join(parts[1:]) + '.js')
        else:
            program = Path('benchmarks/workloads') / (
                'broad-micro.js' if parts[0] == 'broad' else 'generic-sentinels.js')
        assert sha(program) == source['sha256'], (ident, program)
        path = args.output / (ident.replace('/', '--') + '.json')
        if path.exists():
            row = json.load(open(path))
            assert row['source_sha256'] == source['sha256']
            records.append(row)
            continue
        row = {'id': ident, 'source_sha256': source['sha256'], 'args': source['args'],
               'stdout_sha256': source['stdout_sha256'], 'roles': {}}
        for role, (binary, counters) in roles.items():
            measurements = []
            for _ in range(3 if parts[0] == 'repro' else 1):
                stdout, instructions = run(binary, program, source['args'], env, True)
                assert hashlib.sha256(stdout.encode()).hexdigest() == source['stdout_sha256'], ident
                measurements.append(instructions)
            maps = []
            for _ in range(2):
                stdout, counts = run(counters, program, source['args'], env)
                assert hashlib.sha256(stdout.encode()).hexdigest() == source['stdout_sha256'], ident
                maps.append(counts)
            if ident not in census['interpretation']['unstable_original_inputs']:
                assert maps[0] == maps[1], (ident, role)
            row['roles'][role] = {'instructions': measurements,
                'counters': maps[0], 'counter_repeat': 'identical' if maps[0] == maps[1] else maps[1]}
        path.write_text(json.dumps(row, indent=2) + '\n')
        records.append(row)
        ratio = statistics.median(row['roles']['after']['instructions']) / statistics.median(
            row['roles']['before']['instructions'])
        print(ident, f'{ratio:.6f}', flush=True)
    result = {'base': '8322fc983da20800ce612ad148781aea30e28d08',
        'candidate': args.revision, 'timing': False,
        'binary_sha256': {role: {'instructions': sha(pair[0]), 'counters': sha(pair[1])}
                          for role, pair in roles.items()},
        'repeats': {'repro_instructions': 3, 'corpus_instructions': 1, 'counters': 2},
        'records': records}
    (args.output / 'counts.json').write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()
