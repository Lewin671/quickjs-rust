"""Path-independent report assembly for resource evidence."""

from __future__ import annotations

from typing import Any

from .resource_analysis import analyze_resource_run
from .resource_analysis_schema import ResourceAnalysisManifest
from .resource_schema import ResourceManifest
from .resource_validation import ValidatedResourceRun


def build_resource_report(
    validated: ValidatedResourceRun,
    measurement: ResourceManifest,
    analysis: ResourceAnalysisManifest,
) -> dict[str, Any]:
    results = analyze_resource_run(validated, measurement, analysis)
    return {
        "schema_id": "quickjs-resource-benchmark-report",
        "schema_version": 1,
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
            "lane_id": validated.lane.id,
            "protocol_id": measurement.protocol_id,
            "protocol_sha256": measurement.protocol_sha256,
            "protocol_files": list(measurement.protocol_file_ids),
        },
        "analysis_contract": {
            "manifest_sha256": analysis.sha256,
            "schema_version": analysis.schema_version,
            "id": analysis.id,
            "protocol_id": analysis.protocol_id,
            "protocol_sha256": analysis.protocol_sha256,
            "protocol_files": list(analysis.protocol_file_ids),
            "bootstrap": {
                "samples": analysis.bootstrap_samples,
                "seed": analysis.bootstrap_seed,
                "confidence": analysis.confidence,
            },
        },
        "run": {
            "run_id": validated.start["run_id"],
            "profile": measurement.profile.__dict__,
            "lane_id": validated.lane.id,
            "blocks": validated.blocks,
            "seed": validated.start["seed"],
            "host": validated.start["host"],
            "engines": [
                validated.engines[role]
                for role in ("candidate", "base", "quickjs-ng")
                if role in validated.engines
            ],
        },
        "coverage": {
            "physical_plan_complete": True,
            "comparison_input_complete": validated.comparison_input_complete,
            "runner_end_status": validated.end["status"],
            "runner": validated.end["coverage"],
            "valid_blocks": len(validated.valid_blocks),
            "invalid_blocks": len(validated.invalid_blocks),
        },
        **results,
    }
