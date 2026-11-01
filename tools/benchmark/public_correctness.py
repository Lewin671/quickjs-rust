"""Check every public workload's output in three engine modes, without timing.

This is a semantic gate, not benchmark/acceptance evidence. Only the four named
public inventories are resolved; held-out programs are never admitted.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import tempfile
from pathlib import Path

from . import screen
from .records import RESULT_PREFIX, parse_result
from .schema import load_manifest
from .development import opportunity_ids

MODES = {'default': {}, 'ic-off': {'QJS_DISABLE_IC': '1'},
         'ic-verify': {'QJS_VERIFY_IC': '1'}}
PUBLIC = ('broad', 'sentinel', 'external', 'development')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def invocations(case, internal):
    """Use both declared initial and doubled counts; whole programs run intact."""
    if case.kind != 'internal':
        if case.expected_stdout is None:
            raise ValueError(f'{case.id}: missing expected output')
        return [(None, case.expected_stdout)]
    contract = internal[case.id]
    counts = dict.fromkeys((contract.initial_iterations,
                           min(contract.initial_iterations * 2, contract.max_iterations)))
    return [(count, RESULT_PREFIX + json.dumps({
        'case_id': case.case_id, 'iterations': count,
        'operations': contract.expected_operations(count),
        'checksum': contract.expected_checksum(count)}, separators=(',', ':')) + '\n')
        for count in counts]


def check_cases(cases, internal, binaries, output, *, runner=subprocess.run):
    """Continue after failures so the report accounts for every case and mode."""
    if not cases or len({c.id for c in cases}) != len(cases):
        raise ValueError('public correctness requires a unique nonempty inventory')
    if any(c.id.split('/')[0] not in PUBLIC for c in cases):
        raise ValueError('only public workload inventories are allowed')
    environment = {k: v for k, v in os.environ.items() if not k.startswith('QJS_')}
    environment['TZ'] = 'UTC'
    workload_hashes = {c.id: digest(c.workload) for c in cases}
    binary_hashes = {mode: digest(path) for mode, path in binaries.items()}
    request = {'kind': 'public-workload-correctness', 'timing': False,
               'case_ids': [c.id for c in cases], 'modes': MODES,
               'binaries': {m: {'path': str(p), 'sha256': binary_hashes[m]}
                            for m, p in binaries.items()},
               'workload_sha256': workload_hashes,
               'internal_counts': 'manifest initial and min(2*initial,max)',
               'verify_binary_requires': 'debug assertions enabled',
               'producer_sha256': digest(Path(__file__)),
               'source_revision': subprocess.check_output(
                   ['git', '-C', str(screen.ROOT), 'rev-parse', 'HEAD'], text=True).strip(),
               'rust_input_sha256': {
                   str(p.relative_to(screen.ROOT)): digest(p)
                   for p in sorted((screen.ROOT / 'crates').rglob('*.rs'))}}
    (output / 'request.json').write_text(json.dumps(request, indent=2) + '\n')
    rows = []
    with (output / 'cases.jsonl').open('w') as records:
        for mode, flags in MODES.items():
            for case in cases:
                for count, expected in invocations(case, internal):
                    argv = [str(binaries[mode]), '--raw', str(case.workload)]
                    if count is not None:
                        argv += [case.case_id, str(count)]
                    row = {'mode': mode, 'id': case.id, 'iterations': count,
                           'command': argv, 'expected_stdout': expected, 'passed': False}
                    try:
                        run = runner(argv, env=environment | flags, capture_output=True,
                                     text=True, timeout=case.timeout_seconds or 120)
                        row.update(exit=run.returncode, stdout=run.stdout,
                                   stderr=run.stderr[:65536])
                        if count is not None:
                            parse_result(run.stdout)
                        row['passed'] = run.returncode == 0 and run.stdout == expected
                        if digest(case.workload) != workload_hashes[case.id]:
                            raise ValueError('workload changed during check')
                    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
                        row['passed'] = False
                        row['error'] = str(error)
                    rows.append(row)
                    records.write(json.dumps(row) + '\n')
                    records.flush()
                    print(f"{mode} {case.id} count={count}: "
                          f"{'pass' if row['passed'] else 'FAIL'}", flush=True)
    for mode, binary in binaries.items():
        if digest(binary) != binary_hashes[mode]:
            raise ValueError(f'{mode}: executable changed during check')
    failures = [r for r in rows if not r['passed']]
    summary = {'timing': False, 'complete': True, 'cases': len(cases),
               'case_modes': len(cases) * len(MODES), 'invocations': len(rows),
               'passed': len(rows) - len(failures), 'failed': len(failures),
               'failures': [{'mode': r['mode'], 'id': r['id'],
                             'iterations': r['iterations']} for r in failures],
               'request_sha256': digest(output / 'request.json')}
    (output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True,
                        help='candidate release CLI for default and IC-off')
    parser.add_argument('--verify-binary', type=Path, required=True,
                        help='same candidate CLI built with debug assertions for IC verification')
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--cache', type=Path, default=screen.DEFAULT_CACHE)
    args = parser.parse_args()
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=False)
    internal = {}
    for lane, path in screen.INTERNAL_MANIFESTS.items():
        internal.update({f'{lane}/{c.id}': c for c in load_manifest(path).cases})
    # The existing resolvers hash-check public sources and preserve expected
    # external sentinels/development checksums. No held-out resolver is called.
    with tempfile.TemporaryDirectory(dir=output, prefix='work-') as directory:
        cases = screen.resolve_cases(PUBLIC, args.cache, Path(directory))
        external = screen.load_manifest(screen.EXTERNAL_MANIFEST)
        expected = set(internal) | set(opportunity_ids()) | {
            f'external/{suite.id}/{case.id}'
            for suite in external.suites for case in suite.cases}
        if {c.id for c in cases} != expected:
            raise ValueError('resolved public inventory differs from its manifests')
        summary = check_cases(cases, internal, {
            'default': args.binary.resolve(), 'ic-off': args.binary.resolve(),
            'ic-verify': args.verify_binary.resolve()}, output)
    print(json.dumps(summary, indent=2))
    return int(summary['failed'] != 0)


if __name__ == '__main__':
    raise SystemExit(main())
