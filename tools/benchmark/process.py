"""Bounded process execution for benchmark engines and metadata probes."""

from __future__ import annotations

import os
import signal
import subprocess
import threading
import time
from dataclasses import dataclass
from datetime import datetime, timezone

OUTPUT_LIMIT = 64 * 1024


@dataclass(frozen=True)
class ProcessResult:
    started_at: str
    duration_ns: int
    exit_code: int | None
    timed_out: bool
    stdout: str
    stderr: str
    stdout_truncated: bool
    stderr_truncated: bool


class _Capture:
    def __init__(self, limit: int = OUTPUT_LIMIT):
        self.limit = limit
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
            remaining = self.limit - len(self.retained)
            if remaining > 0:
                self.retained.extend(chunk[:remaining])
            if len(chunk) > remaining:
                self.truncated = True

    def text(self) -> str:
        decoded = bytes(self.retained).decode("utf-8", errors="replace")
        encoded = decoded.encode("utf-8")
        if len(encoded) <= self.limit:
            return decoded
        return encoded[: self.limit].decode("utf-8", errors="ignore")


def run_process(argv: list[str], timeout_seconds: float) -> ProcessResult:
    """Run a process with bounded capture and whole-process-group timeout."""
    started_at = datetime.now(timezone.utc).isoformat()
    started = time.perf_counter_ns()
    try:
        process = subprocess.Popen(
            argv,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=os.name == "posix",
        )
    except OSError as error:
        spawn_error = str(error) or error.__class__.__name__
        return ProcessResult(
            started_at=started_at,
            duration_ns=time.perf_counter_ns() - started,
            exit_code=None,
            timed_out=False,
            stdout="",
            stderr=spawn_error[:OUTPUT_LIMIT],
            stdout_truncated=False,
            stderr_truncated=False,
        )

    assert process.stdout is not None
    assert process.stderr is not None
    stdout_capture = _Capture()
    stderr_capture = _Capture()
    stdout_thread = threading.Thread(target=stdout_capture.drain, args=(process.stdout,), daemon=True)
    stderr_thread = threading.Thread(target=stderr_capture.drain, args=(process.stderr,), daemon=True)
    stdout_thread.start()
    stderr_thread.start()
    expired = threading.Event()

    def terminate_at_deadline() -> None:
        if process.poll() is not None:
            return
        expired.set()
        try:
            if os.name == "posix":
                os.killpg(process.pid, signal.SIGKILL)
            else:
                process.kill()
        except ProcessLookupError:
            pass

    watchdog = threading.Timer(timeout_seconds, terminate_at_deadline)
    watchdog.daemon = True
    watchdog.start()
    exit_code = process.wait()
    duration_ns = time.perf_counter_ns() - started
    watchdog.cancel()
    stdout_thread.join(timeout=0.1)
    stderr_thread.join(timeout=0.1)
    if stdout_thread.is_alive() or stderr_thread.is_alive():
        # A child inherited the pipes after the measured shell exited. The
        # benchmark sample ends with the shell, so contain those descendants.
        if os.name == "posix":
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        else:
            process.stdout.close()
            process.stderr.close()
        stdout_thread.join()
        stderr_thread.join()
    process.stdout.close()
    process.stderr.close()
    return ProcessResult(
        started_at=started_at,
        duration_ns=duration_ns,
        exit_code=exit_code,
        timed_out=expired.is_set(),
        stdout=stdout_capture.text(),
        stderr=stderr_capture.text(),
        stdout_truncated=stdout_capture.truncated,
        stderr_truncated=stderr_capture.truncated,
    )
