"""Whether this host's per-process cycle counter follows its clock.

`counters.py` reports whatever `proc_pid_rusage` returns. One x86_64 macOS
host returned about 0.02 cycles per CPU nanosecond, spread 14% between
identical runs whose instruction counts agreed to 0.004%: a number, not a
clock. Judging a comparison by it would be worse than judging wall time.

The probe runs one short busy child and compares its cycles with its CPU
time. A measuring entry point calls `withhold_untrusted_counters` before its
first sample; on a host that fails the probe the frozen collector then
reports no counters, exactly as on a host without them, and every consumer
falls back to wall time. The measurement protocol files are not changed.
"""

from __future__ import annotations

import ctypes
import functools
import os
import subprocess
import sys

from . import counters

# No core that is executing runs this slowly; a lower rate is not a clock.
MIN_CYCLES_PER_CPU_NS = 0.2
_PROBE = "x = 0\nfor i in range(2_000_000):\n    x += i\n"


class _Timebase(ctypes.Structure):
    _fields_ = [("numer", ctypes.c_uint32), ("denom", ctypes.c_uint32)]


def plausible_cycles(cycles: int, cpu_ns: float) -> bool:
    """Whether `cycles` over `cpu_ns` of CPU time can be a core's clock."""
    return cpu_ns > 0 and cycles >= cpu_ns * MIN_CYCLES_PER_CPU_NS


def _probe() -> bool | None:
    """True or False for a host that reports cycles; None if it reports none."""
    rusage = counters._PROC_PID_RUSAGE
    if rusage is None or not counters.supported():
        return None
    try:
        timebase = _Timebase()
        if ctypes.CDLL("/usr/lib/libSystem.B.dylib").mach_timebase_info(ctypes.byref(timebase)) != 0:
            return False
        process = subprocess.Popen([sys.executable, "-c", _PROBE], stdin=subprocess.DEVNULL,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except OSError:
        return False
    info = counters._RusageInfoV4()
    try:
        os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOWAIT)
        read = rusage(process.pid, counters._RUSAGE_INFO_V4, ctypes.byref(info)) == 0
    except ChildProcessError:
        read = False
    finally:
        process.wait()
    if not read or timebase.denom == 0:
        return False
    cpu_ns = (info.ri_user_time + info.ri_system_time) * timebase.numer / timebase.denom
    return plausible_cycles(int(info.ri_cycles), cpu_ns)


@functools.lru_cache(maxsize=1)
def cycles_trusted() -> bool:
    """False only for a host that reports cycles that are not a clock."""
    return _probe() is not False


def withhold_untrusted_counters() -> bool:
    """Make the collector report no counters on a host that fails the probe.

    Returns whether counters were withheld. Call before the first sample.
    """
    if cycles_trusted():
        return False
    counters._PROC_PID_RUSAGE = None
    print("this host's cycle counter does not follow its clock; "
          "recording no hardware counters, wall time decides", file=sys.stderr, flush=True)
    return True


def main(argv: list[str] | None = None) -> int:
    """`python3 -m tools.benchmark.cycle_trust <module> [args...]`: run a
    measuring `tools.benchmark` module with untrusted counters withheld."""
    import runpy

    argv = sys.argv[1:] if argv is None else argv
    if not argv:
        print(f"cycles trusted: {cycles_trusted()}")
        return 0
    withhold_untrusted_counters()
    sys.argv = [f"tools.benchmark.{argv[0]}", *argv[1:]]
    runpy.run_module(f"tools.benchmark.{argv[0]}", run_name="__main__", alter_sys=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
