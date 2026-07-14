"""Direct-child execution with wait4-owned reaping for peak-RSS evidence."""

from __future__ import annotations

import os
import selectors
import signal
import subprocess
import sys
import threading
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any, Callable


OUTPUT_LIMIT = 64 * 1024


class ResourceProcessError(RuntimeError):
    """The platform cannot produce trustworthy direct-child RSS evidence."""


@dataclass(frozen=True)
class ResourceProcessResult:
    exit_code: int | None
    timed_out: bool
    stdout: str
    stderr: str
    stdout_truncated: bool
    stderr_truncated: bool
    duration_ns: int
    started_at: str
    peak_rss_raw: int | None
    peak_rss_bytes: int | None
    descendants_detected: bool
    monitor_error: str | None = None


def normalize_peak_rss(raw_value: Any, platform_name: str) -> tuple[int, str]:
    """Normalize POSIX ru_maxrss using only documented macOS/Linux units."""
    if isinstance(raw_value, bool) or not isinstance(raw_value, (int, float)):
        raise ResourceProcessError("ru_maxrss must be numeric")
    if raw_value < 0 or int(raw_value) != raw_value:
        raise ResourceProcessError("ru_maxrss must be a non-negative integer")
    raw = int(raw_value)
    if platform_name == "darwin":
        return raw, "bytes"
    if platform_name.startswith("linux"):
        return raw * 1024, "kibibytes"
    raise ResourceProcessError(f"unsupported ru_maxrss platform {platform_name!r}")


def _bounded_append(target: bytearray, chunk: bytes) -> bool:
    remaining = max(0, OUTPUT_LIMIT - len(target))
    target.extend(chunk[:remaining])
    return len(chunk) > remaining


def _bounded_text(value: bytearray) -> str:
    decoded = bytes(value).decode("utf-8", errors="replace")
    encoded = decoded.encode("utf-8")
    if len(encoded) <= OUTPUT_LIMIT:
        return decoded
    return encoded[:OUTPUT_LIMIT].decode("utf-8", errors="ignore")


def _decode_status(status: int) -> int:
    try:
        return os.waitstatus_to_exitcode(status)
    except ValueError as error:
        raise ResourceProcessError(f"cannot decode child wait status {status}") from error


