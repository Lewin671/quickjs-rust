#!/usr/bin/env python3
"""Aggregate Test262 case results into a coverage summary and burndown entry.

Combines a QuickJS-NG baseline (cases-*.jsonl from
`test262-baseline.sh --engine quickjs-ng`) with a quickjs-rust scan
(cases-*.jsonl from `test262-baseline.sh --engine quickjs-rust`) into the
markdown coverage summary used by the Test262 Coverage workflow and the
schema-1 burndown entry consumed by `test262-burndown.sh --entry`.

Runs anywhere the two case-result sets are available, including locally on
artifacts downloaded with `gh run download`.
"""

import argparse
import datetime
import glob
import json
import sys


def load_cases(pattern):
    cases = {}
    paths = sorted(glob.glob(pattern))
    if not paths:
        raise SystemExit(f"no case result files matched {pattern}")
    for path in paths:
        with open(path, "r", encoding="utf-8") as handle:
            for line in handle:
                if not line.strip():
                    continue
                row = json.loads(line)
                cases[row["path"]] = row
    return cases, paths


TOTAL_KEYS = [
    "total",
    "configured",
    "eligible",
    "run",
    "skipped",
    "rust_not_run",
    "rust_pass",
    "rust_fail",
    "rust_timeout",
    "rust_skipped",
    "qjsng_pass",
    "qjsng_fail",
    "qjsng_timeout",
    "qjsng_skipped",
    "both_pass",
    "qjsng_pass_rust_nonpass",
    "qjsng_pass_rust_not_run",
    "qjsng_pass_rust_fail",
    "qjsng_pass_rust_timeout",
    "rust_pass_qjsng_nonpass",
    "both_nonpass",
    "both_fail_or_timeout",
]

SKIP_KEYS = [
    "async",
    "features",
    "fixture",
    "includes",
    "intl402",
    "missing",
    "module",
    "negative",
    "raw",
    "syntax",
]


def aggregate(qjsng_cases, rust_cases):
    totals = dict.fromkeys(TOTAL_KEYS, 0)
    skipped = dict.fromkeys(SKIP_KEYS, 0)

    def skipped_bucket(reason):
        if reason == "fixture":
            return "fixture"
        return "features"

    totals["total"] = len(qjsng_cases)
    totals["run"] = len(rust_cases)
    for path, qjsng in qjsng_cases.items():
        rust = rust_cases.get(path, {"rust": "skipped", "rust_skip": "missing"})
        qkind = qjsng["quickjs_ng"]
        rkind = rust["rust"]
        if qkind == "skipped":
            totals["skipped"] += 1
            skipped[skipped_bucket(qjsng.get("quickjs_ng_skip", ""))] += 1
            continue

        totals["configured"] += 1
        if qkind == "pass":
            totals["qjsng_pass"] += 1
        elif qkind == "timeout":
            totals["qjsng_timeout"] += 1
        else:
            totals["qjsng_fail"] += 1

        if rkind == "pass":
            totals["rust_pass"] += 1
        elif rkind == "timeout":
            totals["rust_timeout"] += 1
        elif rkind == "skipped":
            totals["rust_skipped"] += 1
            totals["rust_not_run"] += 1
            reason = rust.get("rust_skip", "")
            if reason in skipped:
                skipped[reason] += 1
            else:
                skipped["missing"] += 1
        else:
            totals["rust_fail"] += 1

        if rkind != "skipped":
            totals["eligible"] += 1

        if rkind == "pass" and qkind == "pass":
            totals["both_pass"] += 1
        elif rkind == "pass":
            totals["rust_pass_qjsng_nonpass"] += 1
        elif qkind == "pass":
            totals["qjsng_pass_rust_nonpass"] += 1
            if rkind == "skipped":
                totals["qjsng_pass_rust_not_run"] += 1
            elif rkind == "timeout":
                totals["qjsng_pass_rust_timeout"] += 1
            else:
                totals["qjsng_pass_rust_fail"] += 1
        else:
            totals["both_nonpass"] += 1
            if rkind != "skipped":
                totals["both_fail_or_timeout"] += 1

    return totals, skipped


