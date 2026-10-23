from __future__ import annotations

import hashlib
import json
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from tools.benchmark import host_manifest, layout_resources

ROOT = Path(__file__).resolve().parents[3]
FACTS = {"rust_target": "x86_64-apple-darwin", "reference_toolchain": "Apple clang version 14.0.0",
         "reference_target": "x86_64-apple-darwin21.6.0",
         "rust_toolchain": "live-rust-toolchain"}
X86_POLICY = {"host": {"system": "Darwin", "machine": "x86_64"}, "host_node": "iMac.local"}
ARM_POLICY = {"host": {"system": "Darwin", "machine": "arm64"}, "host_node": "iMac.local"}


class LayoutResourceManifestTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        frozen = json.loads((ROOT / "benchmarks/resources.json").read_text())
        for relative in [*frozen["protocol"]["files"], "benchmarks/resources.json",
                         "benchmarks/resource-analysis.json"]:
            (self.root / relative).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(ROOT / relative, self.root / relative)
        self.frozen = self.root / "benchmarks/resources.json"

    @staticmethod
    def _report(sha: str) -> dict:
        return {"measurement_contract": {"manifest_sha256": sha}}

    def test_evidence_naming_the_frozen_manifest_selects_it(self):
        sha = hashlib.sha256(self.frozen.read_bytes()).hexdigest()
        selected = layout_resources.resource_manifest_for(
            self._report(sha), ARM_POLICY, self.root, FACTS)
        self.assertEqual(selected.profile.id, "macos-arm64-release-v1")
        self.assertEqual(selected.sha256, sha)

    def test_evidence_naming_the_host_manifest_selects_the_derived_bytes(self):
        [path] = host_manifest.resource_manifests(self.root, "macos-x86_64", FACTS)
        sha = hashlib.sha256(path.read_bytes()).hexdigest()
        selected = layout_resources.resource_manifest_for(
            self._report(sha), X86_POLICY, self.root, FACTS)
        self.assertEqual(selected.sha256, sha)
        self.assertEqual(
            (selected.profile.id, selected.profile.platform, selected.profile.machine),
            ("macos-x86_64-fat16-abort-v1", "darwin", "x86_64"))

    def test_another_hosts_manifest_is_refused(self):
        [path] = host_manifest.resource_manifests(self.root, "macos-x86_64", FACTS)
        sha = hashlib.sha256(path.read_bytes()).hexdigest()
        with self.assertRaisesRegex(ValueError, "re-derive and remeasure"):
            layout_resources.resource_manifest_for(
                self._report(sha), ARM_POLICY, self.root, FACTS)

    def test_an_unknown_manifest_is_refused(self):
        with self.assertRaisesRegex(ValueError, "re-derive and remeasure"):
            layout_resources.resource_manifest_for(
                self._report("b" * 64), X86_POLICY, self.root, FACTS)

    def test_a_host_that_cannot_derive_its_recipe_says_where_to_run(self):
        def missing(*_args, **_kwargs):
            raise host_manifest.HostManifestError("no configured reference build")

        with mock.patch.object(host_manifest, "resource_manifests", missing):
            with self.assertRaisesRegex(ValueError, "host that produced the evidence"):
                layout_resources.resource_manifest_for(
                    self._report("c" * 64), X86_POLICY, self.root, FACTS)


if __name__ == "__main__":
    unittest.main()
