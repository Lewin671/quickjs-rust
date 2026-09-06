from __future__ import annotations

import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.performance_decision import (
    PerformanceDecisionError,
    _atomic_write,
    _read_json,
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

    def setUp(self) -> None:
        from tools.benchmark.tests.decision_fixtures import isolate_test262_inventory
        isolate_test262_inventory(self)

    def test_atomic_output_write_creates_once_without_leaking_temporary_file(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            output = directory / "nested" / "queue.json"
            payload = {"artifact_type": "queue", "value": 1}

            _atomic_write(output, payload)

            self.assertEqual(json.loads(output.read_text(encoding="utf-8")), payload)
            self.assertEqual(list(output.parent.iterdir()), [output])
            with self.assertRaisesRegex(PerformanceDecisionError, "refusing to overwrite"):
                _atomic_write(output, {"artifact_type": "queue", "value": 2})

    def _write(self, directory: Path, name: str, payload: object) -> Path:
        path = directory / name
        path.write_text(json.dumps(payload, sort_keys=True) + "\n", encoding="utf-8")
        return path

    def _summary(self, candidate, base):
        from tools.benchmark.tests.decision_fixtures import summary
        result = summary(candidate, base)
        self.engines = result["engines"]
        return result

    def _external(self, *, target_base=0.9, complete=True):
        from tools.benchmark.tests.decision_fixtures import external
        return external(self.engines, target_base, complete)

    def _broad(self, *, count=25, lane="broad"):
        from tools.benchmark.tests.decision_fixtures import internal
        return internal(self.engines, count, lane)

    def _unit(self, queue_sha: str, *, base: str | None = None) -> dict[str, object]:
        target = "external/sunspider-1.0/3d-cube"
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
                    "source": self.profile_source,
                    "sha256": self.profile_sha,
                    "base_sha": base or self.base_sha,
                    "opportunity_ids": [target],
                    "shared_cost": "Call environment materialization.",
                    "inclusive_fraction": 0.2,
                }
            ],
            "fast_gate": {
                "target_ids": [target],
                "control_ids": ["external/sunspider-1.0/3d-morph", "broad/plain_function_call"],
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
        self.directory = directory
        from tools.benchmark.tests.decision_fixtures import profile
        self.profile_source, self.profile_sha = profile(directory, self.base_sha)
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
            ["external/sunspider-1.0/3d-cube", "external/sunspider-1.0/3d-morph"],
        )
        self.assertEqual([entry["rank"] for entry in queue["external"]], [1, 2])
        self.assertEqual(queue["broad"][0]["id"], "broad/plain_function_call")
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
            uncovered["profile_evidence"][0]["opportunity_ids"] = ["external/sunspider-1.0/3d-morph"]
            with self.assertRaisesRegex(PerformanceDecisionError, "do not cover"):
                validate_unit_against_queue(uncovered, unit_sha, queue, queue_sha)

    def test_unit_rejects_post_hoc_target_or_unexplained_override(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            queue, queue_sha, _unit, _unit_sha = self._queue_and_unit(directory)
            post_hoc = self._unit(queue_sha)
            post_hoc["fast_gate"]["target_ids"] = ["external/sunspider-1.0/3d-morph"]
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
            sentinel=self._broad(lane="sentinel"), sentinel_sha="5" * 64, profile_root=self.directory,
            )
            self.assertEqual(retained["decision"], "retained")

            rejected_external = self._external(target_base=0.98)
            rejected = decide(
                unit, unit_sha, queue, queue_sha,
                summary, "1" * 64, broad, "2" * 64, rejected_external, "3" * 64,
                "fast", None,
            sentinel=self._broad(lane="sentinel"), sentinel_sha="5" * 64, profile_root=self.directory,
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
            sentinel=self._broad(lane="sentinel"), sentinel_sha="5" * 64, profile_root=self.directory,
            )
            self.assertEqual(incomplete["decision"], "inconclusive")

            from tools.benchmark.tests.decision_fixtures import zero_gap
            test262 = zero_gap(self.candidate_sha)
            retained = decide(
                unit, unit_sha, queue, queue_sha,
                summary, "1" * 64, broad, "2" * 64, self._external(), "3" * 64,
                "promotion", (test262, "4" * 64),
            sentinel=self._broad(lane="sentinel"), sentinel_sha="5" * 64, profile_root=self.directory,
            )
            self.assertEqual(retained["decision"], "retained")

            failing_test262 = copy.deepcopy(test262)
            failing_test262["comparison"]["actionable_gap"] = 1
            rejected = decide(
                unit, unit_sha, queue, queue_sha,
                summary, "1" * 64, broad, "2" * 64, self._external(), "3" * 64,
                "promotion", (failing_test262, "4" * 64),
            sentinel=self._broad(lane="sentinel"), sentinel_sha="5" * 64, profile_root=self.directory,
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


class MigrationStageTests(unittest.TestCase):
    """Staged migrations, which are judged at their end state.

    A leaf fast path must pay for itself immediately, and the one- or two-
    attempt rule is right for it. An architectural migration cannot: its
    scaffolding stages move execution onto a new representation before any of
    it is faster. Judging those stages by the leaf gate is what made every
    structural attempt in this campaign unlandable, so the stage vocabulary is
    deliberately separate.
    """

    fixtures = PerformanceDecisionTests

    def setUp(self) -> None:
        from tools.benchmark.tests.decision_fixtures import isolate_test262_inventory
        isolate_test262_inventory(self)
        self._directory = tempfile.TemporaryDirectory()
        self.addCleanup(self._directory.cleanup)
        self.directory = Path(self._directory.name)
        self.helper = PerformanceDecisionTests()
        self.queue, self.queue_sha, _unit, _unit_sha = self.helper._queue_and_unit(self.directory)

    def _migration_unit(self, *, stage: int = 1, stages: int = 8,
                        budget: float = 1.10) -> dict[str, object]:
        unit = self.helper._unit(self.queue_sha)
        unit["schema_version"] = 2
        unit["unit_kind"] = "migration"
        unit["migration"] = {
            "stages": stages,
            "current_stage": stage,
            "cumulative_target_ids": ["external/sunspider-1.0/3d-cube"],
            "stage_max_candidate_over_base": budget,
        }
        return unit

    def _decide(self, unit_payload: dict[str, object], *, target_base: float,
                mode: str = "stage") -> dict[str, object]:
        unit_path = self.helper._write(self.directory, "migration-unit.json", unit_payload)
        unit, unit_sha = load_unit(unit_path)
        summary_path = self.helper._write(
            self.directory, "stage-summary.json",
            self.helper._summary(self.helper.candidate_sha, self.helper.base_sha),
        )
        broad_path = self.helper._write(self.directory, "stage-broad.json", self.helper._broad())
        external_path = self.helper._write(
            self.directory, "stage-external.json", self.helper._external(target_base=target_base)
        )
        summary, summary_sha = _read_json(summary_path, "preview summary")
        broad, broad_sha = _read_json(broad_path, "broad report")
        external, external_sha = _read_json(external_path, "external report")
        return decide(
            unit, unit_sha, self.queue, self.queue_sha, summary, summary_sha, broad, broad_sha,
            external, external_sha, mode, None,
        sentinel=self.helper._broad(lane="sentinel"), sentinel_sha="5" * 64, profile_root=self.directory,
            )

    def test_neutral_scaffolding_stage_advances_without_an_improvement(self) -> None:
        # 1.02x is a regression by the leaf gate and would consume the unit's
        # only attempt. As a stage it is inside the budget.
        payload = self._decide(self._migration_unit(), target_base=1.02)
        self.assertEqual(payload["decision"], "advance")
        self.assertEqual(payload["mode"], "stage")
        self.assertEqual((payload["stage"], payload["stages"]), (1, 8))
        self.assertEqual(payload["schema_version"], 2)
        self.assertIsNone(payload["evidence"]["test262_burndown_sha256"])

    def test_stage_beyond_its_budget_aborts_without_closing_the_family(self) -> None:
        payload = self._decide(self._migration_unit(budget=1.05), target_base=1.4)
        self.assertEqual(payload["decision"], "abort")
        self.assertRegex(payload["reasons"][0], "not its mechanism family")

    def test_missing_evidence_is_inconclusive_not_an_abort(self) -> None:
        unit = self._migration_unit()
        unit["migration"]["cumulative_target_ids"] = ["external/sunspider-1.0/absent"]
        payload = self._decide(unit, target_base=0.5)
        self.assertEqual(payload["decision"], "inconclusive")
        self.assertRegex(payload["reasons"][0], "missing candidate/base evidence")

    def test_intermediate_stage_cannot_claim_a_fast_or_promotion_decision(self) -> None:
        with self.assertRaisesRegex(PerformanceDecisionError, "final stage"):
            self._decide(self._migration_unit(stage=3), target_base=0.5, mode="fast")

    def test_final_stage_is_judged_by_the_ordinary_payoff_gate(self) -> None:
        payload = self._decide(
            self._migration_unit(stage=8, stages=8), target_base=0.5, mode="fast"
        )
        self.assertEqual(payload["decision"], "retained")

    def test_stage_mode_requires_a_migration_unit(self) -> None:
        with self.assertRaisesRegex(PerformanceDecisionError, "requires a migration unit"):
            self._decide(self.helper._unit(self.queue_sha), target_base=0.5)

    def test_migration_budget_must_absorb_cost_without_being_unbounded(self) -> None:
        for budget, message in ((0.95, "leaf gate"), (1.5, "unbounded regression")):
            path = self.helper._write(
                self.directory, f"broken-{budget}.json", self._migration_unit(budget=budget)
            )
            with self.assertRaisesRegex(PerformanceDecisionError, message):
                load_unit(path)

    def test_a_migration_must_name_its_cumulative_targets_and_bound_its_stages(self) -> None:
        empty = self._migration_unit()
        empty["migration"]["cumulative_target_ids"] = []
        path = self.helper._write(self.directory, "no-targets.json", empty)
        with self.assertRaisesRegex(PerformanceDecisionError, "cumulative_target_ids: expected a non-empty"):
            load_unit(path)

        sprawling = self._migration_unit(stages=13)
        path = self.helper._write(self.directory, "sprawling.json", sprawling)
        with self.assertRaisesRegex(PerformanceDecisionError, "one reviewable program"):
            load_unit(path)

    def test_schema_one_plans_still_load_as_leaf_units(self) -> None:
        path = self.helper._write(self.directory, "legacy.json", self.helper._unit(self.queue_sha))
        unit, _ = load_unit(path)
        self.assertEqual(unit["schema_version"], 1)
        self.assertNotIn("unit_kind", unit)
