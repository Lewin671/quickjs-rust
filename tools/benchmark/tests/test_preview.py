from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import subprocess
import sys
import tempfile
import unittest
import uuid
from pathlib import Path

from tools.benchmark.preview import (
    RUST_BUILD_FLAGS,
    PreviewError,
    _github_url,
    escape_markdown,
    prepare,
    summarize,
    verify_source,
)
from tools.benchmark.hosted_preview import (
    BASE_MODE,
    BOOTSTRAP_BASE_SHA,
    BOOTSTRAP_HEAD_REF,
    BOOTSTRAP_MODE,
    BOOTSTRAP_PR_NUMBER,
    HOSTED_BASE_REF,
)
from tools.benchmark.receipts import load_receipt
from tools.benchmark.schema import load_manifest, sha256_file


ROOT = Path(__file__).resolve().parents[3]
HARNESS_REVISION = "a" * 40


def report(ratio_base: float = 1.25, ratio_qjs: float = 0.8) -> dict[str, object]:
    def comparison(ratio: float) -> dict[str, object]:
        return {
            "overall": {
                "ratio": ratio,
                "confidence_interval": {"lower": ratio * 0.9, "upper": ratio * 1.1},
            }
        }

    engines = []
    for index, role in enumerate(("candidate", "base", "quickjs-ng"), 1):
        engines.append({
            "role": role,
            "binary_sha256": str(index) * 64,
            "receipt": {"source": {"revision": str(index) * 40}},
        })
    return {
        "schema_id": "quickjs-benchmark-report",
        "schema_version": 3,
        "claim_eligible": False,
        "analysis_contract": {"bootstrap": {"confidence": 0.95}},
        "run": {
            "profile": {"id": "github-hosted-linux-x86_64-informational-v1"},
            "engines": engines,
        },
        "coverage": {
            "roles": 3, "cases": 7, "blocks": 3,
            "comparison_input_complete": True,
        },
        "health": {
            "status": "inconclusive",
            "blocks": {"valid": 3, "invalid": 0, "status": "non_claim"},
            "linearity": {"status": "pass"},
        },
        "comparisons": {
            "candidate_vs_base": comparison(ratio_base),
            "candidate_vs_quickjs_ng": comparison(ratio_qjs),
        },
    }


def summarize_report(value: dict[str, object]) -> tuple[str, dict[str, object]]:
    return summarize(
        value,
        harness_mode=BASE_MODE,
        harness_revision=HARNESS_REVISION,
    )


