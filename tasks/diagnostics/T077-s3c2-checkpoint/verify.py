#!/usr/bin/env python3
"""S3c v2 correctness checkpoint; comparison inputs never include held-out work."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['stress', 'compare'])
    parser.add_argument('--unit', type=Path)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--mode', action='append', help='Run only named modes')
    parser.add_argument('--jobs', default='4')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    previous = json.load(open('tasks/diagnostics/T077-s3b-final3-owner/checks.json'))
    modes = {'default': {}, 'off': {'QJS_DISABLE_IC': '1'},
             'verify-ic': {'QJS_VERIFY_IC': '1'}}
    modes.update({key: value['environment'] for key, value in previous['stress'].items()
                  if key != 'combined-off'})
    compare_modes = {key: modes[key] for key in ['default', 'off', 'verify-ic',
        'verify', 'force', 'poison', 'activation-poison', 'combined-all']}
    compare_modes['combined-off'] = previous['stress']['combined-off']['environment']
    rows = []
    clean = {key: value for key, value in os.environ.items() if not key.startswith('QJS_')}
    clean['TZ'] = 'UTC'
    if args.phase == 'stress':
        assert args.unit
        for mode, record in previous['stress'].items():
            command = [str(args.unit), '--test-threads=4']
            if mode == 'combined-off':
                for name in Path('tasks/diagnostics/T077-s3b-final3-owner/skipped-ic-controls.txt').read_text().splitlines():
                    if name.strip():
                        command += ['--skip', name.strip()]
            log = args.output / (mode + '.log')
            with log.open('w') as stream:
                run = subprocess.run(command, env={**clean, **record['environment']},
                                     stdout=stream, stderr=subprocess.STDOUT, timeout=1200)
            text = log.read_text()
            match = re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed', text)
            row = {'mode': mode, 'exit': run.returncode, 'environment': record['environment'],
                   'passed': int(match[2]) if match else None, 'log_sha256': digest(log)}
            rows.append(row)
            (args.output / 'stress.json').write_text(json.dumps(rows, indent=2) + '\n')
            print(mode, row['passed'], run.returncode, flush=True)
            assert run.returncode == 0, mode
    else:
        old = json.load(open('tasks/diagnostics/T077-s3b-final3-owner/compare-matrix.json'))
        modules = sorted({row['fixture'] for row in old if row['kind'] == 'module'})
        ng = 'third_party/quickjs-ng/build/qjs'
        for mode, flags in compare_modes.items():
            if args.mode and mode not in args.mode:
                continue
            env = {**clean, **flags, 'QJS_CLI_BIN': str(args.binary.resolve()), 'COMPARE_QJS_JOBS': args.jobs}
            log = args.output / ('compare-' + mode + '.log')
            with log.open('w') as stream:
                run = subprocess.run(['./scripts/compare-qjs.sh'], env=env,
                                     stdout=stream, stderr=subprocess.STDOUT, timeout=1800)
            count = len(re.findall(r'^ok:', log.read_text(), re.M))
            rows.append({'mode': mode, 'kind': 'script', 'exit': run.returncode,
                         'count': count, 'flags': flags, 'jobs': args.jobs,
                         'case_timeout_seconds': int(env.get('COMPARE_QJS_CASE_TIMEOUT_SECONDS', '10')),
                         'log_sha256': digest(log)})
            assert run.returncode == 0 and count == 322, mode
            for path in modules:
                candidate = subprocess.run([str(args.binary.resolve()), '--module', path],
                    capture_output=True, text=True, env=env, timeout=30)
                reference = subprocess.run([ng, '--module', path], capture_output=True,
                    text=True, env=clean, timeout=30)
                assert (candidate.returncode, candidate.stdout) == (reference.returncode, reference.stdout), (mode, path, candidate.stderr)
                rows.append({'mode': mode, 'kind': 'module', 'path': path,
                             'exit': candidate.returncode, 'stdout': candidate.stdout})
            (args.output / 'compare.json').write_text(json.dumps(rows, indent=2) + '\n')
            print(mode, count, len(modules), flush=True)


if __name__ == '__main__':
    main()
