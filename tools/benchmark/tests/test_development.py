"""The development lane: real programs that may profile and select work.

The lane exists to give the profile/queue path real programs. What the held-out
lane refuses, this one allows -- a development case id is a legal workload -- so
these tests prove both halves: the lane's own protocol checks, and the allowance
that distinguishes it from held-out.
"""
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
from unittest.mock import patch

from tools.benchmark.development import (
    MANIFEST_PATH,
    SPEC,
    corpus_inventory,
    development_report,
    development_status,
    load_development_manifest,
    opportunity_ids,
)
from tools.benchmark.external_preview import (
    EXTERNAL_ROLES,
    SENTINEL,
    ExternalPreviewError,
    Source,
    SourceFile,
    _bundle_source,
)
from tools.benchmark.external_report import replay
from tools.benchmark.external_timeouts import Measurement
from tools.benchmark.held_out import opportunity_ids as held_out_ids
from tools.benchmark.npm_lane import (
    NpmCase,
    NpmManifest,
    NpmSuite,
    Package,
    _extract_member,
    _verify_integrity,
    assert_disjoint,
    npm_status,
)
from tools.benchmark.performance_decision import (
    PerformanceDecisionError,
    _development_entries,
    _external_entries,
    build_queue,
    load_queue,
    load_unit,
    validate_unit_against_queue,
)
from tools.benchmark.performance_evidence import inventories
from tools.benchmark.performance_schema import _opportunity_id
from tools.benchmark.planning import role_orders
from tools.benchmark.process import ProcessResult

EXPECTED_IDS = frozenset(
    {
        "development/big.js/arith",
        "development/diff/compare",
        "development/graphlib/paths",
        "development/immutable/persistent",
        "development/js-yaml/document",
        "development/lodash/transform",
        "development/matter-js/simulate",
        "development/meriyah/parse",
        "development/moment/dates",
        "development/papaparse/csv",
    }
)


