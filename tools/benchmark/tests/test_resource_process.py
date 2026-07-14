from __future__ import annotations

import os
import subprocess
import sys
import unittest
from pathlib import Path

from tools.benchmark.resource_process import (
    OUTPUT_LIMIT,
    ResourceProcessError,
    normalize_peak_rss,
    run_process_wait4,
)


class _NoWaitPopen:
    wait_called = False
    poll_called = False
    last_pid = None

    def __init__(self, *args, **kwargs):
        self._inner = subprocess.Popen(*args, **kwargs)
        type(self).last_pid = self._inner.pid

    @property
    def pid(self):
        return self._inner.pid

    @property
    def stdout(self):
        return self._inner.stdout

    @property
    def stderr(self):
        return self._inner.stderr

    @property
    def returncode(self):
        return self._inner.returncode

    @returncode.setter
    def returncode(self, value):
        self._inner.returncode = value

    def wait(self, *args, **kwargs):
        type(self).wait_called = True
        raise AssertionError("Popen.wait must not own RSS reaping")

    def poll(self, *args, **kwargs):
        type(self).poll_called = True
        raise AssertionError("Popen.poll must not own RSS reaping")


@unittest.skipUnless(sys.platform in {"darwin", "linux"}, "requires supported wait4 platform")
class ResourceProcessTests(unittest.TestCase):
    def setUp(self) -> None:
        _NoWaitPopen.wait_called = False
        _NoWaitPopen.poll_called = False
        _NoWaitPopen.last_pid = None

    def test_real_wait4_path_reaps_without_popen_wait_or_poll(self) -> None:
        flags = []

        def wait4(pid, options):
            flags.append(options)
            return os.wait4(pid, options)

        result = run_process_wait4(
            ["/bin/sh", "-c", "printf ok"], 5,
            platform_name=sys.platform, popen_factory=_NoWaitPopen, wait4_fn=wait4,
        )
        self.assertEqual(result.exit_code, 0)
        self.assertEqual(result.stdout, "ok")
        self.assertGreaterEqual(result.peak_rss_bytes, 0)
        self.assertFalse(result.descendants_detected)
        self.assertFalse(_NoWaitPopen.wait_called)
        self.assertFalse(_NoWaitPopen.poll_called)
        self.assertEqual(flags, [0])

    def test_timeout_is_group_contained_and_wait4_reaped(self) -> None:
        result = run_process_wait4(
            ["/bin/sh", "-c", "sleep 5"], 0,
            platform_name=sys.platform, popen_factory=_NoWaitPopen,
        )
        self.assertTrue(result.timed_out)
        self.assertIsNotNone(result.exit_code)
        self.assertIsNotNone(result.peak_rss_raw)
        self.assertFalse(_NoWaitPopen.wait_called)
        self.assertFalse(_NoWaitPopen.poll_called)

    def test_interrupted_wait4_retries_without_aborting_sample(self) -> None:
        calls = 0

        def interrupted_once(pid, options):
            nonlocal calls
            calls += 1
            if calls == 1:
                raise InterruptedError()
            return os.wait4(pid, options)

        result = run_process_wait4(
            ["/bin/sh", "-c", "printf ok"], 5, platform_name=sys.platform,
            popen_factory=_NoWaitPopen, wait4_fn=interrupted_once,
        )
        self.assertEqual(result.exit_code, 0)
        self.assertIsNone(result.monitor_error)
        self.assertEqual(calls, 2)

    def test_wait4_oserror_kills_and_reaps_with_no_rss(self) -> None:
        calls = 0

        def fails_once(pid, options):
            nonlocal calls
            calls += 1
            if calls == 1:
                raise OSError("transient wait4 failure")
            return os.wait4(pid, options)

        result = run_process_wait4(
            ["/bin/sh", "-c", "sleep 30"], 5, platform_name=sys.platform,
            popen_factory=_NoWaitPopen, wait4_fn=fails_once,
        )
        pid = _NoWaitPopen.last_pid
        self.assertIn("transient wait4 failure", result.monitor_error)
        self.assertIsNone(result.peak_rss_raw)
        self.assertIsNone(result.peak_rss_bytes)
        self.assertIsNotNone(result.exit_code)
        with self.assertRaises(ChildProcessError):
            os.waitpid(pid, os.WNOHANG)

    def test_permanent_wait4_error_uses_same_reaper_waitpid_cleanup(self) -> None:
        def always_fails(pid, options):
            raise OSError("permanent wait4 failure")

        result = run_process_wait4(
            ["/bin/sh", "-c", "sleep 30"], 5, platform_name=sys.platform,
            popen_factory=_NoWaitPopen, wait4_fn=always_fails,
        )
        pid = _NoWaitPopen.last_pid
        self.assertIn("permanent wait4 failure", result.monitor_error)
        self.assertIsNone(result.peak_rss_raw)
        self.assertIsNone(result.peak_rss_bytes)
        self.assertIsNotNone(result.exit_code)
        with self.assertRaises(ChildProcessError):
            os.waitpid(pid, os.WNOHANG)

    def test_surviving_descendant_is_detected_and_contained(self) -> None:
        result = run_process_wait4(
            ["/bin/sh", "-c", "sleep 5 & exit 0"], 5,
            platform_name=sys.platform, popen_factory=_NoWaitPopen,
        )
        self.assertEqual(result.exit_code, 0)
        self.assertTrue(result.descendants_detected)

    def test_invalid_utf8_remains_bounded_after_replacement(self) -> None:
        result = run_process_wait4(
            [sys.executable, "-c", "import os; os.write(1, b'\\xff' * 65536)"], 5,
            platform_name=sys.platform, popen_factory=_NoWaitPopen,
        )
        self.assertLessEqual(len(result.stdout.encode("utf-8")), OUTPUT_LIMIT)

    def test_spawn_oserror_returns_durable_error_without_reap_api(self) -> None:
        def fail_spawn(*args, **kwargs):
            raise OSError("exec format error")

        result = run_process_wait4(
            ["/not/executable"], 1, platform_name=sys.platform, popen_factory=fail_spawn
        )
        self.assertIsNone(result.exit_code)
        self.assertEqual(result.stderr, "exec format error")
        self.assertFalse(result.timed_out)

    def test_macos_linux_units_and_unsupported_platform(self) -> None:
        self.assertEqual(normalize_peak_rss(17, "darwin"), (17, "bytes"))
        self.assertEqual(normalize_peak_rss(17, "linux"), (17 * 1024, "kibibytes"))
        with self.assertRaisesRegex(ResourceProcessError, "unsupported"):
            normalize_peak_rss(17, "freebsd")

    def test_implementation_never_uses_aggregate_children_or_popen_reap(self) -> None:
        source = Path(run_process_wait4.__code__.co_filename).read_text(encoding="utf-8")
        self.assertNotIn("RUSAGE_CHILDREN", source)
        self.assertNotIn("process.wait(", source)
        self.assertNotIn("process.poll(", source)
        self.assertNotIn("process.communicate(", source)


if __name__ == "__main__":
    unittest.main()
