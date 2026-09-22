"""Write a build receipt for a locally built engine, refusing untrue claims.

A receipt states which clean source revision and which frozen build recipe
produced an executable. For qjs-rust the live toolchain must equal the
manifest recipe's; for QuickJS-NG the reference submodule must sit clean at
the pinned revision. Used by scripts/perf-compare.sh; hosted previews write
their receipts through `preview prepare` instead.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

from .schema import ManifestError, load_manifest

ROOT = Path(__file__).resolve().parents[2]
RUST_REPO = "https://github.com/Lewin671/quickjs-rust.git"


class LocalReceiptError(RuntimeError):
    pass


def _run(argv: list[str], cwd: Path = ROOT) -> str:
    try:
        completed = subprocess.run(argv, cwd=cwd, capture_output=True, text=True, timeout=60, check=False)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise LocalReceiptError(f"cannot run {argv[0]}: {error}") from error
    if completed.returncode != 0:
        raise LocalReceiptError(f"{' '.join(argv)} failed: {completed.stderr.strip()[:300]}")
    return completed.stdout.strip()


def rust_toolchain() -> str:
    """The recipe's toolchain string for the active rustc/cargo."""
    llvm = next((line.split(":", 1)[1].strip() for line in _run(["rustc", "-vV"]).splitlines()
                 if line.startswith("LLVM version:")), "unknown")
    return f"{_run(['rustc', '--version'])}; {_run(['cargo', '--version'])}; LLVM {llvm}"


def receipt(identity: str, revision: str, binary: Path, manifest_path: Path) -> dict:
    manifest = load_manifest(manifest_path)
    recipe = manifest.build_recipes[identity]
    if identity == "qjs-rust":
        repo = RUST_REPO
        live = rust_toolchain()
        if live != recipe.toolchain:
            raise LocalReceiptError(f"active toolchain {live!r} differs from the recipe's {recipe.toolchain!r}")
    elif identity == manifest.reference_identity:
        repo = manifest.reference_repo
        checkout = ROOT / "third_party/quickjs-ng"
        if _run(["git", "rev-parse", "HEAD"], checkout) != manifest.reference_revision:
            raise LocalReceiptError("third_party/quickjs-ng is not at the pinned reference revision")
        if _run(["git", "status", "--porcelain", "--untracked-files=no"], checkout):
            raise LocalReceiptError("third_party/quickjs-ng has local modifications")
        if revision != manifest.reference_revision:
            raise LocalReceiptError("a reference receipt must name the pinned revision")
    else:
        raise LocalReceiptError(f"unknown engine identity {identity!r}")
    if len(revision) != 40 or any(c not in "0123456789abcdef" for c in revision):
        raise LocalReceiptError("revision must be a full lowercase git SHA")
    build = {key: value for key, value in recipe.__dict__.items() if key != "engine_identity"}
    build["features"] = list(build["features"])
    build["flags"] = list(build["flags"])
    return {
        "schema_version": 1,
        "engine_identity": identity,
        "source": {"repo": repo, "revision": revision, "dirty": False},
        "profile_id": manifest.profile.id,
        "build": build,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python3 -m tools.benchmark.local_receipt", description=__doc__)
    parser.add_argument("--identity", required=True, choices=("qjs-rust", "quickjs-ng"))
    parser.add_argument("--revision", required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=ROOT / "benchmarks/manifest.json")
    args = parser.parse_args(argv)
    try:
        content = receipt(args.identity, args.revision, args.binary, args.manifest)
        if args.output.exists():
            raise LocalReceiptError(f"refusing to overwrite {args.output}")
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(content, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (LocalReceiptError, ManifestError, OSError, KeyError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
