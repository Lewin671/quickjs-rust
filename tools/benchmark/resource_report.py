"""Strict CLI and atomic writer for resource reports."""

from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile
from pathlib import Path

from .resource_analysis_schema import ResourceAnalysisError, load_resource_analysis
from .resource_artifact import build_resource_report
from .resource_schema import ResourceManifestError, load_resource_manifest
from .resource_validation import ResourceReportError, validate_resource_run


def write_resource_report(output: Path, report: dict) -> None:
    output = output.expanduser().resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        raise ResourceReportError(f"refusing to overwrite existing output: {output}")
    encoded = (json.dumps(report, sort_keys=True, indent=2) + "\n").encode("utf-8")
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb", dir=output.parent, prefix=f".{output.name}.", delete=False
        ) as handle:
            temporary = Path(handle.name)
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary, output)
        except FileExistsError as error:
            raise ResourceReportError(f"refusing to overwrite existing output: {output}") from error
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def _parser(root: Path) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="analyze one strict resource JSONL run")
    parser.add_argument("--manifest", type=Path, default=root / "benchmarks/resources.json")
    parser.add_argument(
        "--analysis-manifest", type=Path,
        default=root / "benchmarks/resource-analysis.json",
    )
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    args = _parser(root).parse_args()
    try:
        measurement = load_resource_manifest(args.manifest)
        analysis = load_resource_analysis(args.analysis_manifest, measurement)
        validated = validate_resource_run(args.input, measurement)
        write_resource_report(
            args.output, build_resource_report(validated, measurement, analysis)
        )
        print(args.output.expanduser().resolve())
        return 0
    except (
        OSError, ResourceAnalysisError, ResourceManifestError, ResourceReportError, ValueError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
