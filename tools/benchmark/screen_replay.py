"""Rebuild screen ratios from complete positive per-process sample packets."""
from __future__ import annotations

from .screen import MIN_INCREMENT_NS


def _costs(raw, pairs, binaries, indices=None, accepted=None):
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
    for position in range(pairs):
        pair = indices[position] if indices is not None else position
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
                if accepted is not None and not accepted[position]:
                    continue
                if value <= 0 or (internal and metric == 'wall_ns' and value < MIN_INCREMENT_NS):
                    raise ValueError('invalid screen increment')
                costs[role][metric].append(value)
        if outputs['base'] != outputs['candidate']:
            raise ValueError('screen stdout mismatch')
    ratios = {metric: [c / b for c, b in zip(costs['candidate'][metric], costs['base'][metric])]
              for metric in ('wall_ns', 'instructions')}
    return costs, ratios


def replay_case(raw, pairs, binaries):
    if 'attempts' in raw:
        costs, ratios, samples = replay_attempts(raw, pairs, binaries)
        if samples != raw['samples']:
            raise ValueError('accepted screen samples differ from raw attempts')
    else:
        costs, ratios = _costs(raw, pairs, binaries)
    if costs != raw['costs']:
        raise ValueError('screen costs differ from raw replay')
    if any(ratios[m] != raw['ratios'][m] for m in ratios):
        raise ValueError('screen ratios differ from raw replay')
    return ratios



def replay_attempts(raw, pairs, binaries):
    """Rebuild every round's load-only selection and exact appended inventory."""
    from .pair_disturbance import eligibility
    if raw.get('complete') is not True or not raw.get('rounds'):
        raise ValueError('incomplete pair recovery')
    offset = raw.get('pair_index_offset', 0)
    if type(offset) is not int or offset < 0:
        raise ValueError('invalid pair index offset')
    attempts = raw['attempts']
    previous = 0
    missing = pairs
    flags = []
    previous_text = ''
    for round_ in raw['rounds']:
        count = round_['attempt_count']
        if count != previous + missing or count > len(attempts) or missing <= 0:
            raise ValueError('appended round count mismatch')
        snapshot = round_['load']
        if not snapshot['text'].startswith(previous_text):
            raise ValueError('load history was rewritten')
        previous_text = snapshot['text']
        flags = eligibility(attempts[:count], snapshot)
        missing = pairs - sum(flags)
        previous = count
    if previous != len(attempts) or missing != 0:
        raise ValueError('valid pair count mismatch')
    indices = [i for i, flag in enumerate(flags) if flag]
    if indices != raw['valid_attempt_indices']:
        raise ValueError('load-only pair selection mismatch')
    last = 0
    for i, attempt in enumerate(attempts):
        samples = attempt['samples']
        if (attempt['index'] != i + offset or attempt['started_at'] < last
                or attempt['started_at'] != samples[0]['started_at']
                or attempt['ended_at'] != samples[-1]['ended_at']):
            raise ValueError('pair timestamp/index mismatch')
        for row in samples:
            if not last <= row['started_at'] <= row['ended_at']:
                raise ValueError('sample timestamps overlap/reverse')
            last = row['ended_at']
    all_samples = [s for a in attempts for s in a['samples']]
    costs, ratios = _costs({**raw, 'samples': all_samples}, len(attempts), binaries,
                           list(range(offset, offset + len(attempts))), flags)
    selected = [s for i in indices for s in attempts[i]['samples']]
    return costs, ratios, selected
