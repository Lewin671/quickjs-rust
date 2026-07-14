from __future__ import annotations

import copy
import json
import os
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.performance_policy import (
    PROTOCOL_KEYS,
    SMOKE_COMMANDS,
    EXPECTED_WORKFLOW_BYTES,
    EXPECTED_WORKFLOW_SHA256,
    EXPECTED_WORKFLOW_TEXT,
    PerformancePolicyError,
    cross_check_repository,
    load_policy,
    policy_summary,
    require_gate,
    validate_workflow_bytes,
)


ROOT = Path(__file__).resolve().parents[3]
POLICY = ROOT / "benchmarks/performance-policy.json"
WORKFLOW = ROOT / ".github/workflows/performance-smoke.yml"


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

    def test_checked_in_policy_is_deny_only_and_cross_checked(self) -> None:
        policy = load_policy(POLICY)
        cross_check_repository(policy, ROOT)
        summary = policy_summary(policy)
        self.assertEqual(summary["hosted_pr_tier"], "smoke_only")
        self.assertEqual(summary["external_admitted_count"], 0)
        self.assertFalse(summary["claim_eligible"])
        self.assertFalse(summary["fixed_hardware_configured"])
        self.assertEqual(
            summary["gates"],
            {"nightly": False, "release": False, "pr_sentinel": False},
        )
        self.assertEqual(summary["evidence_entry_count"], 0)
        self.assertEqual(policy.hosted_commands, SMOKE_COMMANDS)
        self.assertEqual(policy.workflow_sha256, EXPECTED_WORKFLOW_SHA256)
        self.assertEqual(WORKFLOW.read_bytes(), EXPECTED_WORKFLOW_BYTES)
        validate_workflow_bytes(policy, WORKFLOW.read_bytes())
        self.assertEqual(set(policy.protocols), set(PROTOCOL_KEYS))

    def test_duplicate_unknown_and_scalar_types_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            duplicate = POLICY.read_text(encoding="utf-8").replace(
                '"schema_version": 1,',
                '"schema_version": 1,\n  "schema_version": 1,',
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
            bool_version = copy.deepcopy(self.data)
            bool_version["schema_version"] = True
            mutations.append((bool_version, "integer version 1"))
            float_version = copy.deepcopy(self.data)
            float_version["schema_version"] = 1.0
            mutations.append((float_version, "integer version 1"))
            claim_integer = copy.deepcopy(self.data)
            claim_integer["claim_eligible"] = 0
            mutations.append((claim_integer, "expected a boolean"))
            evidence_object = copy.deepcopy(self.data)
            evidence_object["evidence_entries"] = {}
            mutations.append((evidence_object, "expected an array"))
            for index, (data, message) in enumerate(mutations):
                path = self._write(directory, data, f"scalar-{index}.json")
                with self.subTest(index=index):
                    with self.assertRaisesRegex(PerformancePolicyError, message):
                        load_policy(path)

    def test_path_url_hash_and_enum_mutations_fail_closed(self) -> None:
        mutations = []
        for value in ("/tmp/manifest.json", "../manifest.json", "benchmarks\\manifest.json"):
            data = copy.deepcopy(self.data)
            data["protocols"]["throughput_measurement"]["manifest_path"] = value
            mutations.append((data, "repository-relative path"))
        for value in (
            "http://docs.github.com/actions",
            "https://host.invalid:notaport/path",
            "https://host.invalid:99999/path",
            "https://host space.invalid/path",
            "https://[::1/path",
        ):
            data = copy.deepcopy(self.data)
            data["hosted_pr"]["provider_url"] = value
            mutations.append((data, "URL|authority"))
        bad_hash = copy.deepcopy(self.data)
        bad_hash["protocols"]["throughput_measurement"]["protocol_sha256"] = "bad"
        mutations.append((bad_hash, "non-zero lowercase SHA-256"))
        zero_hash = copy.deepcopy(self.data)
        zero_hash["protocols"]["throughput_measurement"]["protocol_sha256"] = "0" * 64
        mutations.append((zero_hash, "non-zero lowercase SHA-256"))
        wrong_tier = copy.deepcopy(self.data)
        wrong_tier["hosted_pr"]["tier"] = "gate"
        mutations.append((wrong_tier, "invalid frozen hosted tier"))
        wrong_workflow_path = copy.deepcopy(self.data)
        wrong_workflow_path["hosted_pr"]["workflow_path"] = "workflow.yml"
        mutations.append((wrong_workflow_path, "invalid frozen path"))
        wrong_workflow_hash = copy.deepcopy(self.data)
        wrong_workflow_hash["hosted_pr"]["workflow_sha256"] = "1" * 64
        mutations.append((wrong_workflow_hash, "exact v1 workflow bytes"))

        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, (data, message) in enumerate(mutations):
                with self.subTest(index=index):
                    with self.assertRaisesRegex(PerformancePolicyError, message):
                        load_policy(self._write(directory, data, f"strict-{index}.json"))

    def test_v1_cannot_enable_hardware_gates_or_evidence(self) -> None:
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
        hosted_gate = copy.deepcopy(self.data)
        hosted_gate["hosted_pr"]["gate"] = True
        mutations.append((hosted_gate, "v1 requires false"))
        upload = copy.deepcopy(self.data)
        upload["hosted_pr"]["upload_timing_evidence"] = True
        mutations.append((upload, "v1 requires false"))

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

    def test_custom_policy_is_structural_only_and_repository_cross_check_detects_drift(self) -> None:
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

    def test_every_v1_gate_requirement_fails_closed(self) -> None:
        policy = load_policy(POLICY)
        for gate_id in ("nightly", "release", "pr_sentinel"):
            with self.subTest(gate_id=gate_id):
                with self.assertRaisesRegex(PerformancePolicyError, "disabled"):
                    require_gate(policy, gate_id)


class PerformancePolicyCliTests(unittest.TestCase):
    def _run(self, args: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(ROOT / "scripts/performance-policy-audit.sh"), *args],
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=15,
            check=False,
        )

    def test_default_is_repo_root_independent_and_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            result = self._run([], Path(directory_name))
        self.assertEqual(result.returncode, 0, result.stderr)
        summary = json.loads(result.stdout)
        self.assertFalse(summary["claim_eligible"])
        self.assertEqual(summary["gates"], {
            "nightly": False, "pr_sentinel": False, "release": False
        })
        self.assertNotIn(str(ROOT), result.stdout)
        self.assertEqual(
            result.stdout,
            json.dumps(summary, sort_keys=True, separators=(",", ":")) + "\n",
        )

    def test_all_checked_in_gate_requirements_exit_two(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for gate_id in ("nightly", "release", "pr_sentinel"):
                result = self._run(["--require-gate", gate_id], directory)
                with self.subTest(gate_id=gate_id):
                    self.assertEqual(result.returncode, 2)
                    error = json.loads(result.stderr)["error"]
                    self.assertEqual(error["code"], "performance_policy_audit_failed")
                    self.assertIn("disabled", error["message"])
                    self.assertNotIn("Traceback", result.stderr)

    def test_custom_policy_cannot_be_combined_with_gate_requirement(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            result = self._run(
                ["--policy", str(POLICY), "--require-gate", "nightly"],
                Path(directory_name),
            )
        self.assertEqual(result.returncode, 2)
        self.assertIn("not allowed with argument", json.loads(result.stderr)["error"]["message"])

    def test_custom_policy_is_not_a_repository_trust_root(self) -> None:
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
            result = self._run(["--policy", str(custom)], directory)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(json.loads(result.stdout)["claim_eligible"])

    def test_argparse_errors_are_structured(self) -> None:
        attacks = (("--unknown",), ("--policy",), ("--require-gate",),
                   ("--require-gate", "future"))
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for attack in attacks:
                result = self._run(list(attack), directory)
                with self.subTest(attack=attack):
                    self.assertEqual(result.returncode, 2)
                    error = json.loads(result.stderr)["error"]
                    self.assertEqual(error["code"], "performance_policy_audit_failed")
                    self.assertIn("arguments:", error["message"])
                    self.assertNotIn("Traceback", result.stderr)

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
            self.assertEqual(list(directory.glob(".summary.json.*")), [])

    def test_absolute_wrappers_ignore_malicious_cwd_python_packages(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            package = directory / "tools/benchmark"
            package.mkdir(parents=True)
            (directory / "tools/__init__.py").write_text("", encoding="utf-8")
            (package / "__init__.py").write_text("", encoding="utf-8")
            marker = directory / "shadow-loaded"
            malicious = (
                "import os\n"
                "from pathlib import Path\n"
                "Path(os.environ['SHADOW_MARKER']).write_text('loaded')\n"
                "print('SHADOW')\n"
            )
            for module in ("performance_policy.py", "external_corpora.py"):
                (package / module).write_text(malicious, encoding="utf-8")
            environment = {
                **os.environ,
                "PYTHONPATH": str(directory),
                "SHADOW_MARKER": str(marker),
            }
            commands = (
                ("performance-policy-audit.sh", "fixed_hardware_configured"),
                ("external-corpus-audit.sh", "admitted_count"),
            )
            for script, expected_key in commands:
                result = subprocess.run(
                    [str(ROOT / "scripts" / script)],
                    cwd=directory,
                    env=environment,
                    capture_output=True,
                    text=True,
                    timeout=15,
                    check=False,
                )
                with self.subTest(script=script):
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertIn(expected_key, json.loads(result.stdout))
                    self.assertNotIn("SHADOW", result.stdout)
                    self.assertFalse(marker.exists())


class PerformanceSmokeWorkflowTests(unittest.TestCase):
    def test_workflow_is_exact_hosted_smoke_without_claims_or_evidence(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("name: Performance Smoke (No Claims or Gates)", workflow)
        self.assertIn("pull_request:", workflow)
        self.assertIn("workflow_dispatch:", workflow)
        self.assertIn("- main", workflow)
        self.assertIn("- 'agent/**'", workflow)
        self.assertNotIn("schedule:", workflow)
        self.assertEqual(workflow.count("runs-on: ubuntu-latest"), 1)
        self.assertNotIn("self-hosted", workflow)
        self.assertIn("submodules: false", workflow)
        self.assertNotIn("git submodule", workflow)
        self.assertNotIn("actions/upload-artifact", workflow)
        self.assertIn("no performance claim or gate", workflow)
        commands = tuple(re.findall(r"^\s+run: (\S.*)$", workflow, flags=re.MULTILINE))
        self.assertEqual(commands, SMOKE_COMMANDS)
        for command in commands:
            self.assertNotIn("benchmark-report", command)
            self.assertNotIn("resource-benchmark-report", command)
            self.assertNotIn("--output", command)
            self.assertNotIn("--candidate", command)
        self.assertTrue(commands[-1].endswith("--quick --list"))

    def test_any_workflow_capability_or_command_drift_fails_exact_binding(self) -> None:
        policy = load_policy(POLICY)
        mutations = (
            EXPECTED_WORKFLOW_TEXT.replace(
                "      - name: Setup Rust\n        uses: ./.github/actions/setup-rust",
                "      - name: Setup Rust\n        uses: ./.github/actions/setup-rust\n\n"
                "      - name: Upload pages artifact\n"
                "        uses: actions/upload-pages-artifact@v4",
            ),
            EXPECTED_WORKFLOW_TEXT + "\n  extra-job:\n    runs-on: ubuntu-latest\n",
            EXPECTED_WORKFLOW_TEXT.replace(
                "uses: ./.github/actions/setup-rust",
                "uses: owner/repository/.github/workflows/reusable.yml@main",
            ),
            EXPECTED_WORKFLOW_TEXT.replace(
                "run: ./scripts/external-corpus-audit.sh",
                "run: |\n          ./scripts/external-corpus-audit.sh\n          ./timed-command",
            ),
            EXPECTED_WORKFLOW_TEXT.replace("submodules: false", "submodules: true"),
            EXPECTED_WORKFLOW_TEXT.replace(
                "  workflow_dispatch:\n",
                "  workflow_dispatch:\n  schedule:\n    - cron: '0 0 * * *'\n",
            ),
        )
        for index, workflow in enumerate(mutations):
            with self.subTest(index=index):
                with self.assertRaisesRegex(PerformancePolicyError, "bytes differ"):
                    validate_workflow_bytes(policy, workflow.encode("utf-8"))

    def test_lifecycle_list_surface_contains_exactly_six_ids(self) -> None:
        source = (ROOT / "crates/qjs-runtime/benches/lifecycle.rs").read_text(
            encoding="utf-8"
        )
        self.assertEqual(source.count("BenchmarkId::new("), 3)
        fixture_block = source.split("const FIXTURES", 1)[1].split("];", 1)[0]
        self.assertEqual(fixture_block.count("Fixture {"), 2)


if __name__ == "__main__":
    unittest.main()
