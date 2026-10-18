"""Held-out acceptance lane: real third-party programs never used to select work.

The opportunity queue and every unit plan come from the pinned performance
suites (SunSpider, Kraken, the JetStream subset) and the broad/sentinel
micro-workloads. If those suites were also the only acceptance check, an
optimization tuned to their shapes would be invisible. This lane is the
independent check: unmodified, pinned third-party npm libraries, driven by
first-party scripts and measured with the same whole-process wall-time protocol
as the external lane but reported separately.

`opportunity_ids` is the registry the decision tools use to refuse a plan that
names a held-out case as a target, control or profile workload: this lane
accepts work, it never selects it. The protocol itself, and the rule that keeps
this lane's case ids disjoint from the development lane's, live in the shared
`tools/benchmark/npm_lane.py` module; the development lane is
`tools/benchmark/development.py`.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from .npm_lane import (
    ROOT,
    LaneSpec,
    NpmCase,
    NpmManifest,
    cli,
    lane,
    load_npm_manifest,
    npm_opportunity_ids,
    npm_status,
    run_npm_preview,
)
from .npm_lane import fetch_sources as _fetch_sources
from .npm_lane import npm_report as _npm_report
from .external_preview import Lane
from .process import ProcessResult

SPEC = LaneSpec(
    key="held-out",
    preview_id="quickjs-held-out-preview-v1",
    manifest_path=ROOT / "benchmarks/held-out-preview.json",
    driver_prefix="benchmarks/held-out/",
    report_artifact_type="quickjs-held-out-preview-report",
    user_agent="quickjs-rust-held-out-preview/1",
    label="held-out-preview",
    artifact_prefix="held-out",
    title="Held-out Benchmark Preview",
    subtitle=(
        "These are pinned, unmodified third-party npm libraries, never used to select "
        "work; no row is an official benchmark score."
    ),
)
MANIFEST_PATH = SPEC.manifest_path
PREVIEW_ID = SPEC.preview_id
DRIVER_PREFIX = SPEC.driver_prefix


def load_held_out_manifest(path: Path) -> NpmManifest:
    return load_npm_manifest(SPEC, path)


def opportunity_ids(manifest_path: Path | None = None) -> frozenset[str]:
    """The frozen set of held-out opportunity ids, for the decision tools."""
    return npm_opportunity_ids(SPEC, manifest_path)


def fetch_sources(manifest: NpmManifest, cache_root: Path) -> dict[str, int]:
    return _fetch_sources(SPEC, manifest, cache_root)


def held_out_status(
    result: ProcessResult, role: str, case: NpmCase
) -> tuple[str, str | None]:
    return npm_status(result, role, case)


def held_out_report(manifest: NpmManifest, records: list[dict[str, Any]]) -> dict[str, Any]:
    return _npm_report(SPEC, manifest, records)


HELD_OUT_LANE: Lane = lane(SPEC)


def run_held_out_preview(
    manifest: NpmManifest,
    cache_root: Path,
    work_root: Path,
    output_dir: Path,
    candidate_path: Path,
    base_path: Path,
    quickjs_path: Path,
    *,
    blocks: int | None = None,
    timeout_seconds: int | None = None,
    workers: int = 1,
) -> dict[str, Any]:
    return run_npm_preview(
        SPEC, manifest, cache_root, work_root, output_dir, candidate_path, base_path,
        quickjs_path, blocks=blocks, timeout_seconds=timeout_seconds, workers=workers,
    )


def main(argv: list[str] | None = None) -> int:
    return cli(SPEC, argv)


if __name__ == "__main__":
    raise SystemExit(main())
