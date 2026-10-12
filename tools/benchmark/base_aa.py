"""Collect one frozen external case A/A using its archived base launch protocol."""
from __future__ import annotations

import argparse
from dataclasses import replace
import os
from pathlib import Path
import sys

from .base_aa_evidence import case_context
from .bundle import verify
from .external_preview import _record, _sha256_file, load_manifest
from .measure_lock import measurement_lock, wait_for_builds
from .performance_decision import _atomic_write
from .performance_evidence import replay_reports
from .performance_schema import PerformanceDecisionError, _read_json
from .planning import role_orders
from .process import run_process
from .runner import _host_metadata


def collect(summary_path, case_id, blocks, output, cwd):
    summary_path = summary_path.resolve()
    folder = summary_path.parent
    external_path = folder / "external-report.json"
    verify(summary_path, folder / "report.json", external_path, folder / "sentinel-report.json")
    replay_reports(folder / "report.json", external_path, folder / "sentinel-report.json")
    summary, summary_sha = _read_json(summary_path, "comparison summary")
    context, required, _ = case_context(external_path, case_id)
    if blocks < max(30, required):
        raise PerformanceDecisionError("base A/A requires at least the bundle's block count")
    if _host_metadata() != context["host"]:
        raise PerformanceDecisionError("base A/A host differs from bundle")
    manifest = load_manifest(folder / "external-manifest.json")
    manifest = replace(manifest, measurement=replace(manifest.measurement, blocks=blocks))
    _, suite_id, leaf_id = case_id.split("/")
    suite = next(s for s in manifest.suites if s.id == suite_id)
    case = next(c for c in suite.cases if c.id == leaf_id)
    argv = context["argv"]
    binary, workload = Path(argv[0]), Path(argv[-1])
    base_hash = summary["engines"]["base"]["binary_sha256"]
    if _sha256_file(binary) != base_hash or _sha256_file(workload) != context["bundle_sha256"]:
        raise PerformanceDecisionError("archived base executable/workload changed")
    if output.exists():
        raise PerformanceDecisionError("refusing to overwrite base A/A evidence")
    samples = []
    os.chdir(cwd)
    with measurement_lock("frozen-base case A/A"):
        busy = wait_for_builds(300)
        if busy:
            raise PerformanceDecisionError(f"competing processes remain: {busy}")
        # Same warmup policy as the external runner: one run per logical role.
        for _ in range(2):
            warm = run_process(argv, context["timeout_seconds"])
            from .external_preview import _sample_status
            if _sample_status(warm, "base")[0] != "ok":
                raise PerformanceDecisionError("base A/A warmup failed")
        for block, order in enumerate(role_orders(["candidate", "base"], blocks,
                                                  manifest.measurement.seed)):
            for index, role in enumerate(order):
                busy = wait_for_builds(300)
                if busy:
                    raise PerformanceDecisionError(f"competing processes remain: {busy}")
                if _sha256_file(binary) != base_hash:
                    raise PerformanceDecisionError("base A/A executable changed")
                measured = run_process(argv, context["timeout_seconds"])
                row = _record(manifest, suite, case, role, base_hash, workload,
                              "measurement", block, index, measured, argv)
                if row["status"] != "ok" or row["cycles"] is None or row["instructions"] is None:
                    raise PerformanceDecisionError("base A/A incomplete sample; partial scan is not evidence")
                samples.append(row)
            print(f"base A/A {case_id}: {block + 1}/{blocks}", flush=True)
        if _sha256_file(binary) != base_hash or _sha256_file(workload) != context["bundle_sha256"]:
            raise PerformanceDecisionError("base A/A asset changed during measurement")
    payload = {"schema_version": 1, "artifact_type": "quickjs-base-case-aa-evidence",
               "case_id": case_id, "base_engine": summary["engines"]["base"],
               "summary_sha256": summary_sha, "context": context, "blocks": blocks,
               "samples": samples}
    _atomic_write(output, payload)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--summary", type=Path, required=True)
    parser.add_argument("--case", required=True)
    parser.add_argument("--blocks", type=int, required=True)
    parser.add_argument("--cwd", type=Path, default=Path.cwd())
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        collect(args.summary, args.case, args.blocks, args.output.resolve(), args.cwd.resolve())
        return 0
    except (ValueError, OSError, KeyError, TypeError, StopIteration) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
