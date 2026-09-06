import subprocess
import unittest
from unittest.mock import patch

from tools.benchmark.performance_evaluation import pinned_test262_count
from tools.benchmark.performance_schema import PerformanceDecisionError


class PinnedInventoryTests(unittest.TestCase):
    def test_counts_javascript_in_the_superproject_pin_not_submodule_head(self):
        pin = "a" * 40
        replies = [
            subprocess.CompletedProcess([], 0, stdout=f"160000 commit {pin}\tthird_party/test262\n"),
            subprocess.CompletedProcess([], 0, stdout="test/a.js\ntest/nested/b.js\ntest/README.md\n"),
        ]
        with patch("tools.benchmark.performance_evaluation.subprocess.run", side_effect=replies) as run:
            self.assertEqual(pinned_test262_count(), 2)
        self.assertEqual(run.call_args_list[0].args[0],
                         ["git", "ls-tree", "HEAD", "third_party/test262"])
        self.assertEqual(run.call_args_list[1].args[0],
                         ["git", "ls-tree", "-r", "--name-only", pin, "--", "test"])

    def test_missing_pin_or_checkout_cannot_validate_conformance(self):
        for error in (FileNotFoundError("no checkout"), subprocess.CalledProcessError(128, "git")):
            with self.subTest(error=type(error).__name__):
                with patch("tools.benchmark.performance_evaluation.subprocess.run", side_effect=error):
                    with self.assertRaisesRegex(PerformanceDecisionError, "cannot verify pinned"):
                        pinned_test262_count()

    def test_empty_tree_fails_closed(self):
        replies = [subprocess.CompletedProcess([], 0, stdout="160000 commit " + "a" * 40 + "\tpath\n"),
                   subprocess.CompletedProcess([], 0, stdout="test/README.md\n")]
        with patch("tools.benchmark.performance_evaluation.subprocess.run", side_effect=replies):
            with self.assertRaisesRegex(PerformanceDecisionError, "inventory is empty"):
                pinned_test262_count()
