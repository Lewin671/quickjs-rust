"""Fresh-process wall timing with resource-specific descendant containment."""

from __future__ import annotations

import os
import signal
import subprocess
import threading
import time
from dataclasses import dataclass
from datetime import datetime, timezone


OUTPUT_LIMIT = 64 * 1024


class _Capture:
    def __init__(self) -> None:
        self.retained = bytearray()
        self.truncated = False

    def drain(self, stream: object) -> None:
        read = getattr(stream, "read")
        while True:
            try:
                chunk = read(16 * 1024)
            except (OSError, ValueError):
                return
            if not chunk:
                return
            remaining = max(0, OUTPUT_LIMIT - len(self.retained))
            self.retained.extend(chunk[:remaining])
            self.truncated |= len(chunk) > remaining

    def text(self) -> str:
        decoded = bytes(self.retained).decode("utf-8", errors="replace")
        encoded = decoded.encode("utf-8")
        if len(encoded) <= OUTPUT_LIMIT:
            return decoded
        return encoded[:OUTPUT_LIMIT].decode("utf-8", errors="ignore")


@dataclass(frozen=True)
class ResourceWallResult:
    started_at: str
    duration_ns: int
    exit_code: int | None
    timed_out: bool
    stdout: str
    stderr: str
    stdout_truncated: bool
    stderr_truncated: bool
    descendants_detected: bool
    monitor_error: str | None = None


def run_fresh_process(argv: list[str], timeout_seconds: int) -> ResourceWallResult:
    """Time only spawn-to-direct-reap, then contain any surviving process group."""
    started_at = datetime.now(timezone.utc).isoformat()
    started_ns = time.perf_counter_ns()
    try:
        process = subprocess.Popen(
            argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, start_new_session=True,
        )
    except OSError as error:
        message = str(error) or error.__class__.__name__
        return ResourceWallResult(
            started_at, time.perf_counter_ns() - started_ns, None, False,
            "", message[:OUTPUT_LIMIT], False, False, False,
        )
    assert process.stdout is not None and process.stderr is not None
    stdout = _Capture()
    stderr = _Capture()
    stdout_thread = threading.Thread(target=stdout.drain, args=(process.stdout,), daemon=True)
    stderr_thread = threading.Thread(target=stderr.drain, args=(process.stderr,), daemon=True)
    stdout_thread.start()
    stderr_thread.start()
    direct_done = threading.Event()
    timed_out = threading.Event()

    def kill_at_deadline() -> None:
        if direct_done.is_set():
            return
        timed_out.set()
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass

    watchdog = threading.Timer(timeout_seconds, kill_at_deadline)
    watchdog.daemon = True
    watchdog.start()
    exit_code = process.wait()
    # The metric ends at direct-child reap. Descendant inspection is deliberately
    # excluded so it cannot inflate fresh-process latency.
    duration_ns = time.perf_counter_ns() - started_ns
    direct_done.set()
    watchdog.cancel()
    descendants = False
    if not timed_out.is_set():
        try:
            os.killpg(process.pid, 0)
        except ProcessLookupError:
            pass
        else:
            descendants = True
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    stdout_thread.join()
    stderr_thread.join()
    process.stdout.close()
    process.stderr.close()
    return ResourceWallResult(
        started_at=started_at, duration_ns=duration_ns, exit_code=exit_code,
        timed_out=timed_out.is_set(), stdout=stdout.text(), stderr=stderr.text(),
        stdout_truncated=stdout.truncated, stderr_truncated=stderr.truncated,
        descendants_detected=descendants,
    )
