"""Run one complete, sequential three-engine comparison from prebuilt receipts.

This local entry point measures broad specializer coverage, generic sentinels,
and external process-wall workloads on the same host. It never builds engines
or silently replaces a failed sample. Hosted previews keep their existing path.
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import replace
from pathlib import Path

from .adapters import load_engine
from .analysis_schema import load_analysis_manifest
from .external_preview import load_manifest as load_external, run_preview
from .external_preview_markdown import render_markdown
from .performance_evidence import ROOT, ROLES, validate_bundle, validate_internal, comparison_index, interval, inventories
from .preview import _engine_provenance, _write_replace
from .receipts import load_receipt
from .report import build_report, write_report
from .runner import BenchmarkRun, JsonlWriter
from .schema import load_manifest


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
    rows = comparison_index(broad, external, sentinel)
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
             f"Decision readiness: **{summary['decision_readiness']}**.", ""]
    for name, report in (("Specializer coverage", broad), ("Generic-path sentinels", sentinel)):
        lane = "broad" if report is broad else "sentinel"
        issues = validate_internal(report, summary["engines"], lane)
        if issues:
            lines += [f"## {name}", "", "No ratios: " + "; ".join(issues) + ".", ""]
            continue
        lines += [f"## {name}", "", "| Case | Candidate/base (95% CI) | Candidate/NG (95% CI) |",
                  "|---|---:|---:|"]
        base = report["comparisons"]["candidate_vs_base"] or {"cases": {}}
        ng = report["comparisons"]["candidate_vs_quickjs_ng"] or {"cases": {}}
        for case in base["cases"]:
            cells = []
            for comparison in (base, ng):
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
    phase = "initialization"

    def status(state, error=None):
        payload = {"state": state, "phase": phase, "claim_eligible": False, "error": error}
        _write_replace(args.output_dir / "status.json", (json.dumps(payload) + "\n").encode())

    try:
        status("running")
        reports = []
        for prefix, manifest, engines in (("", broad_manifest, broad_engines),
                                          ("sentinel-", sentinel_manifest, sentinel_engines)):
            phase = prefix + "measurement"
            print(f"compare: {phase} ({args.blocks} blocks)", file=sys.stderr, flush=True)
            status("running")
            raw = args.output_dir / f"{prefix}raw.jsonl"
            (args.output_dir / f"{prefix}manifest.json").write_bytes(manifest.path.read_bytes())
            with raw.open("x") as stream:
                success = BenchmarkRun(manifest, engines, list(manifest.cases), args.blocks,
                                       args.seed, JsonlWriter(stream), ROOT).execute()
            if not success:
                raise ValueError(f"{phase} failed; raw evidence preserved, no conclusion")
            phase = prefix + "analysis"
            analysis = load_analysis_manifest(ROOT / "benchmarks/analysis.json", manifest)
            report = build_report(raw, manifest, analysis)
            write_report(args.output_dir / f"{prefix}report.json", report)
            reports.append(report)
        phase = "external measurement"
        status("running")
        external = run_preview(load_external(ROOT / "benchmarks/external-preview.json"),
                               args.external_cache, args.output_dir / "external-work",
                               args.output_dir, args.candidate, args.base, args.quickjs_ng,
                               blocks=args.blocks)
        phase = "summary"
        summary = summary_for(reports[0], external, reports[1])
        write_report(args.output_dir / "summary.json", summary)
        (args.output_dir / "summary.md").write_text(markdown_for(summary, reports[0], external, reports[1]))
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
    try:
        result = run(parser.parse_args())
        print(json.dumps(result, sort_keys=True))
        return 0
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
