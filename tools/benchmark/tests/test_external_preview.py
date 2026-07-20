from __future__ import annotations

import copy
import io
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark.external_preview import (
    Case,
    ExternalPreviewError,
    Manifest,
    Measurement,
    Source,
    SourceFile,
    Suite,
    _raw_url,
    load_manifest,
    run_preview,
)


ROOT = Path(__file__).resolve().parents[3]
MANIFEST = ROOT / "benchmarks/external-preview.json"


class ExternalPreviewTests(unittest.TestCase):
    def setUp(self) -> None:
        self.data = json.loads(MANIFEST.read_text(encoding="utf-8"))

    def _write(self, directory: Path, data: object) -> Path:
        path = directory / "external.json"
        path.write_text(json.dumps(data, sort_keys=True) + "\n", encoding="utf-8")
        return path

    def test_exact_three_suite_inventory_is_non_claim(self) -> None:
        manifest = load_manifest(MANIFEST)
        self.assertEqual(manifest.preview_id, "quickjs-authoritative-external-preview-v2")
        self.assertEqual(
            {suite.id: len(suite.cases) for suite in manifest.suites},
            {
                "jetstream3-js-subset": 5,
                "kraken-1.1": 14,
                "sunspider-1.0": 26,
            },
        )
        self.assertEqual(manifest.measurement.blocks, 3)
        self.assertEqual(manifest.measurement.metric, "outer_process_wall_time")
        self.assertEqual(manifest.measurement.order, "seeded_role_rotation")
        self.assertEqual(
            sum(len(case.files) for suite in manifest.suites for case in suite.cases),
            71,
        )
        self.assertIn("never an official JetStream score", manifest.suites[0].reporting_rule)

    def test_manifest_rejects_claims_unknown_fields_and_unsafe_sources(self) -> None:
        mutations: list[tuple[dict[str, object], str]] = []
        claim = copy.deepcopy(self.data)
        claim["claim_eligible"] = True
        mutations.append((claim, "must remain false"))
        unknown = copy.deepcopy(self.data)
        unknown["future"] = True
        mutations.append((unknown, "unknown or missing"))
        unsafe = copy.deepcopy(self.data)
        unsafe["suites"][0]["cases"][0]["files"][0]["path"] = "../escape.js"
        mutations.append((unsafe, "repository-relative"))
        digest = copy.deepcopy(self.data)
        digest["suites"][0]["cases"][0]["files"][0]["sha256"] = "0" * 64
        mutations.append((digest, "invalid SHA-256"))
        redistribution = copy.deepcopy(self.data)
        redistribution["suites"][0]["license_review"][
            "redistribute_source_in_artifact"
        ] = True
        mutations.append((redistribution, "redistribution must remain disabled"))
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, (data, message) in enumerate(mutations):
                with self.subTest(index=index):
                    with self.assertRaisesRegex(ExternalPreviewError, message):
                        load_manifest(self._write(directory, data))

    def test_duplicate_json_keys_and_invalid_utf8_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            duplicate = directory / "duplicate.json"
            duplicate.write_text(
                MANIFEST.read_text(encoding="utf-8").replace(
                    '"schema_version": 2,',
                    '"schema_version": 2,\n  "schema_version": 2,',
                    1,
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ExternalPreviewError, "duplicate key"):
                load_manifest(duplicate)
            invalid = directory / "invalid.json"
            invalid.write_bytes(b"{\xff}")
            with self.assertRaisesRegex(ExternalPreviewError, "cannot read"):
                load_manifest(invalid)

    def test_raw_url_is_revision_and_path_bound(self) -> None:
        manifest = load_manifest(MANIFEST)
        suite = manifest.suites[0]
        source_file = suite.cases[0].files[0]
        self.assertEqual(
            _raw_url(suite.source, source_file),
            "https://raw.githubusercontent.com/WebKit/JetStream/"
            "b7babdf323e64e69bd2f6c376189c15825f5c73a/cdjs/constants.js",
        )

    def test_fake_three_engine_run_emits_same_run_base_report_and_removes_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            cache = directory / "cache"
            source = b"var externalValue = 1;\n"
            import hashlib

            digest = hashlib.sha256(source).hexdigest()
            source_path = cache / "suite" / "case.js"
            source_path.parent.mkdir(parents=True)
            source_path.write_bytes(source)
            measurement = Measurement(
                blocks=1,
                timeout_seconds=10,
                metric="outer_process_wall_time",
                phase_boundary="before_process_spawn_to_wait_return",
                order="seeded_role_rotation",
                seed=7,
            )
            case = Case("case", (SourceFile("case.js", digest),), "direct-script", None)
            suite = Suite(
                "suite",
                "Suite",
                "Per-case only.",
                Source("https://github.com/example/suite.git", "a" * 40, ""),
                (case,),
            )
            manifest_file = directory / "manifest.json"
            manifest_file.write_text("{}\n", encoding="utf-8")
            manifest = Manifest(
                manifest_file,
                hashlib.sha256(b"{}\n").hexdigest(),
                "quickjs-authoritative-external-preview-v2",
                measurement,
                (suite,),
            )
            engine = directory / "engine.sh"
            engine.write_text("#!/bin/sh\nprintf '%s\\n' '__QJS_EXTERNAL_OK__'\n", encoding="utf-8")
            engine.chmod(0o755)
            output = directory / "evidence"
            work = directory / "work"
            with patch(
                "tools.benchmark.external_preview.fetch_corpora",
                return_value={"downloaded": 0, "reused": 1},
            ), patch("sys.stderr", new_callable=io.StringIO) as progress:
                report = run_preview(
                    manifest, cache, work, output, engine, engine, engine,
                    blocks=3, timeout_seconds=10,
                )
            progress_lines = progress.getvalue().splitlines()
            self.assertEqual(len(progress_lines), 12)
            self.assertTrue(
                any(
                    line
                    == "external-preview: suite=suite case=case "
                    "phase=capability role=candidate"
                    for line in progress_lines
                )
            )
            self.assertTrue(
                any(
                    line
                    == "external-preview: suite=suite case=case "
                    "phase=measurement block=3/3 role=quickjs-ng"
                    for line in progress_lines
                )
            )
            self.assertFalse(report["claim_eligible"])
            self.assertEqual(report["suites"][0]["comparable_case_count"], 1)
            self.assertEqual(report["suites"][0]["base_comparable_case_count"], 1)
            self.assertGreater(
                report["suites"][0]["cases"][0]["candidate_over_base"], 0
            )
            self.assertEqual(report["roles"], ["candidate", "base", "quickjs-ng"])
            records = [
                json.loads(line)
                for line in (output / "external-raw.jsonl").read_text().splitlines()
            ]
            measurements = [row for row in records if row["phase"] == "measurement"]
            self.assertEqual(len(measurements), 9)
            self.assertEqual(
                {
                    role: {row["order"] for row in measurements if row["role"] == role}
                    for role in ("candidate", "base", "quickjs-ng")
                },
                {role: {0, 1, 2} for role in ("candidate", "base", "quickjs-ng")},
            )
            self.assertIsNone(report["suites"][0]["official_suite_score"])
            self.assertFalse((work / "bundles").exists())
            self.assertFalse((work / "engine-snapshots").exists())
            self.assertNotIn("externalValue", (output / "external-raw.jsonl").read_text())
            markdown = (output / "external-summary.md").read_text()
            self.assertIn("Informational only", markdown)
            self.assertIn("### External per-case performance", markdown)
            self.assertIn("| `suite/case` |", markdown)
            self.assertIn("Candidate ms/run", markdown)
            self.assertIn("Candidate/base", markdown)

    def test_checked_in_cli_audit_works_outside_repository(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            completed = subprocess.run(
                [
                    os.fspath(ROOT / "scripts/external-performance-preview.sh"),
                    "--manifest",
                    os.fspath(MANIFEST),
                    "audit",
                ],
                cwd=directory_name,
                capture_output=True,
                text=True,
                check=False,
            )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        payload = json.loads(completed.stdout)
        self.assertEqual(payload["suite_count"], 3)
        self.assertEqual(payload["case_count"], 45)
        self.assertFalse(payload["claim_eligible"])


if __name__ == "__main__":
    unittest.main()
