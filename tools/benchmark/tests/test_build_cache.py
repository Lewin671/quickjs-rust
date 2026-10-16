from __future__ import annotations

import argparse
import io
import json
import os
import re
import subprocess
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

from tools.benchmark.build_cache import (
    CACHE_SCHEMA_VERSION,
    RUST_KIND,
    RUST_RECIPE,
    _spec,
    _build_rust_command,
    materialize,
    quickjs_spec,
    rust_spec,
    rust_source_digest,
    store,
    validate_entry,
    write_plan,
    write_reference_plan,
    ready_entry,
)
from tools.benchmark.schema import sha256_file

ROOT = Path(__file__).resolve().parents[3]


class BuildCacheTests(unittest.TestCase):
    def _repo(self, root: Path) -> Path:
        subprocess.run(["git", "init", "-q", str(root)], check=True)
        subprocess.run(["git", "-C", str(root), "config", "user.email", "test@example.com"], check=True)
        subprocess.run(["git", "-C", str(root), "config", "user.name", "Test"], check=True)
        files = {
            ".cargo/config.toml": (
                "[build]\n"
                'rustflags = ["-Cllvm-args=-align-all-functions=4"]\n'
            ),
            "Cargo.toml": "[workspace]\nmembers = [\"crates/a\"]\n",
            "Cargo.lock": "version = 4\n",
            "crates/a/Cargo.toml": "[package]\nname = \"a\"\nversion = \"0.1.0\"\n",
            "crates/a/src/lib.rs": "pub fn answer() -> u8 { 42 }\n",
            "docs/readme.md": "documentation only\n",
            ".github/workflows/ci.yml": "name: docs-only-fixture\n",
        }
        for relative, content in files.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
        subprocess.run(["git", "-C", str(root), "add", "."], check=True)
        return root

    def test_source_key_ignores_docs_and_workflow_but_invalidates_build_inputs(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            repo = self._repo(Path(name))
            original = rust_source_digest(repo)
            (repo / "docs/readme.md").write_text("changed\n", encoding="utf-8")
            (repo / ".github/workflows/ci.yml").write_text("name: changed\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "."], check=True)
            self.assertEqual(rust_source_digest(repo), original)
            (repo / ".cargo/config.toml").write_text(
                "[build]\nrustflags = []\n", encoding="utf-8"
            )
            subprocess.run(["git", "-C", str(repo), "add", "."], check=True)
            self.assertNotEqual(rust_source_digest(repo), original)
            (repo / ".cargo/config.toml").write_text(
                "[build]\n"
                'rustflags = ["-Cllvm-args=-align-all-functions=4"]\n',
                encoding="utf-8",
            )
            subprocess.run(
                ["git", "-C", str(repo), "add", ".cargo/config.toml"], check=True
            )
            (repo / "crates/a/src/lib.rs").write_text("pub fn answer() -> u8 { 43 }\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "."], check=True)
            self.assertNotEqual(rust_source_digest(repo), original)

    def test_hosted_rust_recipe_preserves_measured_flags(self) -> None:
        encoded = RUST_RECIPE["environment"]["CARGO_ENCODED_RUSTFLAGS"]
        self.assertEqual(
            encoded.split("\x1f"),
            ["-Csymbol-mangling-version=v0", "-Cllvm-args=-align-all-functions=4"],
        )

    def test_local_builder_uses_hosted_recipe_and_explicit_host_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory).resolve()
            output = io.StringIO()
            with patch("tools.benchmark.build_cache._run", return_value="host: example-target"), \
                 patch("tools.benchmark.build_cache.configured_tool", return_value="cargo"), \
                 patch("tools.benchmark.build_cache.subprocess.run") as run, redirect_stdout(output):
                _build_rust_command(argparse.Namespace(source=source, target_dir=source / "build", cargo="cargo"))
            command = run.call_args
            self.assertEqual(command.args[0], ["cargo", "build", *RUST_RECIPE["cargo_args"]])
            self.assertEqual(command.kwargs["env"]["CARGO_BUILD_TARGET"], "example-target")
            self.assertEqual(command.kwargs["env"]["CARGO_ENCODED_RUSTFLAGS"],
                             RUST_RECIPE["environment"]["CARGO_ENCODED_RUSTFLAGS"])
            self.assertEqual(output.getvalue().strip(), str(source / "build/example-target/release/qjs"))

    def test_hosted_overrides_match_workspace_release_recipe(self) -> None:
        root = Path(__file__).resolve().parents[3]
        # These controlled profile values are JSON scalars as well as TOML.
        profile = (root / "Cargo.toml").read_text().split("[profile.release]", 1)[1]
        release = dict(re.findall(r"^([\w-]+) = (.+)$", profile.split("[", 1)[0], re.M))
        for key, value in release.items():
            override = json.loads(next(
                arg.split("=", 2)[2] for arg in RUST_RECIPE["cargo_args"]
                if arg.startswith(f"--config=profile.release.{key}=")
            ))
            self.assertEqual(override, json.loads(value), key)
        flags = json.loads(re.search(
            r"^rustflags = (.+)$", (root / ".cargo/config.toml").read_text(), re.M
        )[1])
        self.assertEqual(
            RUST_RECIPE["environment"]["CARGO_ENCODED_RUSTFLAGS"].split("\x1f"), flags
        )

    def test_key_binds_toolchain_target_platform_and_recipe(self) -> None:
        base = {"source_sha256": "a" * 64, "platform": "linux", "target": "x86", "toolchain": "rust", "recipe": ["release"]}
        key = _spec(RUST_KIND, base)["key_sha256"]
        for field, value in (
            ("source_sha256", "b" * 64), ("platform", "mac"),
            ("target", "arm"), ("toolchain", "other"), ("recipe", ["debug"]),
        ):
            changed = dict(base)
            changed[field] = value
            self.assertNotEqual(_spec(RUST_KIND, changed)["key_sha256"], key)

    def test_quickjs_key_binds_fixed_manifest_revision(self) -> None:
        spec = quickjs_spec(ROOT / "benchmarks/manifest.json")
        inputs = spec["inputs"]
        self.assertEqual(inputs["source_revision"], "f7830186043e4488f2998759d60a514faf07cbc9")
        changed = dict(inputs)
        changed["source_revision"] = "1" * 40
        self.assertNotEqual(_spec("quickjs-ng", changed)["key_sha256"], spec["key_sha256"])

    def test_rust_key_changes_with_runner_compiler_and_target_linker(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            repo = self._repo(root / "repo")
            tools = root / "tools"
            tools.mkdir()
            for tool_name, release in (("rustc-one", "one"), ("rustc-two", "two"), ("ld-one", "one"), ("ld-two", "two")):
                tool = tools / tool_name
                if tool_name.startswith("rustc"):
                    body = f"#!/bin/sh\nprintf '%s\\n' 'rustc {release}' 'host: x86_64-unknown-linux-gnu'\n"
                else:
                    body = f"#!/bin/sh\necho linker-{release}\n"
                tool.write_text(body, encoding="utf-8")
                tool.chmod(0o755)

            def planned(image: str, rustc: str, linker: str) -> str:
                environment = {
                    "PATH": os.environ["PATH"], "HOME": os.environ.get("HOME", str(root)),
                    "ImageOS": "ubuntu24", "ImageVersion": image,
                    "RUSTC": str(tools / rustc),
                    "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER": str(tools / linker),
                }
                with patch.dict(os.environ, environment, clear=True):
                    return rust_spec(repo)["key_sha256"]

            base = planned("20260701.1", "rustc-one", "ld-one")
            self.assertNotEqual(planned("20260702.1", "rustc-one", "ld-one"), base)
            self.assertNotEqual(planned("20260701.1", "rustc-two", "ld-one"), base)
            self.assertNotEqual(planned("20260701.1", "rustc-one", "ld-two"), base)

    def test_store_hit_and_same_content_role_reuse(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            spec = _spec(RUST_KIND, {"source_sha256": "a" * 64, "recipe": "release"})
            spec_path = root / "spec.json"
            spec_path.write_text(json.dumps(spec), encoding="utf-8")
            built = root / "built"
            built.write_bytes(b"exact executable")
            built.chmod(0o755)
            entry = root / "cache" / spec["key_sha256"]
            digest = store(entry, spec_path, built)
            for role in ("candidate", "base"):
                output = root / role / "qjs"
                hit, actual = materialize(entry, spec_path, output)
                self.assertTrue(hit)
                self.assertEqual(actual, digest)
                self.assertEqual(sha256_file(output), digest)
                self.assertTrue(os.access(output, os.X_OK))

    def test_plan_reuses_one_key_for_identical_candidate_and_base_inputs(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            repo = self._repo(root / "repo")
            reference = _spec("quickjs-ng", {"revision": "f" * 40})
            with patch("tools.benchmark.build_cache.quickjs_spec", return_value=reference):
                plan = write_plan(repo, repo, root / "unused.json", root / "plan")
            self.assertEqual(plan["candidate"]["key_sha256"], plan["base"]["key_sha256"])
            self.assertEqual(plan["quickjs-ng"], reference)

    def test_reference_plan_matches_full_plan_without_rust_sources(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            repo = self._repo(root / "repo")
            reference = _spec("quickjs-ng", {"revision": "f" * 40})
            with patch("tools.benchmark.build_cache.quickjs_spec", return_value=reference):
                full = write_plan(repo, repo, root / "unused.json", root / "full")
                standalone = write_reference_plan(root / "unused.json", root / "reference")
            self.assertEqual(standalone, full["quickjs-ng"])
            self.assertEqual(
                json.loads((root / "reference/quickjs-ng.json").read_text(encoding="utf-8")),
                reference,
            )

    def test_missing_malformed_non_executable_and_digest_mismatch_are_misses(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            spec = _spec(RUST_KIND, {"source_sha256": "a" * 64})
            spec_path = root / "spec.json"
            spec_path.write_text(json.dumps(spec), encoding="utf-8")
            entry = root / "entry"
            self.assertFalse(materialize(entry, spec_path, root / "out")[0])
            entry.mkdir()
            (entry / "metadata.json").write_text("not-json", encoding="utf-8")
            (entry / "binary").write_bytes(b"binary")
            (entry / "binary").chmod(0o755)
            self.assertFalse(validate_entry(entry, spec)[0])

            # A trusted rebuild replaces the malformed exact entry locally.
            built = root / "rebuilt"
            built.write_bytes(b"rebuilt binary")
            built.chmod(0o755)
            store(entry, spec_path, built)
            self.assertTrue(materialize(entry, spec_path, root / "rebuilt-out")[0])
            (entry / "binary").chmod(0o644)
            self.assertFalse(validate_entry(entry, spec)[0])
            (entry / "binary").chmod(0o755)
            metadata = {
                **spec, "binary_sha256": "0" * 64,
                "binary_size": (entry / "binary").stat().st_size,
            }
            (entry / "metadata.json").write_text(json.dumps(metadata), encoding="utf-8")
            self.assertFalse(validate_entry(entry, spec)[0])

    def test_wrong_metadata_never_materializes(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            spec = _spec(RUST_KIND, {"source_sha256": "a" * 64})
            spec_path = root / "spec.json"
            spec_path.write_text(json.dumps(spec), encoding="utf-8")
            built = root / "built"
            built.write_bytes(b"binary")
            built.chmod(0o755)
            entry = root / "entry"
            store(entry, spec_path, built)
            metadata = json.loads((entry / "metadata.json").read_text(encoding="utf-8"))
            metadata["schema_version"] = CACHE_SCHEMA_VERSION + 1
            (entry / "metadata.json").write_text(json.dumps(metadata), encoding="utf-8")
            hit, _ = materialize(entry, spec_path, root / "out")
            self.assertFalse(hit)

    def test_ready_check_rejects_symlinked_cache_parent(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            spec = _spec(RUST_KIND, {"source_sha256": "a" * 64})
            spec_path = root / "spec.json"
            spec_path.write_text(json.dumps(spec), encoding="utf-8")
            real_parent = root / "real"
            entry = real_parent / "entry"
            built = root / "built"
            built.write_bytes(b"binary")
            built.chmod(0o755)
            store(entry, spec_path, built)
            linked_parent = root / "linked"
            linked_parent.symlink_to(real_parent, target_is_directory=True)
            self.assertFalse(ready_entry(linked_parent / "entry", spec)[0])


if __name__ == "__main__":
    unittest.main()
