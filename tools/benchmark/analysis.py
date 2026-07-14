"""Deterministic statistics for validated benchmark runs."""

from __future__ import annotations

import math
import statistics
from collections import defaultdict
from typing import Any

from .analysis_schema import AnalysisManifest
from .health import block_health, precision_health
from .raw_validation import ReportError, ValidatedRun
from .schema import Manifest
from .statistics import paired_block_bootstrap, relative_half_width


def _comparison(
    measurements: dict[tuple[str, str, int], float],
    manifest: Manifest,
    analysis: AnalysisManifest,
    comparator: str,
) -> dict[str, Any]:
    case_blocks = {}
    case_results = {}
    for case in manifest.cases:
        logs = {}
        candidate_values = []
        comparator_values = []
        blocks = sorted(
            block for role, case_id, block in measurements
            if role == "candidate" and case_id == case.id
        )
        for block in blocks:
            candidate = measurements[("candidate", case.id, block)]
            baseline = measurements[(comparator, case.id, block)]
            candidate_values.append(candidate)
            comparator_values.append(baseline)
            logs[block] = math.log(candidate / baseline)
        case_blocks[case.id] = logs
        lower, upper = paired_block_bootstrap(
            {case.id: logs},
            samples=analysis.bootstrap_samples,
            seed=analysis.bootstrap_seed,
            confidence=analysis.confidence,
        )
        estimate = math.exp(statistics.median(logs.values()))
        case_results[case.id] = {
            "critical": case.critical,
            "family": case.family,
            "candidate_median_ns_per_op": statistics.median(candidate_values),
            "comparator_median_ns_per_op": statistics.median(comparator_values),
            "ratio": estimate,
            "confidence_interval": {"lower": lower, "upper": upper},
            "relative_half_width": relative_half_width(estimate, lower, upper),
        }

    family_cases: dict[str, list[str]] = defaultdict(list)
    for case in manifest.cases:
        family_cases[case.family].append(case.id)
    family_results = {}
    for family in sorted(family_cases):
        ids = family_cases[family]
        subset = {case_id: case_blocks[case_id] for case_id in ids}
        case_logs = [statistics.median(case_blocks[case_id].values()) for case_id in ids]
        lower, upper = paired_block_bootstrap(
            subset,
            samples=analysis.bootstrap_samples,
            seed=analysis.bootstrap_seed,
            confidence=analysis.confidence,
        )
        estimate = math.exp(statistics.fmean(case_logs))
        family_results[family] = {
            "cases": ids,
            "ratio": estimate,
            "confidence_interval": {"lower": lower, "upper": upper},
            "relative_half_width": relative_half_width(estimate, lower, upper),
        }
    all_case_logs = [statistics.median(case_blocks[case.id].values()) for case in manifest.cases]
    lower, upper = paired_block_bootstrap(
        case_blocks,
        samples=analysis.bootstrap_samples,
        seed=analysis.bootstrap_seed,
        confidence=analysis.confidence,
    )
    overall_estimate = math.exp(statistics.fmean(all_case_logs))
    return {
        "comparator_role": comparator,
        "cases": case_results,
        "families": family_results,
        "overall": {
            "cases": [case.id for case in manifest.cases],
            "ratio": overall_estimate,
            "confidence_interval": {"lower": lower, "upper": upper},
            "relative_half_width": relative_half_width(overall_estimate, lower, upper),
        },
    }


