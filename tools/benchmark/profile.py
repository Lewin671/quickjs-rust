"""Portable, content-verified sampling receipts for optimization plans.

Import an existing sample/perf artifact with its exact measured executable and
workload. This verifies provenance and bytes, not the human interpretation of
a stack or whether an inclusive percentage identifies removable work.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import sys
from pathlib import Path

from .performance_schema import (
    PerformanceDecisionError, _read_json, _revision, _sha256, _string, _keys,
    _opportunity_id,
)


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def contained(root: Path, name: str) -> Path:
    relative = Path(_string(name, "profile path"))
    resolved = (root / relative).resolve()
    if relative.is_absolute() or not resolved.is_relative_to(root.resolve()):
        raise PerformanceDecisionError("profile paths must remain inside the evidence directory")
    return resolved


def verify_profiles(unit: dict, queue: dict, root: Path | None) -> list[dict]:
    if root is None:
        raise PerformanceDecisionError("verified profile evidence requires --profile-root")
    expected = queue.get("engines", {}).get("candidate", {})
    if expected.get("source_revision") != unit["base_sha"]:
        raise PerformanceDecisionError("profile verification requires a current schema-2 queue")
    verified = []
    for declared in unit["profile_evidence"]:
        path = contained(root, declared["source"])
        receipt, receipt_sha = _read_json(path, "profile receipt")
        if receipt_sha != declared["sha256"]:
            raise PerformanceDecisionError("profile receipt SHA-256 mismatch")
        _keys(receipt, {"schema_version", "artifact_type", "base_sha", "binary_sha256",
                        "opportunity_ids", "tool", "command", "profile", "workload"}, "profile receipt")
        if receipt["schema_version"] != 1 or receipt["artifact_type"] != "quickjs-profile-receipt":
            raise PerformanceDecisionError("unsupported profile receipt schema")
        if (receipt["base_sha"] != unit["base_sha"]
                or receipt["binary_sha256"] != expected.get("binary_sha256")):
            raise PerformanceDecisionError("profile does not describe the exact queue candidate executable")
        if set(receipt["opportunity_ids"]) != set(declared["opportunity_ids"]):
            raise PerformanceDecisionError("profile opportunity coverage mismatch")
        if not isinstance(receipt["command"], list) or not receipt["command"]:
            raise PerformanceDecisionError("profile receipt requires the actual sampling command")
        for word in receipt["command"]:
            _string(word, "profile command argument")
        _string(receipt["tool"], "profile tool/version")
        for field in ("profile", "workload"):
            asset = receipt[field]
            _keys(asset, {"path", "sha256"}, f"profile {field}")
            asset_path = contained(path.parent, asset["path"])
            expected_sha = _sha256(asset["sha256"], f"profile {field} hash")
            if not asset_path.is_file() or asset_path.stat().st_size == 0 or digest(asset_path) != expected_sha:
                raise PerformanceDecisionError(f"profile {field} missing, empty, or changed")
        verified.append({"receipt_sha256": receipt_sha, "binary_sha256": receipt["binary_sha256"],
                         "profile_sha256": receipt["profile"]["sha256"],
                         "workload_sha256": receipt["workload"]["sha256"]})
    return verified


def import_profile(args: argparse.Namespace) -> dict:
    revision = _revision(args.base_sha, "profile base revision")
    ids = [_opportunity_id(value, "profile opportunity") for value in args.opportunity_id]
    command = json.loads(args.command_json)
    if not isinstance(command, list) or not command or not all(isinstance(s, str) and s for s in command):
        raise PerformanceDecisionError("--command-json must be a non-empty JSON argv array")
    for path in (args.input, args.binary, args.workload):
        if not path.is_file() or path.stat().st_size == 0:
            raise PerformanceDecisionError(f"profile input missing or empty: {path}")
    binary_sha = digest(args.binary)
    args.output_dir.mkdir(parents=True, exist_ok=False)
    assets = {}
    for key, source in (("profile", args.input), ("workload", args.workload)):
        name = key + (source.suffix or ".bin")
        destination = args.output_dir / name
        shutil.copyfile(source, destination)
        assets[key] = {"path": name, "sha256": digest(destination)}
    receipt = {"schema_version": 1, "artifact_type": "quickjs-profile-receipt",
               "base_sha": revision, "binary_sha256": binary_sha, "opportunity_ids": ids,
               "tool": _string(args.tool, "tool/version"), "command": command, **assets}
    path = args.output_dir / "receipt.json"
    path.write_text(json.dumps(receipt, sort_keys=True, indent=2) + "\n")
    return {"source": path.name, "sha256": digest(path), "base_sha": revision,
            "opportunity_ids": ids}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True, help="existing raw sampling artifact")
    parser.add_argument("--binary", type=Path, required=True, help="exact sampled executable")
    parser.add_argument("--workload", type=Path, required=True)
    parser.add_argument("--base-sha", required=True)
    parser.add_argument("--opportunity-id", action="append", required=True)
    parser.add_argument("--tool", required=True, help="sampler name and version")
    parser.add_argument("--command-json", required=True, help="actual sampling command argv")
    parser.add_argument("--output-dir", type=Path, required=True)
    try:
        print(json.dumps(import_profile(parser.parse_args()), indent=2))
        return 0
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
