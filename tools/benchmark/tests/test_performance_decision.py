from __future__ import annotations

import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.performance_decision import (
    PerformanceDecisionError,
    build_queue,
    decide,
    load_queue,
    load_unit,
    validate_unit_against_queue,
)


class PerformanceDecisionTests(unittest.TestCase):
    base_sha = "a" * 40
    older_sha = "b" * 40
    candidate_sha = "c" * 40

    def _write(self, directory: Path, name: str, payload: object) -> Path:
        path = directory / name
        path.write_text(json.dumps(payload, sort_keys=True) + "\n", encoding="utf-8")
        return path

    def _summary(self, candidate: str, base: str) -> dict[str, object]:
        return {
            "state": "success",
            "engines": {
                "candidate": {"source_revision": candidate},
                "base": {"source_revision": base},
            },
        }

    def _external(self, *, target_base: float = 0.9, complete: bool = True) -> dict[str, object]:
        return {
            "schema_version": 1,
            "artifact_type": "quickjs-external-preview-report",
            "suites": [
                {
                    "id": "suite",
                    "complete_base_comparison": complete,
                    "complete_comparison": complete,
                    "cases": [
                        {
                            "id": "target",
                            "capability": {"candidate": "ok", "base": "ok", "quickjs-ng": "ok"},
                            "candidate_over_base": target_base,
                            "candidate_over_quickjs_ng": 4.0,
                        },
                        {
                            "id": "control",
                            "capability": {"candidate": "ok", "base": "ok", "quickjs-ng": "ok"},
                            "candidate_over_base": 1.01,
                            "candidate_over_quickjs_ng": 2.0,
                        },
                        {
                            "id": "already-fast",
                            "capability": {"candidate": "ok", "base": "ok", "quickjs-ng": "ok"},
                            "candidate_over_base": 0.98,
                            "candidate_over_quickjs_ng": 0.4,
                        },
                        {
                            "id": "not-comparable",
                            "capability": {"candidate": "ok", "base": "ok", "quickjs-ng": "timeout"},
                            "candidate_over_base": 0.8,
                            "candidate_over_quickjs_ng": None,
                        },
                    ],
                }
            ],
        }

    def _broad(self, *, count: int = 25) -> dict[str, object]:
        quickjs_cases: dict[str, object] = {}
        base_cases: dict[str, object] = {}
        for index in range(count):
            case_id = "broad-hot" if index == 0 else f"case-{index}"
            quickjs_cases[case_id] = {"family": "call", "ratio": 0.8 if index == 0 else 0.3}
            base_cases[case_id] = {"ratio": 0.99}
        return {
            "comparisons": {
                "candidate_vs_quickjs_ng": {"cases": quickjs_cases},
                "candidate_vs_base": {"cases": base_cases},
            }
        }

    def _unit(self, queue_sha: str, *, base: str | None = None) -> dict[str, object]:
        target = "external/suite/target"
        return {
            "schema_version": 1,
            "artifact_type": "quickjs-performance-unit",
            "unit_id": "shared-call-cost",
            "base_sha": base or self.base_sha,
            "queue": {"candidate_sha": self.base_sha, "sha256": queue_sha},
            "priority": {
                "mode": "queue",
                "opportunity_ids": [target],
                "rank_ceiling": 1,
                "override_reason": None,
            },
            "mechanism": {
                "summary": "Remove one shared call setup cost.",
                "generality": "The mechanism applies to ordinary calls, not one workload.",
                "semantic_risks": ["direct-eval"],
            },
            "profile_evidence": [
                {
                    "source": "target/profiles/shared-call.json",
                    "sha256": "d" * 64,
                    "base_sha": base or self.base_sha,
                    "opportunity_ids": [target],
                    "shared_cost": "Call environment materialization.",
                    "inclusive_fraction": 0.2,
                }
            ],
            "fast_gate": {
                "target_ids": [target],
                "control_ids": ["external/suite/control", "broad/broad-hot"],
                "target_max_candidate_over_base": 0.95,
                "control_max_candidate_over_base": 1.03,
                "max_attempts": 2,
            },
            "promotion_gate": {
                "require_complete_broad": True,
                "require_complete_external": True,
                "require_test262_zero_gap": True,
            },
        }

    def _queue_and_unit(self, directory: Path) -> tuple[dict[str, object], str, dict[str, object], str]:
        summary_path = self._write(directory, "queue-summary.json", self._summary(self.base_sha, self.older_sha))
        broad_path = self._write(directory, "queue-broad.json", self._broad())
        external_path = self._write(directory, "queue-external.json", self._external())
        queue_payload = build_queue(summary_path, broad_path, external_path, 0.5)
        queue_path = self._write(directory, "queue.json", queue_payload)
        queue, queue_sha = load_queue(queue_path)
        unit_path = self._write(directory, "unit.json", self._unit(queue_sha))
        unit, unit_sha = load_unit(unit_path)
        return queue, queue_sha, unit, unit_sha

    def test_queue_ranks_only_comparable_cases_above_campaign_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            queue, _queue_sha, _unit, _unit_sha = self._queue_and_unit(directory)
        self.assertEqual(
            [entry["id"] for entry in queue["external"]],
            ["external/suite/target", "external/suite/control"],
        )
        self.assertEqual([entry["rank"] for entry in queue["external"]], [1, 2])
        self.assertEqual(queue["broad"][0]["id"], "broad/broad-hot")
        self.assertEqual(queue["candidate_sha"], self.base_sha)

    def test_unit_requires_current_queue_and_profile_coverage(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            queue, queue_sha, unit, unit_sha = self._queue_and_unit(directory)
            validation = validate_unit_against_queue(unit, unit_sha, queue, queue_sha)
            self.assertEqual(validation["status"], "valid")

            stale = copy.deepcopy(unit)
            stale["base_sha"] = self.older_sha
            with self.assertRaisesRegex(PerformanceDecisionError, "base SHA"):
                validate_unit_against_queue(stale, unit_sha, queue, queue_sha)

            uncovered = copy.deepcopy(unit)
            uncovered["profile_evidence"][0]["opportunity_ids"] = ["external/suite/control"]
            with self.assertRaisesRegex(PerformanceDecisionError, "do not cover"):
                validate_unit_against_queue(uncovered, unit_sha, queue, queue_sha)

    def test_unit_rejects_post_hoc_target_or_unexplained_override(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            queue, queue_sha, _unit, _unit_sha = self._queue_and_unit(directory)
            post_hoc = self._unit(queue_sha)
            post_hoc["fast_gate"]["target_ids"] = ["external/suite/control"]
            post_hoc_path = self._write(directory, "post-hoc.json", post_hoc)
            with self.assertRaisesRegex(PerformanceDecisionError, "must equal"):
                load_unit(post_hoc_path)

            override = self._unit(queue_sha)
            override["priority"]["mode"] = "override"
            override["priority"]["override_reason"] = None
            override_path = self._write(directory, "override.json", override)
            with self.assertRaisesRegex(PerformanceDecisionError, "non-empty"):
                load_unit(override_path)

    def test_fast_decision_retains_or_rejects_against_predeclared_gates(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            queue, queue_sha, unit, unit_sha = self._queue_and_unit(directory)
            summary = self._summary(self.candidate_sha, self.base_sha)
            broad = self._broad()
            external = self._external()
            retained = decide(
                unit, unit_sha, queue, queue_sha,
                summary, "1" * 64, broad, "2" * 64, external, "3" * 64,
                "fast", None,
            )
            self.assertEqual(retained["decision"], "retained")

            rejected_external = self._external(target_base=0.98)
            rejected = decide(
                unit, unit_sha, queue, queue_sha,
                summary, "1" * 64, broad, "2" * 64, rejected_external, "3" * 64,
                "fast", None,
            )
            self.assertEqual(rejected["decision"], "rejected")
            self.assertIn("target improvement", rejected["reasons"][0])

    def test_promotion_requires_complete_external_and_exact_zero_gap_test262(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            queue, queue_sha, unit, unit_sha = self._queue_and_unit(directory)
            summary = self._summary(self.candidate_sha, self.base_sha)
            broad = self._broad()
            external = self._external(complete=False)
            incomplete = decide(
                unit, unit_sha, queue, queue_sha,
                summary, "1" * 64, broad, "2" * 64, external, "3" * 64,
                "promotion", None,
            )
            self.assertEqual(incomplete["decision"], "inconclusive")

            test262 = {
                "commit": self.candidate_sha,
                "rust": {"fail": 0, "timeout": 0, "not_run": 0},
                "comparison": {
                    "actionable_gap": 0,
                    "ng_pass_rust_fail": 0,
                    "ng_pass_rust_timeout": 0,
                    "ng_pass_rust_not_run": 0,
                },
            }
            retained = decide(
                unit, unit_sha, queue, queue_sha,
                summary, "1" * 64, broad, "2" * 64, self._external(), "3" * 64,
                "promotion", (test262, "4" * 64),
            )
            self.assertEqual(retained["decision"], "retained")

            failing_test262 = copy.deepcopy(test262)
            failing_test262["comparison"]["actionable_gap"] = 1
            rejected = decide(
                unit, unit_sha, queue, queue_sha,
                summary, "1" * 64, broad, "2" * 64, self._external(), "3" * 64,
                "promotion", (failing_test262, "4" * 64),
            )
            self.assertEqual(rejected["decision"], "rejected")

    def test_queue_output_is_sha_bound_to_all_three_preview_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            summary_path = self._write(directory, "summary.json", self._summary(self.base_sha, self.older_sha))
            broad_path = self._write(directory, "broad.json", self._broad())
            external_path = self._write(directory, "external.json", self._external())
            queue = build_queue(summary_path, broad_path, external_path, 0.5)
            self.assertEqual(
                queue["evidence"]["preview_summary_sha256"],
                hashlib.sha256(summary_path.read_bytes()).hexdigest(),
            )
            self.assertEqual(
                queue["evidence"]["broad_report_sha256"],
                hashlib.sha256(broad_path.read_bytes()).hexdigest(),
            )
            self.assertEqual(
                queue["evidence"]["external_report_sha256"],
                hashlib.sha256(external_path.read_bytes()).hexdigest(),
            )


if __name__ == "__main__":
    unittest.main()
