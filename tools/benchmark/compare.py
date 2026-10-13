"""Run one complete three-engine comparison from prebuilt receipts.

This local entry point measures broad specializer coverage, generic sentinels,
and external process-wall workloads on the same host. It never builds engines
or silently replaces a failed sample. Hosted previews keep their existing path.

The lanes run one after another, or with `--concurrent-lanes` side by side:
one forked process per lane, each measuring its own lane in its usual order.
`--workers N` instead shares the cases of all three lanes among N processes:
the broad cases in N // 2 shards, the sentinels in one, and each external
case on its own, longest first. A case's three engines always run in the
same process, in the order the sequential schedule gives them.
The files and the analysis are the same; only wall-clock overlap differs, and
the summary records it as `lane_schedule`. Use either on a host with an idle
core per process whose ratios were shown not to move (docs/performance-workflow.md).
"""
from __future__ import annotations

import argparse
import json
import multiprocessing
import shutil
import sys
import tempfile
from dataclasses import replace
from functools import partial
from pathlib import Path

from .adapters import load_engine
from .analysis_schema import load_analysis_manifest
from .external_preview import PreviewRun, load_manifest as load_external, run_preview
from .external_preview_markdown import render_markdown
from .measure_lock import measurement_lock, wait_for_builds
from .performance_evidence import (
    ROOT, ROLES, validate_bundle, validate_internal, comparison_index, interval, inventories,
    promotion_metric,
)
from .preview import _engine_provenance, _write_replace
from .receipts import load_receipt
from .report import build_report, write_report
from .runner import BenchmarkRun, JsonlWriter
from .schema import load_manifest
from .shards import ShardedRun
from .worker_pool import Task, run_tasks


def _engines(args, manifest):
    engines = []
    for role in ROLES:
        name = role.replace("-", "_")
        identity = "quickjs-ng" if role == "quickjs-ng" else "qjs-rust"
        adapter = "qjs-file" if role == "quickjs-ng" else "qjs-rust-raw"
        engine = load_engine(role, adapter, identity, getattr(args, name))
        receipt = load_receipt(
            getattr(args, name + "_receipt"), expected_binary_sha256=engine.binary_sha256,
            expected_engine_identity=identity, expected_profile_id=manifest.profile.id,
            expected_recipe=manifest.build_recipes[identity],
            pinned_reference=(manifest.reference_identity, manifest.reference_repo,
                              manifest.reference_revision) if role == "quickjs-ng" else None,
        )
        engines.append(replace(engine, receipt=receipt))
    return engines


def summary_for(broad, external, sentinel):
    summary = {"schema_version": 2, "state": "success", "claim_eligible": False,
               "classification": "same_host_diagnostic_not_fixed_hardware_claim",
               "engines": _engine_provenance(broad["run"])}
    reasons = validate_bundle(summary, broad, external, sentinel)
    metric = promotion_metric(broad, external, sentinel)
    summary["promotion_metric"] = metric
    rows = comparison_index(broad, external, sentinel, metric)
    imprecise = [key for key, row in rows.items() if not interval(row, key)[2]]
    if len(rows) != sum(len(cases) for cases in inventories().values()):
        reasons.append("missing candidate/base comparisons")
    if any(s.get("complete_comparison") is not True for s in external["suites"]):
        reasons.append("missing candidate/QuickJS-NG comparisons")
    summary["measurement_issues"] = reasons
    summary["imprecise_cases"] = imprecise
    summary["decision_readiness"] = "inconclusive" if reasons or imprecise else "ready_for_unit_gates"
    return summary


def markdown_for(summary, broad, external, sentinel):
    lines = ["# Three-engine comparison", "",
             "Ratios are candidate/comparator; lower is faster. Informational, not a fixed-hardware claim.",
             "Broad measures specializer coverage; sentinels measure dynamic workloads; external measures whole-process latency.",
             "These lanes are never pooled into a headline engine score.", "",
             f"Decision readiness: **{summary['decision_readiness']}**; "
             f"promotion metric: **{summary['promotion_metric']}**.", ""]
    for name, report in (("Specializer coverage", broad), ("Generic-path sentinels", sentinel)):
        lane = "broad" if report is broad else "sentinel"
        issues = validate_internal(report, summary["engines"], lane)
        if issues:
            lines += [f"## {name}", "", "No ratios: " + "; ".join(issues) + ".", ""]
            continue
        lines += [f"## {name}", "",
                  "| Case | Candidate/base (95% CI) | Candidate/NG (95% CI) | Cycles candidate/base (95% CI) |",
                  "|---|---:|---:|---:|"]
        base = report["comparisons"]["candidate_vs_base"] or {"cases": {}}
        ng = report["comparisons"]["candidate_vs_quickjs_ng"] or {"cases": {}}
        cycles = report["counter_comparisons"]["cycles"]["candidate_vs_base"] or {"cases": {}}
        for case in base["cases"]:
            cells = []
            for comparison in (base, ng, cycles):
                row = comparison["cases"].get(case)
                if row is None:
                    cells.append("missing")
                else:
                    ci = row["confidence_interval"]
                    cells.append(f"{row['ratio']:.4f} [{ci['lower']:.4f}, {ci['upper']:.4f}]")
            lines.append(f"| `{case}` | " + " | ".join(cells) + " |")
        lines.append("")
    lines.append(render_markdown(external))
    return "\n".join(lines)