def process_result(stdout: str, *, exit_code: int = 0, timed_out: bool = False) -> ProcessResult:
    return ProcessResult(
        started_at="2026-10-17T00:00:00+00:00",
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


class DevelopmentManifestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.data = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))

    def _write(self, directory: Path, data: object) -> Path:
        path = directory / "development.json"
        path.write_text(json.dumps(data, sort_keys=True) + "\n", encoding="utf-8")
        return path

    def test_inventory_is_non_claim_and_every_case_is_development(self) -> None:
        manifest = load_development_manifest(MANIFEST_PATH)
        self.assertEqual(manifest.preview_id, "quickjs-development-preview-v1")
        self.assertFalse(self.data["claim_eligible"])
        self.assertEqual(manifest.role, "development")
        self.assertEqual(len(manifest.suites), 10)
        self.assertEqual(sum(len(suite.cases) for suite in manifest.suites), 10)
        self.assertTrue(
            all(case.role == "development" for suite in manifest.suites for case in suite.cases)
        )
        self.assertEqual(opportunity_ids(), EXPECTED_IDS)
        self.assertEqual(manifest.opportunity_ids(), EXPECTED_IDS)
        self.assertEqual(corpus_inventory(), {
            suite.id: {case.id for case in suite.cases} for suite in manifest.suites
        })
        self.assertEqual(manifest.measurement.metric, "outer_process_wall_time")
        self.assertEqual(manifest.measurement.order, "seeded_role_rotation")
        for suite in manifest.suites:
            self.assertTrue(suite.package.sha256)
            self.assertTrue(suite.package.license)
            self.assertTrue(suite.package.integrity)
            self.assertEqual(suite.license_review["status"], "execution-only")
            for case in suite.cases:
                self.assertTrue(case.files[-1].path.startswith("benchmarks/development/"))
                self.assertTrue(case.expected_checksum)

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
        mutations.append((role, "must be marked development"))
        escape = copy.deepcopy(self.data)
        escape["suites"][0]["cases"][0]["driver"]["path"] = "benchmarks/development/../escape.js"
        mutations.append((escape, "repository-relative"))
        outside = copy.deepcopy(self.data)
        outside["suites"][0]["cases"][0]["driver"]["path"] = "benchmarks/held-out/acorn.js"
        mutations.append((outside, "benchmarks/development/"))
        dup = copy.deepcopy(self.data)
        dup["suites"][0]["cases"][0]["files"].append(
            dict(dup["suites"][0]["cases"][0]["files"][0])
        )
        mutations.append((dup, "duplicate file path"))
        digest = copy.deepcopy(self.data)
        digest["suites"][0]["cases"][0]["files"][0]["sha256"] = "0" * 64
        mutations.append((digest, "non-zero lowercase SHA-256"))
        host = copy.deepcopy(self.data)
        host["suites"][0]["package"]["tarball_url"] = "https://evil.test/lodash.tgz"
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
                        load_development_manifest(self._write(directory, data))

    def test_lane_case_ids_are_disjoint_from_held_out(self) -> None:
        self.assertEqual(opportunity_ids() & held_out_ids(), frozenset())

    def test_a_program_pinned_in_both_lanes_is_refused_by_the_loader(self) -> None:
        held_out = json.loads(
            (MANIFEST_PATH.parent / "held-out-preview.json").read_text(encoding="utf-8")
        )
        shared_library = copy.deepcopy(self.data)
        shared_library["suites"][0]["package"]["name"] = held_out["suites"][0]["package"]["name"]
        with tempfile.TemporaryDirectory() as directory_name:
            with self.assertRaisesRegex(ValueError, "must not share a program"):
                load_development_manifest(self._write(Path(directory_name), shared_library))
        shared_tarball = copy.deepcopy(self.data)
        shared_tarball["suites"][0]["package"]["sha256"] = held_out["suites"][0]["package"]["sha256"]
        with tempfile.TemporaryDirectory() as directory_name:
            with self.assertRaisesRegex(ValueError, "must not share a program"):
                load_development_manifest(self._write(Path(directory_name), shared_tarball))

    def test_a_shared_case_id_is_refused_even_though_lanes_prefix_their_ids(self) -> None:
        # The guard does not rely on the id prefix: a manifest whose ids collide
        # with the other lane's is refused as well.
        other = SimpleNamespace(
            key="held-out",
            manifest_path=Path("unused.json"),
            suites=(),
            opportunity_ids=lambda: frozenset({"held-out/acorn/parse"}),
        )
        colliding = SimpleNamespace(
            suites=(),
            opportunity_ids=lambda: frozenset({"held-out/acorn/parse"}),
        )
        with patch("tools.benchmark.npm_lane.lane_specs", return_value=(SPEC, other)), \
             patch("tools.benchmark.npm_lane.load_npm_manifest", return_value=other):
            with self.assertRaisesRegex(ValueError, "case_ids"):
                assert_disjoint(SPEC, colliding)


class DevelopmentStatusTests(unittest.TestCase):
    case = SimpleNamespace(expected_checksum="dd804f67")

    def test_exact_checksum_and_sentinel_contract_is_accepted(self) -> None:
        for role, lines in (("candidate", 2), ("base", 2), ("quickjs-ng", 1)):
            with self.subTest(role=role):
                stdout = "CHECKSUM dd804f67\n" + f"{SENTINEL}\n" * lines
                self.assertEqual(
                    development_status(process_result(stdout), role, self.case), ("ok", None)
                )

    def test_wrong_checksum_sentinel_or_exit_is_invalid(self) -> None:
        wrong = f"CHECKSUM deadbeef\n{SENTINEL}\n{SENTINEL}\n"
        self.assertEqual(
            development_status(process_result(wrong), "candidate", self.case)[0], "invalid"
        )
        short = f"CHECKSUM dd804f67\n{SENTINEL}\n"
        self.assertEqual(
            development_status(process_result(short), "candidate", self.case)[0], "invalid"
        )
        self.assertEqual(
            development_status(process_result("", exit_code=1), "candidate", self.case)[0], "failed"
        )
        self.assertEqual(
            development_status(process_result("", timed_out=True), "candidate", self.case)[0],
            "timeout",
        )


