"""Rebuild screen ratios from complete positive per-process sample packets."""
from __future__ import annotations

from .screen import MIN_INCREMENT_NS


def replay_case(raw, pairs, binaries):
    if raw['kind'] not in ('internal', 'whole'):
        raise ValueError('unknown screen case kind')
    internal = raw['kind'] == 'internal'
    expected_internal = raw['id'].startswith(('broad/', 'sentinel/'))
    if internal != expected_internal:
        raise ValueError('screen case kind differs from its lane')
    if internal and (type(raw.get('iterations')) is not int or raw['iterations'] <= 0):
        raise ValueError('screen internal iterations must be a positive integer')
    samples = raw['samples']
    if len(samples) != pairs * (4 if internal else 2):
        raise ValueError('screen raw sample count mismatch')
    costs = {r: {m: [] for m in ('wall_ns', 'instructions')} for r in ('candidate', 'base')}
    cursor = 0
    workload = None
    for pair in range(pairs):
        outputs = {}
        for role in (('base', 'candidate') if pair % 2 == 0 else ('candidate', 'base')):
            taken = samples[cursor:cursor + (2 if internal else 1)]
            cursor += len(taken)
            if any(s['argv'][0] != binaries[role] for s in taken):
                raise ValueError('screen sample binary path mismatch')
            for i, row in enumerate(taken):
                if any(type(row['sample'][m]) is not int or row['sample'][m] <= 0
                       for m in ('wall_ns', 'instructions')):
                    raise ValueError('screen sample metrics must be positive integers')
                argv = row['argv']
                if len(argv) != (5 if internal else 3) or argv[1] != '--raw':
                    raise ValueError('screen sample command mismatch')
                workload = workload or argv[2]
                if argv[2] != workload:
                    raise ValueError('screen sample workload changed')
                if internal and (argv[3] != raw['id'].split('/', 1)[1]
                                 or int(argv[4]) != raw['iterations'] * (i+1)):
                    raise ValueError('screen sample iteration/case mismatch')
            outputs[role] = [s['sample']['stdout'] for s in taken]
            for metric in ('wall_ns', 'instructions'):
                value = taken[-1]['sample'][metric]
                if internal:
                    value -= taken[0]['sample'][metric]
                if value <= 0 or (internal and metric == 'wall_ns' and value < MIN_INCREMENT_NS):
                    raise ValueError('invalid screen increment')
                costs[role][metric].append(value)
        if outputs['base'] != outputs['candidate']:
            raise ValueError('screen stdout mismatch')
    if costs != raw['costs']:
        raise ValueError('screen costs differ from raw replay')
    ratios = {metric: [c / b for c, b in zip(costs['candidate'][metric], costs['base'][metric])]
              for metric in ('wall_ns', 'instructions')}
    if any(ratios[m] != raw['ratios'][m] for m in ratios):
        raise ValueError('screen ratios differ from raw replay')
    return ratios
