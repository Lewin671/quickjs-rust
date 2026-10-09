from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.hosted_preview import PUSH_MODE, collect, publish
from tools.benchmark.preview import summarize
from tools.benchmark.tests.test_preview import report
from tools.benchmark.tests.test_preview_summary import external, external_case, sentinel

ROOT = Path(__file__).resolve().parents[3]
BINARIES = {"candidate": "1" * 64, "base": "2" * 64, "quickjs-ng": "3" * 64}


def write(path: Path, value: object) -> None:
    path.write_text(json.dumps(value), encoding="utf-8")


def stage(output: Path, name: str, state: str = "success", phase: str = "complete",
          message: str = "done", attempt: str | None = "1") -> None:
    write(output / f"{name}-status.json", {
        "schema_version": 2, "stage": name, "state": state, "phase": phase,
        "message": message, "run_attempt": attempt,
    })


def whole_run(output: Path) -> None:
    """Evidence as four successful stage jobs would leave it after merging."""
    output.mkdir(parents=True, exist_ok=True)
    stage(output, "build")
    write(output / "build-identity.json", {"schema_version": 1, "binaries": BINARIES})
    _, machine = summarize(report(1.02, 1.4), harness_mode=PUSH_MODE, harness_revision="1" * 40)
    stage(output, "broad")
    write(output / "summary.json", machine)
    stage(output, "external")
    write(output / "external-report.json", {
        **external([external_case("ai-astar", 1.015, 1.353)]), "binary_sha256": BINARIES,
    })
    stage(output, "sentinel")
    write(output / "sentinel-summary.json", sentinel({"recursive_call_tree": 0.97}))
    for role, digest in BINARIES.items():
        write(output / f"sentinel-{role}-receipt.json", {"binary_sha256": digest})


class PublishTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.output = self.root / "evidence"
        self.step_summary = self.root / "step-summary.md"

    def run_publish(self) -> tuple[bool, str, dict]:
        complete = publish(self.output, self.step_summary)
        markdown = (self.output / "summary.md").read_text(encoding="utf-8")
        self.assertEqual(self.step_summary.read_text(encoding="utf-8"), markdown)
        return complete, markdown, json.loads((self.output / "status.json").read_text())

    def test_whole_evidence_publishes_every_lane_and_seals_the_bundle(self) -> None:
        whole_run(self.output)
        complete, markdown, status = self.run_publish()
        self.assertTrue(complete)
        self.assertEqual(status["state"], "success")
        self.assertEqual(status["lanes_with_evidence"], ["broad", "external", "sentinel"])
        self.assertEqual(status["stages"]["build"]["state"], "success")
        for row in (
            "| Interpreter sentinels | 1/1 |", "| Kraken 1\\.1 | 1/1 |",
            "| Broad microbenchmarks (specializer coverage) | 25/25 | +2.0% | 1.400× |",
        ):
            self.assertIn(row, markdown)
        self.assertNotIn("No complete performance conclusion", markdown)
        self.assertIn("bundle", json.loads((self.output / "summary.json").read_text()))

    def test_a_failed_required_lane_fails_the_preview_but_keeps_the_others(self) -> None:
        whole_run(self.output)
        (self.output / "summary.json").unlink()
        stage(self.output, "broad", "failed", "measurement", "the broad stage failed")
        complete, markdown, status = self.run_publish()
        self.assertFalse(complete)
        self.assertEqual(status["state"], "failed")
        self.assertEqual(status["classification"], "no_performance_conclusion")
        self.assertEqual(status["lanes_with_evidence"], ["external", "sentinel"])
        self.assertIn(
            "> **No complete performance conclusion was produced.** The broad lane is "
            "required and has no admitted evidence.",
            markdown,
        )
        self.assertIn("the broad lane did not finish \\(state failed, phase measurement\\)", markdown)
        self.assertIn("| Kraken 1\\.1 | 1/1 |", markdown)
        self.assertIn("| Interpreter sentinels | 1/1 |", markdown)

    def test_an_incomplete_sentinel_lane_is_reported_without_failing(self) -> None:
        whole_run(self.output)
        (self.output / "sentinel-summary.json").unlink()
        stage(
            self.output, "sentinel", "incomplete", "sentinel_measurement",
            "the sentinel lane did not produce a valid reading within its deadline.",
        )
        complete, markdown, status = self.run_publish()
        self.assertTrue(complete)
        self.assertEqual(status["lanes_with_evidence"], ["broad", "external"])
        self.assertIn("| Interpreter sentinels | — | — | — |", markdown)
        self.assertIn("did not produce a valid reading within its deadline", markdown)

    def test_a_lane_that_measured_other_executables_is_not_admitted(self) -> None:
        whole_run(self.output)
        report_path = self.output / "external-report.json"
        tampered = json.loads(report_path.read_text())
        tampered["binary_sha256"] = {**BINARIES, "candidate": "9" * 64}
        write(report_path, tampered)
        write(self.output / "sentinel-base-receipt.json", {"binary_sha256": "8" * 64})
        complete, markdown, status = self.run_publish()
        self.assertFalse(complete)
        self.assertEqual(status["lanes_with_evidence"], ["broad"])
        self.assertEqual(
            markdown.count("did not measure the executables the build stage recorded"), 2
        )
        self.assertNotIn("Kraken", markdown)

    def test_success_without_readable_evidence_is_not_admitted(self) -> None:
        whole_run(self.output)
        (self.output / "summary.json").write_text("{not json", encoding="utf-8")
        (self.output / "external-report.json").unlink()
        _, evidence, notes = collect(self.output)
        self.assertEqual(sorted(evidence), ["sentinel"])
        self.assertIn("reported success but its evidence is unreadable", notes["broad"])
        self.assertIn("reported success but its evidence is unreadable", notes["external"])

    def test_parseable_but_malformed_evidence_cannot_stop_publication(self) -> None:
        whole_run(self.output)
        # Correct hashes, so only the missing structure can keep these out.
        write(self.output / "external-report.json", {"binary_sha256": BINARIES})
        machine = json.loads((self.output / "summary.json").read_text())
        del machine["comparisons"]["candidate vs base"]["cases"][0]["ratio"]
        write(self.output / "summary.json", machine)
        write(self.output / "sentinel-summary.json", {"comparisons": {
            "candidate vs base": {"cases": "not a list"},
            "candidate vs QuickJS-NG": {"cases": []},
        }})
        complete, markdown, status = self.run_publish()
        self.assertFalse(complete)
        self.assertEqual(status["lanes_with_evidence"], [])
        self.assertEqual(markdown.count("reported success but its evidence is malformed"), 3)

        # One malformed lane leaves the healthy ones published.
        self.step_summary.unlink()
        whole_run(self.output)
        write(self.output / "sentinel-summary.json", {"comparisons": {"candidate vs base": 7}})
        complete, markdown, status = self.run_publish()
        self.assertTrue(complete)
        self.assertEqual(status["lanes_with_evidence"], ["broad", "external"])
        self.assertIn("the sentinel lane reported success but its evidence is malformed", markdown)

    def test_an_earlier_attempts_build_record_cannot_outvote_a_failed_build(self) -> None:
        whole_run(self.output)
        complete = publish(self.output, self.step_summary, build_succeeded=False)
        status = json.loads((self.output / "status.json").read_text())
        self.assertFalse(complete)
        self.assertEqual(status["lanes_with_evidence"], [])
        self.assertEqual(status["stages"]["build"]["state"], "superseded")
        result = subprocess.run(
            [
                sys.executable, "-m", "tools.benchmark.hosted_preview", "publish",
                "--output-dir", str(self.output),
                "--step-summary", str(self.root / "again.md"), "--build-result", "failure",
            ],
            cwd=ROOT, capture_output=True, text=True, timeout=20, check=False,
        )
        self.assertEqual(result.returncode, 2, result.stderr)

    def test_a_lane_record_older_than_the_build_is_a_leftover_not_evidence(self) -> None:
        # Everything was rerun: the build repeated as attempt 2, the broad
        # lane failed before it could upload, and its attempt-1 artifact with
        # identical executables is still there.
        whole_run(self.output)
        stage(self.output, "build", attempt="2")
        stage(self.output, "external", attempt="2")
        stage(self.output, "sentinel", attempt="2")
        complete, markdown, status = self.run_publish()
        self.assertFalse(complete)
        self.assertEqual(status["state"], "failed")
        self.assertEqual(status["lanes_with_evidence"], ["external", "sentinel"])
        self.assertIn("predates this run's build", markdown)

        # Only the failed lane was rerun: the build and the other lanes keep
        # attempt 1, and the repeated lane's attempt 2 is newer than the build.
        self.step_summary.unlink()
        whole_run(self.output)
        stage(self.output, "broad", attempt="2")
        complete, _, status = self.run_publish()
        self.assertTrue(complete)
        self.assertEqual(status["lanes_with_evidence"], ["broad", "external", "sentinel"])

        # Outside Actions no attempt is recorded anywhere, which is consistent.
        self.step_summary.unlink()
        whole_run(self.output)
        for name in ("build", "broad", "external", "sentinel"):
            stage(self.output, name, attempt=None)
        self.assertTrue(self.run_publish()[0])
        # A record without an attempt cannot follow a build that has one.
        self.step_summary.unlink()
        stage(self.output, "build", attempt="1")
        self.assertFalse(self.run_publish()[0])

    def test_a_lane_without_any_record_is_reported_and_the_rest_published(self) -> None:
        for lane, expected in (("sentinel", True), ("broad", False), ("external", False)):
            with self.subTest(lane=lane):
                self.step_summary.unlink(missing_ok=True)
                whole_run(self.output)
                (self.output / f"{lane}-status.json").unlink()
                complete, markdown, status = self.run_publish()
                self.assertEqual(complete, expected)
                self.assertEqual(status["stages"][lane]["state"], "missing")
                self.assertNotIn(lane, status["lanes_with_evidence"])
                self.assertEqual(len(status["lanes_with_evidence"]), 2)
                self.assertIn(f"the {lane} lane left no record", markdown)

    def test_a_lane_that_stopped_before_measuring_replaces_its_earlier_success(self) -> None:
        # The record a lane job uploads before its checkout, exactly as the
        # workflow writes it, standing where an earlier success used to be.
        whole_run(self.output)
        (self.output / "broad-status.json").write_text(
            '{"message":"the lane job stopped before measurement began",'
            '"phase":"job_setup","run_attempt":"2","schema_version":2,'
            '"stage":"broad","state":"pending"}\n', encoding="utf-8",
        )
        complete, markdown, status = self.run_publish()
        self.assertFalse(complete)
        self.assertEqual(status["lanes_with_evidence"], ["external", "sentinel"])
        self.assertIn("state pending, phase job\\_setup", markdown)

    def test_a_stage_record_names_the_attempt_that_produced_it(self) -> None:
        import os

        result = subprocess.run(
            [
                sys.executable, "-m", "tools.benchmark.preview", "status",
                "--stage", "external", "--state", "success", "--phase", "complete",
                "--output-dir", str(self.output), "--harness-mode", PUSH_MODE,
                "--harness-revision", "a" * 40, "--candidate-revision", "a" * 40,
                "--base-revision", "c" * 40, "--reference-revision", "d" * 40,
                "--message", "done",
            ],
            cwd=ROOT, env={**os.environ, "GITHUB_RUN_ATTEMPT": "3"},
            capture_output=True, text=True, timeout=20, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        stages, _, _ = collect(self.output)
        self.assertEqual(stages["external"]["run_attempt"], "3")

    def test_no_build_means_no_lane_is_admitted_whatever_it_claims(self) -> None:
        whole_run(self.output)
        stage(self.output, "build", "failed", "build_candidate", "cargo failed")
        complete, markdown, status = self.run_publish()
        self.assertFalse(complete)
        self.assertEqual(status["lanes_with_evidence"], [])
        self.assertEqual(
            markdown.count("the build stage did not produce the three executables"), 3
        )
        self.assertIn("phase build\\_candidate", markdown)
        self.assertNotRegex(markdown, r"\d\.\d+×|[+-]\d+\.\d%")

    def test_an_empty_run_still_publishes_a_truthful_failure(self) -> None:
        complete, markdown, status = self.run_publish()
        self.assertFalse(complete)
        self.assertEqual(status["stages"]["build"]["state"], "missing")
        self.assertIn("No complete performance conclusion was produced", markdown)
        self.assertIn("state missing, phase not\\_started", markdown)

    def test_markdown_a_stage_left_behind_is_never_published(self) -> None:
        whole_run(self.output)
        (self.output / "summary.md").write_text("<script>alert(1)</script>", encoding="utf-8")
        stage(self.output, "sentinel", "incomplete", "sentinel_measurement",
              "![x](https://attacker.invalid/x) <img src=x>")
        (self.output / "sentinel-summary.json").unlink()
        _, markdown, _ = self.run_publish()
        self.assertNotIn("<script>", markdown)
        self.assertNotIn("<img", markdown)
        self.assertNotIn("![x](", markdown)

    def test_the_command_publishes_first_and_then_fails_for_missing_evidence(self) -> None:
        for prepare, expected in ((whole_run, 0), (lambda output: None, 2)):
            with self.subTest(expected=expected), tempfile.TemporaryDirectory() as name:
                output = Path(name) / "evidence"
                prepare(output)
                summary = Path(name) / "step-summary.md"
                result = subprocess.run(
                    [
                        sys.executable, "-m", "tools.benchmark.hosted_preview", "publish",
                        "--output-dir", str(output), "--step-summary", str(summary),
                    ],
                    cwd=ROOT, capture_output=True, text=True, timeout=20, check=False,
                )
                self.assertEqual(result.returncode, expected, result.stderr)
                self.assertEqual(summary.read_bytes(), (output / "summary.md").read_bytes())
                self.assertTrue((output / "status.json").is_file())


if __name__ == "__main__":
    unittest.main()
