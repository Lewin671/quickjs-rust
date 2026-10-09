"""The identity that carries three built executables from one job to the next.

The hosted preview builds its executables once and measures each lane on its
own runner. A lane job has no source trees and no Rust toolchain, so the
build job records what it built -- revisions, toolchains, and the SHA-256 of
each executable -- and every lane refuses to measure anything that does not
match that record byte for byte, or that was built for a different event than
the one the lane itself was started for.

The record is transfer integrity inside one cooperative run, not a defence
against a malicious candidate: the build job compiles candidate code.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from .schema import sha256_file

IDENTITY_NAME = "build-identity.json"
BINARY_NAMES = {
    "candidate": "candidate-qjs",
    "base": "base-qjs",
    "quickjs-ng": "quickjs-ng-qjs",
}
# The order `verify` prints the facts a lane needs, one per line.
FACTS = (
    "profile_id", "profile_platform", "rust_toolchain", "rust_target",
    "quickjs_toolchain", "quickjs_target", "quickjs_cc",
)


class IdentityError(ValueError):
    """The executables on disk are not the ones this run built."""


def _fact(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value or value != value.strip() or "\n" in value:
        raise IdentityError(f"{where}: expected one non-empty line")
    return value


def _expected(args: argparse.Namespace) -> dict[str, Any]:
    return {
        "harness": {"mode": args.harness_mode, "revision": args.harness_revision},
        "sources": {
            "candidate": {"repo": args.candidate_repo, "revision": args.candidate_revision},
            "base": {"repo": args.base_repo, "revision": args.base_revision},
            "quickjs-ng": {"revision": args.reference_revision},
        },
    }


def record(args: argparse.Namespace) -> None:
    binaries = args.binaries.expanduser().resolve()
    payload = {
        "schema_version": 1,
        **_expected(args),
        "facts": {name: _fact(getattr(args, name), name) for name in FACTS},
        "binaries": {
            role: sha256_file(binaries / name) for role, name in BINARY_NAMES.items()
        },
    }
    (binaries / IDENTITY_NAME).write_text(
        json.dumps(payload, sort_keys=True, indent=2) + "\n", encoding="utf-8"
    )


def load(binaries: Path) -> dict[str, Any]:
    path = binaries / IDENTITY_NAME
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise IdentityError(f"cannot read {IDENTITY_NAME}: {error}") from error
    if not isinstance(payload, dict) or payload.get("schema_version") != 1:
        raise IdentityError(f"{IDENTITY_NAME}: unsupported schema")
    return payload


def verify(args: argparse.Namespace) -> None:
    """Admit the executables for a lane, then print the facts it needs."""
    binaries = args.binaries.expanduser().resolve()
    payload = load(binaries)
    for key, expected in _expected(args).items():
        if payload.get(key) != expected:
            raise IdentityError(
                f"{IDENTITY_NAME}: {key} does not match the event this lane was started for"
            )
    recorded = payload.get("binaries")
    if not isinstance(recorded, dict) or set(recorded) != set(BINARY_NAMES):
        raise IdentityError(f"{IDENTITY_NAME}: expected exactly three executables")
    for role, name in BINARY_NAMES.items():
        path = binaries / name
        if not path.is_file() or path.is_symlink():
            raise IdentityError(f"{role} executable is missing")
        if sha256_file(path) != recorded[role]:
            raise IdentityError(f"{role} executable does not match the build job's record")
        # Artifact transfer drops the executable bit.
        path.chmod(0o755)
    facts = payload.get("facts")
    if not isinstance(facts, dict) or set(facts) != set(FACTS):
        raise IdentityError(f"{IDENTITY_NAME}: malformed build facts")
    for name in FACTS:
        print(_fact(facts[name], name))


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name, function in (("record", record), ("verify", verify)):
        command = commands.add_parser(name)
        command.add_argument("--binaries", type=Path, required=True)
        for option in (
            "harness-mode", "harness-revision", "candidate-repo", "candidate-revision",
            "base-repo", "base-revision", "reference-revision",
        ):
            command.add_argument(f"--{option}", required=True)
        if name == "record":
            for fact in FACTS:
                command.add_argument(f"--{fact.replace('_', '-')}", required=True)
        command.set_defaults(function=function)
    return parser


def main() -> int:
    try:
        args = _parser().parse_args()
        args.function(args)
        return 0
    except (IdentityError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
