"""Seal a completed comparison's artifact inventory against accidental mixing."""
from __future__ import annotations

import argparse
import json
import sys
import uuid
from pathlib import Path

from .performance_schema import PerformanceDecisionError, _read_json, _sha256
from .profile import digest

ARTIFACTS = ("manifest.json", "raw.jsonl", "report.json",
             "external-manifest.json", "external-raw.jsonl", "external-report.json",
             "sentinel-manifest.json", "sentinel-raw.jsonl", "sentinel-report.json")


def seal(directory: Path) -> dict:
    from .preview import _write_replace
    summary_path = directory / "summary.json"
    summary, _ = _read_json(summary_path, "comparison summary")
    if "bundle" in summary:
        raise PerformanceDecisionError("comparison is already sealed; use a new output directory")
    hashes = {name: digest(directory / name) for name in ARTIFACTS if (directory / name).is_file()}
    summary["bundle"] = {"schema_version": 1, "experiment_id": str(uuid.uuid4()),
                         "artifacts": hashes, "missing": sorted(set(ARTIFACTS) - set(hashes))}
    _write_replace(summary_path, (json.dumps(summary, sort_keys=True, indent=2) + "\n").encode())
    return summary


def verify(summary_path: Path, broad_path: Path, external_path: Path, sentinel_path: Path | None):
    summary, _ = _read_json(summary_path, "comparison summary")
    bundle = summary.get("bundle", {})
    if bundle.get("schema_version") != 1:
        raise PerformanceDecisionError("comparison requires a sealed artifact inventory; run a fresh comparison")
    try:
        uuid.UUID(bundle["experiment_id"])
    except (ValueError, KeyError, TypeError, AttributeError) as error:
        raise PerformanceDecisionError("invalid comparison experiment id") from error
    directory = summary_path.resolve().parent
    for path, name in ((broad_path, "report.json"), (external_path, "external-report.json"),
                       (sentinel_path, "sentinel-report.json")):
        if path is not None and path.resolve() != directory / name:
            raise PerformanceDecisionError("report paths must belong to the same sealed comparison directory")
    hashes = bundle.get("artifacts", {})
    required = set(ARTIFACTS if sentinel_path is not None else ARTIFACTS[:6])
    if not isinstance(hashes, dict) or not required <= set(hashes) or set(hashes) - set(ARTIFACTS):
        raise PerformanceDecisionError("comparison bundle is missing required artifacts")
    if bundle.get("missing") != sorted(set(ARTIFACTS) - set(hashes)):
        raise PerformanceDecisionError("comparison bundle missing inventory is inconsistent")
    for name, expected in hashes.items():
        _sha256(expected, f"bundle {name}")
        if digest(directory / name) != expected:
            raise PerformanceDecisionError(f"sealed comparison artifact changed: {name}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True)
    try:
        seal(parser.parse_args().output_dir)
        return 0
    except (ValueError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