class PreviewSummaryTests(unittest.TestCase):
    def test_ratio_uses_precise_ns_per_operation_language(self) -> None:
        markdown, machine = summarize_report(report())
        self.assertIn("Informational only — non-gating — not a fixed-hardware claim", markdown)
        self.assertIn("candidate vs base | 1.2500×", markdown)
        self.assertIn("25.00% higher ns/op", markdown)
        self.assertIn("candidate vs QuickJS-NG | 0.8000×", markdown)
        self.assertIn("20.00% lower ns/op", markdown)
        self.assertNotIn("faster", markdown.lower())
        self.assertNotIn("slower", markdown.lower())
        self.assertIn("Valid blocks: `3/3`", markdown)
        self.assertEqual(
            machine["ratio_semantics"],
            "candidate_over_comparator_wall_ns_per_operation",
        )
        self.assertEqual(machine["harness"]["revision"], HARNESS_REVISION)
        self.assertEqual(
            machine["integrity_scope"], "cooperative_same_repository_pull_request"
        )
        self.assertFalse(machine["malicious_candidate_resistant"])
        self.assertEqual(set(machine["engines"]), {"candidate", "base", "quickjs-ng"})

    def test_higher_and_lower_ratios_remain_non_gating_success(self) -> None:
        for base_ratio, quickjs_ratio in ((1.5, 1.2), (0.5, 0.8)):
            with self.subTest(base_ratio=base_ratio, quickjs_ratio=quickjs_ratio):
                markdown, machine = summarize_report(report(base_ratio, quickjs_ratio))
                self.assertEqual(machine["state"], "success")
                self.assertIn("non-gating", markdown)

    def test_invalid_health_or_missing_comparison_never_emits_direction(self) -> None:
        attacks = []
        for section, field, value in (
            ("health", "status", "invalid"),
            ("blocks", "valid", 2),
            ("blocks", "invalid", 1),
            ("blocks", "status", "invalid"),
            ("linearity", "status", "fail"),
        ):
            value_report = report()
            target = value_report["health"] if section == "health" else value_report["health"][section]
            target[field] = value
            attacks.append(value_report)
        missing = report()
        missing["comparisons"]["candidate_vs_base"] = None
        attacks.append(missing)
        incomplete = report()
        incomplete["coverage"]["comparison_input_complete"] = False
        attacks.append(incomplete)
        for value_report in attacks:
            with self.subTest(value=value_report):
                with self.assertRaises(PreviewError):
                    summarize_report(value_report)

    def test_profile_and_markdown_payloads_fail_or_escape(self) -> None:
        unsafe = report()
        unsafe["run"]["profile"]["id"] = "![image](https://attacker.invalid/x)"
        with self.assertRaisesRegex(PreviewError, "unsafe Markdown"):
            summarize_report(unsafe)
        escaped = escape_markdown("![x](https://a.invalid)<script>*bold*")
        self.assertNotIn("![", escaped)
        self.assertNotIn("<script>", escaped)
        self.assertIn("\\!\\[x\\]\\(", escaped)


