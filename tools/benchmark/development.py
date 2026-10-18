"""Development lane: real programs that may be profiled and may select work.

Held-out programs are acceptance-only: profiling one, or letting one rank in the
opportunity queue, would tune the engine to its own acceptance check. That rule
leaves nothing to profile except the pinned performance suites and the
micro-workloads, so this lane supplies the missing half: real third-party npm
libraries, disjoint from the held-out set and from the pinned SunSpider, Kraken
and JetStream subsets, that a unit plan may name and a queue may rank.

Unlike held-out, a development case id is a legal workload reference: the plan
loader accepts it as a priority, control or profile workload, and
`build_queue` ranks its cases in the queue's own `development` channel. It is
still never an official score, and the shared npm-lane loader refuses a case id
that the held-out lane already owns. Protocol and lane machinery:
`tools/benchmark/npm_lane.py`.
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
    key="development",
    preview_id="quickjs-development-preview-v1",
    manifest_path=ROOT / "benchmarks/development-preview.json",
    driver_prefix="benchmarks/development/",
    report_artifact_type="quickjs-development-preview-report",
    user_agent="quickjs-rust-development-preview/1",
    label="development-preview",
    artifact_prefix="development",
    title="Development Benchmark Lane",
    subtitle=(
        "These are pinned, unmodified third-party npm libraries that may select "
        "optimization work; no row is an official benchmark score."
    ),
)
MANIFEST_PATH = SPEC.manifest_path
PREVIEW_ID = SPEC.preview_id
DRIVER_PREFIX = SPEC.driver_prefix


def load_development_manifest(path: Path) -> NpmManifest:
    return load_npm_manifest(SPEC, path)


def opportunity_ids(manifest_path: Path | None = None) -> frozenset[str]:
    """The frozen set of development opportunity ids.

    This lane has no forbidden-id rule -- these ids may appear in a queue, a
    plan or a profile receipt -- so this exists for the tests that keep it
    disjoint from the held-out lane and for the queue's inventory check.
    """
    return npm_opportunity_ids(SPEC, manifest_path)


def corpus_inventory(manifest_path: Path | None = None) -> dict[str, set[str]]:
    """The frozen suite -> case inventory, checked before a report is ranked."""
    manifest = load_npm_manifest(SPEC, manifest_path or MANIFEST_PATH)
    return {suite.id: {case.id for case in suite.cases} for suite in manifest.suites}


def fetch_sources(manifest: NpmManifest, cache_root: Path) -> dict[str, int]:
    return _fetch_sources(SPEC, manifest, cache_root)


def development_status(
    result: ProcessResult, role: str, case: NpmCase
) -> tuple[str, str | None]:
    return npm_status(result, role, case)


def development_report(manifest: NpmManifest, records: list[dict[str, Any]]) -> dict[str, Any]:
    return _npm_report(SPEC, manifest, records)


DEVELOPMENT_LANE: Lane = lane(SPEC)


def run_development_preview(
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
