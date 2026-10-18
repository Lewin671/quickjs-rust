"""Case-owned npm lane workers; retain each case's original three-role plan."""
from __future__ import annotations

import json
import os
from functools import partial
from pathlib import Path

from .case_schedule import binding, plan, require_same, verify
from .worker_pool import Task, run_tasks


def _work(run, indices, path, schedule, shard):
    with path.open('x') as stream:
        for index in indices:
            capability = run.capability(index)
            records = capability + run.measurement(index, capability)
            for row in records:
                stream.write(json.dumps({**row, 'measurement_schedule': schedule, 'case_shard': shard, 'worker_pid': os.getpid()}) + '\n')
                stream.flush()


def measure(run, output, workers):
    schedule = binding(workers)
    ids = [s.id + '/' + c.id for s, c, _, _ in run.cases]
    shards = plan(ids, workers)
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    paths = [output / f'case-shard-{i}.jsonl' for i in range(workers)]
    run_tasks([Task(f'npm shard {i}', partial(_work, run, list(range(len(ids)))[i::workers],
               path, schedule, i)) for i, path in enumerate(paths) if shards[i]], workers)
    records = [json.loads(line) for i, path in enumerate(paths) if shards[i] for line in path.read_text().splitlines()]
    order = {name: i for i, name in enumerate(ids)}
    records.sort(key=lambda r: order[r['suite_id'] + '/' + r['case_id']])
    return records


def replay_schedule(manifest, records):
    if not any('measurement_schedule' in r for r in records):
        return {}
    from .case_schedule import identity
    schedule = identity(records[0])
    require_same(*records)
    ids = [s.id + '/' + c.id for s in manifest.suites for c in s.cases]
    value = {'measurement_schedule': schedule, 'case_shards': plan(ids, schedule['workers'])}
    verify(value, ids, [{**r, 'id': r['suite_id'] + '/' + r['case_id']} for r in records])
    return value
