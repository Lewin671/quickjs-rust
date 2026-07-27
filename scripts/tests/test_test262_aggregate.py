import contextlib
import importlib.util
import io
from pathlib import Path
import sys
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "test262-aggregate.py"
SPEC = importlib.util.spec_from_file_location("test262_aggregate", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"unable to load {SCRIPT}")
AGGREGATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AGGREGATE)


def totals(**updates):
    result = dict.fromkeys(AGGREGATE.TOTAL_KEYS, 0)
    result.update(updates)
    return result


class CompleteParityGateTests(unittest.TestCase):
    def test_accepts_complete_parity(self):
        self.assertIsNone(AGGREGATE.complete_parity_error(totals()))

    def test_rejects_failures_timeouts_and_not_run_cases(self):
        error = AGGREGATE.complete_parity_error(
            totals(
                qjsng_pass_rust_fail=2,
                qjsng_pass_rust_timeout=3,
                qjsng_pass_rust_not_run=5,
            )
        )

        self.assertIsNotNone(error)
        self.assertIn("actionable_gap=5", error)
        self.assertIn("fail=2", error)
        self.assertIn("timeout=3", error)
        self.assertIn("quickjs_ng_pass_rust_not_run=5", error)

    def test_cli_flag_returns_nonzero_after_writing_the_summary(self):
        qjsng_cases = {"test/example.js": {"quickjs_ng": "pass"}}
        rust_cases = {"test/example.js": {"rust": "timeout"}}
        stdout = io.StringIO()
        stderr = io.StringIO()
        with (
            mock.patch.object(
                AGGREGATE,
                "load_cases",
                side_effect=[(qjsng_cases, ["qjsng.jsonl"]), (rust_cases, ["rust.jsonl"])],
            ),
            mock.patch.object(
                sys,
                "argv",
                [
                    str(SCRIPT),
                    "--ng-cases",
                    "qjsng.jsonl",
                    "--rust-cases",
                    "rust.jsonl",
                    "--commit",
                    "test",
                    "--require-complete-parity",
                ],
            ),
            contextlib.redirect_stdout(stdout),
            contextlib.redirect_stderr(stderr),
        ):
            self.assertEqual(AGGREGATE.main(), 1)

        self.assertIn("# Test262 Coverage", stdout.getvalue())
        self.assertIn("actionable_gap=1", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
