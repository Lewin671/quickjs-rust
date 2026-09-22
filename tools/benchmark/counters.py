"""Per-process hardware counters for an exited, not yet reaped child.

On macOS, `proc_pid_rusage(RUSAGE_INFO_V4)` reports the instructions retired
and cycles of a zombie process, so a measured engine runs with its argv and
timer unchanged. Other hosts report no counters (None); callers must treat a
missing counter as unavailable evidence, never as zero.
"""

from __future__ import annotations

import ctypes
import os
import platform

_RUSAGE_INFO_V4 = 4
_V4_FIELDS = (
    "ri_user_time", "ri_system_time", "ri_pkg_idle_wkups", "ri_interrupt_wkups",
    "ri_pageins", "ri_wired_size", "ri_resident_size", "ri_phys_footprint",
    "ri_proc_start_abstime", "ri_proc_exit_abstime", "ri_child_user_time",
    "ri_child_system_time", "ri_child_pkg_idle_wkups", "ri_child_interrupt_wkups",
    "ri_child_pageins", "ri_child_elapsed_abstime", "ri_diskio_bytesread",
    "ri_diskio_byteswritten", "ri_cpu_time_qos_default", "ri_cpu_time_qos_maintenance",
    "ri_cpu_time_qos_background", "ri_cpu_time_qos_utility", "ri_cpu_time_qos_legacy",
    "ri_cpu_time_qos_user_initiated", "ri_cpu_time_qos_user_interactive",
    "ri_billed_system_time", "ri_serviced_system_time", "ri_logical_writes",
    "ri_lifetime_max_phys_footprint", "ri_instructions", "ri_cycles", "ri_billed_energy",
    "ri_serviced_energy", "ri_interval_max_phys_footprint", "ri_runnable_time",
)


class _RusageInfoV4(ctypes.Structure):
    _fields_ = [("ri_uuid", ctypes.c_uint8 * 16)] + [(name, ctypes.c_uint64) for name in _V4_FIELDS]


def _load_libproc():
    if platform.system() != "Darwin":
        return None
    try:
        library = ctypes.CDLL("/usr/lib/libproc.dylib")
        function = library.proc_pid_rusage
    except (OSError, AttributeError):
        return None
    function.argtypes = (ctypes.c_int, ctypes.c_int, ctypes.c_void_p)
    function.restype = ctypes.c_int
    return function


_PROC_PID_RUSAGE = _load_libproc()


def supported() -> bool:
    return _PROC_PID_RUSAGE is not None and hasattr(os, "waitid") and hasattr(os, "WNOWAIT")


def read_zombie(pid: int) -> tuple[int, int] | None:
    """(instructions, cycles) of exited child `pid`, or None if unavailable."""
    if _PROC_PID_RUSAGE is None:
        return None
    info = _RusageInfoV4()
    if _PROC_PID_RUSAGE(pid, _RUSAGE_INFO_V4, ctypes.byref(info)) != 0:
        return None
    if info.ri_instructions == 0 or info.ri_cycles == 0:
        return None
    return int(info.ri_instructions), int(info.ri_cycles)