class PreviewPreparationTests(unittest.TestCase):
    def test_shell_orchestrator_enforces_sources_builds_and_partial_status(self) -> None:
        script = (ROOT / "scripts/performance-preview.sh").read_text(encoding="utf-8")
        for value in (
            "--harness-mode", "--candidate-source", "--base-source",
            BASE_MODE, BOOTSTRAP_MODE, "verify-source",
            "CARGO_ENCODED_RUSTFLAGS", "profile.release.lto=false",
            'make -C "$QUICKJS_SOURCE" "CC=$QUICKJS_CC" BUILD_TYPE=Release all',
            "--manifest \"$MANIFEST\" --blocks 3", "--candidate-receipt",
            "--base-receipt", "--quickjs-ng-receipt", "--state pending",
            "--state failed", "--status-output", 'cp "$MANIFEST" "$OUTPUT/manifest.json"',
            "trap record_error ERR", 'CURRENT_PHASE="build_candidate"',
            'CURRENT_PHASE="build_base"', 'CURRENT_PHASE="build_quickjs_ng"',
            'CURRENT_PHASE="measurement"', 'CURRENT_PHASE="summary"',
            'CURRENT_PHASE="post_measure_validation"', "GITHUB_ENV GITHUB_PATH",
            "ACTIONS_ID_TOKEN_REQUEST_TOKEN", "./scripts/performance-policy-audit.sh",
            "./scripts/external-corpus-audit.sh",
        ):
            self.assertIn(value, script)
        self.assertGreaterEqual(script.count('verify_source "$CANDIDATE_SOURCE"'), 3)
        self.assertGreaterEqual(script.count('verify_source "$BASE_SOURCE"'), 3)
        self.assertGreaterEqual(script.count('verify_source "$QUICKJS_SOURCE"'), 2)
        self.assertLess(
            script.index('CURRENT_PHASE="post_measure_validation"'),
            script.index('CURRENT_PHASE="summary"'),
        )
        self.assertNotIn("fetch --no-tags", script)
        self.assertNotIn("third_party/test262", script)
        self.assertNotIn("--threshold", script)

    def test_verify_source_detects_mock_build_dirtying_tracked_source(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            source = Path(directory_name)
            subprocess.run(["git", "init", "-q", str(source)], check=True)
            subprocess.run(["git", "-C", str(source), "config", "user.email", "test@example.com"], check=True)
            subprocess.run(["git", "-C", str(source), "config", "user.name", "Test"], check=True)
            tracked = source / "tracked.txt"
            tracked.write_text("clean\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(source), "add", "tracked.txt"], check=True)
            subprocess.run(["git", "-C", str(source), "commit", "-qm", "fixture"], check=True)
            revision = subprocess.check_output(
                ["git", "-C", str(source), "rev-parse", "HEAD"], text=True
            ).strip()
            args = argparse.Namespace(source=source, revision=revision)
            verify_source(args)
            # Mock a build script that wrongly rewrites tracked source.
            subprocess.run(["sh", "-c", "printf dirty > tracked.txt"], cwd=source, check=True)
            with self.assertRaisesRegex(
                PreviewError, f"{re.escape(str(source))}.*dirty after build"
            ):
                verify_source(args)
            result = subprocess.run(
                [
                    sys.executable, "-m", "tools.benchmark.preview", "verify-source",
                    "--source", str(source), "--revision", revision,
                ],
                cwd=ROOT, capture_output=True, text=True, timeout=10, check=False,
            )
            self.assertEqual(result.returncode, 2)
            self.assertIn(str(source), result.stderr)
            self.assertIn("tracked.txt", result.stderr)

    def test_repository_urls_reject_option_and_markdown_attacks(self) -> None:
        self.assertEqual(
            _github_url("https://github.com/org/repo.git", "repo"),
            "https://github.com/org/repo.git",
        )
        for value in (
            "--upload-pack=attacker", "git@github.com:org/repo.git",
            "https://evil.invalid/org/repo.git", "https://github.com/org/repo",
            "https://github.com/org/repo.git\n--depth=1",
        ):
            with self.assertRaisesRegex(PreviewError, "canonical HTTPS"):
                _github_url(value, "repo")

    def test_dynamic_manifest_and_receipts_bind_actual_inputs(self) -> None:
        template = load_manifest(ROOT / "benchmarks/manifest.json")
        manifest_path = ROOT / "benchmarks" / f".hosted-preview-test-{uuid.uuid4().hex}.json"
        try:
            with tempfile.TemporaryDirectory() as directory_name:
                directory = Path(directory_name)
                binaries = {}
                for role in ("candidate", "base", "quickjs"):
                    path = directory / role
                    path.write_bytes(f"{role}-binary".encode())
                    path.chmod(0o700)
                    binaries[role] = path
                paths = {role: directory / f"{role}-receipt.json" for role in binaries}
                args = argparse.Namespace(
                    template=ROOT / "benchmarks/manifest.json",
                    manifest_output=manifest_path,
                    candidate_binary=binaries["candidate"], base_binary=binaries["base"],
                    quickjs_binary=binaries["quickjs"],
                    candidate_receipt=paths["candidate"], base_receipt=paths["base"],
                    quickjs_receipt=paths["quickjs"],
                    candidate_repo="https://github.com/example/fork.git",
                    candidate_revision="1" * 40,
                    base_repo="https://github.com/example/base.git",
                    base_revision="2" * 40,
                    profile_id="github-hosted-linux-x86_64-informational-v1",
                    platform="Linux-x86_64", rust_toolchain="rustc test; cargo test",
                    rust_target="x86_64-unknown-linux-gnu",
                    quickjs_toolchain="cc test; cmake test; make test",
                    quickjs_target="x86_64-linux-gnu", quickjs_cc="/usr/bin/cc",
                )
                prepare(args)
                dynamic = load_manifest(manifest_path)
                self.assertEqual(dynamic.protocol_sha256, template.protocol_sha256)
                self.assertEqual(len(dynamic.cases), 7)
                rust = dynamic.build_recipes["qjs-rust"]
                self.assertEqual(rust.flags, RUST_BUILD_FLAGS)
                self.assertEqual(rust.lto, "off-forced-by-cargo-config")
                self.assertEqual(
                    rust.allocator, "source-controlled-not-independently-verified"
                )
                quick = dynamic.build_recipes["quickjs-ng"]
                self.assertEqual(quick.flags, ("CC=/usr/bin/cc", "BUILD_TYPE=Release", "all"))
                self.assertEqual(
                    quick.allocator, "source-controlled-not-independently-verified"
                )
                for role, identity, recipe in (
                    ("candidate", "qjs-rust", rust), ("base", "qjs-rust", rust),
                    ("quickjs", "quickjs-ng", quick),
                ):
                    receipt = load_receipt(
                        paths[role], expected_binary_sha256=sha256_file(binaries[role]),
                        expected_engine_identity=identity,
                        expected_profile_id=dynamic.profile.id, expected_recipe=recipe,
                        pinned_reference=(
                            template.reference_identity, template.reference_repo,
                            template.reference_revision,
                        ) if role == "quickjs" else None,
                    )
                    self.assertFalse(receipt.source_dirty)
                data = json.loads(paths["base"].read_text(encoding="utf-8"))
                self.assertEqual(data["source"]["revision"], "2" * 40)
        finally:
            manifest_path.unlink(missing_ok=True)


class HostedPreviewControlTests(unittest.TestCase):
    def _admit(
        self, event_name: str, head: str, base: str, base_sha: str,
        *, base_ref: str = HOSTED_BASE_REF,
        pr_number: int = BOOTSTRAP_PR_NUMBER,
        head_ref: str = BOOTSTRAP_HEAD_REF,
    ) -> dict[str, object]:
        result = subprocess.run(
            [
                sys.executable, "-m", "tools.benchmark.hosted_preview", "admit",
                "--event-name", event_name, "--head-repository", head,
                "--base-repository", base, "--base-sha", base_sha,
                "--base-ref", base_ref, "--pr-number", str(pr_number),
                "--head-ref", head_ref,
            ],
            cwd=ROOT, capture_output=True, text=True, timeout=10, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def test_executable_admission_transition_contract(self) -> None:
        same = "example/quickjs-rust"
        bootstrap = self._admit("pull_request", same, same, BOOTSTRAP_BASE_SHA)
        self.assertTrue(bootstrap["run"])
        self.assertEqual(bootstrap["mode"], BOOTSTRAP_MODE)

        closed = self._admit("pull_request", same, same, "a" * 40)
        self.assertFalse(closed["run"])
        self.assertEqual(closed["reason"], "bootstrap_base_sha_mismatch")

        long_term = self._admit("pull_request_target", same, same, "a" * 40)
        self.assertTrue(long_term["run"])
        self.assertEqual(long_term["mode"], BASE_MODE)

        fork = self._admit("pull_request_target", "fork/repo", same, "a" * 40)
        self.assertFalse(fork["run"])
        self.assertEqual(fork["reason"], "fork_preview_unsupported")

        non_main = self._admit(
            "pull_request_target", same, same, "a" * 40, base_ref="release"
        )
        self.assertFalse(non_main["run"])
        self.assertEqual(non_main["reason"], "base_branch_unsupported")

        wrong_pr = self._admit(
            "pull_request", same, same, BOOTSTRAP_BASE_SHA, pr_number=125
        )
        self.assertFalse(wrong_pr["run"])
        self.assertEqual(wrong_pr["reason"], "bootstrap_pr_number_mismatch")

        wrong_head = self._admit(
            "pull_request", same, same, BOOTSTRAP_BASE_SHA,
            head_ref="agent/performance-benchmark-system/other",
        )
        self.assertFalse(wrong_head["run"])
        self.assertEqual(wrong_head["reason"], "bootstrap_head_ref_mismatch")

    def test_shell_env_passes_hostile_head_ref_as_literal_cli_argument(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            marker = Path(directory_name) / "must-not-exist"
            hostile_ref = f'x"$(touch {marker})"'
            command = " ".join([
                "exec", shlex.quote(sys.executable), "-m",
                "tools.benchmark.hosted_preview", "admit",
                "--event-name", '"$PREVIEW_EVENT_NAME"',
                "--head-repository", '"$PR_HEAD_REPOSITORY"',
                "--base-repository", '"$PR_BASE_REPOSITORY"',
                "--base-sha", '"$PR_BASE_SHA"',
                "--base-ref", '"$PR_BASE_REF"',
                "--pr-number", '"$PR_NUMBER"',
                "--head-ref", '"$PR_HEAD_REF"',
            ])
            environment = {
                "PATH": os.environ["PATH"],
                "PYTHONPATH": str(ROOT),
                "PREVIEW_EVENT_NAME": "pull_request",
                "PR_HEAD_REPOSITORY": "example/repo",
                "PR_BASE_REPOSITORY": "example/repo",
                "PR_BASE_SHA": BOOTSTRAP_BASE_SHA,
                "PR_BASE_REF": HOSTED_BASE_REF,
                "PR_NUMBER": str(BOOTSTRAP_PR_NUMBER),
                "PR_HEAD_REF": hostile_ref,
            }
            result = subprocess.run(
                ["bash", "-c", command], cwd=ROOT, env=environment,
                capture_output=True, text=True, timeout=10, check=False,
            )
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn("head ref", result.stderr)
            self.assertFalse(marker.exists())

    def test_executable_publish_creates_pre_orchestrator_failure_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            root = Path(directory_name)
            output = root / "evidence"
            step_summary = root / "step-summary.md"
            result = subprocess.run(
                [
                    sys.executable, "-m", "tools.benchmark.hosted_preview", "publish",
                    "--output-dir", str(output), "--step-summary", str(step_summary),
                ],
                cwd=ROOT, capture_output=True, text=True, timeout=10, check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            status = json.loads((output / "status.json").read_text())
            self.assertEqual(status["state"], "failed")
            self.assertEqual(status["phase"], "pre_orchestrator")
            self.assertIn("No performance conclusion", (output / "summary.md").read_text())
            self.assertEqual(step_summary.read_bytes(), (output / "summary.md").read_bytes())

    def test_executable_failure_status_preserves_phase(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            output = Path(directory_name) / "evidence"
            result = subprocess.run(
                [
                    sys.executable, "-m", "tools.benchmark.preview", "status",
                    "--state", "failed", "--phase", "build_candidate",
                    "--output-dir", str(output), "--harness-mode", BASE_MODE,
                    "--harness-revision", "a" * 40,
                    "--candidate-revision", "b" * 40,
                    "--base-revision", "c" * 40,
                    "--reference-revision", "d" * 40,
                    "--message", "mock candidate build failed",
                ],
                cwd=ROOT, capture_output=True, text=True, timeout=10, check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            status = json.loads((output / "status.json").read_text())
            self.assertEqual(status["phase"], "build_candidate")
            self.assertIn("build\\_candidate", (output / "summary.md").read_text())

    def test_publisher_upgrades_pre_orchestrator_pending_to_failure(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            root = Path(directory_name)
            output = root / "evidence"
            output.mkdir()
            (output / "status.json").write_text(
                json.dumps({"state": "pending", "phase": "audit"}), encoding="utf-8"
            )
            (output / "summary.md").write_text("pending\n", encoding="utf-8")
            result = subprocess.run(
                [
                    sys.executable, "-m", "tools.benchmark.hosted_preview", "publish",
                    "--output-dir", str(output),
                    "--step-summary", str(root / "step-summary.md"),
                    "--job-status", "failure",
                ],
                cwd=ROOT, capture_output=True, text=True, timeout=10, check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            status = json.loads((output / "status.json").read_text())
            self.assertEqual(status["state"], "failed")
            self.assertEqual(status["phase"], "audit")


if __name__ == "__main__":
    unittest.main()
