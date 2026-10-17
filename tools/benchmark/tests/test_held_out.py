from __future__ import annotations

import base64
import copy
import hashlib
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from tools.benchmark.external_preview import SENTINEL, SourceFile, _bundle_source
from tools.benchmark.held_out import (
    MANIFEST_PATH,
    held_out_status,
    load_held_out_manifest,
    opportunity_ids,
)
from tools.benchmark.npm_lane import _extract_member, _verify_integrity
from tools.benchmark.performance_decision import (
    PerformanceDecisionError,
    _external_entries,
    build_queue,
    load_queue,
    load_unit,
)
from tools.benchmark.process import ProcessResult

ROOT = Path(__file__).resolve().parents[3]
EXPECTED_IDS = frozenset(
    {
        "held-out/acorn/parse",
        "held-out/crypto-js/hash",
        "held-out/d3-array/aggregate",
        "held-out/decimal.js/arith",
        "held-out/esprima/parse",
        "held-out/handlebars/render",
        "held-out/marked/render",
        "held-out/mustache/render",
        "held-out/ramda/pipeline",
    }
)


def process_result(stdout: str, *, exit_code: int = 0, timed_out: bool = False) -> ProcessResult:
    return ProcessResult(
        started_at="2026-10-16T00:00:00+00:00",
        timer_started_ns=1,
        timer_finished_ns=2,
        duration_ns=1,
        exit_code=exit_code,
        timed_out=timed_out,
        stdout=stdout,
        stderr="",
        stdout_truncated=False,
        stderr_truncated=False,
    )


class HeldOutManifestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.data = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))

    def _write(self, directory: Path, data: object) -> Path:
        path = directory / "held-out.json"
        path.write_text(json.dumps(data, sort_keys=True) + "\n", encoding="utf-8")
        return path

    def test_inventory_is_non_claim_and_every_case_is_held_out(self) -> None:
        manifest = load_held_out_manifest(MANIFEST_PATH)
        self.assertEqual(manifest.preview_id, "quickjs-held-out-preview-v1")
        self.assertFalse(self.data["claim_eligible"])
        self.assertEqual(manifest.role, "held-out")
        self.assertEqual(len(manifest.suites), 9)
        self.assertEqual(sum(len(suite.cases) for suite in manifest.suites), 9)
        self.assertTrue(
            all(item.role == "held-out" for suite in manifest.suites for item in suite.cases)
        )
        self.assertEqual(opportunity_ids(), EXPECTED_IDS)
        self.assertEqual(manifest.opportunity_ids(), EXPECTED_IDS)
        self.assertEqual(manifest.measurement.metric, "outer_process_wall_time")
        self.assertEqual(manifest.measurement.order, "seeded_role_rotation")
        for suite in manifest.suites:
            self.assertTrue(suite.package.sha256)
            self.assertTrue(suite.package.license)
            self.assertTrue(suite.package.integrity)
            self.assertEqual(suite.license_review["status"], "execution-only")
            # The driver is concatenated after the library, so the API exists first.
            for case in suite.cases:
                self.assertTrue(case.files[-1].path.startswith("benchmarks/held-out/"))

    def test_manifest_rejects_claims_unknown_fields_and_unsafe_sources(self) -> None:
        mutations: list[tuple[dict[str, object], str]] = []
        claim = copy.deepcopy(self.data)
        claim["claim_eligible"] = True
        mutations.append((claim, "must remain non-claim"))
        unknown = copy.deepcopy(self.data)
        unknown["future"] = True
        mutations.append((unknown, "unknown or missing"))
        role = copy.deepcopy(self.data)
        role["suites"][0]["cases"][0]["role"] = "target"
        mutations.append((role, "must be marked held-out"))
        escape = copy.deepcopy(self.data)
        escape["suites"][0]["cases"][0]["driver"]["path"] = "benchmarks/held-out/../escape.js"
        mutations.append((escape, "repository-relative"))
        outside = copy.deepcopy(self.data)
        outside["suites"][0]["cases"][0]["driver"]["path"] = "benchmarks/other.js"
        mutations.append((outside, "benchmarks/held-out/"))
        dup = copy.deepcopy(self.data)
        dup["suites"][0]["cases"][0]["files"].append(
            dict(dup["suites"][0]["cases"][0]["files"][0])
        )
        mutations.append((dup, "duplicate file path"))
        digest = copy.deepcopy(self.data)
        digest["suites"][0]["cases"][0]["files"][0]["sha256"] = "0" * 64
        mutations.append((digest, "non-zero lowercase SHA-256"))
        host = copy.deepcopy(self.data)
        host["suites"][0]["package"]["tarball_url"] = "https://evil.test/acorn.tgz"
        mutations.append((host, "npm registry host"))
        metric = copy.deepcopy(self.data)
        metric["measurement"]["metric"] = "instructions"
        mutations.append((metric, "unsupported protocol"))
        redistribution = copy.deepcopy(self.data)
        redistribution["suites"][0]["license_review"]["redistribute_source_in_artifact"] = True
        mutations.append((redistribution, "redistribution must remain disabled"))
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, (data, message) in enumerate(mutations):
                with self.subTest(index=index):
                    with self.assertRaisesRegex(ValueError, message):
                        load_held_out_manifest(self._write(directory, data))


