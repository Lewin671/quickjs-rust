"""Lead-approved unstable-base rule: positive, negative and provenance gates."""
from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.base_aa_evidence import classify, replay_aa
from tools.benchmark.external_preview import _EXPECTED_STDOUT_BY_ROLE
from tools.benchmark.performance_schema import PerformanceDecisionError
from tools.benchmark.tests import test_performance_decision as helpers


class BaseAaEvidenceTests(unittest.TestCase):
    case = "external/sunspider-1.0/3d-morph"

    def setUp(self):
        self.summary = {"engines": {"base": {"source_revision": "a" * 40,
                                            "binary_sha256": "b" * 64}}}
        self.context = {
            "host": {"node": "test-host"}, "manifest_sha256": "c" * 64,
            "bundle_sha256": "d" * 64, "argv": ["/base", "--raw", "/case.js"],
            "metric": "outer_process_wall_time", "timer": "python.perf_counter_ns",
            "timer_phase_boundary": "before_process_spawn_to_wait_return",
            "timeout_seconds": 15, "source_revision": "e" * 40, "source_files": [],
            "preview_id": "test", "schema_version": 3, "record_type": "sample",
            "suite_id": "sunspider-1.0", "case_id": "3d-morph",
        }
        samples = []
        for block in range(30):
            for role in ["candidate", "base"]:
                second = (block % 2 == 0) == (role == "candidate")
                samples.append({**self.context, "role": role, "block": block,
                                "measurement_blocks": 30, "phase": "measurement",
                                "binary_sha256": "b" * 64, "status": "ok", "error": None,
                                "cycles": 200 if second else 100,
                                "instructions": 100100 if second else 100000,
                                "duration_ns": 200 if second else 100,
                                "timer_started_ns": 1000,
                                "timer_finished_ns": 1200 if second else 1100,
                                "stdout": _EXPECTED_STDOUT_BY_ROLE[role], "stderr": "",
                                "exit_code": 0, "timed_out": False,
                                "stdout_truncated": False, "stderr_truncated": False})
        self.payload = {"schema_version": 1, "artifact_type": "quickjs-base-case-aa-evidence",
                        "case_id": self.case, "base_engine": self.summary["engines"]["base"],
                        "summary_sha256": "f" * 64, "context": self.context,
                        "blocks": 30, "samples": samples}

    def replay(self, payload=None, required=30):
        return replay_aa(payload or self.payload, self.summary, "f" * 64,
                         self.context, required, "cycles")

    def test_floor_and_intervals_are_replayed_from_every_pair(self):
        result = self.replay()
        self.assertGreater(result["cycles"]["relative_half_width"], .03)
        self.assertAlmostEqual(result["instruction_floor"]["upper"], 1.001)
        self.assertAlmostEqual(result["instruction_floor"]["lower"], 1 / 1.001)
        self.assertEqual(result["cycles"]["valid_blocks"], 30)

    def test_precise_base_cannot_claim_instability(self):
        p = copy.deepcopy(self.payload)
        for row in p["samples"]:
            row["cycles"] = 100
        with self.assertRaisesRegex(PerformanceDecisionError, "does not exceed"):
            self.replay(p)

    def test_missing_short_and_duplicate_evidence_refused(self):
        missing = copy.deepcopy(self.payload)
        del missing["samples"]
        with self.assertRaises(PerformanceDecisionError):
            self.replay(missing)
        with self.assertRaisesRegex(PerformanceDecisionError, "fewer blocks"):
            self.replay(required=60)
        for mutation in [lambda p: p["samples"].pop(),
                         lambda p: p["samples"].__setitem__(1, p["samples"][0])]:
            p = copy.deepcopy(self.payload)
            mutation(p)
            with self.assertRaises(PerformanceDecisionError):
                self.replay(p)

    def test_binary_revision_host_workload_protocol_and_bundle_mismatch_refused(self):
        for field, value in [("host", {"node": "other"}), ("bundle_sha256", "0" * 64),
                             ("argv", ["/other"]), ("timer", "other"),
                             ("manifest_sha256", "0" * 64), ("case_id", "other")]:
            p = copy.deepcopy(self.payload)
            p["samples"][0][field] = value
            with self.subTest(field=field), self.assertRaises(PerformanceDecisionError):
                self.replay(p)
        for key in ["binary_sha256", "source_revision"]:
            p = copy.deepcopy(self.payload)
            p["base_engine"][key] = "0" * len(p["base_engine"][key])
            with self.subTest(key=key), self.assertRaisesRegex(PerformanceDecisionError, "executable/revision"):
                self.replay(p)
        p = copy.deepcopy(self.payload)
        p["summary_sha256"] = "0" * 64
        with self.assertRaisesRegex(PerformanceDecisionError, "bundle"):
            self.replay(p)

    def test_invalid_output_counter_or_timer_refused(self):
        for field, value in [("stdout", "wrong"), ("instructions", None), ("cycles", 0),
                             ("timed_out", True), ("duration_ns", 99), ("measurement_blocks", 3)]:
            p = copy.deepcopy(self.payload)
            p["samples"][0][field] = value
            with self.subTest(field=field), self.assertRaises(PerformanceDecisionError):
                self.replay(p)


