"""The full gate must propagate each plan failure from its conditional stage."""

import re
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class PerformancePlanValidationTests(unittest.TestCase):
    def run_stage(self, failing_plan):
        source = (ROOT / "scripts/check.sh").read_text()
        functions = []
        for name in ("qjs_check_stage", "qjs_check_performance_units"):
            match = re.search(rf"^{name}\(\) \{{\n.*?^\}}", source, re.M | re.S)
            self.assertIsNotNone(match)
            functions.append(match.group())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "scripts").mkdir()
            plans = root / "tasks/performance-units"
            plans.mkdir(parents=True)
            for name in ("a-first.json", "z-last.json"):
                (plans / name).write_text("{}")
            validator = root / "scripts/performance-decision.sh"
            validator.write_text(
                '#!/usr/bin/env bash\n'
                'printf "%s\\n" "${3##*/}" >> "$ROOT_DIR/visited"\n'
                '[ "${3##*/}" != "$FAILING_PLAN" ] || exit 17\n'
            )
            validator.chmod(0o755)
            script = "\n".join([
                "set -euo pipefail",
                "export PATH=/usr/bin:/bin",
                "export ROOT_DIR=$1 FAILING_PLAN=$2",
                *functions,
                'qjs_check_stage "performance unit plans" qjs_check_performance_units',
            ])
            result = subprocess.run(
                ["bash", "-c", script, "plan-test", str(root), failing_plan],
                capture_output=True, text=True, check=False,
            )
            visited = (root / "visited").read_text().splitlines()
        return result, visited

    def test_first_failure_propagates_and_stops_validation(self):
        result, visited = self.run_stage("a-first.json")
        self.assertEqual(result.returncode, 17, result.stdout + result.stderr)
        self.assertEqual(visited, ["a-first.json"])
        self.assertIn("error: failed performance unit plans", result.stderr)

    def test_last_failure_propagates(self):
        result, visited = self.run_stage("z-last.json")
        self.assertEqual(result.returncode, 17, result.stdout + result.stderr)
        self.assertEqual(visited, ["a-first.json", "z-last.json"])

    def test_valid_plans_all_run(self):
        result, visited = self.run_stage("")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(visited, ["a-first.json", "z-last.json"])
        self.assertIn("check: ok performance unit plans", result.stdout)


if __name__ == "__main__":
    unittest.main()
