"""Measurement manifests for the host that is measuring.

The checked-in manifests name one platform profile and the build recipes of
the host they were frozen on. This module always writes copies whose `profile` and
`build_recipes` describe the live host (the Rust target, and the C compiler
and target the reference build was configured with), so a receipt never
claims the frozen host's recipe for an executable built elsewhere.

Nothing else differs: cases, measurement policy and the protocol hash are
copied byte for byte from the frozen lane, and decisions compare archived
manifests ignoring exactly these two keys
(`performance_evidence.load_archived_manifest`). The copies sit next to the
frozen manifests because the loader resolves protocol files relative to its
manifest.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import sys
import tempfile
from pathlib import Path

from .preview import RUST_BUILD_FLAGS
from .local_receipt import rust_toolchain

ROOT = Path(__file__).resolve().parents[2]
LANES = ("manifest.json", "generic-sentinels-manifest.json")
REFERENCE_BUILD = ROOT / "third_party/quickjs-ng/build"


class HostManifestError(RuntimeError):
    pass


def host_platform(system: str | None = None, machine: str | None = None) -> str:
    system = system or platform.system()
    machine = machine or platform.machine()
    return f"{'macos' if system == 'Darwin' else system.lower()}-{machine.lower()}"


def _run(argv: list[str]) -> str:
    try:
        completed = subprocess.run(argv, capture_output=True, text=True, timeout=60, check=False)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise HostManifestError(f"cannot run {argv[0]}: {error}") from error
    if completed.returncode != 0:
        raise HostManifestError(f"{' '.join(argv)} failed: {completed.stderr.strip()[:300]}")
    return completed.stdout.strip()


def reference_compiler(build: Path) -> str:
    """The C compiler the reference build was configured with."""
    try:
        cache = (build / "CMakeCache.txt").read_text(encoding="utf-8")
    except OSError as error:
        raise HostManifestError(f"no configured reference build at {build}: {error}") from error
    for line in cache.splitlines():
        if line.startswith("CMAKE_C_COMPILER:"):
            return line.split("=", 1)[1].strip()
    raise HostManifestError(f"{build}/CMakeCache.txt names no C compiler")


def live_facts(build: Path = REFERENCE_BUILD) -> dict[str, str]:
    compiler = reference_compiler(build)
    rust_target = next((line.split(":", 1)[1].strip() for line in _run(["rustc", "-vV"]).splitlines()
                        if line.startswith("host:")), "")
    if not rust_target:
        raise HostManifestError("rustc -vV reported no host target")
    return {
        "rust_target": rust_target,
        "rust_toolchain": rust_toolchain(),
        "reference_toolchain": _run([compiler, "--version"]).splitlines()[0].strip(),
        "reference_target": _run([compiler, "-dumpmachine"]),
    }


def derive(frozen: dict, host: str, facts: dict[str, str]) -> dict:
    """`frozen` with its profile and recipes rewritten for `host`."""
    reference = frozen["reference_engine"]["identity"]
    derived = dict(frozen)
    derived["profile"] = {"id": f"{host}-fat16-abort-v1", "platform": host}
    recipes = []
    for recipe in frozen["build_recipes"]:
        recipe = dict(recipe)
        if recipe["engine_identity"] == reference:
            recipe["toolchain"] = facts["reference_toolchain"]
            recipe["target"] = facts["reference_target"]
        else:
            recipe.update(
                target=facts["rust_target"], toolchain=facts["rust_toolchain"],
                features=["agents"], flags=list(RUST_BUILD_FLAGS),
                lto="fat-forced-by-cargo-config", strip="none-release-default",
                host_features="target-default-cpu-v0-symbols-and-16-byte-function-alignment-forced-by-cargo-encoded-rustflags",
            )
        recipes.append(recipe)
    derived["build_recipes"] = recipes
    return derived


def _write_replace(path: Path, content: bytes) -> None:
    with tempfile.NamedTemporaryFile(dir=path.parent, prefix=f".{path.name}.", delete=False) as stream:
        stream.write(content)
    os.replace(stream.name, path)


def manifests(root: Path = ROOT, host: str | None = None, facts: dict[str, str] | None = None) -> list[Path]:
    """The broad and sentinel manifests to measure with on this host."""
    host = host or host_platform()
    result = []
    for name in LANES:
        path = root / "benchmarks" / name
        frozen = json.loads(path.read_text(encoding="utf-8"))
        facts = facts or live_facts(root / "third_party/quickjs-ng/build")
        path = root / "benchmarks" / f".host-{host}-{name}"
        _write_replace(path, (json.dumps(derive(frozen, host, facts), indent=2) + "\n").encode())
        result.append(path)
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python3 -m tools.benchmark.host_manifest", description=__doc__)
    parser.parse_args(argv)
    try:
        for path in manifests():
            print(path)
    except (HostManifestError, OSError, KeyError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
