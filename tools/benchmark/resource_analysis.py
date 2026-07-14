"""Deterministic paired analysis for validated resource lanes."""

from __future__ import annotations

import math
import statistics
from typing import Any

from .resource_analysis_schema import ResourceAnalysisManifest, ResourceHealthPolicy
from .resource_schema import ResourceManifest
from .resource_validation import ValidatedResourceRun
from .statistics import paired_block_bootstrap, relative_half_width


WIDTH_REL_TOL = 1e-12
WIDTH_ABS_TOL = 1e-15


def _block_health(
    blocks: int, invalid_ids: list[int], policy: ResourceHealthPolicy
) -> dict[str, Any]:
    initial = [block for block in invalid_ids if block < policy.initial_blocks]
    initial_limit = math.floor(policy.initial_blocks * policy.max_invalid_block_fraction)
    total_limit = math.floor(blocks * policy.max_invalid_block_fraction)
    standard = blocks in {policy.initial_blocks, policy.max_blocks}
    exceeded = len(initial) > initial_limit or (standard and len(invalid_ids) > total_limit)
    return {
        "status": "invalid" if exceeded else ("pass" if standard else "non_claim"),
        "standard_cohort": standard,
        "initial_invalid_blocks": initial,
        "initial_invalid_limit": initial_limit,
        "total_invalid_limit": total_limit if standard else None,
        "initial_limit_exceeded": len(initial) > initial_limit,
        "total_limit_exceeded": standard and len(invalid_ids) > total_limit,
    }


def _precision(
    blocks: int, widths: list[float], prerequisites: bool, policy: ResourceHealthPolicy
) -> dict[str, Any]:
    maximum = max(widths, default=None)
    boundary = maximum is not None and math.isclose(
        maximum, policy.max_relative_half_width,
        rel_tol=WIDTH_REL_TOL, abs_tol=WIDTH_ABS_TOL,
    )
    wide = maximum is None or (maximum > policy.max_relative_half_width and not boundary)
    extension = []
    if not prerequisites:
        status = "invalid"
    elif blocks == policy.initial_blocks:
        if wide:
            status = "extension_required"
            extension = list(range(policy.initial_blocks, policy.max_blocks))
        else:
            status = "healthy"
    elif blocks == policy.max_blocks:
        status = "inconclusive" if wide else "healthy"
    else:
        status = "inconclusive"
    return {
        "status": status,
        "maximum_relative_half_width": maximum,
        "width_limit": policy.max_relative_half_width,
        "boundary_rel_tol": WIDTH_REL_TOL,
        "boundary_abs_tol": WIDTH_ABS_TOL,
        "extension_block_ids": extension,
    }


def _dynamic_comparison(
    validated: ValidatedResourceRun,
    analysis: ResourceAnalysisManifest,
    comparator: str,
) -> dict[str, Any]:
    case = validated.lane.case
    assert case is not None
    logs = {
        block: math.log(
            validated.values[("candidate", block)]
            / validated.values[(comparator, block)]
        )
        for block in validated.valid_blocks
    }
    lower, upper = paired_block_bootstrap(
        {case.id: logs}, samples=analysis.bootstrap_samples,
        seed=analysis.bootstrap_seed, confidence=analysis.confidence,
    )
    estimate = math.exp(statistics.median(logs.values()))
    return {
        "comparator_role": comparator,
        "candidate_median": statistics.median(
            validated.values[("candidate", block)] for block in validated.valid_blocks
        ),
        "comparator_median": statistics.median(
            validated.values[(comparator, block)] for block in validated.valid_blocks
        ),
        "ratio": estimate,
        "confidence_interval": {"lower": lower, "upper": upper},
        "relative_half_width": relative_half_width(estimate, lower, upper),
        "shared_valid_blocks": list(validated.valid_blocks),
        "unit": validated.lane.unit,
    }


def analyze_resource_run(
    validated: ValidatedResourceRun,
    measurement: ResourceManifest,
    analysis: ResourceAnalysisManifest,
) -> dict[str, Any]:
    analysis.assert_compatible(measurement)
    lane = validated.lane
    report_grade_roles = set(validated.engines) == {"candidate", "base", "quickjs-ng"}
    verified = all(
        engine["provenance_status"] == "verified"
        for engine in validated.engines.values()
    )
    if lane.kind == "static":
        complete = report_grade_roles and verified and len(validated.values) == 3
        comparisons = {
            "candidate_vs_base": None,
            "candidate_vs_quickjs_ng": None,
        }
        if complete:
            candidate = validated.values[("candidate", None)]
            base = validated.values[("base", None)]
            reference = validated.values[("quickjs-ng", None)]
            comparisons = {
                "candidate_vs_base": {
                    "candidate_bytes": int(candidate), "comparator_bytes": int(base),
                    "comparator_role": "base", "ratio": candidate / base, "unit": "bytes",
                },
                "candidate_vs_quickjs_ng": {
                    "candidate_bytes": int(candidate), "comparator_bytes": int(reference),
                    "comparator_role": "quickjs-ng", "ratio": candidate / reference,
                    "unit": "bytes",
                },
            }
        return {
            "health": {
                "input_valid": True,
                "status": "healthy" if complete else "invalid",
                "policy": "complete-three-role-exact-size",
                "reason": None if complete else "requires clean verified values for all three roles",
            },
            "comparisons": comparisons,
        }

    invalid_ids = [item["block"] for item in validated.invalid_blocks]
    block_result = _block_health(validated.blocks, invalid_ids, analysis.health)
    can_compare = report_grade_roles and verified and bool(validated.valid_blocks)
    comparisons = {
        "candidate_vs_base": None,
        "candidate_vs_quickjs_ng": None,
    }
    widths = []
    if can_compare:
        comparisons = {
            "candidate_vs_base": _dynamic_comparison(validated, analysis, "base"),
            "candidate_vs_quickjs_ng": _dynamic_comparison(
                validated, analysis, "quickjs-ng"
            ),
        }
        widths = [
            comparison["relative_half_width"] for comparison in comparisons.values()
            if comparison is not None
        ]
    precision = _precision(
        validated.blocks, widths,
        can_compare and block_result["status"] != "invalid", analysis.health,
    )
    return {
        "health": {
            "input_valid": True,
            "status": precision["status"],
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
                "valid": len(validated.valid_blocks),
                "invalid": len(validated.invalid_blocks),
                "valid_block_ids": list(validated.valid_blocks),
                "invalid_blocks": list(validated.invalid_blocks),
                **block_result,
            },
            "precision": precision,
        },
        "comparisons": comparisons,
    }