class UnstableBaseDecisionTests(unittest.TestCase):
    """Exercise actual decide, including ordinary gates and explicit targets."""
    base_sha, older_sha, candidate_sha = "a" * 40, "b" * 40, "c" * 40
    setUp = helpers.PerformanceDecisionTests.setUp
    _write = helpers.PerformanceDecisionTests._write
    _summary = helpers.PerformanceDecisionTests._summary
    _external = helpers.PerformanceDecisionTests._external
    _broad = helpers.PerformanceDecisionTests._broad
    _unit = helpers.PerformanceDecisionTests._unit
    _queue_and_unit = helpers.PerformanceDecisionTests._queue_and_unit

    def check(self, ratio=1, lower=.9999, upper=1.0001):
        fixture = BaseAaEvidenceTests()
        fixture.setUp()
        return {fixture.case: {"aa": fixture.replay(), "sha256": "8" * 64,
                               "candidate_over_base_instructions": {
                                   "ratio": ratio, "confidence_interval": {"lower": lower, "upper": upper},
                                   "valid_blocks": 30}}}

    def decision(self, checks=None, target=False):
        from tools.benchmark.performance_evaluation import decide
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            queue, queue_sha, unit, unit_sha = self._queue_and_unit(directory)
            summary = self._summary(self.candidate_sha, self.base_sha)
            external = self._external()
            if checks:
                for suite in external["suites"]:
                    for case in suite["cases"]:
                        if case["id"] == "3d-morph":
                            case["paired_comparisons"]["base"]["confidence_interval"] = {
                                "lower": .8, "upper": 1.3}
            if target:
                unit["fast_gate"]["target_ids"].append(BaseAaEvidenceTests.case)
            return decide(unit, unit_sha, queue, queue_sha, summary, "1" * 64,
                          self._broad(), "2" * 64, external, "3" * 64, "fast", None,
                          sentinel=self._broad(lane="sentinel"), sentinel_sha="5" * 64,
                          profile_root=directory, unstable_base_checks=checks)

    def test_precise_case_unaffected_without_exemption(self):
        result = self.decision()
        self.assertEqual(result["decision"], "retained")
        self.assertEqual(result["precision_exclusions"], [])
        self.assertEqual(result["unstable_on_base"], {})

    def test_unstable_control_with_unchanged_instructions_passes_and_binds_hash(self):
        result = self.decision(self.check())
        self.assertEqual(result["decision"], "retained")
        self.assertEqual(result["precision_exclusions"], [BaseAaEvidenceTests.case])
        self.assertEqual(result["evidence"]["base_aa_evidence"], {BaseAaEvidenceTests.case: "8" * 64})
        self.assertEqual(result["unstable_on_base"][BaseAaEvidenceTests.case]["instruction_gate"], "pass")

    def test_unstable_control_with_grown_instructions_fails(self):
        result = self.decision(self.check(1.01, 1.009, 1.011))
        self.assertEqual(result["decision"], "rejected")
        self.assertNotIn(BaseAaEvidenceTests.case, result["precision_exclusions"])
        self.assertTrue(any("instructions exceed" in reason for reason in result["reasons"]))

    def test_instruction_uncertainty_cannot_count_as_pass(self):
        result = self.decision(self.check(1, .999, 1.005))
        self.assertEqual(result["decision"], "inconclusive")
        self.assertEqual(result["unstable_on_base"], {})

    def test_unstable_declared_target_stays_inconclusive(self):
        result = self.decision(self.check(), target=True)
        self.assertEqual(result["decision"], "inconclusive")
        self.assertTrue(any("cannot count as a passed target" in reason for reason in result["reasons"]))