def run(args):
    broad_manifest = load_manifest(args.manifest)
    sentinel_manifest = load_manifest(args.sentinel_manifest)
    # Fail before measurement if any role/recipe differs between lanes.
    broad_engines = _engines(args, broad_manifest)
    sentinel_engines = _engines(args, sentinel_manifest)
    args.output_dir.mkdir(parents=True, exist_ok=False)
    with measurement_lock("compare"):
        competing = wait_for_builds(args.settle)
        if competing:
            print(f"compare: warning: competing processes still running: {', '.join(competing[:8])}",
                  file=sys.stderr, flush=True)
        return _measure(args, broad_manifest, sentinel_manifest, broad_engines, sentinel_engines)


LANES = ("broad", "sentinel", "external")
_PREFIX = {"broad": "", "sentinel": "sentinel-"}


def _measure_lane(lane, args, manifests, engines):
    """Measure one lane into `args.output_dir` and return its report."""
    if lane == "external":
        return run_preview(load_external(ROOT / "benchmarks/external-preview.json"),
                           args.external_cache, args.output_dir / "external-work",
                           args.output_dir, args.candidate, args.base, args.quickjs_ng,
                           blocks=args.blocks)
    prefix, manifest = _PREFIX[lane], manifests[lane]
    print(f"compare: {prefix}measurement ({args.blocks} blocks)", file=sys.stderr, flush=True)
    raw = args.output_dir / f"{prefix}raw.jsonl"
    (args.output_dir / f"{prefix}manifest.json").write_bytes(manifest.path.read_bytes())
    with raw.open("x") as stream:
        success = BenchmarkRun(manifest, engines[lane], list(manifest.cases), args.blocks,
                               args.seed, JsonlWriter(stream), ROOT).execute()
    return _internal_report(lane, args, manifest, success)


def _internal_report(lane, args, manifest, success):
    prefix = _PREFIX[lane]
    raw = args.output_dir / f"{prefix}raw.jsonl"
    if not success:
        raise ValueError(f"{prefix}measurement failed; raw evidence preserved, no conclusion")
    analysis = load_analysis_manifest(ROOT / "benchmarks/analysis.json", manifest)
    report = build_report(raw, manifest, analysis)
    write_report(args.output_dir / f"{prefix}report.json", report)
    return report


def _lane_process(lane, args, manifests, engines):
    try:
        _measure_lane(lane, args, manifests, engines)
    except Exception as error:  # the parent reports the lane; keep its reason visible
        print(f"compare: {lane} lane failed: {error}", file=sys.stderr, flush=True)
        raise SystemExit(1)


def _measure_concurrently(args, manifests, engines):
    """One forked process per lane, so no lane's timer waits on another's
    interpreter; a fork also inherits withheld counters (cycle_trust.py)."""
    context = multiprocessing.get_context("fork")
    processes = {lane: context.Process(target=_lane_process, args=(lane, args, manifests, engines))
                 for lane in LANES}
    for process in processes.values():
        process.start()
    for process in processes.values():
        process.join()
    failed = [lane for lane, process in processes.items() if process.exitcode != 0]
    if failed:
        raise ValueError(f"{', '.join(failed)} lane failed; raw evidence preserved, no conclusion")
    names = {"broad": "report.json", "sentinel": "sentinel-report.json", "external": "external-report.json"}
    return {lane: json.loads((args.output_dir / names[lane]).read_text(encoding="utf-8")) for lane in LANES}


def _read_samples(path):
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]


def _write_samples(path, samples):
    path.write_text("".join(json.dumps(sample) + "\n" for sample in samples), encoding="utf-8")


def _measure_shard(run, case_ids, path):
    with path.open("x") as stream:
        run.measure_shard(case_ids, JsonlWriter(stream))


