"""Failure classification must preserve evidence and fail closed on mode defects."""
import copy
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from .run import PROFILES, STRESS, classify, execute, normalize, program


def result(value="ok"):
    return {"exit": 0, "stdout": value, "stderr": "", "normalized": [value],
            "outcome": "ok", "timeout": False, "panic": False, "crash": False,
            "reference_failure": False}


class DifferentialTests(unittest.TestCase):
    def modes(self):
        return {key: result() for key in ("ng", "base", "default", "off", "verify", "stress")}

    def test_common_difference_is_separate_from_regression_and_improvement(self):
        modes = self.modes()
        self.assertEqual(classify(modes), ([], []))
        modes["base"] = result("old bug")
        self.assertEqual(classify(modes)[0], ["FIXED"])
        for mode in ("default", "off", "verify", "stress"):
            modes[mode] = result("old bug")
        self.assertEqual(classify(modes)[0], ["PRE"])
        modes["base"] = result()
        self.assertEqual(classify(modes)[0], ["REGR"])
        modes["base"] = result("another old bug")
        self.assertEqual(classify(modes)[0], ["CHANGED"])

    def test_observation_order_is_not_lost_by_label_comparison(self):
        modes = self.modes()
        for mode in modes:
            modes[mode]["normalized"] = ["first: 1", "second: 2"]
        modes["off"]["normalized"].reverse()
        self.assertIn("INCONS", classify(modes)[0])
        self.assertIn("REGR", classify(modes)[0])

    def test_stress_panic_is_retained_and_cannot_hide_mode_disagreement(self):
        modes = self.modes()
        modes["stress"].update(panic=True, exit=-6, crash=True,
                               normalized=["thread panicked at assertion failed"], outcome="exit:-6")
        before = copy.deepcopy(modes)
        tags, _ = classify(modes)
        self.assertTrue({"PANIC", "CRASH", "INCONS"} <= set(tags))
        self.assertEqual(modes, before)

    def test_normalization_keeps_observations_and_non_error_throws(self):
        self.assertEqual(normalize("Undefined\nUndefined\n", "", True), ["Undefined"])
        self.assertEqual(normalize("v: 1\nUndefined\n", "TypeError: detail\n    at file:1\n", True),
                         ["v: 1", "UNCAUGHT TypeError"])
        self.assertNotEqual(normalize("", "error: throw statement executed: 1", True),
                            normalize("", "error: throw statement executed: 2", True))

    def test_subprocess_clears_inherited_controls_and_keeps_raw_output(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "fake"
            binary.write_text("#!/usr/bin/env python3\nimport os\n"
                              "print(os.environ.get('QJS_UNRELATED','absent'))\n"
                              "print(os.environ.get('QJS_FORCE_EVAL_EXPOSURE','absent'))\n"
                              "print(os.environ['TZ'])\nprint('Undefined')\n")
            binary.chmod(0o755)
            with patch.dict(os.environ, {"QJS_UNRELATED": "bad", "QJS_FORCE_EVAL_EXPOSURE": "bad"}):
                row = execute(binary, Path(directory) / "input.js", STRESS, 3)
            self.assertEqual(row["normalized"], ["absent", "1", "UTC"])
            self.assertTrue(row["stdout"].endswith("Undefined\n"))
            self.assertEqual(row["exit"], 0)

    def test_every_profile_generates_deterministic_source(self):
        for profile in PROFILES:
            self.assertEqual(program(profile, 9), program(profile, 9), profile)
            self.assertNotEqual(program(profile, 9), program(profile, 10), profile)
        self.assertTrue(program("global-store", 0).endswith('SNAP("final");\n'))


if __name__ == "__main__":
    unittest.main()