def comparison_kind(qkind, rkind):
    if qkind == "skipped":
        return "quickjs-ng-skipped"
    if rkind == "pass" and qkind == "pass":
        return "both-pass"
    if rkind == "pass":
        return "rust-pass-quickjs-ng-nonpass"
    if qkind == "pass":
        if rkind == "skipped":
            return "quickjs-ng-pass-rust-not-run"
        if rkind == "timeout":
            return "quickjs-ng-pass-rust-timeout"
        return "quickjs-ng-pass-rust-fail"
    if rkind == "skipped":
        return "both-nonpass-rust-not-run"
    return "both-fail-or-timeout"


def write_comparison_cases(path, qjsng_cases, rust_cases):
    with open(path, "w", encoding="utf-8") as handle:
        for case_path in sorted(qjsng_cases):
            qjsng = qjsng_cases[case_path]
            rust = rust_cases.get(case_path, {"rust": "skipped", "rust_skip": "missing"})
            qkind = qjsng["quickjs_ng"]
            rkind = rust["rust"]
            row = {
                "path": case_path,
                "quickjs_ng": qkind,
                "quickjs_ng_result": qjsng.get("quickjs_ng_result", qkind),
                "quickjs_ng_skip": qjsng.get("quickjs_ng_skip", ""),
                "rust": rkind,
                "rust_result": rust.get("rust_result", rkind),
                "rust_skip": rust.get("rust_skip", ""),
                "comparison": comparison_kind(qkind, rkind),
                "actionable_gap": qkind == "pass" and rkind in {"fail", "timeout"},
            }
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


def pct(value, denominator):
    if denominator == 0:
        return "n/a"
    return f"{value / denominator * 100:.2f}%"


def summary_markdown(totals, skipped, commit, rust_paths, qjsng_paths):
    lines = [
        "# Test262 Coverage",
        "",
        f"Aggregated `{len(rust_paths)}` rust shard result files against "
        f"`{len(qjsng_paths)}` cached QuickJS-NG baseline files for `{commit}`.",
        "",
        "## Corpus Scope",
        "",
        "| Metric | Count | Share |",
        "| --- | ---: | ---: |",
        f"| Upstream cases | {totals['total']} | 100.00% |",
        f"| Skipped by QuickJS-NG config | {totals['skipped']} | {pct(totals['skipped'], totals['total'])} |",
        f"| Comparison baseline | {totals['configured']} | {pct(totals['configured'], totals['total'])} |",
        "",
        "## quickjs-rust Readiness",
        "",
        "| Metric | Count | Share of Baseline |",
        "| --- | ---: | ---: |",
        f"| Runnable by quickjs-rust harness | {totals['eligible']} | {pct(totals['eligible'], totals['configured'])} |",
        f"| Not run by quickjs-rust harness | {totals['rust_not_run']} | {pct(totals['rust_not_run'], totals['configured'])} |",
        "",
        "## Engine Results",
        "",
        "| Engine | Pass | Fail | Timeout | Not Run | Baseline Pass Rate | Runnable Pass Rate |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
        f"| quickjs-rust | {totals['rust_pass']} | {totals['rust_fail']} | {totals['rust_timeout']} | "
        f"{totals['rust_skipped']} | {pct(totals['rust_pass'], totals['configured'])} | "
        f"{pct(totals['rust_pass'], totals['eligible'])} |",
        f"| QuickJS-NG | {totals['qjsng_pass']} | {totals['qjsng_fail']} | {totals['qjsng_timeout']} | 0 | "
        f"{pct(totals['qjsng_pass'], totals['configured'])} | {pct(totals['qjsng_pass'], totals['configured'])} |",
        "",
        "## Coverage Difference",
        "",
        "| Comparison | Count | Share of Baseline |",
        "| --- | ---: | ---: |",
        f"| Both pass | {totals['both_pass']} | {pct(totals['both_pass'], totals['configured'])} |",
        f"| QuickJS-NG passes, quickjs-rust not run | {totals['qjsng_pass_rust_not_run']} | "
        f"{pct(totals['qjsng_pass_rust_not_run'], totals['configured'])} |",
        f"| QuickJS-NG passes, quickjs-rust fails | {totals['qjsng_pass_rust_fail']} | "
        f"{pct(totals['qjsng_pass_rust_fail'], totals['configured'])} |",
        f"| QuickJS-NG passes, quickjs-rust timeouts | {totals['qjsng_pass_rust_timeout']} | "
        f"{pct(totals['qjsng_pass_rust_timeout'], totals['configured'])} |",
        f"| quickjs-rust passes, QuickJS-NG does not | {totals['rust_pass_qjsng_nonpass']} | "
        f"{pct(totals['rust_pass_qjsng_nonpass'], totals['configured'])} |",
        f"| Both fail or timeout | {totals['both_fail_or_timeout']} | "
        f"{pct(totals['both_fail_or_timeout'], totals['configured'])} |",
        "",
        "## Skipped / Not-Run Reasons",
        "",
        "| Reason | Count |",
        "| --- | ---: |",
    ]
    for key in sorted(skipped):
        lines.append(f"| {key} | {skipped[key]} |")
    return "\n".join(lines) + "\n"