def _measure_with_workers(args, manifests, engines):
    """Share the three lanes' cases among `args.workers` processes."""
    scratch = Path(tempfile.mkdtemp(prefix="qjs-compare-workers."))
    runs, streams, shards, preview = {}, {}, {}, None
    try:
        tasks = []
        for lane, count in (("broad", max(1, args.workers // 2)), ("sentinel", 1)):
            prefix, manifest = _PREFIX[lane], manifests[lane]
            print(f"compare: {prefix}measurement ({args.blocks} blocks)", file=sys.stderr, flush=True)
            (args.output_dir / f"{prefix}manifest.json").write_bytes(manifest.path.read_bytes())
            streams[lane] = (args.output_dir / f"{prefix}raw.jsonl").open("x")
            runs[lane] = ShardedRun(manifest, engines[lane], list(manifest.cases), args.blocks,
                                    args.seed, JsonlWriter(streams[lane]), ROOT)
            case_ids = [case.id for case in manifest.cases]
            shards[lane] = [scratch / f"{lane}-{index}.jsonl" for index in range(count)]
            tasks += [Task(f"{lane} shard {index + 1}",
                           partial(_measure_shard, runs[lane], case_ids[index::count], path))
                      for index, path in enumerate(shards[lane])]
        preview = PreviewRun(load_external(ROOT / "benchmarks/external-preview.json"),
                             args.external_cache, args.output_dir / "external-work",
                             args.candidate, args.base, args.quickjs_ng, blocks=args.blocks)
        cases = range(len(preview.cases))
        capability = [scratch / f"external-capability-{index}.jsonl" for index in cases]
        measured = [scratch / f"external-{index}.jsonl" for index in cases]

        def probe():
            for index in cases:
                _write_samples(capability[index], preview.capability(index))

        def measure(index):
            _write_samples(measured[index],
                           preview.measurement(index, _read_samples(capability[index])))

        def external_cases():
            # Longest first, by the capability runs, so the last worker to
            # finish is not the one that drew the longest case late.
            cost = {index: sum(sample["duration_ns"] for sample in _read_samples(capability[index]))
                    for index in cases}
            return [Task(f"external case {preview.cases[index][1].id}", partial(measure, index))
                    for index in sorted(cases, key=lambda index: -cost[index])]

        tasks.append(Task("external capability", probe, external_cases))
        run_tasks(tasks, args.workers)
        reports = {}
        for lane, run in runs.items():
            success = run.merge_shards([sample for path in shards[lane] for sample in _read_samples(path)])
            streams[lane].close()
            reports[lane] = _internal_report(lane, args, manifests[lane], success)
        reports["external"] = preview.finish(
            [sample for index in cases for path in (capability[index], measured[index])
             for sample in _read_samples(path)], args.output_dir)
        return reports
    finally:
        for run in runs.values():
            run.close()
        for stream in streams.values():
            stream.close()
        if preview is not None:
            preview.close()
        shutil.rmtree(scratch, ignore_errors=True)


def _measure(args, broad_manifest, sentinel_manifest, broad_engines, sentinel_engines):
    phase = "initialization"
    manifests = {"broad": broad_manifest, "sentinel": sentinel_manifest}
    engines = {"broad": broad_engines, "sentinel": sentinel_engines}
    concurrent = getattr(args, "concurrent_lanes", False)
    workers = getattr(args, "workers", None)

    def status(state, error=None):
        payload = {"state": state, "phase": phase, "claim_eligible": False, "error": error}
        _write_replace(args.output_dir / "status.json", (json.dumps(payload) + "\n").encode())

    try:
        status("running")
        if workers:
            phase = f"measurement on {workers} workers"
            status("running")
            reports = _measure_with_workers(args, manifests, engines)
        elif concurrent:
            phase = "concurrent measurement"
            status("running")
            reports = _measure_concurrently(args, manifests, engines)
        else:
            reports = {}
            for lane in LANES:
                phase = f"{lane} measurement"
                status("running")
                reports[lane] = _measure_lane(lane, args, manifests, engines)
        phase = "summary"
        summary = summary_for(reports["broad"], reports["external"], reports["sentinel"])
        summary["lane_schedule"] = (f"workers-{workers}" if workers
                                    else "concurrent" if concurrent else "sequential")
        write_report(args.output_dir / "summary.json", summary)
        (args.output_dir / "summary.md").write_text(
            markdown_for(summary, reports["broad"], reports["external"], reports["sentinel"]))
        from .bundle import seal
        summary = seal(args.output_dir)
        status("success")
        return summary
    except Exception as error:
        status("failed", str(error))
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for role in ROLES:
        parser.add_argument(f"--{role}", type=Path, required=True)
        parser.add_argument(f"--{role}-receipt", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=ROOT / "benchmarks/manifest.json")
    parser.add_argument("--sentinel-manifest", type=Path, default=ROOT / "benchmarks/generic-sentinels-manifest.json")
    parser.add_argument("--external-cache", type=Path, default=ROOT / "target/benchmarks/external-cache")
    parser.add_argument("--blocks", type=int, choices=(3, 30, 60), default=30)
    parser.add_argument("--seed", type=int, default=20250713)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--concurrent-lanes", action="store_true",
                        help="measure the three lanes side by side, one process each")
    parser.add_argument("--workers", type=int, choices=range(2, 9), metavar="N",
                        help="share the cases of all lanes among N processes (2 to 8)")
    parser.add_argument("--settle", type=float, default=300.0,
                        help="seconds to wait for running builds or engines to finish before measuring")
    try:
        args = parser.parse_args()
        if args.workers and args.concurrent_lanes:
            parser.error("--workers and --concurrent-lanes are two schedules; pass one")
        result = run(args)
        print(json.dumps(result, sort_keys=True))
        return 0
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
