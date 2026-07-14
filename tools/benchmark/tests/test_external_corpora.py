from __future__ import annotations

import copy
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.external_corpora import (
    ExternalCorpusError,
    load_registry,
    registry_summary,
    require_admitted,
)


ROOT = Path(__file__).resolve().parents[3]
REGISTRY = ROOT / "benchmarks/external-corpora.json"


class ExternalCorpusRegistryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.data = json.loads(REGISTRY.read_text(encoding="utf-8"))

    def _write(self, directory: Path, data: object | None = None) -> Path:
        path = directory / "registry.json"
        path.write_text(
            json.dumps(self.data if data is None else data, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        return path

    def test_exact_v1_inventory_has_zero_admitted_corpora(self) -> None:
        registry = load_registry(REGISTRY)
        expected = {
            "jetstream3": ("b7babdf323e64e69bd2f6c376189c15825f5c73a", "blocked"),
            "kraken-1.1": ("0112a237e0f3f87a465fd2d04ac9674c27595d76", "blocked"),
            "octane": (None, "excluded"),
            "quickjs-ng-bench-v8": ("147c9c51efd6ba36ef5e80214946d21f81856bf8", "blocked"),
            "quickjs-ng-web-tooling": ("a0b6a350b8840a16ea4d6cff58b0345eed934652", "blocked"),
            "speedometer": ("d580beea58cb88eec37404324c4fe58832255730", "excluded"),
            "sunspider-1.0": ("c3b0e52db623f0dea86b977f08a9ba766770cf8d", "blocked"),
        }
        self.assertEqual([corpus.id for corpus in registry.corpora], sorted(expected))
        for corpus in registry.corpora:
            prefix, decision = expected[corpus.id]
            self.assertEqual(corpus.decision, decision)
            if prefix is None:
                self.assertIsNone(corpus.revision)
            else:
                self.assertEqual(corpus.revision, prefix)
        self.assertEqual(
            registry_summary(registry),
            {
                "admitted_count": 0,
                "blocked_count": 5,
                "claim_eligible": False,
                "corpus_count": 7,
                "excluded_count": 2,
                "registry_id": "quickjs-external-corpora-v1",
                "registry_sha256": registry.sha256,
                "schema_version": 1,
            },
        )

    def test_duplicate_and_unknown_keys_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            duplicate = REGISTRY.read_text(encoding="utf-8").replace(
                '"schema_version": 1,',
                '"schema_version": 1,\n  "schema_version": 1,',
                1,
            )
            duplicate_path = directory / "duplicate.json"
            duplicate_path.write_text(duplicate, encoding="utf-8")
            with self.assertRaisesRegex(ExternalCorpusError, "duplicate key"):
                load_registry(duplicate_path)

            unknown = copy.deepcopy(self.data)
            unknown["corpora"][0]["future"] = True
            with self.assertRaisesRegex(ExternalCorpusError, "unknown"):
                load_registry(self._write(directory, unknown))

    def test_scalar_types_revision_urls_enums_hashes_and_ids_are_strict(self) -> None:
        mutations = []

        wrong_version = copy.deepcopy(self.data)
        wrong_version["schema_version"] = True
        mutations.append((wrong_version, "integer version 1"))

        float_version = copy.deepcopy(self.data)
        float_version["schema_version"] = 1.0
        mutations.append((float_version, "integer version 1"))

        wrong_claim = copy.deepcopy(self.data)
        wrong_claim["claim_eligible"] = 0
        mutations.append((wrong_claim, "expected a boolean"))

        wrong_revision = copy.deepcopy(self.data)
        wrong_revision["corpora"][0]["source"]["revision"] = "b7babdf3"
        mutations.append((wrong_revision, "full lowercase git SHA"))

        wrong_url = copy.deepcopy(self.data)
        wrong_url["corpora"][0]["source"]["url"] = "http://example.com/corpus"
        mutations.append((wrong_url, "expected an HTTPS URL"))

        wrong_enum = copy.deepcopy(self.data)
        wrong_enum["corpora"][0]["decision"] = "candidate"
        mutations.append((wrong_enum, "expected one of"))

        wrong_id = copy.deepcopy(self.data)
        wrong_id["corpora"][0]["id"] = "Jet Stream"
        mutations.append((wrong_id, "stable lowercase token id"))

        wrong_hash = copy.deepcopy(self.data)
        notice = wrong_hash["corpora"][-1]["license_audit"]["notice"]
        notice.update({
            "status": "complete",
            "sha256": "not-a-sha256",
            "evidence_urls": ["https://webkit.org/NOTICE"],
        })
        mutations.append((wrong_hash, "expected lowercase SHA-256"))

        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, (data, message) in enumerate(mutations):
                with self.subTest(index=index, message=message):
                    path = directory / f"mutation-{index}.json"
                    path.write_text(json.dumps(data), encoding="utf-8")
                    with self.assertRaisesRegex(ExternalCorpusError, message):
                        load_registry(path)

    def test_invalid_utf8_and_registry_identity_are_wrapped(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            invalid_utf8 = directory / "invalid-utf8.json"
            invalid_utf8.write_bytes(b"{\xff}")
            with self.assertRaisesRegex(ExternalCorpusError, "cannot read"):
                load_registry(invalid_utf8)

            wrong_id = copy.deepcopy(self.data)
            wrong_id["registry_id"] = "quickjs-external-corpora-v2"
            with self.assertRaisesRegex(ExternalCorpusError, "version 1 requires"):
                load_registry(self._write(directory, wrong_id))

    def test_duplicate_ids_names_cases_and_unsorted_ids_are_rejected(self) -> None:
        mutations = []
        duplicate_id = copy.deepcopy(self.data)
        duplicate_id["corpora"][1]["id"] = duplicate_id["corpora"][0]["id"]
        mutations.append((duplicate_id, "ids must be unique and sorted"))

        duplicate_name = copy.deepcopy(self.data)
        duplicate_name["corpora"][1]["name"] = duplicate_name["corpora"][0]["name"]
        mutations.append((duplicate_name, "names must be unique"))

        unsorted = copy.deepcopy(self.data)
        unsorted["corpora"][0], unsorted["corpora"][1] = (
            unsorted["corpora"][1], unsorted["corpora"][0]
        )
        mutations.append((unsorted, "ids must be unique and sorted"))

        duplicate_cases = copy.deepcopy(self.data)
        duplicate_cases["corpora"][-1]["expected_cases"]["cases"] = ["x", "x"]
        mutations.append((duplicate_cases, "values must be unique"))

        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, (data, message) in enumerate(mutations):
                with self.subTest(index=index):
                    path = directory / f"duplicate-{index}.json"
                    path.write_text(json.dumps(data), encoding="utf-8")
                    with self.assertRaisesRegex(ExternalCorpusError, message):
                        load_registry(path)

    def test_blocked_entries_require_pin_and_blockers(self) -> None:
        mutations = []
        no_pin = copy.deepcopy(self.data)
        no_pin["corpora"][0]["source"]["revision"] = None
        mutations.append((no_pin, "blocked corpus requires a full pin"))
        no_blocker = copy.deepcopy(self.data)
        no_blocker["corpora"][0]["blockers"] = []
        mutations.append((no_blocker, "requires at least one blocker"))
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, (data, message) in enumerate(mutations):
                path = directory / f"blocked-{index}.json"
                path.write_text(json.dumps(data), encoding="utf-8")
                with self.assertRaisesRegex(ExternalCorpusError, message):
                    load_registry(path)

    def test_v1_is_deny_only_and_rejects_placeholder_trust_material(self) -> None:
        mutations = []
        fake_admitted = copy.deepcopy(self.data)
        fake_admitted["corpora"][-1]["decision"] = "admitted"
        mutations.append((fake_admitted, "expected one of"))

        zero_revision = copy.deepcopy(self.data)
        zero_revision["corpora"][-1]["source"]["revision"] = "0" * 40
        mutations.append((zero_revision, "all-zero git SHA"))

        zero_notice = copy.deepcopy(self.data)
        zero_notice["corpora"][-1]["license_audit"]["notice"] = {
            "status": "complete",
            "sha256": "0" * 64,
            "evidence_urls": ["https://webkit.org/NOTICE"],
        }
        mutations.append((zero_notice, "all-zero SHA-256"))

        example_source = copy.deepcopy(self.data)
        example_source["corpora"][-1]["source"]["url"] = "https://example.com/corpus"
        mutations.append((example_source, "placeholder example hostname"))

        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, (data, message) in enumerate(mutations):
                path = directory / f"deny-only-{index}.json"
                path.write_text(json.dumps(data), encoding="utf-8")
                with self.subTest(index=index):
                    with self.assertRaisesRegex(ExternalCorpusError, message):
                        load_registry(path)

    def test_url_authority_attacks_are_wrapped_as_registry_errors(self) -> None:
        attacks = (
            "https://host.invalid:notaport/path",
            "https://host.invalid:99999/path",
            "https://host space.invalid/path",
            "https://host.invalid/line\nbreak",
            "https://[::1/path",
            "https://bad_host.invalid/path",
            "https://host.invalid/\x01path",
        )
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, url in enumerate(attacks):
                data = copy.deepcopy(self.data)
                data["corpora"][-1]["source"]["url"] = url
                path = directory / f"url-{index}.json"
                path.write_text(json.dumps(data), encoding="utf-8")
                with self.subTest(url=url):
                    with self.assertRaises(ExternalCorpusError):
                        load_registry(path)

    def test_blocked_and_excluded_corpora_can_never_be_required(self) -> None:
        registry = load_registry(REGISTRY)
        for corpus in registry.corpora:
            with self.subTest(corpus=corpus.id):
                with self.assertRaisesRegex(ExternalCorpusError, "not admitted"):
                    require_admitted(registry, corpus.id)


class ExternalCorpusCliTests(unittest.TestCase):
    def test_wrapper_is_repo_root_independent_and_output_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            result = subprocess.run(
                [str(ROOT / "scripts/external-corpus-audit.sh")],
                cwd=directory_name,
                capture_output=True,
                text=True,
                timeout=10,
                check=False,
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        summary = json.loads(result.stdout)
        self.assertEqual(summary["corpus_count"], 7)
        self.assertEqual(summary["admitted_count"], 0)
        self.assertFalse(summary["claim_eligible"])
        self.assertNotIn(str(ROOT), result.stdout)
        self.assertEqual(
            result.stdout,
            json.dumps(summary, sort_keys=True, separators=(",", ":")) + "\n",
        )

    def test_default_trust_root_require_admitted_fails_structurally(self) -> None:
        corpus_ids = [
            corpus["id"]
            for corpus in json.loads(REGISTRY.read_text(encoding="utf-8"))["corpora"]
        ]
        with tempfile.TemporaryDirectory() as directory_name:
            for corpus_id in corpus_ids:
                result = subprocess.run(
                    [
                        str(ROOT / "scripts/external-corpus-audit.sh"),
                        "--require-admitted", corpus_id,
                    ],
                    cwd=directory_name,
                    capture_output=True,
                    text=True,
                    timeout=10,
                    check=False,
                )
                with self.subTest(corpus_id=corpus_id):
                    self.assertEqual(result.returncode, 2)
                    error = json.loads(result.stderr)["error"]
                    self.assertEqual(error["code"], "external_corpus_audit_failed")
                    self.assertIn("not admitted", error["message"])
                    self.assertNotIn("Traceback", result.stderr)

    def test_custom_registry_cannot_be_combined_with_require_admitted(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            result = subprocess.run(
                [
                    str(ROOT / "scripts/external-corpus-audit.sh"),
                    "--registry", str(REGISTRY),
                    "--require-admitted", "sunspider-1.0",
                ],
                cwd=directory_name,
                capture_output=True,
                text=True,
                timeout=10,
                check=False,
            )
        self.assertEqual(result.returncode, 2)
        error = json.loads(result.stderr)["error"]
        self.assertEqual(error["code"], "external_corpus_audit_failed")
        self.assertIn("not allowed with argument", error["message"])
        self.assertNotIn("Traceback", result.stderr)

    def test_invalid_custom_trust_material_always_exits_two(self) -> None:
        base = json.loads(REGISTRY.read_text(encoding="utf-8"))
        mutations = []
        admitted = copy.deepcopy(base)
        admitted["corpora"][-1]["decision"] = "admitted"
        mutations.append(admitted)
        zero_sha = copy.deepcopy(base)
        zero_sha["corpora"][-1]["source"]["revision"] = "0" * 40
        mutations.append(zero_sha)
        example_url = copy.deepcopy(base)
        example_url["corpora"][-1]["evidence_urls"] = ["https://example.org/audit"]
        mutations.append(example_url)

        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            for index, data in enumerate(mutations):
                registry = directory / f"invalid-{index}.json"
                registry.write_text(json.dumps(data), encoding="utf-8")
                result = subprocess.run(
                    [
                        str(ROOT / "scripts/external-corpus-audit.sh"),
                        "--registry", str(registry),
                    ],
                    cwd=directory,
                    capture_output=True,
                    text=True,
                    timeout=10,
                    check=False,
                )
                with self.subTest(index=index):
                    self.assertEqual(result.returncode, 2)
                    self.assertEqual(
                        json.loads(result.stderr)["error"]["code"],
                        "external_corpus_audit_failed",
                    )
                    self.assertNotIn("Traceback", result.stderr)

    def test_argparse_errors_are_structured_without_tracebacks(self) -> None:
        attacks = (("--unknown",), ("--registry",), ("--require-admitted",))
        with tempfile.TemporaryDirectory() as directory_name:
            for attack in attacks:
                result = subprocess.run(
                    [str(ROOT / "scripts/external-corpus-audit.sh"), *attack],
                    cwd=directory_name,
                    capture_output=True,
                    text=True,
                    timeout=10,
                    check=False,
                )
                with self.subTest(attack=attack):
                    self.assertEqual(result.returncode, 2)
                    error = json.loads(result.stderr)["error"]
                    self.assertEqual(error["code"], "external_corpus_audit_failed")
                    self.assertIn("arguments:", error["message"])
                    self.assertNotIn("usage:", result.stderr)
                    self.assertNotIn("Traceback", result.stderr)

    def test_output_is_atomic_and_never_overwrites(self) -> None:
        with tempfile.TemporaryDirectory() as directory_name:
            directory = Path(directory_name)
            output = directory / "audit.json"
            command = [
                str(ROOT / "scripts/external-corpus-audit.sh"),
                "--registry", str(REGISTRY), "--output", str(output),
            ]
            first = subprocess.run(
                command, cwd=directory, capture_output=True, text=True,
                timeout=10, check=False,
            )
            self.assertEqual(first.returncode, 0, first.stderr)
            summary = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(summary["admitted_count"], 0)
            original = output.read_bytes()

            second = subprocess.run(
                command, cwd=directory, capture_output=True, text=True,
                timeout=10, check=False,
            )
            self.assertEqual(second.returncode, 2)
            self.assertEqual(output.read_bytes(), original)
            self.assertIn("refusing to overwrite", second.stderr)
            self.assertNotIn("Traceback", second.stderr)
            self.assertEqual(list(directory.glob(".audit.json.*")), [])


if __name__ == "__main__":
    unittest.main()