class HeldOutStatusTests(unittest.TestCase):
    case = SimpleNamespace(expected_checksum="a1301bb6")

    def test_exact_checksum_and_sentinel_contract_is_accepted(self) -> None:
        for role, lines in (("candidate", 2), ("base", 2), ("quickjs-ng", 1)):
            with self.subTest(role=role):
                stdout = f"CHECKSUM a1301bb6\n" + f"{SENTINEL}\n" * lines
                self.assertEqual(held_out_status(process_result(stdout), role, self.case), ("ok", None))

    def test_wrong_checksum_sentinel_or_exit_is_invalid(self) -> None:
        wrong_checksum = f"CHECKSUM deadbeef\n{SENTINEL}\n{SENTINEL}\n"
        self.assertEqual(
            held_out_status(process_result(wrong_checksum), "candidate", self.case)[0], "invalid"
        )
        short = f"CHECKSUM a1301bb6\n{SENTINEL}\n"
        self.assertEqual(
            held_out_status(process_result(short), "candidate", self.case)[0], "invalid"
        )
        extra = f"CHECKSUM a1301bb6\n{SENTINEL}\n{SENTINEL}\n"
        self.assertEqual(
            held_out_status(process_result(extra), "quickjs-ng", self.case)[0], "invalid"
        )
        self.assertEqual(
            held_out_status(process_result("", exit_code=1), "candidate", self.case)[0], "failed"
        )
        self.assertEqual(
            held_out_status(process_result("", timed_out=True), "candidate", self.case)[0], "timeout"
        )


class HeldOutBundleTests(unittest.TestCase):
    def test_bundle_places_library_before_driver_and_appends_sentinel(self) -> None:
        library = "var heldOutLibrary = 1;\n"
        driver = "print('CHECKSUM ' + heldOutLibrary);\n"
        with tempfile.TemporaryDirectory() as directory_name:
            cache = Path(directory_name)
            suite_dir = cache / "s"
            suite_dir.mkdir()
            (suite_dir / "lib.js").write_text(library, encoding="utf-8")
            driver_path = "benchmarks/held-out/driver.js"
            (suite_dir / "benchmarks/held-out").mkdir(parents=True)
            (suite_dir / driver_path).write_text(driver, encoding="utf-8")
            suite = SimpleNamespace(id="s", source=SimpleNamespace(revision="a" * 64))
            case = SimpleNamespace(
                id="c",
                harness="direct-script",
                iteration_count=None,
                files=(
                    SourceFile("lib.js", hashlib.sha256(library.encode()).hexdigest()),
                    SourceFile(driver_path, hashlib.sha256(driver.encode()).hexdigest()),
                ),
            )
            bundle = _bundle_source(suite, case, cache)
        self.assertLess(bundle.index("var heldOutLibrary"), bundle.index("CHECKSUM ' + heldOutLibrary"))
        self.assertLess(bundle.index("CHECKSUM ' + heldOutLibrary"), bundle.index(SENTINEL))
        self.assertTrue(bundle.rstrip().endswith(f"'{SENTINEL}';"))


class HeldOutFetchTests(unittest.TestCase):
    def test_extract_member_verifies_integrity_and_missing_member(self) -> None:
        content = b"var x = 1;\n"
        with tempfile.TemporaryDirectory() as directory_name:
            tarball = Path(directory_name) / "pkg.tgz"
            with tarfile.open(tarball, "w:gz") as archive:
                info = tarfile.TarInfo("package/dist/lib.js")
                info.size = len(content)
                archive.addfile(info, io.BytesIO(content))
            self.assertEqual(_extract_member(tarball, "dist/lib.js"), content)
            with self.assertRaisesRegex(ValueError, "not present"):
                _extract_member(tarball, "dist/missing.js")
        digest = base64.b64encode(hashlib.sha512(content).digest()).decode()
        _verify_integrity(f"sha512-{digest}", content)
        with self.assertRaisesRegex(ValueError, "integrity"):
            _verify_integrity(f"sha512-{base64.b64encode(b'nope').decode()}", content)


