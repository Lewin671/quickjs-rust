"""Portable report artifact assembly, separate from validation and statistics."""

from __future__ import annotations

from typing import Any

from .analysis import analyze_run
from .analysis_schema import AnalysisManifest
from .raw_validation import ValidatedRun
from .schema import Manifest


def _analysis_contract(analysis: AnalysisManifest) -> dict[str, Any]:
    return {
        "manifest_sha256": analysis.sha256,
        "schema_version": analysis.schema_version,
        "id": analysis.id,
        "protocol_id": analysis.protocol_id,
        "protocol_sha256": analysis.protocol_sha256,
        "protocol_files": list(analysis.protocol_file_ids),
        "compatible_measurement": {
            "schema_version": analysis.compatible_measurement_schema,
            "protocol_id": analysis.compatible_measurement_protocol,
        },
        "bootstrap": {
            "samples": analysis.bootstrap_samples,
            "seed": analysis.bootstrap_seed,
            "confidence": analysis.confidence,
        },
        "linearity": {
            "normalized_per_op_lower": analysis.linearity_lower,
            "normalized_per_op_upper": analysis.linearity_upper,
        },
        "health": {
            "initial_blocks": analysis.health.initial_blocks,
            "extension_blocks": analysis.health.extension_blocks,
            "max_blocks": analysis.health.max_blocks,
            "max_relative_half_width": analysis.health.max_relative_half_width,
            "max_invalid_block_fraction": analysis.health.max_invalid_block_fraction,
            "block_invalidation": analysis.health.block_invalidation,
            "outlier_policy": analysis.health.outlier_policy,
            "retry_policy": analysis.health.retry_policy,
        },
    }


def build_report(
    validated: ValidatedRun,
    measurement: Manifest,
    analysis: AnalysisManifest,
) -> dict[str, Any]:
    results = analyze_run(validated, measurement, analysis)
    start = validated.start
    return {
        "schema_id": "quickjs-benchmark-report",
        "schema_version": 3,
        "claim_eligible": False,
        "input": {
            "sha256": validated.input_sha256,
            "byte_length": validated.input_bytes,
        },
        "measurement_contract": {
            "manifest_sha256": measurement.sha256,
            "schema_version": measurement.schema_version,
            "series_id": measurement.series_id,
            "suite_id": measurement.suite_id,
            "lane_id": measurement.lane_id,
            "protocol_id": measurement.protocol_id,
            "protocol_sha256": measurement.protocol_sha256,
            "protocol_files": list(measurement.protocol_file_ids),
        },
        "analysis_contract": _analysis_contract(analysis),
        "run": {
            "run_id": start["run_id"],
            "profile": measurement.profile.__dict__,
            "blocks": validated.blocks,
            "lane_id": measurement.lane_id,
            "seed": start["seed"],
            "host": start["host"],
            "engines": [
                validated.engines[role]
                for role in ("candidate", "base", "quickjs-ng")
            ],
        },
        "coverage": {
            "roles": 3,
            "cases": len(measurement.cases),
            "blocks": validated.blocks,
            "requested_measurement_records": (
                validated.blocks * len(measurement.cases) * 3
            ),
            "attempted_measurement_records": len(validated.measurement_records),
            "valid_measurement_records": len(validated.measurements),
            "invalid_measurement_records": (
                len(validated.measurement_records) - len(validated.measurements)
            ),
            "linearity_records": len(validated.linearity),
            "physical_plan_complete": True,
            "comparison_input_complete": validated.comparison_input_complete,
            "runner_end_status": validated.runner_end_status,
            "runner": validated.runner_coverage,
        },
        **results,
    }
