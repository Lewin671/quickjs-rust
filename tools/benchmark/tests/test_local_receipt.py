from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest import mock

from tools.benchmark import local_receipt
from tools.benchmark.receipts import load_receipt
from tools.benchmark.schema import load_manifest

ROOT = Path(__file__).resolve().parents[3]
MANIFEST = ROOT / "benchmarks/manifest.json"


class LocalReceiptTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.binary = Path(temporary.name) / "qjs"
        self.binary.write_bytes(b"engine")
        self.manifest = load_manifest(MANIFEST)
        self.recipe = self.manifest.build_recipes["qjs-rust"]

    def test_rust_receipt_round_trips_through_the_strict_loader(self):
        with mock.patch.object(local_receipt, "rust_toolchain", return_value=self.recipe.toolchain):
            content = local_receipt.receipt("qjs-rust", "a" * 40, self.binary, MANIFEST)
        path = self.binary.with_suffix(".json")
        import json
        path.write_text(json.dumps(content))
        import hashlib
        loaded = load_receipt(
            path, expected_binary_sha256=hashlib.sha256(b"engine").hexdigest(),
            expected_engine_identity="qjs-rust", expected_profile_id=self.manifest.profile.id,
            expected_recipe=self.recipe)
        self.assertFalse(loaded.source_dirty)

    def test_a_different_toolchain_is_refused(self):
        with mock.patch.object(local_receipt, "rust_toolchain", return_value="rustc 0.0.0"):
            with self.assertRaisesRegex(local_receipt.LocalReceiptError, "differs from the recipe"):
                local_receipt.receipt("qjs-rust", "a" * 40, self.binary, MANIFEST)

    def test_short_revisions_are_refused(self):
        with mock.patch.object(local_receipt, "rust_toolchain", return_value=self.recipe.toolchain):
            with self.assertRaisesRegex(local_receipt.LocalReceiptError, "full lowercase"):
                local_receipt.receipt("qjs-rust", "abc123", self.binary, MANIFEST)

    def test_reference_receipt_must_name_the_pin(self):
        def fake_run(argv, cwd=None):
            return self.manifest.reference_revision if "rev-parse" in argv else ""
        with mock.patch.object(local_receipt, "_run", side_effect=fake_run):
            with self.assertRaisesRegex(local_receipt.LocalReceiptError, "pinned revision"):
                local_receipt.receipt("quickjs-ng", "b" * 40, self.binary, MANIFEST)
            content = local_receipt.receipt("quickjs-ng", self.manifest.reference_revision,
                                            self.binary, MANIFEST)
        self.assertEqual(content["source"]["repo"], self.manifest.reference_repo)

    def test_modified_reference_checkout_is_refused(self):
        def fake_run(argv, cwd=None):
            return self.manifest.reference_revision if "rev-parse" in argv else " M quickjs.c"
        with mock.patch.object(local_receipt, "_run", side_effect=fake_run):
            with self.assertRaisesRegex(local_receipt.LocalReceiptError, "local modifications"):
                local_receipt.receipt("quickjs-ng", self.manifest.reference_revision,
                                      self.binary, MANIFEST)


if __name__ == "__main__":
    unittest.main()
