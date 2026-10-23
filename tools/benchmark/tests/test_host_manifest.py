from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path

from tools.benchmark import host_manifest
from tools.benchmark.resource_schema import load_resource_manifest
from tools.benchmark.schema import load_manifest

ROOT = Path(__file__).resolve().parents[3]
FACTS = {"rust_target": "x86_64-apple-darwin", "reference_toolchain": "Apple clang version 14.0.0",
         "reference_target": "x86_64-apple-darwin21.6.0",
         "rust_toolchain": "live-rust-toolchain"}
MANIFESTS = (*host_manifest.LANES, host_manifest.RESOURCE_LANE)


class HostManifestTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        frozen = json.loads((ROOT / "benchmarks/manifest.json").read_text())
        protocol = {relative for name in MANIFESTS
                    for relative in json.loads((ROOT / "benchmarks" / name).read_text())["protocol"]["files"]}
        for relative in [*protocol, *(f"benchmarks/{name}" for name in MANIFESTS)]:
            (self.root / relative).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(ROOT / relative, self.root / relative)
        self.frozen_platform = frozen["profile"]["platform"]

    def test_platform_names(self):
        self.assertEqual(host_manifest.host_platform("Darwin", "arm64"), "macos-arm64")
        self.assertEqual(host_manifest.host_platform("Darwin", "x86_64"), "macos-x86_64")
        self.assertEqual(host_manifest.host_platform("Linux", "x86_64"), "linux-x86_64")

    def test_the_frozen_platform_also_gets_the_current_recipe(self):
        paths = host_manifest.manifests(self.root, self.frozen_platform, facts=FACTS)
        self.assertEqual(paths, [self.root / "benchmarks" / f".host-{self.frozen_platform}-{name}"
                                 for name in host_manifest.LANES])

    def test_another_platform_gets_its_own_profile_and_recipes_and_nothing_else(self):
        paths = host_manifest.manifests(self.root, "macos-x86_64", FACTS)
        self.assertEqual([path.name for path in paths],
                         [f".host-macos-x86_64-{name}" for name in host_manifest.LANES])
        for path, name in zip(paths, host_manifest.LANES, strict=True):
            frozen = json.loads((self.root / "benchmarks" / name).read_text())
            derived = json.loads(path.read_text())
            keep = lambda data: {key: value for key, value in data.items()
                                 if key not in {"profile", "build_recipes"}}
            self.assertEqual(keep(derived), keep(frozen))
            loaded = load_manifest(path)
            self.assertEqual((loaded.profile.id, loaded.profile.platform),
                             ("macos-x86_64-fat16-abort-v1", "macos-x86_64"))
            rust, reference = loaded.build_recipes["qjs-rust"], loaded.build_recipes["quickjs-ng"]
            self.assertEqual(rust.target, "x86_64-apple-darwin")
            self.assertEqual(rust.toolchain, FACTS["rust_toolchain"])
            self.assertEqual(rust.features, ("agents",))
            self.assertEqual(rust.lto, "fat-forced-by-cargo-config")
            self.assertEqual((reference.toolchain, reference.target),
                             ("Apple clang version 14.0.0", "x86_64-apple-darwin21.6.0"))

    def test_the_resource_manifest_is_derived_in_its_own_profile_shape(self):
        [path] = host_manifest.resource_manifests(self.root, "macos-x86_64", FACTS)
        self.assertEqual(path.name, ".host-macos-x86_64-resources.json")
        frozen = json.loads((self.root / "benchmarks" / host_manifest.RESOURCE_LANE).read_text())
        derived = json.loads(path.read_text())
        keep = lambda data: {key: value for key, value in data.items()
                             if key not in {"profile", "build_recipes"}}
        self.assertEqual(keep(derived), keep(frozen))
        loaded = load_resource_manifest(path)
        self.assertEqual(
            (loaded.profile.id, loaded.profile.platform, loaded.profile.machine,
             loaded.profile.rss_raw_unit),
            ("macos-x86_64-fat16-abort-v1", "darwin", "x86_64", "bytes"))
        self.assertEqual(set(loaded.lanes), set(load_resource_manifest(
            self.root / "benchmarks" / host_manifest.RESOURCE_LANE).lanes))
        rust = loaded.build_recipes["qjs-rust"]
        self.assertEqual(
            (rust.target, rust.toolchain, rust.features, rust.lto),
            ("x86_64-apple-darwin", FACTS["rust_toolchain"], ("agents",),
             "fat-forced-by-cargo-config"))

    def test_deriving_leaves_every_frozen_manifest_byte_identical(self):
        before = {name: (self.root / "benchmarks" / name).read_bytes() for name in MANIFESTS}
        host_manifest.manifests(self.root, "macos-x86_64", FACTS)
        host_manifest.resource_manifests(self.root, "macos-x86_64", FACTS)
        self.assertEqual(before, {name: (self.root / "benchmarks" / name).read_bytes()
                                  for name in MANIFESTS})

    def test_a_linux_host_derives_its_own_resource_profile(self):
        [path] = host_manifest.resource_manifests(self.root, "linux-x86_64", FACTS)
        loaded = load_resource_manifest(path)
        self.assertEqual(
            (loaded.profile.id, loaded.profile.platform, loaded.profile.machine,
             loaded.profile.rss_raw_unit),
            ("linux-x86_64-fat16-abort-v1", "linux", "x86_64", "kibibytes"))

    def test_an_unconfigured_reference_build_is_an_error(self):
        with self.assertRaises(host_manifest.HostManifestError):
            host_manifest.reference_compiler(self.root / "third_party/quickjs-ng/build")


if __name__ == "__main__":
    unittest.main()