class DevelopmentBundleTests(unittest.TestCase):
    def test_bundle_places_library_before_driver_and_appends_sentinel(self) -> None:
        library = "var developmentLibrary = 1;\n"
        driver = "print('CHECKSUM ' + developmentLibrary);\n"
        with tempfile.TemporaryDirectory() as directory_name:
            cache = Path(directory_name)
            suite_dir = cache / "s"
            suite_dir.mkdir()
            (suite_dir / "lib.js").write_text(library, encoding="utf-8")
            driver_path = "benchmarks/development/driver.js"
            (suite_dir / "benchmarks/development").mkdir(parents=True)
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
        self.assertLess(bundle.index("var developmentLibrary"), bundle.index("' + developmentLibrary"))
        self.assertTrue(bundle.rstrip().endswith(f"'{SENTINEL}';"))

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


class DevelopmentAllowanceTests(unittest.TestCase):
    """A development id is a workload a plan, queue or profile may name."""

    base_sha = "a" * 40
    older_sha = "b" * 40
    target = "development/lodash/transform"

    def _unit(self, *, schema=1, target=None, controls=None, profile_ids=None) -> dict:
        target = target or self.target
        unit = {
            "schema_version": schema,
            "artifact_type": "quickjs-performance-unit",
            "unit_id": "development-target",
            "base_sha": self.base_sha,
            "queue": {"candidate_sha": self.base_sha, "sha256": "b" * 64},
            "priority": {
                "mode": "queue",
                "opportunity_ids": [target],
                "rank_ceiling": 4,
                "override_reason": None,
            },
            "mechanism": {"summary": "s", "generality": "g", "semantic_risks": ["r"]},
            "profile_evidence": [
                {
                    "source": "receipt",
                    "sha256": "c" * 64,
                    "base_sha": self.base_sha,
                    "opportunity_ids": list(profile_ids if profile_ids is not None else [target]),
                    "shared_cost": "cost",
                    "inclusive_fraction": 0.2,
                }
            ],
            "fast_gate": {
                "target_ids": [target],
                "target_max_candidate_over_base": 0.95,
                "max_attempts": 1,
            },
            "promotion_gate": {
                "require_complete_broad": True,
                "require_complete_external": True,
                "require_test262_zero_gap": True,
            },
        }
        if schema == 1:
            unit["fast_gate"]["control_ids"] = list(controls or ["broad/plain_function_call"])
            unit["fast_gate"]["control_max_candidate_over_base"] = 1.03
        else:
            unit["unit_kind"] = "leaf"
            unit["layout_gate"] = {
                "policy_path": "benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v2.json",
                "policy_sha256": "d" * 64,
                "mechanism_class": "cpu-work",
                "memory_wall_targets": [],
                "panel_required": False,
                "panel_pairs": 5,
                "held_out_pairs": 30,
                "formal_blocks": 30,
            }
        return unit

    def _load(self, unit: dict) -> dict:
        with tempfile.TemporaryDirectory() as directory_name:
            path = Path(directory_name) / "unit.json"
            path.write_text(json.dumps(unit, sort_keys=True) + "\n", encoding="utf-8")
            loaded, _sha = load_unit(path)
            return loaded

    def test_the_id_vocabulary_accepts_a_development_case(self) -> None:
        self.assertEqual(_opportunity_id(self.target, "id"), self.target)

    def test_a_plan_may_target_control_and_profile_a_development_case(self) -> None:
        for unit in (
            self._unit(),
            self._unit(controls=[self.target, "broad/plain_function_call"]),
            self._unit(profile_ids=[self.target, "development/papaparse/csv"]),
        ):
            with self.subTest(unit=unit["profile_evidence"][0]["opportunity_ids"]):
                with tempfile.TemporaryDirectory() as directory_name:
                    path = Path(directory_name) / "unit.json"
                    path.write_text(json.dumps(unit, sort_keys=True), encoding="utf-8")
                    loaded, _sha = load_unit(path)
                    self.assertEqual(loaded["priority"]["opportunity_ids"][0], self.target)

    def test_a_held_out_case_stays_refused_in_the_same_plan_shape(self) -> None:
        with self.assertRaisesRegex(PerformanceDecisionError, "acceptance check"):
            self._load(self._unit(target="held-out/acorn/parse"))

    def test_the_queue_ranks_development_cases_in_their_own_channel(self) -> None:
        from tools.benchmark.tests.decision_fixtures import (
            development, external, internal, summary,
        )
        engines = summary(self.base_sha, self.older_sha)["engines"]
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            paths = {}
            for name, data in (
                ("summary.json", summary(self.base_sha, self.older_sha)),
                ("broad.json", internal(engines)),
                ("external.json", external(engines)),
                ("development.json", development(engines)),
            ):
                paths[name] = directory / name
                paths[name].write_text(json.dumps(data), encoding="utf-8")
            payload = build_queue(
                paths["summary.json"], paths["broad.json"], paths["external.json"], 0.5,
                development_path=paths["development.json"],
            )
            self.assertEqual(payload["schema_version"], 4)
            self.assertEqual(
                len(payload["development"]), len(corpus_inventory())
            )
            self.assertEqual(payload["development"][0]["id"], "development/big.js/arith")
            self.assertEqual(
                [entry["rank"] for entry in payload["development"]],
                list(range(1, len(payload["development"]) + 1)),
            )
            self.assertEqual(
                payload["evidence"]["development_report_sha256"],
                hashlib.sha256(paths["development.json"].read_bytes()).hexdigest(),
            )
            queue_path = directory / "queue.json"
            queue_path.write_text(json.dumps(payload), encoding="utf-8")
            queue, queue_sha = load_queue(queue_path)
            self.assertEqual(len(queue["development"]), len(payload["development"]))

    def test_a_development_report_must_be_the_frozen_corpus_of_this_run(self) -> None:
        from tools.benchmark.tests.decision_fixtures import development, summary
        engines = summary(self.base_sha, self.older_sha)["engines"]
        entries = _development_entries(development(engines), 0.5, engines, development(engines)["host"])
        self.assertEqual(len(entries), len(corpus_inventory()))
        unseen = development(engines)
        unseen["binary_sha256"]["candidate"] = "e" * 64
        with self.assertRaisesRegex(PerformanceDecisionError, "binary identities"):
            _development_entries(unseen, 0.5, engines, unseen["host"])
        other_host = development(engines, host={"node": "elsewhere"})
        with self.assertRaisesRegex(PerformanceDecisionError, "different measurement host"):
            _development_entries(other_host, 0.5, engines, development(engines)["host"])
        truncated = development(engines)
        truncated["suites"] = truncated["suites"][:-1]
        with self.assertRaisesRegex(PerformanceDecisionError, "wrong frozen suite inventory"):
            _development_entries(truncated, 0.5, engines, truncated["host"])
        partial = development(engines)
        partial["suites"][0]["cases"] = partial["suites"][0]["cases"][:-1]
        with self.assertRaisesRegex(PerformanceDecisionError, "incomplete frozen case inventory"):
            _development_entries(partial, 0.5, engines, partial["host"])

    def test_development_evidence_is_not_external_evidence(self) -> None:
        report = {
            "artifact_type": "quickjs-development-preview-report",
            "schema_version": 3,
            "suites": [],
        }
        with self.assertRaisesRegex(PerformanceDecisionError, "unsupported artifact type"):
            _external_entries(report, 0.5)

    def test_a_queue_plan_validates_against_a_development_rank(self) -> None:
        from tools.benchmark.tests.decision_fixtures import (
            development, external, internal, summary,
        )
        engines = summary(self.base_sha, self.older_sha)["engines"]
        expected = {
            f"{lane}/{case}" if lane in ("broad", "sentinel") else f"external/{lane}/{case}"
            for lane, cases in inventories().items() for case in cases
        }
        policy = {"host_node": "test-host", "cases": expected | set(opportunity_ids()),
                  "held_out": held_out_ids(), "lanes": {"development": 1.01}}
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            data = {}
            for name, payload in (
                ("summary.json", summary(self.base_sha, self.older_sha)),
                ("broad.json", internal(engines)),
                ("external.json", external(engines)),
                ("development.json", development(engines)),
            ):
                data[name] = directory / name
                data[name].write_text(json.dumps(payload), encoding="utf-8")
            queue_payload = build_queue(
                data["summary.json"], data["broad.json"], data["external.json"], 0.5,
                development_path=data["development.json"],
            )
            queue_payload["host"]["node"] = "test-host"
            queue_path = directory / "queue.json"
            queue_path.write_text(json.dumps(queue_payload), encoding="utf-8")
            queue, queue_sha = load_queue(queue_path)
            ranks = {entry["id"]: entry["rank"] for entry in queue["development"]}
            self.assertIn(self.target, ranks)
            unit = self._unit(schema=3)
            unit["priority"]["rank_ceiling"] = ranks[self.target]
            unit["queue"] = {"candidate_sha": queue["candidate_sha"], "sha256": queue_sha}
            unit_path = directory / "unit.json"
            unit_path.write_text(json.dumps(unit, sort_keys=True), encoding="utf-8")
            with patch("tools.benchmark.layout_policy.load_policy", return_value=policy):
                loaded, unit_sha = load_unit(unit_path)
                validation = validate_unit_against_queue(loaded, unit_sha, queue, queue_sha)
            self.assertEqual(validation["status"], "valid")

            legacy = {**policy, 'cases': expected, 'lanes': {}}
            with patch('tools.benchmark.layout_policy.load_policy', return_value=legacy):
                with self.assertRaisesRegex(PerformanceDecisionError, 'qualified development policy'):
                    validate_unit_against_queue(loaded, unit_sha, queue, queue_sha)
                profile_only = copy.deepcopy(loaded)
                target = 'external/sunspider-1.0/3d-cube'
                profile_only['fast_gate']['target_ids'] = [target]
                profile_only['priority']['opportunity_ids'] = [target]
                with self.assertRaisesRegex(PerformanceDecisionError, 'qualified development policy'):
                    validate_unit_against_queue(profile_only, unit_sha, queue, queue_sha)

            outside = copy.deepcopy(loaded)
            outside["priority"]["rank_ceiling"] = ranks[self.target] - 1
            with patch("tools.benchmark.layout_policy.load_policy", return_value=policy):
                with self.assertRaisesRegex(PerformanceDecisionError, "rank ceiling"):
                    validate_unit_against_queue(outside, unit_sha, queue, queue_sha)


