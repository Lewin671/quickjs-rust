from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.hosted_preview import (
    BROAD_STAGES,
    PUSH_MODE,
    HostedPreviewError,
    collect,
    merge_broad_shards,
    publish,
)
from tools.benchmark.preview import HOSTED_CASES, shard_cases, summarize
from tools.benchmark.tests.test_preview import report, without_legend
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


def shard_machine(number: int, ratio_base: float = 1.02) -> dict:
    return summarize(
        report(ratio_base, 1.4, shard_cases(number)), harness_mode=PUSH_MODE,
        harness_revision="1" * 40, shard=number,
    )[1]


def whole_run(output: Path) -> None:
    """Evidence as four successful stage jobs would leave it after merging."""
    output.mkdir(parents=True, exist_ok=True)
    stage(output, "build")
    write(output / "build-identity.json", {"schema_version": 1, "binaries": BINARIES})
    for number, name in enumerate(BROAD_STAGES, 1):
        stage(output, name)
        write(output / f"{name}-summary.json", shard_machine(number))
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

    def test_whole_evidence_publishes_every_lane_and_indexes_its_bytes(self) -> None:
        whole_run(self.output)
        complete, markdown, status = self.run_publish()
        self.assertTrue(complete)
        self.assertEqual(status["state"], "success")
        self.assertEqual(status["lanes_with_evidence"], ["broad", "external", "sentinel"])
        self.assertEqual(status["stages"]["build"]["state"], "success")
        for row in (
            "| Interpreter basics | 1 tests |", "| Kraken 1\\.1 | 1 programs |",
            "| Micro-operations | 25 tests | 🔴 2.0% slower | 🔴 1.40× slower |",
        ):
            self.assertIn(row, markdown)
        self.assertNotIn("No complete performance conclusion", markdown)
        # The composed broad lane is written beside the shards it came from,
        # and the index binds the published summary to every evidence file.
        composed = json.loads((self.output / "summary.json").read_text())
        self.assertEqual(composed["shards"], len(BROAD_STAGES))
        self.assertEqual(
            [case["id"] for case in composed["comparisons"]["candidate vs base"]["cases"]],
            list(HOSTED_CASES),
        )
        import hashlib

        index = status["evidence_sha256"]
        self.assertEqual(
            set(index),
            {path.name for path in self.output.iterdir()} - {"status.json", "summary.md"},
        )
        for name, digest in index.items():
            self.assertEqual(hashlib.sha256((self.output / name).read_bytes()).hexdigest(), digest)
        self.assertIn("measured as 2 shards on separate runners", markdown)
        self.assertIn("- Micro-operations, candidate vs base: 1.0200×\n", markdown)

    def test_a_failed_required_lane_fails_the_preview_but_keeps_the_others(self) -> None:
        whole_run(self.output)
        (self.output / "broad-2-summary.json").unlink()
        stage(self.output, "broad-2", "failed", "measurement", "the broad-2 stage failed")
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
        self.assertIn(
            "the broad lane shard 2 did not finish \\(state failed, phase measurement\\)",
            markdown,
        )
        # One shard is not the lane: nothing of the broad portfolio is published.
        self.assertNotIn("plain_function_call", markdown)
        self.assertIn("| Kraken 1\\.1 | 1 programs |", markdown)
        self.assertIn("| Interpreter basics | 1 tests |", markdown)

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
        self.assertIn("| Interpreter basics | — | — | — |", markdown)
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
        # A broad shard is held to the same record.
        tampered_shard = shard_machine(1)
        tampered_shard["engines"]["base"]["binary_sha256"] = "7" * 64
        write(self.output / "broad-1-summary.json", tampered_shard)
        self.assertNotIn("broad", collect(self.output)[1])
        self.assertNotIn("Kraken", markdown)

    def test_success_without_readable_evidence_is_not_admitted(self) -> None:
        whole_run(self.output)
        (self.output / "broad-1-summary.json").write_text("{not json", encoding="utf-8")
        (self.output / "external-report.json").unlink()
        _, evidence, notes = collect(self.output)
        self.assertEqual(sorted(evidence), ["sentinel"])
        self.assertIn("reported success but its evidence is unreadable", notes["broad"])
        self.assertIn("reported success but its evidence is unreadable", notes["external"])

    def test_parseable_but_malformed_evidence_cannot_stop_publication(self) -> None:
        whole_run(self.output)
        # Correct hashes, so only the missing structure can keep these out.
        write(self.output / "external-report.json", {"binary_sha256": BINARIES})
        machine = json.loads((self.output / "broad-1-summary.json").read_text())
        del machine["comparisons"]["candidate vs base"]["cases"][0]["ratio"]
        write(self.output / "broad-1-summary.json", machine)
        write(self.output / "sentinel-summary.json", {"comparisons": {
            "candidate vs base": {"cases": "not a list"},
            "candidate vs QuickJS-NG": {"cases": []},
        }})
        complete, markdown, status = self.run_publish()
        self.assertFalse(complete)
        self.assertEqual(status["lanes_with_evidence"], [])
        self.assertEqual(markdown.count("reported success but its evidence is malformed"), 3)
        self.assertIn("the broad lane shard 1 reported success but its evidence is malformed", markdown)

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
        stage(self.output, "broad-1", attempt="2")
        complete, _, status = self.run_publish()
        self.assertTrue(complete)
        self.assertEqual(status["lanes_with_evidence"], ["broad", "external", "sentinel"])

        # Outside Actions no attempt is recorded anywhere, which is consistent.
        self.step_summary.unlink()
        whole_run(self.output)
        for name in ("build", *BROAD_STAGES, "external", "sentinel"):
            stage(self.output, name, attempt=None)
        self.assertTrue(self.run_publish()[0])
        # A record without an attempt cannot follow a build that has one.
        self.step_summary.unlink()
        stage(self.output, "build", attempt="1")
        self.assertFalse(self.run_publish()[0])

    def test_a_lane_without_any_record_is_reported_and_the_rest_published(self) -> None:
        for stage_name, lane, label, expected in (
            ("sentinel", "sentinel", "sentinel lane", True),
            ("broad-1", "broad", "broad lane shard 1", False),
            ("external", "external", "external lane", False),
        ):
            with self.subTest(stage=stage_name):
                self.step_summary.unlink(missing_ok=True)
                whole_run(self.output)
                (self.output / f"{stage_name}-status.json").unlink()
                complete, markdown, status = self.run_publish()
                self.assertEqual(complete, expected)
                self.assertEqual(status["stages"][stage_name]["state"], "missing")
                self.assertNotIn(lane, status["lanes_with_evidence"])
                self.assertEqual(len(status["lanes_with_evidence"]), 2)
                self.assertIn(f"the {label} left no record", markdown)

    def test_a_lane_that_stopped_before_measuring_replaces_its_earlier_success(self) -> None:
        # The record a lane job uploads before its checkout, exactly as the
        # workflow writes it, standing where an earlier success used to be.
        whole_run(self.output)
        (self.output / "broad-1-status.json").write_text(
            '{"message":"the lane job stopped before measurement began",'
            '"phase":"job_setup","run_attempt":"2","schema_version":2,'
            '"stage":"broad-1","state":"pending"}\n', encoding="utf-8",
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
        self.assertNotRegex(without_legend(markdown), r"\d\.\d+×|\d+\.\d% (slower|faster)")

    def test_an_empty_run_still_publishes_a_truthful_failure(self) -> None:
        complete, markdown, status = self.run_publish()
        self.assertFalse(complete)
        self.assertEqual(status["stages"]["build"]["state"], "missing")
        self.assertIn("No complete performance conclusion was produced", markdown)
        self.assertIn("state missing, phase not\\_started", markdown)

    def test_shards_are_joined_only_when_they_fit_together(self) -> None:
        shards = [shard_machine(1, 1.10), shard_machine(2, 0.90)]
        merged = merge_broad_shards(list(reversed(shards)))
        comparison = merged["comparisons"]["candidate vs base"]
        # 13 cases at 1.10 and 12 at 0.90: the lane's ratio is the geometric
        # mean over all 25 cases, as the analysis defines it unsharded.
        import math

        expected = math.exp((13 * math.log(1.10) + 12 * math.log(0.90)) / 25)
        self.assertAlmostEqual(comparison["ratio"], expected, places=12)
        self.assertNotIn("ci_lower", comparison)
        self.assertEqual([case["id"] for case in comparison["cases"]], list(HOSTED_CASES))
        self.assertIn("ci_lower", comparison["cases"][0])
        self.assertNotIn("shard", merged)

        for label, broken in (
            ("each shard exactly once", [shards[0], shards[0]]),
            ("each shard exactly once", [shards[0]]),
            ("each shard exactly once", [shards[0], {**shards[1], "shard": None}]),
            ("disagree about what they measured",
             [shards[0], {**shards[1], "harness": {"mode": PUSH_MODE, "revision": "9" * 40}}]),
            ("disagree about what they measured",
             [shards[0], {**shards[1], "profile_id": "another-profile"}]),
        ):
            with self.subTest(label=label), self.assertRaisesRegex(HostedPreviewError, label):
                merge_broad_shards(broken)

        # The right shard numbers over the wrong cases: one case twice, one never.
        overlapping = json.loads(json.dumps(shards[1]))
        for comparison in overlapping["comparisons"].values():
            comparison["cases"][0] = json.loads(json.dumps(
                shards[0]["comparisons"][comparison["label"]]["cases"][0]
            ))
        with self.assertRaisesRegex(HostedPreviewError, "frozen portfolio exactly once"):
            merge_broad_shards([shards[0], overlapping])

    def test_a_shard_that_failed_linearity_leaves_the_lane_without_a_direction(self) -> None:
        whole_run(self.output)
        invalid = shard_machine(2)
        invalid.update({"health": "invalid", "linearity": "fail", "comparisons": {}})
        write(self.output / "broad-2-summary.json", invalid)
        complete, markdown, status = self.run_publish()
        # Measured and validated, so the preview is whole; it reports no ratio.
        self.assertTrue(complete)
        self.assertIn("broad", status["lanes_with_evidence"])
        self.assertIn(
            "**Micro-operations:** measured, but the "
            "linearity diagnostic failed",
            markdown,
        )
        self.assertNotIn("plain_function_call", markdown)

    def test_shards_that_do_not_fit_cost_only_the_broad_lane(self) -> None:
        whole_run(self.output)
        write(self.output / "broad-2-summary.json", {**shard_machine(2), "profile_id": "other"})
        complete, markdown, status = self.run_publish()
        self.assertFalse(complete)
        self.assertEqual(status["lanes_with_evidence"], ["external", "sentinel"])
        self.assertIn("shards do not fit together", markdown)

        self.step_summary.unlink()
        whole_run(self.output)
        missing_field = shard_machine(2)
        del missing_field["valid_blocks"]
        write(self.output / "broad-2-summary.json", missing_field)
        complete, markdown, status = self.run_publish()
        self.assertFalse(complete)
        self.assertEqual(status["lanes_with_evidence"], ["external", "sentinel"])
        self.assertIn("reported success but", markdown)

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