class HeldOutEnforcementTests(unittest.TestCase):
    base_sha = "a" * 40
    older_sha = "b" * 40

    def _unit(self, *, targets, controls=None, profile_ids=None, migration=None) -> dict:
        unit = {
            "schema_version": 2 if migration is not None else 1,
            "artifact_type": "quickjs-performance-unit",
            "unit_id": "held-out-guard",
            "base_sha": self.base_sha,
            "queue": {"candidate_sha": self.base_sha, "sha256": "b" * 64},
            "priority": {
                "mode": "queue",
                "opportunity_ids": list(targets),
                "rank_ceiling": 1,
                "override_reason": None,
            },
            "mechanism": {"summary": "s", "generality": "g", "semantic_risks": ["r"]},
            "profile_evidence": [
                {
                    "source": "receipt",
                    "sha256": "c" * 64,
                    "base_sha": self.base_sha,
                    "opportunity_ids": list(profile_ids if profile_ids is not None else targets),
                    "shared_cost": "cost",
                    "inclusive_fraction": 0.2,
                }
            ],
            "fast_gate": {
                "target_ids": list(targets),
                "control_ids": list(controls if controls is not None else ["broad/plain_function_call"]),
                "target_max_candidate_over_base": 0.95,
                "control_max_candidate_over_base": 1.03,
                "max_attempts": 1,
            },
            "promotion_gate": {
                "require_complete_broad": True,
                "require_complete_external": True,
                "require_test262_zero_gap": True,
            },
        }
        if migration is not None:
            unit["unit_kind"] = "migration"
            unit["migration"] = migration
        return unit

    def _load(self, unit: dict) -> dict:
        with tempfile.TemporaryDirectory() as directory_name:
            path = Path(directory_name) / "unit.json"
            path.write_text(json.dumps(unit, sort_keys=True) + "\n", encoding="utf-8")
            loaded, _sha = load_unit(path)
            return loaded

    def test_a_plan_cannot_target_a_held_out_case(self) -> None:
        with self.assertRaisesRegex(PerformanceDecisionError, "acceptance check"):
            self._load(self._unit(targets=["held-out/acorn/parse"]))

    def test_a_plan_cannot_use_a_held_out_case_as_a_control(self) -> None:
        with self.assertRaisesRegex(PerformanceDecisionError, "acceptance check"):
            self._load(self._unit(
                targets=["broad/plain_function_call"], controls=["held-out/acorn/parse"]
            ))

    def test_a_plan_cannot_bind_a_held_out_profile_receipt(self) -> None:
        with self.assertRaisesRegex(PerformanceDecisionError, "acceptance check"):
            self._load(self._unit(
                targets=["broad/plain_function_call"],
                profile_ids=["held-out/acorn/parse"],
            ))

    def test_a_migration_cannot_name_a_held_out_cumulative_target(self) -> None:
        with self.assertRaisesRegex(PerformanceDecisionError, "acceptance check"):
            self._load(self._unit(
                targets=["broad/plain_function_call"],
                migration={
                    "stages": 4,
                    "current_stage": 1,
                    "cumulative_target_ids": ["held-out/acorn/parse"],
                    "stage_max_candidate_over_base": 1.1,
                },
            ))

    def test_ordinary_plans_still_load(self) -> None:
        unit = self._load(self._unit(targets=["broad/plain_function_call"]))
        self.assertNotIn("unit_kind", unit)

    def test_queue_loader_refuses_a_held_out_entry(self) -> None:
        from tools.benchmark.tests.decision_fixtures import external, internal, summary

        engines = summary(self.base_sha, self.older_sha)["engines"]
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            summary_path = directory / "summary.json"
            summary_path.write_text(json.dumps(summary(self.base_sha, self.older_sha)), encoding="utf-8")
            broad_path = directory / "broad.json"
            broad_path.write_text(json.dumps(internal(engines)), encoding="utf-8")
            external_path = directory / "external.json"
            external_path.write_text(json.dumps(external(engines)), encoding="utf-8")
            payload = build_queue(summary_path, broad_path, external_path, 0.5)
            payload["external"].append({
                "id": "held-out/acorn/parse",
                "suite": "acorn",
                "case": "parse",
                "candidate_over_quickjs_ng": 4.0,
                "candidate_over_base": 1.0,
                "rank": len(payload["external"]) + 1,
            })
            queue_path = directory / "queue.json"
            queue_path.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaisesRegex(PerformanceDecisionError, "acceptance check"):
                load_queue(queue_path)

    def test_queue_builder_refuses_a_held_out_report_as_external_evidence(self) -> None:
        report = {"artifact_type": "quickjs-held-out-preview-report", "schema_version": 3, "suites": []}
        with self.assertRaisesRegex(PerformanceDecisionError, "unsupported artifact type"):
            _external_entries(report, 0.5)


if __name__ == "__main__":
    unittest.main()
