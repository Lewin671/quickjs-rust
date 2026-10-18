"""Case-owned process scheduling and replay; pair/load semantics live elsewhere.

Only the parent prepares assets and seals the run. A fork owns each case's
entire layout sequence; deterministic shards and raw ownership are replayed.
Absent metadata denotes historical serial evidence, never parallel evidence.
"""
from __future__ import annotations

import dataclasses
import time
from functools import partial

from .worker_pool import Task, run_tasks


def identity(value=None):
    schedule = (value or {}).get('measurement_schedule', {'kind': 'serial', 'workers': 1})
    if not isinstance(schedule, dict):
        raise ValueError('invalid measurement schedule')
    workers = schedule.get('workers')
    if type(workers) is not int or not 1 <= workers <= 8:
        raise ValueError('invalid measurement schedule workers (1..8)')
    expected = 'serial' if workers == 1 else 'case-shards-v1'
    if schedule.get('kind') != expected:
        raise ValueError('invalid measurement schedule kind')
    return {'kind': expected, 'workers': workers}


def binding(workers):
    return identity({'measurement_schedule': {'kind': 'serial' if workers == 1 else 'case-shards-v1',
                                               'workers': workers}})


def require_same(*values):
    if len({tuple(identity(v).items()) for v in values}) != 1:
        raise ValueError('measurement schedule mismatch: recalibration required')


def plan(ids, workers):
    binding(workers)
    if not ids or len(set(ids)) != len(ids):
        raise ValueError('case schedule needs unique nonempty inventory')
    return [list(ids[i::workers]) for i in range(workers)]


def verify(value, ids, raw=()):
    """Conserve deterministic shard ownership for every supplied raw packet."""
    schedule = identity(value)
    workers = schedule['workers']
    shards = plan(list(ids), workers)
    if 'case_shards' in value and value['case_shards'] != shards:
        raise ValueError('case shard inventory/order mismatch')
    if workers > 1 and 'case_shards' not in value:
        raise ValueError('parallel evidence missing case shards')
    owners = {name: i for i, names in enumerate(shards) for name in names}
    pids = {}
    for packet in raw:
        require_same(value, packet)
        if packet['id'] not in owners:
            raise ValueError('unknown scheduled case')
        if workers > 1 and packet.get('case_shard') != owners[packet['id']]:
            raise ValueError('raw case shard ownership mismatch')
        if workers > 1:
            pid, owner = packet.get('worker_pid'), owners[packet['id']]
            if type(pid) is not int or pid <= 0 or pids.setdefault(owner, pid) != pid:
                raise ValueError('case shard process identity mismatch')
    if workers > 1 and len(set(pids.values())) != len(pids):
        raise ValueError('distinct case shards share one process')
    return schedule


def _shard(cases, variants, binaries, pairs, tool, timeout, output, monitor, schedule, index, measure):
    try:
        for case in cases:
            for k in variants:
                print('COLLECT', index, k, case.id, flush=True)
                target = output / f'variant{k}' / (case.id.replace('/', '--') + '.json')
                result = measure(case, binaries[k]['candidate'], binaries[k]['base'], pairs,
                                 tool, timeout, target, monitor,
                                 schedule=schedule, case_shard=index)
                # Summaries are reconstructed by the parent from the lossless raw packet.
                if result.id != case.id:
                    raise ValueError('worker returned another case')
    except Exception as error:
        from .layout_identity import write_json
        write_json(output / f'worker-error-{index}.json', {'error': str(error)})
        raise


def collect(cases, variants, binaries, pairs, tool, timeout, output, monitor, workers=1, *, measure=None):
    from .pair_recording import measure as recorder
    from . import screen
    measure = measure or recorder
    schedule = binding(workers)
    ids = [c.id for c in cases]
    shards = plan(ids, workers)
    for k in variants:
        (output / f'variant{k}').mkdir()
    started = time.monotonic()
    if workers == 1:
        # Preserve the legacy layout-major schedule exactly, including mocks.
        packets = []
        for k in variants:
            results = [dataclasses.asdict(measure(c, binaries[k]['candidate'], binaries[k]['base'],
                        pairs, tool, timeout, output / f'variant{k}' / (c.id.replace('/', '--') + '.json'),
                        monitor)) for c in cases]
            packets.append({'pairs': pairs, 'cases': results, 'measurement_schedule': schedule})
    else:
        # Probe counter availability before fork, as the qualified formal collector does.
        screen.counter_tool()
        tasks = [Task(f'layout shard {i}', partial(_shard, cases[i::workers], variants, binaries,
                       pairs, tool, timeout, output, monitor, schedule, i, measure))
                 for i in range(workers) if shards[i]]
        try:
            run_tasks(tasks, workers)
        except ValueError:
            import json
            errors = [json.loads(p.read_text())['error'] for p in sorted(output.glob('worker-error-*.json'))]
            if errors and all(e.startswith('DISTURBED:') for e in errors):
                raise ValueError('DISTURBED: worker pair caps failed: ' + '; '.join(errors)) from None
            raise
        import json
        packets = []
        for k in variants:
            results = []
            for case in cases:
                raw = json.loads((output / f'variant{k}' / (case.id.replace('/', '--') + '.json')).read_text())
                results.append(dataclasses.asdict(screen.CaseResult(raw['id'], raw['kind'], raw['iterations'], raw['ratios'])))
            packets.append({'pairs': pairs, 'cases': results, 'measurement_schedule': schedule})
    return packets, {'measurement_schedule': schedule, 'case_shards': shards,
                     'collection_seconds': time.monotonic() - started}
