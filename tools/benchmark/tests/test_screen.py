from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from tools.benchmark import screen
from tools.benchmark.process import ProcessResult

MAC_TAIL = """        {real:.2f} real         {user:.2f} user         0.00 sys
             5423104  maximum resident set size
            {instructions}  instructions retired
             {cycles}  cycles elapsed
             2113848  peak memory footprint
"""


def _result(stdout: str, stderr: str, exit_code: int = 0, wall_ns: int = 1_000) -> ProcessResult:
    return ProcessResult(
        started_at="2026-09-22T00:00:00+00:00",
        timer_started_ns=0,
        timer_finished_ns=wall_ns,
        duration_ns=wall_ns,
        exit_code=exit_code,
        timed_out=False,
        stdout=stdout,
        stderr=stderr,
        stdout_truncated=False,
        stderr_truncated=False,
    )


class FakeEngine:
    """Replays `/usr/bin/time -l` output for a linear cost model per binary."""

    def __init__(self, per_iteration: dict[str, int], startup: int = 5_000_000, checksum=None):
        self.per_iteration = per_iteration
        self.startup = startup
        self.checksum = checksum or {}
        self.calls: list[list[str]] = []

    def __call__(self, argv: list[str], timeout: float) -> ProcessResult:
        self.calls.append(argv)
        binary = argv[2]
        iterations = int(argv[-1]) if argv[-1].isdigit() else 1
        instructions = self.startup + self.per_iteration[binary] * iterations
        stdout = "QJS_BENCH_RESULT " + json.dumps({
            "case_id": "c", "iterations": iterations, "operations": iterations,
            "checksum": self.checksum.get(binary, iterations * 3),
        }) + "\n"
        stderr = MAC_TAIL.format(real=0.5, user=0.5, instructions=instructions, cycles=instructions // 2)
        return _result(stdout, stderr, wall_ns=instructions)


class ParseTests(unittest.TestCase):
    def test_macos_time_counters_after_engine_stderr(self):
        stderr = "engine warning\n" + MAC_TAIL.format(real=1.02, user=1.01, instructions=17828271888,
                                                      cycles=3273487779)
        values = screen.parse_macos_time(stderr)
        self.assertEqual(values["instructions"], 17828271888)
        self.assertEqual(values["cycles"], 3273487779)
        self.assertEqual(values["user_ns"], 1_010_000_000)

    def test_macos_time_without_counters_is_unavailable(self):
        with self.assertRaises(screen.CountersUnavailable):
            screen.parse_macos_time("        0.00 real         0.00 user         0.00 sys\n")

    def test_perf_stat_csv(self):
        stderr = (
            "12.50,msec,task-clock,12500000,100.00,0.950,CPUs utilized\n"
            "41000000,,instructions:u,12500000,100.00,,\n"
            "20000000,,cycles:u,12500000,100.00,,\n"
        )
        values = screen.parse_perf_stat(stderr)
        self.assertEqual(values, {"user_ns": 12_500_000, "instructions": 41000000, "cycles": 20000000})

    def test_perf_stat_not_supported_is_unavailable_not_zero(self):
        stderr = "<not supported>,,instructions:u,0,100.00,,\n20000000,,cycles:u,1,100.00,,\n"
        with self.assertRaises(screen.CountersUnavailable):
            screen.parse_perf_stat(stderr)

    def test_unknown_platform_has_no_counter_tool(self):
        with self.assertRaises(screen.CountersUnavailable):
            screen.counter_tool("Plan9")


class ScreenCaseTests(unittest.TestCase):
    tool = screen.counter_tool("Darwin")
    case = screen.CaseSpec("sentinel/c", "internal", Path("w.js"), "c")

    def test_increment_removes_startup_cost(self):
        engine = FakeEngine({"/cand": 90, "/base": 100}, startup=50_000_000)
        result = screen.screen_case(self.case, Path("/cand"), Path("/base"), 3, self.tool, 10,
                                    iterations=1000, runner=engine)
        # Whole-process ratio would be (50M + 90k) / (50M + 100k); the increment is exact.
        self.assertEqual(result.ratios["instructions"], [0.9, 0.9, 0.9])
        self.assertEqual(result.median("cycles"), 0.9)

    def test_pairs_alternate_role_order(self):
        engine = FakeEngine({"/cand": 1, "/base": 1})
        screen.screen_case(self.case, Path("/cand"), Path("/base"), 2, self.tool, 10,
                           iterations=10, runner=engine)
        first = [call[2] for call in engine.calls[:4]]
        second = [call[2] for call in engine.calls[4:]]
        self.assertEqual(first, ["/base", "/base", "/cand", "/cand"])
        self.assertEqual(second, ["/cand", "/cand", "/base", "/base"])

    def test_checksum_disagreement_fails_the_screen(self):
        engine = FakeEngine({"/cand": 1, "/base": 1}, checksum={"/cand": 7})
        with self.assertRaises(screen.ScreenError):
            screen.screen_case(self.case, Path("/cand"), Path("/base"), 1, self.tool, 10,
                               iterations=10, runner=engine)

    def test_failed_process_fails_the_screen(self):
        def failing(argv, timeout):
            return _result("", "boom", exit_code=1)

        with self.assertRaises(screen.ScreenError):
            screen.screen_case(self.case, Path("/cand"), Path("/base"), 1, self.tool, 10,
                               iterations=10, runner=failing)

    def test_whole_process_case_checks_external_sentinel(self):
        case = screen.CaseSpec("external/s/c", "whole", Path("b.js"), expected_stdout="OK\n")

        def engine(argv, timeout):
            return _result("NOPE\n", MAC_TAIL.format(real=0.1, user=0.1, instructions=10, cycles=5))

        with self.assertRaises(screen.ScreenError):
            screen.screen_case(case, Path("/cand"), Path("/base"), 1, self.tool, 10, runner=engine)

    def test_calibration_grows_until_target(self):
        seen = []

        def measure(iterations):
            seen.append(iterations)
            return screen.Sample(1, 1, iterations * 1_000, iterations * 1_000, "")

        chosen = screen.calibrate(measure, start=1024)
        self.assertGreaterEqual(chosen * 1_000, screen.CALIBRATION_TARGET_NS)
        self.assertEqual(seen, sorted(seen))


class CliTests(unittest.TestCase):
    def test_busy_host_is_reported_not_refused(self):
        with mock.patch("os.getloadavg", return_value=(9.0, 9.0, 9.0)):
            self.assertEqual(screen.check_load(4.0, require_quiet=False), 9.0)
            with self.assertRaises(screen.ScreenError):
                screen.check_load(4.0, require_quiet=True)

    def test_load_is_given_time_to_settle(self):
        loads = iter([9.0, 7.0, 3.0])
        slept = []
        load = screen.check_load(4.0, require_quiet=True, settle_seconds=60,
                                 sleep=slept.append, loadavg=lambda: (next(loads), 0.0, 0.0))
        self.assertEqual((load, slept), (3.0, [5.0, 5.0]))

    def test_settling_stops_after_its_budget(self):
        slept = []
        load = screen.check_load(4.0, require_quiet=False, settle_seconds=10,
                                 sleep=slept.append, loadavg=lambda: (9.0, 0.0, 0.0))
        self.assertEqual((load, slept), (9.0, [5.0, 5.0]))

    def test_aa_rejects_an_explicit_base(self):
        with tempfile.TemporaryDirectory() as work:
            binary = Path(work) / "qjs"
            binary.write_text("#!/bin/sh\n")
            binary.chmod(0o755)
            self.assertEqual(screen.main(["--candidate", str(binary), "--base", str(binary), "--aa"]), 2)

    def test_resolve_named_and_file_cases(self):
        with tempfile.TemporaryDirectory() as work:
            script = Path(work) / "s.js"
            script.write_text("1;\n")
            cases = screen.resolve_cases(["sentinel/recursive_call_tree", f"file:{script}"],
                                         Path(work), Path(work))
            self.assertEqual([c.kind for c in cases], ["internal", "whole"])
            with self.assertRaises(screen.ScreenError):
                screen.resolve_cases(["sentinel/no_such_case"], Path(work), Path(work))

    def test_render_marks_output_as_not_decision_evidence(self):
        result = screen.CaseResult("x", "internal", 10,
                                   {"instructions": [1.0], "cycles": [0.9], "wall_ns": [1.1]})
        text = screen.render([result], 1, aa=False)
        self.assertIn("not decision evidence", text)
        self.assertIn("geomean cycles: 0.9000", text)


if __name__ == "__main__":
    unittest.main()
