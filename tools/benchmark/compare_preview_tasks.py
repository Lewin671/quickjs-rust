"""Whole-process lane tasks sharing the existing qualified worker limit."""
from functools import partial
import json

from .worker_pool import Task


def preview_tasks(run, scratch, lane):
    cases = range(len(run.cases))
    capability = [scratch / f'{lane}-capability-{i}.jsonl' for i in cases]
    measured = [scratch / f'{lane}-{i}.jsonl' for i in cases]

    def write(path, rows):
        path.write_text(''.join(json.dumps(row) + '\n' for row in rows))

    def read(path):
        return [json.loads(line) for line in path.read_text().splitlines()]

    def probe():
        for i in cases:
            write(capability[i], run.capability(i))

    def measure(i):
        write(measured[i], run.measurement(i, read(capability[i])))

    def schedule():
        costs = {i: sum(r['duration_ns'] for r in read(capability[i])) for i in cases}
        return [Task(f'{lane} case {run.cases[i][1].id}', partial(measure, i))
                for i in sorted(cases, key=lambda i: -costs[i])]

    def records():
        return [row for i in cases for path in (capability[i], measured[i]) for row in read(path)]

    return [Task(f'{lane} capability', probe, schedule)], records
