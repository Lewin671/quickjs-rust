#!/usr/bin/env python3
"""Frozen public control instruction counts; no local timing metrics retained."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
from tools.benchmark import screen
from tools.benchmark.public_correctness import invocations
from tools.benchmark.schema import load_manifest


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def run(binary, source, arguments, env, counter=False):
    argv = [str(binary), '--raw', str(source), *arguments]
    if not counter:
        argv = ['/usr/bin/time', '-l', *argv]
    result = subprocess.run(argv, env=env, capture_output=True, text=True, timeout=600)
    assert result.returncode == 0, (source, result.returncode)
    if counter:
        value = {k: int(v) for k, v in re.findall(r'^\s+(\w+) (\d+)$', result.stderr, re.M)}
    else:
        value = int(re.search(r'(\d+)\s+instructions retired', result.stderr)[1])
    return result.stdout, value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['before', 'after', 'before-counters', 'after-counters', 'output']:
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    args.output.mkdir(exist_ok=False, parents=True)
    work = args.output / 'work'; work.mkdir()
    ids = ['external/sunspider-1.0/access-fannkuch',
           'external/jetstream3-js-subset/hash-map',
           'external/sunspider-1.0/date-format-tofte', 'broad/method_call']
    internal = {f'{lane}/{c.id}': c for lane, path in screen.INTERNAL_MANIFESTS.items()
                for c in load_manifest(path).cases}
    roles = {'before': (args.before, args.before_counters), 'after': (args.after, args.after_counters)}
    identity = {k: {'default': sha(b), 'counters': sha(c)} for k, (b, c) in roles.items()}
    request = {'timing': False, 'ids': ids + ['repro/method-call'], 'binary_sha256': identity,
               'samples': 5, 'order': 'before/after interleaved', 'base': '8322fc98'}
    (args.output / 'request.json').write_text(json.dumps(request, indent=2) + '\n')
    cases = []
    for case in screen.resolve_cases(ids, screen.DEFAULT_CACHE, work):
        count, expected = invocations(case, internal)[0]
        cases.append((case.id, case.workload, [] if count is None else [case.case_id, str(count)], expected))
    repro = Path('tasks/diagnostics/T077-s3c-c2/attribution/method-call.js')
    cases.append(('repro/method-call', repro, ['1000000'], '1000000\n'))
    env = {k: v for k, v in os.environ.items() if not k.startswith('QJS_')}; env['TZ'] = 'UTC'
    rows = []
    for ident, source, arguments, expected in cases:
        row = {'id': ident, 'source_sha256': sha(source), 'args': arguments,
               'stdout_sha256': hashlib.sha256(expected.encode()).hexdigest(),
               'roles': {r: {'instructions': []} for r in roles}}
        for _ in range(5):
            for role, (binary, _) in roles.items():
                stdout, instructions = run(binary, source, arguments, env)
                assert stdout == expected, ident
                row['roles'][role]['instructions'].append(instructions)
        for role, (_, counter) in roles.items():
            stdout, events = run(counter, source, arguments, env, True)
            assert stdout == expected, ident
            row['roles'][role]['counters'] = events
        row['ratio'] = statistics.median(row['roles']['after']['instructions']) / statistics.median(row['roles']['before']['instructions'])
        row['target_met'] = row['ratio'] <= 1.0
        rows.append(row)
        (args.output / 'controls.json').write_text(json.dumps({'request': request, 'records': rows}, indent=2) + '\n')
        print(ident, row['ratio'], row['target_met'], flush=True)
    assert identity == {k: {'default': sha(b), 'counters': sha(c)} for k, (b, c) in roles.items()}


if __name__ == '__main__':
    main()
