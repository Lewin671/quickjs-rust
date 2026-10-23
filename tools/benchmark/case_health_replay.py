"""Authenticate terminal health from retained processes, not caller summaries."""
from __future__ import annotations

from pathlib import Path

from . import screen
from .case_health import CaseHealthFailure, diagnostics


def process_health(raw, pairs, binaries):
    from .screen_replay import selection
    if raw.get('complete') is not False:
        raise ValueError('failed collection claims completion')
    flags = selection(raw, pairs, terminal=True)
    reasons = []
    workload = None
    width = 2 if raw['kind'] == 'internal' else 1
    if (raw['kind'] == 'internal') != raw['id'].startswith(('broad/', 'sentinel/')):
        raise ValueError('failed case kind differs from lane')
    if raw['kind'] not in ('internal', 'whole'):
        raise ValueError('unknown failed case kind')
    if width == 2 and (type(raw['iterations']) is not int or raw['iterations'] <= 0):
        # Calibration may fail before a usable N exists.
        if raw['attempts'] or raw['iterations'] is not None:
            raise ValueError('invalid failed case iterations')
    case = screen.CaseSpec(raw['id'], raw['kind'], Path('/unused'),
                           expected_stdout=raw.get('expected_stdout'))

    def check(row, role, count, location):
        nonlocal workload
        argv = row['argv']
        if len(argv) != (5 if width == 2 else 3) or argv[:2] != [binaries[role], '--raw']:
            raise ValueError('failed process command/binary mismatch')
        workload = workload or argv[2]
        if workload != argv[2]:
            raise ValueError('failed process workload changed')
        if width == 2 and (argv[3] != raw['id'].split('/', 1)[1] or int(argv[4]) != count):
            raise ValueError('failed process iteration/case mismatch')
        if row['started_at'] > row['ended_at']:
            raise ValueError('failed process timestamps reverse')
        if 'process_failure' in row:
            if 'sample' in row:
                raise ValueError('failed process also claims a sample')
            result = row['process_failure']
            if result['timed_out'] is True:
                reasons.append(location + ': timeout')
            elif type(result['exit_code']) is int and result['exit_code'] != 0:
                reasons.append(location + ': exit status ' + str(result['exit_code']))
            else:
                raise ValueError('process failure is not timeout/nonzero exit')
            return None
        sample = row['sample']
        if any(type(sample[m]) is not int or sample[m] <= 0 for m in ('instructions', 'wall_ns')):
            raise ValueError('failed case has invalid process metrics')
        try:
            return screen._check_output(case, sample['stdout'])
        except screen.CorrectnessFailure:
            reasons.append(location + ': correctness')
            return None

    for i, row in enumerate(raw['calibration']):
        check(row, 'base', int(row['argv'][-1]), f'calibration {i}')
    for i, attempt in enumerate(raw['attempts']):
        roles = ('base', 'candidate') if attempt['index'] % 2 == 0 else ('candidate', 'base')
        samples = attempt['samples']
        if not 1 <= len(samples) <= width * 2 or (i < len(raw['attempts']) - 1 and len(samples) != width*2):
            raise ValueError('failed case process inventory mismatch')
        outputs = {}
        for j, row in enumerate(samples):
            role, count = roles[j // width], j % width + 1
            result = check(row, role, raw['iterations'] * count if width == 2 else None,
                           f'attempt {attempt["index"]} process {j}')
            if result is not None:
                if count in outputs and outputs[count] != result:
                    reasons.append(f'attempt {attempt["index"]}: correctness')
                outputs[count] = result
    if not reasons:
        raise ValueError('failed collection has no replayable process failure')
    return diagnostics(raw, flags, reasons, collection='failed')


def replay_health(raw, pairs, binaries):
    from .screen_replay import replay_attempts
    if any(raw.get(key) for key in ('ratios', 'costs', 'samples')):
        raise ValueError('failed case carries usable analysis')
    if raw['health']['collection'] == 'failed':
        health = process_health(raw, pairs, binaries)
    else:
        try:
            replay_attempts(raw, pairs, binaries)
        except CaseHealthFailure as error:
            health = error.health
        else:
            raise ValueError('case health failure not reproduced')
    if health != raw['health']:
        raise ValueError('case health differs from raw replay')
    return health
