"""Frozen whole-block and 30-to-60 experiment-health decisions."""

from __future__ import annotations

import math
from typing import Iterable

from .analysis_schema import HealthPolicy

WIDTH_REL_TOL = 1e-12
WIDTH_ABS_TOL = 1e-15


def allowed_invalid_blocks(blocks: int, policy: HealthPolicy) -> int:
    """Return the inclusive whole-block loss budget for a declared cohort."""
    return math.floor(blocks * policy.max_invalid_block_fraction)


def block_health(
    blocks: int, invalid_blocks: Iterable[int], policy: HealthPolicy
) -> dict[str, object]:
    """Classify loss without allowing a 60-block run to dilute its first cohort."""
    invalid = tuple(sorted(set(invalid_blocks)))
    if any(block < 0 or block >= blocks for block in invalid):
        raise ValueError("invalid block id is outside the requested cohort")
    initial_invalid = tuple(
        block for block in invalid if block < policy.initial_blocks
    )
    standard = blocks in {policy.initial_blocks, policy.max_blocks}
    exceeds_initial = (
        blocks >= policy.initial_blocks
        and len(initial_invalid) > allowed_invalid_blocks(policy.initial_blocks, policy)
    )
    exceeds_total = (
        standard and len(invalid) > allowed_invalid_blocks(blocks, policy)
    )
    return {
        "status": "invalid" if exceeds_initial or exceeds_total else (
            "pass" if standard else "non_claim"
        ),
        "standard_cohort": standard,
        "initial_invalid_blocks": list(initial_invalid),
        "initial_invalid_limit": allowed_invalid_blocks(policy.initial_blocks, policy),
        "total_invalid_limit": allowed_invalid_blocks(blocks, policy) if standard else None,
        "initial_limit_exceeded": exceeds_initial,
        "total_limit_exceeded": exceeds_total,
    }


def precision_health(
    blocks: int,
    relative_half_widths: Iterable[float],
    prerequisites_healthy: bool,
    policy: HealthPolicy,
) -> dict[str, object]:
    """Apply the frozen width rule without inventing append/retry semantics."""
    widths = tuple(relative_half_widths)
    if any(not math.isfinite(width) or width < 0 for width in widths):
        raise ValueError("relative half-widths must be finite and non-negative")
    maximum = max(widths, default=None)
    at_boundary = maximum is not None and math.isclose(
        maximum,
        policy.max_relative_half_width,
        rel_tol=WIDTH_REL_TOL,
        abs_tol=WIDTH_ABS_TOL,
    )
    wide = maximum is None or (
        maximum > policy.max_relative_half_width and not at_boundary
    )
    extension_ids: list[int] = []
    if not prerequisites_healthy:
        status = "invalid"
    elif blocks == policy.initial_blocks:
        if wide:
            status = "extension_required"
            extension_ids = list(range(policy.initial_blocks, policy.max_blocks))
        else:
            status = "healthy"
    elif blocks == policy.max_blocks:
        status = "inconclusive" if wide else "healthy"
    else:
        status = "inconclusive"
    return {
        "status": status,
        "maximum_critical_family_relative_half_width": maximum,
        "width_limit": policy.max_relative_half_width,
        "boundary_rel_tol": WIDTH_REL_TOL,
        "boundary_abs_tol": WIDTH_ABS_TOL,
        "extension_block_ids": extension_ids,
    }
