"""Serialize timing on one host and wait out concurrent builds.

Parallel agents may build and test at the same time; measurements must not.
Every measuring entry point holds one host-wide lock for its whole run, and
before starting waits a bounded time for compiler and engine processes that
would compete for cores and caches. See docs/harness.md, "Parallel Task".
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from contextlib import contextmanager
from pathlib import Path
from typing import Callable, Iterator

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_LOCK = ROOT / "target/.perf-measure.lock"
# Processes whose presence means another build, test or measurement is running.
BUSY_COMMANDS = frozenset({"cargo", "rustc", "qjs", "ld", "ld64", "clang", "cc"})
POLL_SECONDS = 2.0


def _alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def _owner(lock: Path) -> dict | None:
    try:
        return json.loads((lock / "owner.json").read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


@contextmanager
def measurement_lock(label: str, lock: Path = DEFAULT_LOCK,
                     sleep: Callable[[float], None] = time.sleep) -> Iterator[None]:
    """Hold the host-wide measurement lock; wait while a live owner holds it."""
    lock.parent.mkdir(parents=True, exist_ok=True)
    announced = False
    while True:
        try:
            lock.mkdir()
            break
        except FileExistsError:
            owner = _owner(lock)
            if owner is not None and not _alive(int(owner.get("pid", 0))):
                _release(lock)  # stale: its owner exited without cleanup
                continue
            if owner is None:
                # Created but not yet described; give the owner a moment.
                sleep(POLL_SECONDS)
                owner = _owner(lock)
                if owner is None:
                    _release(lock)
                    continue
            if not announced:
                print(f"waiting for the measurement lock held by pid {owner.get('pid')} "
                      f"({owner.get('label')}) ...", file=sys.stderr, flush=True)
                announced = True
            sleep(POLL_SECONDS)
    try:
        (lock / "owner.json").write_text(
            json.dumps({"pid": os.getpid(), "label": label, "started": time.time()}), encoding="utf-8")
        yield
    finally:
        _release(lock)


def _release(lock: Path) -> None:
    (lock / "owner.json").unlink(missing_ok=True)
    try:
        lock.rmdir()
    except FileNotFoundError:
        pass


def parse_ps(output: str, own_pid: int) -> list[str]:
    """`pid command` lines -> "command(pid)" for competing processes."""
    busy = []
    for line in output.splitlines():
        parts = line.strip().split(None, 1)
        if len(parts) != 2 or not parts[0].isdigit():
            continue
        pid, command = int(parts[0]), os.path.basename(parts[1].strip())
        if pid != own_pid and command in BUSY_COMMANDS:
            busy.append(f"{command}({pid})")
    return busy


def busy_processes() -> list[str]:
    try:
        completed = subprocess.run(["ps", "-Ao", "pid=,comm="], capture_output=True,
                                   text=True, timeout=10, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return []
    return parse_ps(completed.stdout, os.getpid())


def wait_for_builds(settle_seconds: float, sleep: Callable[[float], None] = time.sleep,
                    probe: Callable[[], list[str]] = busy_processes) -> list[str]:
    """Wait up to `settle_seconds` for competing processes; return any left."""
    waited = 0.0
    busy = probe()
    while busy and waited < settle_seconds:
        if waited == 0.0:
            print(f"waiting for competing processes to finish: {', '.join(busy[:8])}",
                  file=sys.stderr, flush=True)
        sleep(POLL_SECONDS)
        waited += POLL_SECONDS
        busy = probe()
    return busy
