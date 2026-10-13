"""Run measurement tasks on a fixed number of forked worker processes.

A task is work that one process does from start to finish, so its timers
never wait on another task's interpreter. The pool starts the tasks in the
order given, at most `workers` at a time; a finished task may hand back
further tasks, which join the end of the queue. After a failure nothing new
starts and the pool reports every task that failed.
"""
from __future__ import annotations

import multiprocessing
import sys
from collections import deque
from dataclasses import dataclass
from multiprocessing.connection import wait
from typing import Callable


@dataclass(frozen=True)
class Task:
    name: str
    run: Callable[[], None]  # in the worker
    then: Callable[[], list["Task"]] | None = None  # in the parent, after success


def _work(task: Task) -> None:
    try:
        task.run()
    except Exception as error:  # the parent reports the task; keep its reason visible
        print(f"compare: {task.name} failed: {error}", file=sys.stderr, flush=True)
        raise SystemExit(1)


def run_tasks(tasks: list[Task], workers: int) -> None:
    # A fork inherits the parent's open run and withheld counters (cycle_trust.py).
    context = multiprocessing.get_context("fork")
    pending = deque(tasks)
    running = {}
    failed: list[str] = []
    while running or (pending and not failed):
        while pending and not failed and len(running) < workers:
            task = pending.popleft()
            process = context.Process(target=_work, args=(task,))
            process.start()
            running[process.sentinel] = (process, task)
        for sentinel in wait(list(running)):
            process, task = running.pop(sentinel)
            process.join()
            if process.exitcode != 0:
                failed.append(task.name)
            elif task.then is not None and not failed:
                pending.extend(task.then())
    if failed:
        raise ValueError(f"{', '.join(failed)} failed; no conclusion")
