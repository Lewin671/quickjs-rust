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
    HOSTED_CASES,
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
    HOSTED_BASE_REF,
    HOSTED_PUSH_REF,
    PUSH_MODE,
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
            "roles": 3, "cases": len(HOSTED_CASES), "blocks": 3,
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

    def test_main_push_summary_binds_harness_candidate_and_base_revisions(self) -> None:
        markdown, machine = summarize(
            report(), harness_mode=PUSH_MODE, harness_revision="1" * 40
        )
        self.assertEqual(machine["harness"], {
            "mode": PUSH_MODE,
            "revision": "1" * 40,
        })
        self.assertEqual(machine["engines"]["candidate"]["source_revision"], "1" * 40)
        self.assertEqual(machine["engines"]["base"]["source_revision"], "2" * 40)
        self.assertEqual(machine["integrity_scope"], "trusted_main_push")
        self.assertIn(PUSH_MODE, markdown)

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

    def test_complete_linearity_failure_is_successfully_inconclusive(self) -> None:
        value_report = report()
        value_report["health"]["status"] = "invalid"
        value_report["health"]["linearity"]["status"] = "fail"
        markdown, machine = summarize_report(value_report)
        self.assertEqual(machine["state"], "success")
        self.assertEqual(
            machine["classification"],
            "informational_measurement_inconclusive_not_fixed_hardware_claim",
        )
        self.assertEqual(machine["health"], "invalid")
        self.assertEqual(machine["linearity"], "fail")
        self.assertEqual(machine["comparisons"], {})
        self.assertIn("No performance direction is reported", markdown)
        self.assertNotIn("Overall ratio", markdown)
        self.assertNotIn("candidate vs", markdown)

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
            BASE_MODE, PUSH_MODE, "verify-source",
            "CARGO_ENCODED_RUSTFLAGS", "--kind rust --field cargo_args",
            'make -C "$QUICKJS_SOURCE" "CC=$QUICKJS_CC" "${QUICKJS_MAKE_ARGS[@]}"',
            "tools.benchmark.build_cache plan", "tools.benchmark.build_cache materialize",
            "tools.benchmark.build_cache store", "build-cache.json",
            "--manifest \"$MANIFEST\" --blocks 3", "--candidate-receipt",
            "--base-receipt", "--quickjs-ng-receipt", "--state pending",
            "--state failed", "--status-output", 'cp "$MANIFEST" "$OUTPUT/manifest.json"',
            "trap record_error ERR", 'CURRENT_PHASE="build_candidate"',
            'CURRENT_PHASE="build_base"', 'CURRENT_PHASE="build_quickjs_ng"',
            'CURRENT_PHASE="measurement"', 'CURRENT_PHASE="summary"',
            'CURRENT_PHASE="external_corpus_preview"',
            'CURRENT_PHASE="post_measure_validation"', "GITHUB_ENV GITHUB_PATH",
            "ACTIONS_ID_TOKEN_REQUEST_TOKEN", "./scripts/performance-policy-audit.sh",
            "./scripts/external-corpus-audit.sh", "./scripts/external-performance-preview.sh",
            'if [ "$HARNESS_MODE" = "main_push_head_owned_harness" ]',
            'cat "$OUTPUT/external-summary.md" >> "$OUTPUT/summary.md"',
        ):
            self.assertIn(value, script)
        self.assertGreaterEqual(script.count('verify_source "$CANDIDATE_SOURCE"'), 3)
        self.assertGreaterEqual(script.count('verify_source "$BASE_SOURCE"'), 3)
        self.assertGreaterEqual(script.count('verify_source "$QUICKJS_SOURCE"'), 2)
        for build_marker, verify_marker, store_marker in (
            (
                'build_rust "$CANDIDATE_SOURCE"',
                'verify_source "$CANDIDATE_SOURCE" "$CANDIDATE_REVISION"',
                'store_and_materialize_cache "$CANDIDATE_CACHE_ENTRY"',
            ),
            (
                'build_rust "$BASE_SOURCE"',
                'verify_source "$BASE_SOURCE" "$BASE_REVISION"',
                'store_and_materialize_cache "$BASE_CACHE_ENTRY"',
            ),
            (
                'make -C "$QUICKJS_SOURCE"',
                'verify_source "$QUICKJS_SOURCE" "$REFERENCE_REVISION"',
                'store_and_materialize_cache "$QUICKJS_CACHE_ENTRY"',
            ),
        ):
            build_index = script.index(build_marker, script.index("miss/rebuild"))
            self.assertLess(
                script.index(verify_marker, build_index),
                script.index(store_marker, build_index),
            )
        self.assertLess(
            script.index('CURRENT_PHASE="post_measure_validation"'),
            script.index('CURRENT_PHASE="summary"'),
        )
        self.assertLess(
            script.index('CURRENT_PHASE="summary"'),
            script.index('CURRENT_PHASE="external_corpus_preview"'),
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
                self.assertEqual(len(dynamic.cases), 25)
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
        pr_number: int = 126,
        head_ref: str = "feature/performance-change",
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

    def _admit_push(
        self, *, event_name: str = "push", repository: str = "example/quickjs-rust",
        event_repository: str = "example/quickjs-rust", ref: str = HOSTED_PUSH_REF,
        before_sha: str = "1" * 40, after_sha: str = "2" * 40,
        workflow_sha: str = "2" * 40,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable, "-m", "tools.benchmark.hosted_preview", "admit-push",
                "--event-name", event_name, "--repository", repository,
                "--event-repository", event_repository, "--ref", ref,
                "--before-sha", before_sha, "--after-sha", after_sha,
                "--workflow-sha", workflow_sha, "--require-mode", PUSH_MODE,
            ],
            cwd=ROOT, capture_output=True, text=True, timeout=10, check=False,
        )

    def test_executable_pr_admission_contract(self) -> None:
        same = "example/quickjs-rust"
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

    def test_main_push_admission_succeeds_and_fail_closed_inputs_fail(self) -> None:
        admitted = self._admit_push()
        self.assertEqual(admitted.returncode, 0, admitted.stderr)
        payload = json.loads(admitted.stdout)
        self.assertTrue(payload["run"])
        self.assertEqual(payload["mode"], PUSH_MODE)
        for name, arguments, reason in (
            ("ref", {"ref": "refs/heads/release"}, "push_ref_unsupported"),
            ("repository", {"event_repository": "other/repo"}, "push_repository_mismatch"),
            ("zero-before", {"before_sha": "0" * 40}, "zero_before_sha"),
            ("zero-after", {"after_sha": "0" * 40}, "zero_after_sha"),
            ("workflow-sha", {"workflow_sha": "3" * 40}, "workflow_sha_mismatch"),
            (
                "unchanged",
                {"before_sha": "2" * 40, "after_sha": "2" * 40,
                 "workflow_sha": "2" * 40},
                "unchanged_push_sha",
            ),
        ):
            with self.subTest(name=name):
                rejected = self._admit_push(**arguments)
                self.assertEqual(rejected.returncode, 2)
                self.assertIn(reason, rejected.stderr)

    def test_main_push_rejects_wrong_event_and_malformed_sha(self) -> None:
        wrong_event = self._admit_push(event_name="pull_request_target")
        self.assertEqual(wrong_event.returncode, 2)
        self.assertIn("event name: expected push", wrong_event.stderr)
        for name, arguments, label in (
            ("before", {"before_sha": "not-a-sha"}, "before SHA"),
            ("after", {"after_sha": "not-a-sha"}, "after SHA"),
            ("workflow", {"workflow_sha": "not-a-sha"}, "workflow SHA"),
        ):
            with self.subTest(name=name):
                malformed = self._admit_push(**arguments)
                self.assertEqual(malformed.returncode, 2)
                self.assertIn(label, malformed.stderr)

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
                "PREVIEW_EVENT_NAME": "pull_request_target",
                "PR_HEAD_REPOSITORY": "example/repo",
                "PR_BASE_REPOSITORY": "example/repo",
                "PR_BASE_SHA": "a" * 40,
                "PR_BASE_REF": HOSTED_BASE_REF,
                "PR_NUMBER": "126",
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
