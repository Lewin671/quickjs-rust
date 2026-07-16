"""Content-addressed, fail-closed executable cache for hosted previews."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from .schema import load_manifest, sha256_file
from .build_cache_identity import (
    cargo_configuration_identity,
    configured_tool,
    configured_tool_identities,
    effective_build_environment,
    run as _run,
    rust_linker_identity,
    system_identity,
    tool as _tool,
    version_identity,
)


CACHE_SCHEMA_VERSION = 1
CACHE_MISS = 10
RUST_KIND = "rust-qjs-cli"
QUICKJS_KIND = "quickjs-ng"
SOURCE_PATHS = ("Cargo.toml", "Cargo.lock", "rust-toolchain", "rust-toolchain.toml", "crates")
RUST_RECIPE = {
    "cargo_args": [
        "--locked", "--release", "-p", "qjs-cli",
        "--config=profile.release.opt-level=3",
        "--config=profile.release.debug=false",
        "--config=profile.release.debug-assertions=false",
        "--config=profile.release.overflow-checks=false",
        "--config=profile.release.lto=false",
        "--config=profile.release.codegen-units=16",
        "--config=profile.release.panic=\"unwind\"",
        "--config=profile.release.incremental=false",
        "--config=profile.release.strip=\"none\"",
    ],
    "environment": {
        "CARGO_INCREMENTAL": "0",
        "CARGO_ENCODED_RUSTFLAGS": "-Ctarget-cpu=generic",
    },
    "binary": "qjs",
}
QUICKJS_RECIPE = {
    "make_args": ["BUILD_TYPE=Release", "all"],
    "binary": "build/qjs",
}


class BuildCacheError(ValueError):
    """A cache plan, entry, or command is malformed."""


def _json_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + "\n").encode()


def _hash_object(value: Any) -> str:
    return hashlib.sha256(_json_bytes(value)).hexdigest()


def rust_source_digest(source: Path) -> str:
    """Hash tracked Rust workspace inputs while excluding docs/workflows/results."""
    source = source.expanduser().resolve()
    listing = subprocess.run(
        ["git", "-C", str(source), "ls-files", "--stage", "-z", "--", *SOURCE_PATHS],
        capture_output=True, timeout=30, check=False,
    )
    if listing.returncode != 0:
        raise BuildCacheError("cannot enumerate tracked Rust build inputs")
    if not listing.stdout:
        raise BuildCacheError("Rust build input inventory is empty")
    return hashlib.sha256(listing.stdout).hexdigest()


def _spec(kind: str, inputs: dict[str, Any]) -> dict[str, Any]:
    key = _hash_object({"schema_version": CACHE_SCHEMA_VERSION, "kind": kind, "inputs": inputs})
    return {
        "schema_version": CACHE_SCHEMA_VERSION,
        "kind": kind,
        "key_sha256": key,
        "inputs": inputs,
    }


def rust_spec(source: Path) -> dict[str, Any]:
    environment = effective_build_environment()
    rustc = configured_tool(
        environment.get("CARGO_BUILD_RUSTC", environment.get("RUSTC", "")), "rustc"
    )
    cargo = _tool("cargo")
    rustc_identity = _run([rustc, "-vV"], cwd=source)
    cargo_identity = _run([cargo, "-V"], cwd=source)
    target = next(
        (line.removeprefix("host: ") for line in rustc_identity.splitlines() if line.startswith("host: ")),
        "",
    )
    if not target:
        raise BuildCacheError("rustc did not report a host target")
    inputs = {
        "source_sha256": rust_source_digest(source),
        "system": system_identity(),
        "target": target,
        "effective_environment": environment,
        "cargo_configuration": cargo_configuration_identity(),
        "configured_tool_identities": configured_tool_identities(environment),
        "linker": rust_linker_identity(target, environment),
        "toolchain": {
            "rustc_path": rustc,
            "rustc_identity": rustc_identity,
            "rustc_sha256": sha256_file(Path(rustc)),
            "cargo_path": cargo,
            "cargo_identity": cargo_identity,
            "cargo_sha256": sha256_file(Path(cargo)),
        },
        "recipe": RUST_RECIPE,
    }
    return _spec(RUST_KIND, inputs)


def quickjs_spec(manifest: Path) -> dict[str, Any]:
    measurement = load_manifest(manifest.expanduser().resolve())
    cc = _tool("cc")
    cmake = _tool("cmake")
    make = _tool("make")
    inputs = {
        "source_repo": measurement.reference_repo,
        "source_revision": measurement.reference_revision,
        "system": system_identity(),
        "target": _run([cc, "-dumpmachine"]),
        "toolchain": {
            "cc": version_identity(cc),
            "cmake": version_identity(cmake),
            "make": version_identity(make),
        },
        "effective_environment": effective_build_environment(),
        "recipe": QUICKJS_RECIPE,
    }
    return _spec(QUICKJS_KIND, inputs)


def write_plan(candidate: Path, base: Path, manifest: Path, output: Path) -> dict[str, dict[str, Any]]:
    output = output.expanduser().resolve()
    output.mkdir(parents=True, exist_ok=True)
    plan = {
        "candidate": rust_spec(candidate),
        "base": rust_spec(base),
        "quickjs-ng": quickjs_spec(manifest),
    }
    for role, spec in plan.items():
        (output / f"{role}.json").write_bytes(_json_bytes(spec))
    return plan


def write_reference_plan(manifest: Path, output: Path) -> dict[str, Any]:
    """Write the pinned reference-engine cache plan without Rust source inputs."""
    output = output.expanduser().resolve()
    output.mkdir(parents=True, exist_ok=True)
    spec = quickjs_spec(manifest)
    (output / "quickjs-ng.json").write_bytes(_json_bytes(spec))
    return spec


def load_spec(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise BuildCacheError(f"cannot load cache specification: {error}") from error
    if not isinstance(value, dict) or set(value) != {"schema_version", "kind", "key_sha256", "inputs"}:
        raise BuildCacheError("cache specification has the wrong shape")
    if value["schema_version"] != CACHE_SCHEMA_VERSION or value["kind"] not in (RUST_KIND, QUICKJS_KIND):
        raise BuildCacheError("cache specification identity is unsupported")
    if not isinstance(value["inputs"], dict):
        raise BuildCacheError("cache specification inputs must be an object")
    expected = _spec(value["kind"], value["inputs"])
    if value != expected:
        raise BuildCacheError("cache specification key does not match its inputs")
    return value


def _regular_executable(path: Path) -> bool:
    try:
        mode = path.lstat().st_mode
    except OSError:
        return False
    return stat.S_ISREG(mode) and not path.is_symlink() and bool(mode & stat.S_IXUSR)


def _absolute_unresolved(path: Path) -> Path:
    return Path(os.path.abspath(path.expanduser()))


def validate_entry(entry: Path, spec: dict[str, Any]) -> tuple[bool, str]:
    metadata_path = entry / "metadata.json"
    binary = entry / "binary"
    if not entry.is_dir() or entry.is_symlink():
        return False, "entry directory is missing or unsafe"
    if not metadata_path.is_file() or metadata_path.is_symlink():
        return False, "metadata is missing or unsafe"
    if not _regular_executable(binary):
        return False, "binary is missing, unsafe, or not executable"
    try:
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return False, "metadata is malformed"
    expected_keys = {
        "schema_version", "kind", "key_sha256", "inputs", "binary_sha256", "binary_size"
    }
    if not isinstance(metadata, dict) or set(metadata) != expected_keys:
        return False, "metadata has the wrong shape"
    for field in ("schema_version", "kind", "key_sha256", "inputs"):
        if metadata.get(field) != spec[field]:
            return False, f"metadata {field} does not match the requested build"
    try:
        size = binary.stat().st_size
        digest = sha256_file(binary)
    except OSError:
        return False, "binary cannot be inspected"
    if type(metadata.get("binary_size")) is not int or metadata["binary_size"] != size:
        return False, "binary size does not match metadata"
    if metadata.get("binary_sha256") != digest:
        return False, "binary digest does not match metadata"
    return True, digest


def ready_entry(entry: Path, spec: dict[str, Any]) -> tuple[bool, str]:
    entry = _absolute_unresolved(entry)
    if entry.parent.is_symlink():
        return False, "entry parent is an unsafe symlink"
    return validate_entry(entry, spec)


def materialize(entry: Path, spec_path: Path, output: Path) -> tuple[bool, str]:
    spec = load_spec(spec_path)
    entry = _absolute_unresolved(entry)
    valid, detail = ready_entry(entry, spec)
    if not valid:
        return False, detail
    output = output.expanduser().resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=output.parent, prefix=f".{output.name}.", delete=False) as handle:
        temporary = Path(handle.name)
    try:
        shutil.copyfile(entry / "binary", temporary)
        temporary.chmod(0o755)
        if sha256_file(temporary) != detail:
            raise BuildCacheError("materialized binary digest changed during copy")
        os.replace(temporary, output)
    finally:
        temporary.unlink(missing_ok=True)
    return True, detail


def store(entry: Path, spec_path: Path, binary: Path) -> str:
    spec = load_spec(spec_path)
    binary = binary.expanduser().resolve()
    if not _regular_executable(binary):
        raise BuildCacheError("built cache input is not a regular executable")
    digest = sha256_file(binary)
    metadata = {**spec, "binary_sha256": digest, "binary_size": binary.stat().st_size}
    entry = _absolute_unresolved(entry)
    if entry.parent.is_symlink():
        raise BuildCacheError("cache entry parent cannot be a symlink")
    entry.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = Path(
        tempfile.mkdtemp(dir=entry.parent, prefix=f".{entry.name}.")
    )
    try:
        shutil.copyfile(binary, temporary / "binary")
        (temporary / "binary").chmod(0o755)
        (temporary / "metadata.json").write_bytes(_json_bytes(metadata))
        valid, detail = validate_entry(temporary, spec)
        if not valid or detail != digest:
            raise BuildCacheError(f"new cache entry failed validation: {detail}")
        if entry.exists() or entry.is_symlink():
            if entry.is_dir() and not entry.is_symlink():
                shutil.rmtree(entry)
            else:
                entry.unlink()
        os.replace(temporary, entry)
        temporary = None
    finally:
        if temporary is not None and temporary.exists():
            shutil.rmtree(temporary)
    return digest


def _plan_command(args: argparse.Namespace) -> None:
    plan = write_plan(args.candidate_source, args.base_source, args.manifest, args.output_dir)
    outputs = {
        "candidate-key": plan["candidate"]["key_sha256"],
        "base-key": plan["base"]["key_sha256"],
        "quickjs-key": plan["quickjs-ng"]["key_sha256"],
    }
    for name, value in outputs.items():
        print(f"{name}={value}")
    if args.github_output is not None:
        with args.github_output.open("a", encoding="utf-8") as handle:
            for name, value in outputs.items():
                handle.write(f"{name}={value}\n")


def _reference_plan_command(args: argparse.Namespace) -> None:
    spec = write_reference_plan(args.manifest, args.output_dir)
    value = spec["key_sha256"]
    print(f"quickjs-key={value}")
    if args.github_output is not None:
        with args.github_output.open("a", encoding="utf-8") as handle:
            handle.write(f"quickjs-key={value}\n")


def _materialize_command(args: argparse.Namespace) -> None:
    hit, detail = materialize(args.entry, args.spec, args.output)
    if not hit:
        print(f"cache miss: {detail}", file=sys.stderr)
        raise SystemExit(CACHE_MISS)
    print(f"cache hit: sha256={detail}")


def _store_command(args: argparse.Namespace) -> None:
    print(f"cache stored: sha256={store(args.entry, args.spec, args.binary)}")


def _ready_command(args: argparse.Namespace) -> None:
    spec = load_spec(args.spec)
    valid, detail = ready_entry(args.entry, spec)
    if not valid:
        print(f"cache not ready: {detail}", file=sys.stderr)
        raise SystemExit(CACHE_MISS)
    print(f"cache ready: sha256={detail}")


def _recipe_command(args: argparse.Namespace) -> None:
    recipe = RUST_RECIPE if args.kind == "rust" else QUICKJS_RECIPE
    value = recipe[args.field]
    if isinstance(value, list):
        for item in value:
            print(item)
    elif isinstance(value, dict):
        print(value[args.name])
    else:
        print(value)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    plan = commands.add_parser("plan")
    plan.add_argument("--candidate-source", type=Path, required=True)
    plan.add_argument("--base-source", type=Path, required=True)
    plan.add_argument("--manifest", type=Path, required=True)
    plan.add_argument("--output-dir", type=Path, required=True)
    plan.add_argument("--github-output", type=Path)
    plan.set_defaults(function=_plan_command)
    reference_plan = commands.add_parser("reference-plan")
    reference_plan.add_argument("--manifest", type=Path, required=True)
    reference_plan.add_argument("--output-dir", type=Path, required=True)
    reference_plan.add_argument("--github-output", type=Path)
    reference_plan.set_defaults(function=_reference_plan_command)
    restore = commands.add_parser("materialize")
    restore.add_argument("--entry", type=Path, required=True)
    restore.add_argument("--spec", type=Path, required=True)
    restore.add_argument("--output", type=Path, required=True)
    restore.set_defaults(function=_materialize_command)
    install = commands.add_parser("store")
    install.add_argument("--entry", type=Path, required=True)
    install.add_argument("--spec", type=Path, required=True)
    install.add_argument("--binary", type=Path, required=True)
    install.set_defaults(function=_store_command)
    ready = commands.add_parser("ready")
    ready.add_argument("--entry", type=Path, required=True)
    ready.add_argument("--spec", type=Path, required=True)
    ready.set_defaults(function=_ready_command)
    recipe = commands.add_parser("recipe")
    recipe.add_argument("--kind", choices=("rust", "quickjs"), required=True)
    recipe.add_argument("--field", choices=("cargo_args", "environment", "make_args", "binary"), required=True)
    recipe.add_argument("--name", choices=("CARGO_INCREMENTAL", "CARGO_ENCODED_RUSTFLAGS"))
    recipe.set_defaults(function=_recipe_command)
    return parser


def main() -> int:
    try:
        args = _parser().parse_args()
        args.function(args)
        return 0
    except BuildCacheError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