class DevelopmentReplayTests(unittest.TestCase):
    """`queue --development-report` replays raw evidence under the lane's contract."""

    checksum = "cafe"

    def _manifest(self) -> NpmManifest:
        package = Package(
            name="fixture",
            version="1.0.0",
            tarball_url="https://registry.npmjs.org/fixture/-/fixture-1.0.0.tgz",
            sha256="1" * 64,
            integrity=None,
            license="MIT",
            license_evidence_urls=("https://www.npmjs.com/package/fixture/v/1.0.0",),
        )
        case = NpmCase(
            id="run",
            role="development",
            expected_checksum=self.checksum,
            files=(SourceFile("dist/lib.js", "2" * 64),),
        )
        suite = NpmSuite(
            id="fixture",
            name="fixture",
            reporting_rule="fixture only",
            package=package,
            license_review={},
            source=Source(package.tarball_url, "3" * 64, ""),
            cases=(case,),
        )
        return NpmManifest(
            path=Path("fixture.json"),
            sha256="4" * 64,
            preview_id=SPEC.preview_id,
            role="development",
            measurement=Measurement(
                blocks=1, timeout_seconds=30, metric="outer_process_wall_time",
                phase_boundary="before_process_spawn_to_wait_return",
                order="seeded_role_rotation", seed=20261017,
            ),
            suites=(suite,),
        )

    def _record(self, manifest, suite, case, role, phase, block, order, clock):
        stdout = f"CHECKSUM {case.expected_checksum}\n" + f"{SENTINEL}\n" * (
            1 if role == "quickjs-ng" else 2
        )
        return {
            "host": {"node": "fixture"}, "schema_version": 3, "record_type": "sample",
            "measurement_blocks": 1, "timeout_seconds": 30,
            "preview_id": manifest.preview_id, "manifest_sha256": manifest.sha256,
            "claim_eligible": False, "suite_id": suite.id, "case_id": case.id, "role": role,
            "phase": phase, "block": block, "order": order,
            "metric": manifest.measurement.metric, "timer": "python.perf_counter_ns",
            "timer_phase_boundary": manifest.measurement.phase_boundary,
            "timer_started_ns": clock, "timer_finished_ns": clock + 5, "duration_ns": 5,
            "instructions": None, "cycles": None, "started_at": "2026-10-17T00:00:00+00:00",
            "status": "ok", "error": None, "exit_code": 0, "timed_out": False,
            "stdout": stdout, "stderr": "", "stdout_truncated": False, "stderr_truncated": False,
            "binary_sha256": "5" * 64, "bundle_sha256": "6" * 64,
            "argv": ["qjs", "--script" if role == "quickjs-ng" else "--raw", "bundle.js"],
            "source_revision": suite.source.revision,
            "source_files": [{"path": f.path, "sha256": f.sha256} for f in case.files],
        }

    def _raw(self, manifest) -> list[dict]:
        suite, case = manifest.suites[0], manifest.suites[0].cases[0]
        seed = manifest.measurement.seed
        records, clock = [], 10
        for order, role in enumerate(role_orders(list(EXTERNAL_ROLES), 1, seed + 1)[0]):
            records.append(self._record(manifest, suite, case, role, "capability", None, order, clock))
            clock += 10
        for order, role in enumerate(role_orders(list(EXTERNAL_ROLES), 1, seed)[0]):
            records.append(self._record(manifest, suite, case, role, "measurement", 0, order, clock))
            clock += 10
        return records

    def _write(self, directory: Path, records: list[dict]) -> Path:
        path = directory / "development-raw.jsonl"
        path.write_text(
            "".join(json.dumps(record, sort_keys=True) + "\n" for record in records),
            encoding="utf-8",
        )
        return path

    def test_replay_uses_the_lane_status_and_report_shape(self) -> None:
        manifest = self._manifest()
        with tempfile.TemporaryDirectory() as directory_name:
            raw = self._write(Path(directory_name), self._raw(manifest))
            replayed = replay(raw, manifest, status=npm_status, report=development_report)
            expected = development_report(manifest, self._raw(manifest))
            expected["input"] = {
                "sha256": hashlib.sha256(raw.read_bytes()).hexdigest(),
                "byte_length": raw.stat().st_size,
            }
        self.assertEqual(replayed, expected)
        self.assertEqual(replayed["artifact_type"], "quickjs-development-preview-report")
        self.assertEqual(
            replayed["suites"][0]["cases"][0]["expected_checksum"], self.checksum
        )

    def test_the_bare_sentinel_contract_does_not_accept_a_development_sample(self) -> None:
        manifest = self._manifest()
        with tempfile.TemporaryDirectory() as directory_name:
            raw = self._write(Path(directory_name), self._raw(manifest))
            with self.assertRaisesRegex(ExternalPreviewError, "status does not match"):
                replay(raw, manifest)


if __name__ == "__main__":
    unittest.main()