def analyze_run(
    validated: ValidatedRun,
    measurement: Manifest,
    analysis: AnalysisManifest,
) -> dict[str, Any]:
    analysis.assert_compatible(measurement)
    linearity_results = {}
    health_counts = {"pass": 0, "fail": 0, "inconclusive": 0}
    failure_by_pair = {
        (failure["role"], failure["case_id"]): failure
        for failure in validated.diagnostic_failures
    }
    for role in ("candidate", "base", "quickjs-ng"):
        role_results = {}
        for case in measurement.cases:
            pair = (role, case.id)
            if pair in failure_by_pair:
                health_counts["fail"] += 1
                role_results[case.id] = {
                    "iterations_n": None,
                    "iterations_2n": None,
                    "normalized_per_op_ratio": None,
                    "status": "fail",
                    "execution_failure": failure_by_pair[pair],
                }
                continue
            n = validated.linearity[(role, case.id, "n")]
            twice = validated.linearity[(role, case.id, "2n")]
            n_iterations = n["iterations"]
            twice_iterations = twice["iterations"]
            if twice_iterations != n_iterations * 2:
                raise ReportError(f"linearity iterations are not exact 2x for {role}/{case.id}")
            startup_ns = statistics.median(validated.startup[(role, case.id)])
            n_adjusted = n["duration_ns"] - startup_ns
            twice_adjusted = twice["duration_ns"] - startup_ns
            if n_adjusted <= 0 or twice_adjusted <= 0:
                ratio = None
                status = "inconclusive"
            else:
                ratio = (
                    (twice_adjusted / twice["operations"])
                    / (n_adjusted / n["operations"])
                )
                status = (
                    "pass"
                    if analysis.linearity_lower <= ratio <= analysis.linearity_upper
                    else "fail"
                )
            health_counts[status] += 1
            role_results[case.id] = {
                "iterations_n": n_iterations,
                "iterations_2n": twice_iterations,
                "normalized_per_op_ratio": ratio,
                "status": status,
                "execution_failure": None,
            }
        linearity_results[role] = role_results
    linearity_status = (
        "fail" if health_counts["fail"] else
        "inconclusive" if health_counts["inconclusive"] else "pass"
    )

    block_result = block_health(
        validated.blocks,
        (item["block"] for item in validated.invalid_blocks),
        analysis.health,
    )
    comparisons = {
        "candidate_vs_base": None,
        "candidate_vs_quickjs_ng": None,
    }
    critical_widths = []
    if validated.valid_blocks:
        comparisons = {
            "candidate_vs_base": _comparison(
                validated.measurements, measurement, analysis, "base"
            ),
            "candidate_vs_quickjs_ng": _comparison(
                validated.measurements, measurement, analysis, "quickjs-ng"
            ),
        }
        critical_families = {
            case.family for case in measurement.cases if case.critical
        }
        for comparison in comparisons.values():
            if comparison is not None:
                critical_widths.extend(
                    comparison["families"][family]["relative_half_width"]
                    for family in sorted(critical_families)
                )
    prerequisites_healthy = (
        block_result["status"] != "invalid"
        and linearity_status == "pass"
        and bool(validated.valid_blocks)
    )
    precision = precision_health(
        validated.blocks, critical_widths, prerequisites_healthy, analysis.health
    )

    return {
        "health": {
            "input_valid": True,
            "policy": {
                "initial_blocks": analysis.health.initial_blocks,
                "extension_blocks": analysis.health.extension_blocks,
                "max_blocks": analysis.health.max_blocks,
                "max_relative_half_width": analysis.health.max_relative_half_width,
                "max_invalid_block_fraction": analysis.health.max_invalid_block_fraction,
                "block_invalidation": analysis.health.block_invalidation,
                "outlier_policy": analysis.health.outlier_policy,
                "retry_policy": analysis.health.retry_policy,
            },
            "blocks": {
                "requested": validated.blocks,
                "attempted": validated.blocks,
                "valid": len(validated.valid_blocks),
                "invalid": len(validated.invalid_blocks),
                "valid_block_ids": list(validated.valid_blocks),
                "invalid_blocks": list(validated.invalid_blocks),
                **block_result,
            },
            "linearity": {
                "bounds": {
                    "lower": analysis.linearity_lower,
                    "upper": analysis.linearity_upper,
                },
                "counts": health_counts,
                "status": linearity_status,
                "roles": linearity_results,
            },
            "precision": precision,
            "status": precision["status"],
        },
        "comparisons": comparisons,
    }