def burndown_entry(totals, commit, recorded):
    return {
        "schema": 1,
        "recorded": recorded,
        "commit": commit,
        "source": "ci-aggregate",
        "total": totals["total"],
        "ng_config_skipped": totals["skipped"],
        "configured": totals["configured"],
        "rust": {
            "pass": totals["rust_pass"],
            "fail": totals["rust_fail"],
            "timeout": totals["rust_timeout"],
            "not_run": totals["rust_skipped"],
        },
        "ng": {
            "pass": totals["qjsng_pass"],
            "fail": totals["qjsng_fail"],
            "timeout": totals["qjsng_timeout"],
        },
        "comparison": {
            "both_pass": totals["both_pass"],
            "actionable_gap": totals["qjsng_pass_rust_fail"] + totals["qjsng_pass_rust_timeout"],
            "ng_pass_rust_fail": totals["qjsng_pass_rust_fail"],
            "ng_pass_rust_timeout": totals["qjsng_pass_rust_timeout"],
            "ng_pass_rust_not_run": totals["qjsng_pass_rust_not_run"],
            "rust_pass_ng_nonpass": totals["rust_pass_qjsng_nonpass"],
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ng-cases", required=True, help="glob for QuickJS-NG cases-*.jsonl files")
    parser.add_argument("--rust-cases", required=True, help="glob for quickjs-rust cases-*.jsonl files")
    parser.add_argument("--commit", required=True, help="commit sha the scan measured")
    parser.add_argument("--summary-out", help="append the markdown summary to this file")
    parser.add_argument("--burndown-out", help="write the schema-1 burndown JSON line to this file")
    parser.add_argument("--comparison-cases-out", help="write merged per-case comparison JSONL")
    parser.add_argument(
        "--recorded",
        default=datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d"),
        help="recorded date for the burndown entry (default: today UTC)",
    )
    args = parser.parse_args()

    qjsng_cases, qjsng_paths = load_cases(args.ng_cases)
    rust_cases, rust_paths = load_cases(args.rust_cases)
    totals, skipped = aggregate(qjsng_cases, rust_cases)
    if args.comparison_cases_out:
        write_comparison_cases(args.comparison_cases_out, qjsng_cases, rust_cases)

    markdown = summary_markdown(totals, skipped, args.commit, rust_paths, qjsng_paths)
    if args.summary_out:
        with open(args.summary_out, "a", encoding="utf-8") as handle:
            handle.write(markdown)
    else:
        sys.stdout.write(markdown)

    entry = json.dumps(burndown_entry(totals, args.commit, args.recorded), separators=(",", ":"))
    if args.burndown_out:
        with open(args.burndown_out, "w", encoding="utf-8") as handle:
            handle.write(entry + "\n")
    print(entry)


if __name__ == "__main__":
    main()
