import importlib.util
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "diagnostic_ab", Path(__file__).resolve().parents[3] / "scripts/external-corpus-ab.py")
ab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ab)


class DiagnosticAbTests(unittest.TestCase):
    def test_same_binary_aa_keeps_both_independent_timings(self):
        with patch.object(ab, "run", side_effect=[1, 2, 4, 2]):
            ratios, base, candidate = ab.measure("same-qjs", "same-qjs", "case.js", 2)
        self.assertEqual(ratios, [2, 2])
        self.assertEqual(base, [1, 2])
        self.assertEqual(candidate, [2, 4])

    def test_ng_adapter_forces_script_mode(self):
        with patch.object(ab.subprocess, "run") as run:
            run.return_value.returncode = 0
            ab.run("ng", "case.js", "quickjs-ng")
        self.assertEqual(run.call_args.args[0], ["ng", "--script", "case.js"])

    def test_timeout_is_a_skip_not_a_suite_abort(self):
        with patch.object(ab.subprocess, "run", side_effect=subprocess.TimeoutExpired("qjs", 900)):
            self.assertIsNone(ab.run("qjs", "case.js"))