def run_process_wait4(
    argv: list[str],
    timeout_seconds: int,
    *,
    platform_name: str | None = None,
    wait4_fn: Callable[[int, int], tuple[int, int, Any]] | None = None,
    popen_factory: Callable[..., Any] = subprocess.Popen,
    clock_ns: Callable[[], int] = time.perf_counter_ns,
) -> ResourceProcessResult:
    """Spawn one process and exclusively reap that PID with wait4.

    No Popen wait/poll/communicate API is used: those APIs compete for the
    single wait status and can erase the per-child rusage required here.
    """
    platform_name = platform_name or sys.platform
    # Fail before spawning on platforms whose ru_maxrss unit is not frozen.
    normalize_peak_rss(0, platform_name)
    if not hasattr(os, "wait4") and wait4_fn is None:
        raise ResourceProcessError("os.wait4 is unavailable")
    wait4_fn = wait4_fn or os.wait4
    started_at = datetime.now(timezone.utc).isoformat()
    started_ns = clock_ns()
    try:
        process = popen_factory(
            argv,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
    except OSError as error:
        message = str(error) or error.__class__.__name__
        return ResourceProcessResult(
            None, False, "", message, False, False, clock_ns() - started_ns,
            started_at, None, None, False, None,
        )

    selector = selectors.DefaultSelector()
    streams = {"stdout": bytearray(), "stderr": bytearray()}
    truncated = {"stdout": False, "stderr": False}
    for name, stream in (("stdout", process.stdout), ("stderr", process.stderr)):
        if stream is None:
            raise ResourceProcessError(f"child {name} pipe was not created")
        os.set_blocking(stream.fileno(), False)
        selector.register(stream, selectors.EVENT_READ, name)

    deadline_ns = started_ns + timeout_seconds * 1_000_000_000
    waited_box: list[tuple[int, int, Any | None]] = []
    monitor_errors: list[str] = []
    reaped = threading.Event()
    timed_out = False

    def kill_invocation() -> None:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        except OSError:
            # A racing session/group transition must not strand the sole
            # reaper. The direct child is always ours, so retain a cleanup
            # path even when the group operation is unavailable.
            try:
                os.kill(process.pid, signal.SIGKILL)
            except OSError:
                pass

    def reap_direct_child() -> None:
        failures = 0
        while True:
            try:
                waited_box.append(wait4_fn(process.pid, 0))
                break
            except InterruptedError:
                continue
            except Exception as error:
                failures += 1
                if not monitor_errors:
                    monitor_errors.append(f"{error.__class__.__name__}: {error}")
                kill_invocation()
                if failures < 2:
                    continue
                # Permanent wait4 failure: the same sole reaper owner may use
                # waitpid only for cleanup. No RSS is accepted from this sample.
                while True:
                    try:
                        pid, status = os.waitpid(process.pid, 0)
                        waited_box.append((pid, status, None))
                        break
                    except InterruptedError:
                        continue
                break
        reaped.set()

    reaper = threading.Thread(target=reap_direct_child, daemon=True)
    reaper.start()

    def read_ready(timeout: float) -> None:
        for key, _events in selector.select(timeout):
            try:
                chunk = os.read(key.fileobj.fileno(), 8192)
            except BlockingIOError:
                continue
            if chunk:
                truncated[key.data] |= _bounded_append(streams[key.data], chunk)
            else:
                selector.unregister(key.fileobj)
                key.fileobj.close()

    try:
        while not reaped.is_set():
            read_ready(0.01)
            if not timed_out and clock_ns() >= deadline_ns:
                timed_out = True
                kill_invocation()
        reaper.join()

        # The direct child is reaped. Drain bytes already committed to its
        # pipes, then close; descendants are intentionally outside RSS scope.
        for _ in range(64):
            if not selector.get_map():
                break
            ready = selector.select(0)
            if not ready:
                break
            read_ready(0)
    finally:
        for key in list(selector.get_map().values()):
            selector.unregister(key.fileobj)
            key.fileobj.close()
        selector.close()

    waited = waited_box[0] if waited_box else None
    if waited is None or waited[0] != process.pid:
        raise ResourceProcessError("wait4 returned the wrong direct child")
    exit_code = _decode_status(waited[1])
    process.returncode = exit_code  # Suppress Popen's destructor reap attempt.
    descendants_detected = False
    if not timed_out and not monitor_errors:
        # start_new_session makes PGID equal the direct child's PID. Immediately
        # after reaping that leader, a surviving group can only be a descendant
        # from this invocation; contain it before PID/PGID reuse becomes plausible.
        try:
            os.killpg(process.pid, 0)
        except ProcessLookupError:
            pass
        except PermissionError as error:
            raise ResourceProcessError("cannot inspect direct child's process group") from error
        else:
            descendants_detected = True
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    peak_raw = None
    peak_bytes = None
    if not monitor_errors and waited[2] is not None:
        peak_raw = int(waited[2].ru_maxrss)
        peak_bytes, _raw_unit = normalize_peak_rss(peak_raw, platform_name)
    return ResourceProcessResult(
        exit_code=exit_code,
        timed_out=timed_out,
        stdout=_bounded_text(streams["stdout"]),
        stderr=_bounded_text(streams["stderr"]),
        stdout_truncated=truncated["stdout"],
        stderr_truncated=truncated["stderr"],
        duration_ns=clock_ns() - started_ns,
        started_at=started_at,
        peak_rss_raw=peak_raw,
        peak_rss_bytes=peak_bytes,
        descendants_detected=descendants_detected,
        monitor_error=monitor_errors[0] if monitor_errors else None,
    )
