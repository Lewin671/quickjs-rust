"""CLI and atomic writer for deterministic benchmark reports."""

from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile
from pathlib import Path
from typing import Any

from .analysis_schema import AnalysisManifestError, load_analysis_manifest
from .artifact import build_report as assemble_report
from .raw_validation import ReportError, validate_run
from .schema import ManifestError, load_manifest


def build_report(input_path: Path, measurement_manifest: Any, analysis_manifest: Any) -> dict[str, Any]:
    """Compatibility entry point used by tests and programmatic callers."""
    return assemble_report(
        validate_run(input_path, measurement_manifest),
        measurement_manifest,
        analysis_manifest,
    )


def write_report(output: Path, report: dict[str, Any]) -> None:
    output = output.expanduser().resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        raise ReportError(f"refusing to overwrite existing output: {output}")
    encoded = (json.dumps(report, sort_keys=True, indent=2) + "\n").encode("utf-8")
    temporary_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb", dir=output.parent, prefix=f".{output.name}.", delete=False
        ) as handle:
            temporary_path = Path(handle.name)
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary_path, output)
        except FileExistsError as error:
            raise ReportError(f"refusing to overwrite existing output: {output}") from error
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)


def _parser(root: Path) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="analyze one physically complete M3 benchmark JSONL plan"
    )
    parser.add_argument("--manifest", type=Path, default=root / "benchmarks/manifest.json")
    parser.add_argument(
        "--analysis-manifest", type=Path, default=root / "benchmarks/analysis.json"
    )
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    args = _parser(root).parse_args()
    try:
        measurement = load_manifest(args.manifest)
        analysis = load_analysis_manifest(args.analysis_manifest, measurement)
        report = build_report(args.input.expanduser().resolve(), measurement, analysis)
        write_report(args.output, report)
        print(args.output.expanduser().resolve())
        return 0
    except (
        ManifestError, AnalysisManifestError, ReportError, OSError, ValueError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
