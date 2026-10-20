"""Run a test's child command bounded by CPU time, not by host wall time.

A wall-clock `timeout=` on a CLI subprocess fails whenever the host is
contended or stalled (other builds, swap), even though the child does
little work: the resource report CLI needs about 0.1 s of CPU yet once
exceeded a 10 s wall timeout at load 58. The real hazard these bounds
guard against is a child that spins forever, so the limit is CPU seconds
(`ulimit -t`, inherited across exec) with a long wall backstop that only a
child blocked forever reaches.
"""

from __future__ import annotations

import subprocess
from typing import Any, Sequence

CPU_SECONDS = 60
# Well inside the 10-35 minute CI job limits.
WALL_BACKSTOP_SECONDS = 300


def run_bounded(
    argv: Sequence[Any], *, cpu_seconds: int = CPU_SECONDS, **kwargs: Any
) -> subprocess.CompletedProcess:
    kwargs.setdefault("timeout", WALL_BACKSTOP_SECONDS)
    if "input" not in kwargs:
        kwargs.setdefault("stdin", subprocess.DEVNULL)
    return subprocess.run(
        ["/bin/sh", "-c", 'ulimit -t "$1" && shift && exec "$@"', "bounded",
         str(cpu_seconds), *(str(item) for item in argv)],
        **kwargs,
    )
