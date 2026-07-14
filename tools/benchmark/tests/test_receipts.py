from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.receipts import (
    ReceiptError,
    canonical_receipt_sha256,
    load_receipt,
)
from tools.benchmark.schema import BuildRecipe


class ReceiptTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.binary = self.root / "qjs"
        self.binary.write_bytes(b"engine")
        self.binary_hash = hashlib.sha256(b"engine").hexdigest()
        self.data = {
            "schema_version": 1,
            "engine_identity": "qjs-rust",
            "source": {
                "repo": "https://example.invalid/qjs-rust.git",
                "revision": "a" * 40,
                "dirty": False,
            },
            "profile_id": "release-native-v1",
            "build": {
                "build_mode": "release",
                "toolchain": "rustc 1.95.0",
                "target": "aarch64-apple-darwin",
                "features": [],
                "flags": [],
                "lto": "workspace-default",
                "strip": "unstripped",
                "allocator": "system",
                "host_features": "target-default",
            },
            "binary_sha256": self.binary_hash,
        }
        self.receipt = self.root / "receipt.json"
        self.recipe = BuildRecipe(
            engine_identity="qjs-rust",
            build_mode="release",
            toolchain="rustc 1.95.0",
            target="aarch64-apple-darwin",
            features=(),
            flags=(),
            lto="workspace-default",
            strip="unstripped",
            allocator="system",
            host_features="target-default",
        )

    def _write(self) -> None:
        self.receipt.write_text(json.dumps(self.data), encoding="utf-8")

    def _load(self):
        return load_receipt(
            self.receipt,
            expected_binary_sha256=self.binary_hash,
            expected_engine_identity="qjs-rust",
            expected_profile_id="release-native-v1",
            expected_recipe=self.recipe,
        )

    def test_valid_receipt_is_bound_to_binary_and_profile(self) -> None:
        self._write()
        receipt = self._load()
        self.assertEqual(receipt.binary_sha256, self.binary_hash)
        self.assertFalse(receipt.source_dirty)
        self.assertEqual(receipt.sha256, canonical_receipt_sha256(self.data))

    def test_digest_is_canonical_across_json_formatting(self) -> None:
        self.receipt.write_text(
            json.dumps(self.data, indent=4, ensure_ascii=False), encoding="utf-8"
        )
        pretty_digest = self._load().sha256
        self.receipt.write_text(
            json.dumps(self.data, sort_keys=True, separators=(",", ":")),
            encoding="utf-8",
        )
        compact_digest = self._load().sha256
        self.assertEqual(pretty_digest, compact_digest)
        self.assertEqual(compact_digest, canonical_receipt_sha256(self.data))

    def test_unknown_duplicate_and_boolean_schema_fail_closed(self) -> None:
        self.data["unknown"] = 1
        self._write()
        with self.assertRaisesRegex(ReceiptError, "expected fields"):
            self._load()
        self.data.pop("unknown")
        encoded = json.dumps(self.data).replace(
            '{"schema_version": 1,', '{"schema_version": 1, "schema_version": 1,'
        )
        self.receipt.write_text(encoded, encoding="utf-8")
        with self.assertRaisesRegex(ReceiptError, "duplicate key"):
            self._load()
        self.data["schema_version"] = True
        self._write()
        with self.assertRaisesRegex(ReceiptError, "integer version"):
            self._load()
        self.receipt.write_text(
            json.dumps(self.data).replace("true", "NaN", 1), encoding="utf-8"
        )
        with self.assertRaisesRegex(ReceiptError, "non-standard numeric constant"):
            self._load()

    def test_binary_hash_and_reference_pin_must_match(self) -> None:
        self.data["binary_sha256"] = "0" * 64
        self._write()
        with self.assertRaisesRegex(ReceiptError, "does not match"):
            self._load()
        self.data["binary_sha256"] = self.binary_hash
        self.data["engine_identity"] = "quickjs-ng"
        self._write()
        ng_recipe = BuildRecipe(
            **{**self.recipe.__dict__, "engine_identity": "quickjs-ng"}
        )
        with self.assertRaisesRegex(ReceiptError, "manifest pin"):
            load_receipt(
                self.receipt,
                expected_binary_sha256=self.binary_hash,
                expected_engine_identity="quickjs-ng",
                expected_profile_id="release-native-v1",
                expected_recipe=ng_recipe,
                pinned_reference=(
                    "quickjs-ng", "https://official.invalid/quickjs.git", "b" * 40
                ),
            )

    def test_build_settings_must_match_profile(self) -> None:
        self.data["build"]["lto"] = "fat"
        self._write()
        with self.assertRaisesRegex(ReceiptError, "recipe value"):
            self._load()

    def test_every_recipe_dimension_and_revision_are_exact(self) -> None:
        mutations = {
            "build_mode": "debug",
            "toolchain": "garbage",
            "features": ["agents"],
            "flags": ["-Ctarget-cpu=native"],
            "host_features": "native",
        }
        for field, value in mutations.items():
            with self.subTest(field=field):
                original = self.data["build"][field]
                self.data["build"][field] = value
                self._write()
                with self.assertRaisesRegex(ReceiptError, "recipe value"):
                    self._load()
                self.data["build"][field] = original
        self.data["source"]["revision"] = "short"
        self._write()
        with self.assertRaisesRegex(ReceiptError, "full lowercase git SHA"):
            self._load()


if __name__ == "__main__":
    unittest.main()
