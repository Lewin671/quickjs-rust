from __future__ import annotations

import copy
import hashlib
import json
import os
import subprocess
import shutil
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.performance_policy import (
    EXPECTED_WORKFLOW_SHA256,
    PREVIEW_ORCHESTRATOR,
    PREVIEW_IMPLEMENTATION_FILES,
    REFERENCE_ENGINE,
    PREVIEW_ROLES,
    PROTOCOL_KEYS,
    PerformancePolicyError,
    cross_check_repository,
    load_policy,
    policy_summary,
    require_gate,
    validate_workflow_bytes,
    _implementation_sha256,
)
from tools.benchmark.hosted_preview import (
    BASE_MODE,
    HOSTED_BASE_REF,
    HOSTED_PUSH_REF,
    PUSH_INTEGRITY_SCOPE,
    PUSH_MODE,
)


ROOT = Path(__file__).resolve().parents[3]
POLICY = ROOT / "benchmarks/performance-policy.json"
WORKFLOW = ROOT / ".github/workflows/performance-smoke.yml"


def yaml_run_scalars(source: str) -> list[str]:
    """Extract literal block values for every YAML run key in this workflow."""
    lines = source.splitlines()
    result: list[str] = []
    index = 0
    while index < len(lines):
        line = lines[index]
        stripped = line.lstrip()
        indent = len(line) - len(stripped)
        if stripped != "run: |":
            index += 1
            continue
        index += 1
        body: list[str] = []
        while index < len(lines):
            current = lines[index]
            current_stripped = current.lstrip()
            current_indent = len(current) - len(current_stripped)
            if current_stripped and current_indent <= indent:
                break
            body.append(current)
            index += 1
        result.append("\n".join(body))
    return result


class PerformancePolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.data = json.loads(POLICY.read_text(encoding="utf-8"))

    def _write(self, directory: Path, data: object | None = None, name: str = "policy.json") -> Path:
        path = directory / name
        path.write_text(
            json.dumps(self.data if data is None else data, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        return path

    def test_checked_in_v2_policy_is_informational_and_cross_checked(self) -> None:
        policy = load_policy(POLICY)
        cross_check_repository(policy, ROOT)
        summary = policy_summary(policy)
        self.assertEqual(summary["hosted_pr_tier"], "informational_preview")
        self.assertEqual(
            summary["hosted_integrity_scope"],
            "cooperative_same_repository_pull_request",
        )
        self.assertEqual(summary["external_admitted_count"], 0)
        self.assertFalse(summary["claim_eligible"])
        self.assertFalse(summary["fixed_hardware_configured"])
        self.assertEqual(
            summary["gates"],
            {"nightly": False, "release": False, "pr_sentinel": False},
        )
        self.assertEqual(summary["evidence_entry_count"], 0)
        self.assertEqual(policy.hosted_orchestrator_path, PREVIEW_ORCHESTRATOR)
        self.assertEqual(policy.hosted_blocks, 3)
        self.assertEqual(policy.hosted_retention_days, 14)
        self.assertEqual(
            policy.hosted_implementation_sha256,
            _implementation_sha256(ROOT, PREVIEW_IMPLEMENTATION_FILES),
        )
        self.assertEqual(
            (policy.reference_identity, policy.reference_repo, policy.reference_revision),
            REFERENCE_ENGINE,
        )
        self.assertEqual(policy.workflow_sha256, EXPECTED_WORKFLOW_SHA256)
        self.assertEqual(policy.hosted_base_ref, HOSTED_BASE_REF)
        self.assertEqual(policy.hosted_push_ref, HOSTED_PUSH_REF)
        self.assertEqual(policy.hosted_push_mode, PUSH_MODE)
        self.assertEqual(policy.hosted_push_integrity_scope, PUSH_INTEGRITY_SCOPE)
        self.assertEqual(hashlib.sha256(WORKFLOW.read_bytes()).hexdigest(), EXPECTED_WORKFLOW_SHA256)
        validate_workflow_bytes(policy, WORKFLOW.read_bytes())
        self.assertEqual(set(policy.protocols), set(PROTOCOL_KEYS))

    def test_duplicate_unknown_and_scalar_types_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            duplicate = POLICY.read_text(encoding="utf-8").replace(
                '"schema_version": 2,',
                '"schema_version": 2,\n  "schema_version": 2,',
                1,
            )
            duplicate_path = directory / "duplicate.json"
            duplicate_path.write_text(duplicate, encoding="utf-8")
            with self.assertRaisesRegex(PerformancePolicyError, "duplicate key"):
                load_policy(duplicate_path)

            mutations = []
            unknown = copy.deepcopy(self.data)
            unknown["hosted_pr"]["future"] = True
            mutations.append((unknown, "unknown"))
            for value in (True, 2.0, 1):
                version = copy.deepcopy(self.data)
                version["schema_version"] = value
                mutations.append((version, "integer version 2"))
            claim_integer = copy.deepcopy(self.data)
            claim_integer["hosted_pr"]["claim_eligible"] = 0
            mutations.append((claim_integer, "expected a boolean"))
            evidence_object = copy.deepcopy(self.data)
            evidence_object["evidence_entries"] = {}
            mutations.append((evidence_object, "expected an array"))
            for index, (data, message) in enumerate(mutations):
                with self.subTest(index=index):
                    with self.assertRaisesRegex(PerformancePolicyError, message):
                        load_policy(self._write(directory, data, f"scalar-{index}.json"))

    def test_hosted_preview_contract_mutations_fail_closed(self) -> None:
        mutations: list[tuple[dict[str, object], str]] = []
        fields = (
            ("tier", "gate", "frozen hosted tier"),
            ("evidence_class", "claim", "informational"),
            ("claim_eligible", True, "remain false"),
            ("gate", True, "requires false"),
            ("slowdown_threshold", 1.05, "remain null"),
            ("upload_timing_evidence", False, "must be true"),
            ("orchestrator_path", "scripts/other.sh", "frozen path"),
            ("portfolio", "one-case", "frozen portfolio"),
            ("blocks", 4, "exactly 3"),
            ("roles", list(reversed(PREVIEW_ROLES)), "frozen roles"),
            ("artifact_retention_days", 90, "must be 14"),
            ("workflow_sha256", "1" * 64, "exact v2 workflow bytes"),
        )
        for field, value, message in fields:
            data = copy.deepcopy(self.data)
            data["hosted_pr"][field] = value
            mutations.append((data, message))
        for field, value in (
            ("pr_mode", "candidate_owned_harness"),
            ("malicious_candidate_resistant", True),
            ("forks_supported", True),
            ("base_ref", "release"),
            ("push_mode", "candidate_owned_harness"),
            ("push_event", "pull_request"),
            ("push_ref", "refs/heads/release"),
            ("push_comparison", "after_vs_quickjs_only"),
            ("push_harness_owner", "event_before_base"),
        ):
            data = copy.deepcopy(self.data)
            data["hosted_pr"]["harness"][field] = value
            mutations.append((data, "integrity boundary"))
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, (data, message) in enumerate(mutations):
                with self.subTest(index=index):
                    with self.assertRaisesRegex(PerformancePolicyError, message):
                        load_policy(self._write(directory, data, f"hosted-{index}.json"))

    def test_reference_pin_and_implementation_inventory_drift_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for field, value in (
                ("identity", "quickjs"),
                ("source_repo", "https://github.com/example/quickjs.git"),
                ("revision", "1" * 40),
            ):
                data = copy.deepcopy(self.data)
                data["reference_engine"][field] = value
                with self.assertRaisesRegex(PerformancePolicyError, "frozen QuickJS-NG pin"):
                    load_policy(self._write(directory, data, f"reference-{field}.json"))
            inventory = copy.deepcopy(self.data)
            inventory["hosted_implementation"]["files"].pop()
            with self.assertRaisesRegex(PerformancePolicyError, "frozen inventory"):
                load_policy(self._write(directory, inventory, "inventory.json"))

    def test_aggregate_hash_detects_each_hosted_implementation_file_drift(self) -> None:
        policy = load_policy(POLICY)
        with tempfile.TemporaryDirectory() as directory_name:
            replica = Path(directory_name)
            for relative in PREVIEW_IMPLEMENTATION_FILES:
                destination = replica / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / relative, destination)
            self.assertEqual(
                _implementation_sha256(replica, PREVIEW_IMPLEMENTATION_FILES),
                policy.hosted_implementation_sha256,
            )
            for index, relative in enumerate(PREVIEW_IMPLEMENTATION_FILES):
                destination = replica / relative
                original = destination.read_bytes()
                destination.write_bytes(original + b"\n# drift\n")
                with self.subTest(relative=relative):
                    self.assertNotEqual(
                        _implementation_sha256(replica, PREVIEW_IMPLEMENTATION_FILES),
                        policy.hosted_implementation_sha256,
                    )
                destination.write_bytes(original)

    def test_paths_urls_and_protocol_hashes_remain_strict(self) -> None:
        mutations = []
        for value in ("/tmp/manifest.json", "../manifest.json", "benchmarks\\manifest.json"):
            data = copy.deepcopy(self.data)
            data["protocols"]["throughput_measurement"]["manifest_path"] = value
            mutations.append((data, "repository-relative path"))
        for value in (
            "http://docs.github.com/actions", "https://host.invalid:notaport/path",
            "https://host.invalid:99999/path", "https://host space.invalid/path",
            "https://[::1/path",
        ):
            data = copy.deepcopy(self.data)
            data["hosted_pr"]["provider_url"] = value
            mutations.append((data, "URL|authority"))
        for value in ("bad", "0" * 64):
            data = copy.deepcopy(self.data)
            data["protocols"]["throughput_measurement"]["protocol_sha256"] = value
            mutations.append((data, "non-zero lowercase SHA-256"))
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, (data, message) in enumerate(mutations):
                with self.subTest(index=index):
                    with self.assertRaisesRegex(PerformancePolicyError, message):
                        load_policy(self._write(directory, data, f"strict-{index}.json"))

    def test_v2_cannot_enable_hardware_gates_or_claim_entries(self) -> None:
        mutations = []
        fixed = copy.deepcopy(self.data)
        fixed["fixed_hardware"]["configured"] = True
        mutations.append((fixed, "configured=false"))
        fingerprint = copy.deepcopy(self.data)
        fingerprint["fixed_hardware"]["hardware_fingerprint"] = "pretend-host"
        mutations.append((fingerprint, "null fingerprint"))
        gate = copy.deepcopy(self.data)
        gate["gates"]["nightly"]["enabled"] = True
        mutations.append((gate, "deny-only"))
        evidence = copy.deepcopy(self.data)
        evidence["evidence_entries"] = [{"claim": "fast"}]
        mutations.append((evidence, "empty array"))
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, (data, message) in enumerate(mutations):
                with self.subTest(index=index):
                    with self.assertRaisesRegex(PerformancePolicyError, message):
                        load_policy(self._write(directory, data, f"deny-{index}.json"))

    def test_activation_prerequisites_and_protocol_bindings_are_frozen(self) -> None:
        mutations = []
        minimum = copy.deepcopy(self.data)
        minimum["gates"]["nightly"]["activation_prerequisites"][
            "minimum_independent_aa_shadow_reports"
        ] = 1
        mutations.append((minimum, "A/A report minimum"))
        aa = copy.deepcopy(self.data)
        aa["gates"]["release"]["activation_prerequisites"][
            "aa_shadow_requirements"
        ] = ["same_binary"]
        mutations.append((aa, "A/A requirements"))
        artifacts = copy.deepcopy(self.data)
        artifacts["gates"]["pr_sentinel"]["activation_prerequisites"][
            "required_artifacts"
        ].remove("false_positive_budget")
        mutations.append((artifacts, "required artifacts"))
        protocol = copy.deepcopy(self.data)
        protocol["gates"]["nightly"]["activation_prerequisites"][
            "required_protocol_sha256"
        ]["throughput_measurement"] = "1" * 64
        mutations.append((protocol, "must match policy.protocols"))
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, (data, message) in enumerate(mutations):
                with self.subTest(index=index):
                    with self.assertRaisesRegex(PerformancePolicyError, message):
                        load_policy(self._write(directory, data, f"prereq-{index}.json"))

    def test_repository_cross_check_detects_protocol_drift(self) -> None:
        data = copy.deepcopy(self.data)
        digest = "1" * 64
        data["protocols"]["throughput_measurement"]["protocol_sha256"] = digest
        for gate in data["gates"].values():
            gate["activation_prerequisites"]["required_protocol_sha256"][
                "throughput_measurement"
            ] = digest
        with tempfile.TemporaryDirectory() as directory_name:
            policy = load_policy(self._write(Path(directory_name), data))
        with self.assertRaisesRegex(PerformancePolicyError, "does not match"):
            cross_check_repository(policy, ROOT)

    def test_every_gate_requirement_fails_closed(self) -> None:
        policy = load_policy(POLICY)
        for gate_id in ("nightly", "release", "pr_sentinel"):
            with self.subTest(gate_id=gate_id):
                with self.assertRaisesRegex(PerformancePolicyError, "disabled"):
                    require_gate(policy, gate_id)


class PerformancePolicyCliTests(unittest.TestCase):
    def _run(self, args: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(ROOT / "scripts/performance-policy-audit.sh"), *args],
            cwd=cwd, capture_output=True, text=True, timeout=15, check=False,
        )

    def test_default_is_repo_root_independent_and_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            result = self._run([], Path(directory_name))
        self.assertEqual(result.returncode, 0, result.stderr)
        summary = json.loads(result.stdout)
        self.assertEqual(summary["hosted_pr_tier"], "informational_preview")
        self.assertFalse(summary["claim_eligible"])
        self.assertEqual(summary["gates"], {
            "nightly": False, "pr_sentinel": False, "release": False,
        })
        self.assertNotIn(str(ROOT), result.stdout)
        self.assertEqual(
            result.stdout,
            json.dumps(summary, sort_keys=True, separators=(",", ":")) + "\n",
        )

    def test_gate_requirements_and_argparse_fail_structurally(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for gate_id in ("nightly", "release", "pr_sentinel"):
                result = self._run(["--require-gate", gate_id], directory)
                self.assertEqual(result.returncode, 2)
                self.assertIn("disabled", json.loads(result.stderr)["error"]["message"])
            for attack in (
                ("--unknown",), ("--policy",), ("--require-gate",),
                ("--require-gate", "future"),
            ):
                result = self._run(list(attack), directory)
                self.assertEqual(result.returncode, 2)
                error = json.loads(result.stderr)["error"]
                self.assertEqual(error["code"], "performance_policy_audit_failed")
                self.assertIn("arguments:", error["message"])

    def test_custom_policy_is_structural_only_and_cannot_authorize_gate(self) -> None:
        data = json.loads(POLICY.read_text(encoding="utf-8"))
        digest = "1" * 64
        data["protocols"]["throughput_measurement"]["protocol_sha256"] = digest
        for gate in data["gates"].values():
            gate["activation_prerequisites"]["required_protocol_sha256"][
                "throughput_measurement"
            ] = digest
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            custom = directory / "custom.json"
            custom.write_text(json.dumps(data), encoding="utf-8")
            structural = self._run(["--policy", str(custom)], directory)
            self.assertEqual(structural.returncode, 0, structural.stderr)
            self.assertFalse(json.loads(structural.stdout)["claim_eligible"])
            rejected = self._run(
                ["--policy", str(custom), "--require-gate", "nightly"], directory
            )
            self.assertEqual(rejected.returncode, 2)
            self.assertIn(
                "not allowed with argument",
                json.loads(rejected.stderr)["error"]["message"],
            )

    def test_output_is_atomic_and_never_overwrites(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            output = directory / "summary.json"
            first = self._run(["--output", str(output)], directory)
            self.assertEqual(first.returncode, 0, first.stderr)
            original = output.read_bytes()
            second = self._run(["--output", str(output)], directory)
            self.assertEqual(second.returncode, 2)
            self.assertEqual(output.read_bytes(), original)
            self.assertIn("refusing to overwrite", second.stderr)

    def test_absolute_wrappers_ignore_malicious_cwd_packages(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            package = directory / "tools/benchmark"
            package.mkdir(parents=True)
            (directory / "tools/__init__.py").write_text("", encoding="utf-8")
            (package / "__init__.py").write_text("", encoding="utf-8")
            marker = directory / "shadow-loaded"
            malicious = (
                "import os\nfrom pathlib import Path\n"
                "Path(os.environ['SHADOW_MARKER']).write_text('loaded')\n"
            )
            for module in ("performance_policy.py", "external_corpora.py"):
                (package / module).write_text(malicious, encoding="utf-8")
            environment = {
                **os.environ, "PYTHONPATH": str(directory), "SHADOW_MARKER": str(marker),
            }
            for script in ("performance-policy-audit.sh", "external-corpus-audit.sh"):
                result = subprocess.run(
                    [str(ROOT / "scripts" / script)], cwd=directory, env=environment,
                    capture_output=True, text=True, timeout=15, check=False,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertFalse(marker.exists())


class PerformancePreviewWorkflowTests(unittest.TestCase):
    def test_workflow_contract_builds_reports_summarizes_and_uploads_without_gating(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        setup_action = (ROOT / ".github/actions/setup-rust/action.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("name: Performance Preview (Informational, Non-Gating)", workflow)
        self.assertIn("pull_request_target:", workflow)
        self.assertIn("push:", workflow)
        self.assertEqual(workflow.count("branches: [main]"), 2)
        self.assertNotIn("workflow_dispatch:", workflow)
        self.assertNotIn("\n  pull_request:\n", workflow)
        self.assertNotIn("schedule:", workflow)
        self.assertEqual(workflow.count("runs-on: ubuntu-latest"), 3)
        self.assertNotIn("self-hosted", workflow)
        self.assertIn("repository: ${{ github.event.pull_request.base.repo.full_name }}", workflow)
        self.assertIn("ref: ${{ github.event.pull_request.base.sha }}", workflow)
        self.assertIn("repository: ${{ github.event.pull_request.head.repo.full_name }}", workflow)
        self.assertIn("ref: ${{ github.event.pull_request.head.sha }}", workflow)
        self.assertIn("path: target/performance-preview/candidate-source", workflow)
        self.assertIn("repository: ${{ github.repository }}", workflow)
        self.assertIn("ref: ${{ github.event.after }}", workflow)
        self.assertIn("repository: ${{ github.event.repository.full_name }}", workflow)
        self.assertIn("ref: ${{ github.event.before }}", workflow)
        self.assertIn("path: target/performance-preview/base-source", workflow)
        self.assertEqual(workflow.count("fetch-depth: 1"), 4)
        self.assertIn("persist-credentials: false", workflow)
        self.assertIn("submodules: false", workflow)
        self.assertIn(BASE_MODE, workflow)
        self.assertIn(PUSH_MODE, workflow)
        self.assertNotIn("bootstrap", workflow.lower())
        self.assertEqual(workflow.count("PR_BASE_REF: ${{ github.event.pull_request.base.ref }}"), 1)
        self.assertEqual(workflow.count("PR_HEAD_REF: ${{ github.event.pull_request.head.ref }}"), 1)
        self.assertEqual(workflow.count('--base-ref "$PR_BASE_REF"'), 1)
        self.assertEqual(workflow.count('--pr-number "$PR_NUMBER"'), 1)
        self.assertEqual(workflow.count('--head-ref "$PR_HEAD_REF"'), 1)
        self.assertIn("PUSH_BEFORE_SHA: ${{ github.event.before }}", workflow)
        self.assertIn("PUSH_AFTER_SHA: ${{ github.event.after }}", workflow)
        self.assertIn("PUSH_WORKFLOW_SHA: ${{ github.sha }}", workflow)
        self.assertIn('--before-sha "$PUSH_BEFORE_SHA"', workflow)
        self.assertIn('--after-sha "$PUSH_AFTER_SHA"', workflow)
        self.assertIn('--workflow-sha "$PUSH_WORKFLOW_SHA"', workflow)
        self.assertIn("fork-preview-unsupported", workflow)
        self.assertIn("github.event.pull_request.number || github.run_id", workflow)
        self.assertIn("github.event_name == 'pull_request_target'", workflow)
        self.assertIn("github.event_name == 'push'", workflow)
        self.assertIn("uses: ./.github/actions/setup-rust", workflow)
        self.assertNotIn("candidate-source/.github/actions/setup-rust", workflow)
        self.assertEqual(workflow.count("PYTHONDONTWRITEBYTECODE: '1'"), 2)
        self.assertIn("source-root:", setup_action)
        self.assertIn("working-directory: ${{ inputs.source-root }}", setup_action)
        self.assertIn("inputs.source-root", setup_action)
        self.assertIn('"$QJS_HARNESS_ROOT/scripts/performance-preview.sh"', workflow)
        self.assertIn("timeout-minutes: 35", workflow)
        self.assertIn("$GITHUB_STEP_SUMMARY", workflow)
        self.assertIn("actions/upload-artifact@v6", workflow)
        self.assertEqual(workflow.count("uses: actions/cache/restore@v5"), 6)
        self.assertEqual(workflow.count("uses: actions/cache/save@v5"), 3)
        self.assertEqual(workflow.count("continue-on-error: true"), 9)
        self.assertNotIn("restore-keys:", workflow)
        base_job = workflow.split("  base-owned-preview:", 1)[1].split(
            "  main-push-preview:", 1
        )[0]
        main_job = workflow.split("  main-push-preview:", 1)[1].split(
            "  fork-preview-unsupported:", 1
        )[0]
        self.assertIn("actions/cache/restore@v5", base_job)
        self.assertNotIn("actions/cache/save@v5", base_job)
        self.assertIn("actions/cache/save@v5", main_job)
        self.assertIn("Save candidate executable cache from trusted main", main_job)
        self.assertEqual(main_job.count("tools.benchmark.build_cache ready"), 3)
        self.assertGreaterEqual(main_job.count("always() && !cancelled()"), 6)
        self.assertLess(
            main_job.index("Build and measure three pinned engines"),
            main_job.index("Revalidate candidate executable cache for trusted save"),
        )
        self.assertLess(
            main_job.index("Save QuickJS-NG executable cache from trusted main"),
            main_job.index("Publish complete or durable failure summary"),
        )
        self.assertGreaterEqual(workflow.count("if: always()"), 4)
        self.assertIn("retention-days: 14", workflow)
        self.assertEqual(workflow.count("if-no-files-found: error"), 2)
        self.assertGreaterEqual(workflow.count('mkdir -p "$EVIDENCE_DIR"'), 4)
        self.assertIn("tools.benchmark.hosted_preview publish", workflow)
        self.assertNotIn("pull-requests: write", workflow)
        self.assertNotIn("secrets.", workflow)
        self.assertNotIn("threshold", workflow.lower())
        self.assertNotIn("candidate==base", workflow)

        run_scalars = yaml_run_scalars(workflow)
        self.assertGreaterEqual(len(run_scalars), 10)
        for index, body in enumerate(run_scalars):
            with self.subTest(run_scalar=index):
                self.assertNotIn("${{ github.event.pull_request", body)

    def test_workflow_is_exactly_hash_bound(self) -> None:
        policy = load_policy(POLICY)
        original = WORKFLOW.read_bytes()
        validate_workflow_bytes(policy, original)
        mutations = (
            original + b"\n",
            original.replace(b"submodules: false", b"submodules: true"),
            original.replace(b"retention-days: 14", b"retention-days: 90"),
            original.replace(b"pull_request_target:", b"pull_request:"),
        )
        for value in mutations:
            with self.assertRaisesRegex(PerformancePolicyError, "bytes differ"):
                validate_workflow_bytes(policy, value)

    def test_lifecycle_list_surface_still_contains_exactly_six_ids(self) -> None:
        source = (ROOT / "crates/qjs-runtime/benches/lifecycle.rs").read_text(
            encoding="utf-8"
        )
        self.assertEqual(source.count("BenchmarkId::new("), 3)
        fixture_block = source.split("const FIXTURES", 1)[1].split("];", 1)[0]
        self.assertEqual(fixture_block.count("Fixture {"), 2)


if __name__ == "__main__":
    unittest.main()
