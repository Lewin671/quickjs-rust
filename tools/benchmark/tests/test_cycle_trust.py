from __future__ import annotations

import sys
import unittest
from unittest import mock

from tools.benchmark import counters, cycle_trust
from tools.benchmark.process import run_process


class CycleTrustTests(unittest.TestCase):
    def test_a_rate_no_running_core_has_is_not_a_clock(self):
        # 2.5 s of CPU: an Apple silicon core, then the x86_64 host of the module header.
        self.assertTrue(cycle_trust.plausible_cycles(8_000_000_000, 2.5e9))
        self.assertFalse(cycle_trust.plausible_cycles(49_301_441, 2.5e9))
        self.assertFalse(cycle_trust.plausible_cycles(1, 0))

    def test_a_host_without_counters_is_not_distrusted(self):
        with mock.patch.object(counters, "_PROC_PID_RUSAGE", None):
            self.assertIsNone(cycle_trust._probe())

    def test_a_trusted_host_keeps_its_counters(self):
        with mock.patch.object(cycle_trust, "cycles_trusted", return_value=True), \
                mock.patch.object(counters, "_PROC_PID_RUSAGE", "kept"):
            self.assertFalse(cycle_trust.withhold_untrusted_counters())
            self.assertEqual(counters._PROC_PID_RUSAGE, "kept")

    def test_an_untrusted_host_reports_no_counters_at_all(self):
        with mock.patch.object(cycle_trust, "cycles_trusted", return_value=False), \
                mock.patch.object(counters, "_PROC_PID_RUSAGE", counters._PROC_PID_RUSAGE), \
                mock.patch.object(sys, "stderr"):
            self.assertTrue(cycle_trust.withhold_untrusted_counters())
            self.assertFalse(counters.supported())
            result = run_process([sys.executable, "-c", "print(sum(range(100000)))"], 30)
        self.assertEqual((result.exit_code, result.instructions, result.cycles), (0, None, None))


if __name__ == "__main__":
    unittest.main()
