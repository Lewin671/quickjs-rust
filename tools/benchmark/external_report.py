"""Paired analysis and offline replay of external process-wall evidence.

Three-block hosted previews remain diagnostics. Decisions require at least
30 complete paired blocks and a narrow interval; no sample is discarded.
"""
from __future__ import annotations

import hashlib
import json
import math
import statistics
from pathlib import Path
from typing import Any, TYPE_CHECKING

from .statistics import paired_block_bootstrap, relative_half_width

if TYPE_CHECKING:
    from .external_preview import Manifest

EXTERNAL_ROLES = ("candidate", "base", "quickjs-ng")
MIN_BLOCKS = 30
MAX_RELATIVE_HALF_WIDTH = 0.03
CONFIDENCE = 0.95
BOOTSTRAP_SAMPLES = 20000
BOOTSTRAP_SEED = 20260905


def paired_effect(candidate: list[dict], comparator: list[dict], blocks: int) -> dict | None:
    if len(candidate) != blocks or len(comparator) != blocks:
        return None
    if any(row["status"] != "ok" for row in candidate + comparator):
        return None
    left = {row["block"]: row["duration_ns"] for row in candidate}
    right = {row["block"]: row["duration_ns"] for row in comparator}
    if set(left) != set(range(blocks)) or set(right) != set(left):
        return None
    logs = {block: math.log(left[block] / right[block]) for block in left}
    ratio = math.exp(statistics.median(logs.values()))
    lower, upper = paired_block_bootstrap(
        {"case": logs}, samples=BOOTSTRAP_SAMPLES, seed=BOOTSTRAP_SEED, confidence=CONFIDENCE
    )
    width = relative_half_width(ratio, lower, upper)
    return {"ratio": ratio, "confidence_interval": {"lower": lower, "upper": upper},
            "relative_half_width": width, "valid_blocks": blocks,
            "status": "healthy" if blocks >= MIN_BLOCKS and width <= MAX_RELATIVE_HALF_WIDTH
                      else "inconclusive"}


def _report(manifest: Manifest, records: list[dict[str, Any]]) -> dict[str, Any]:
    samples: dict[tuple[str, str, str, str], list[dict[str, Any]]] = {}
    for record in records:
        key = (
            record["suite_id"], record["case_id"], record["role"], record["phase"]
        )
        samples.setdefault(key, []).append(record)
    suites: list[dict[str, Any]] = []
    for suite in manifest.suites:
        case_reports: list[dict[str, Any]] = []
        quickjs_ratios: list[float] = []
        base_ratios: list[float] = []
        wins = {"candidate": 0, "quickjs-ng": 0, "tie": 0, "inconclusive": 0}
        base_wins = {"candidate": 0, "base": 0, "tie": 0, "inconclusive": 0}
        for case in suite.cases:
            capability: dict[str, str] = {}
            medians: dict[str, int | None] = {}
            for role in EXTERNAL_ROLES:
                probe = samples.get((suite.id, case.id, role, "capability"), [])
                capability[role] = probe[0]["status"] if probe else "not_run"
                measured = samples.get((suite.id, case.id, role, "measurement"), [])
                durations = [
                    row["duration_ns"] for row in measured if row["status"] == "ok"
                ]
                medians[role] = (
                    int(statistics.median(durations))
                    if len(durations) == manifest.measurement.blocks else None
                )
            effects = {
                role: paired_effect(
                    samples.get((suite.id, case.id, "candidate", "measurement"), []),
                    samples.get((suite.id, case.id, role, "measurement"), []),
                    manifest.measurement.blocks,
                ) for role in ("base", "quickjs-ng")
            }
            ratio = None
            base_ratio = None
            if effects["quickjs-ng"] is not None:
                ratio = effects["quickjs-ng"]["ratio"]
                quickjs_ratios.append(ratio)
                effect = effects["quickjs-ng"]
                ci = effect["confidence_interval"]
                if effect["status"] != "healthy" or ci["lower"] <= 1 <= ci["upper"]:
                    wins["inconclusive"] += 1
                elif ci["upper"] < 1:
                    wins["candidate"] += 1
                else:
                    wins["quickjs-ng"] += 1
            if effects["base"] is not None:
                base_ratio = effects["base"]["ratio"]
                base_ratios.append(base_ratio)
                effect = effects["base"]
                ci = effect["confidence_interval"]
                if effect["status"] != "healthy" or ci["lower"] <= 1 <= ci["upper"]:
                    base_wins["inconclusive"] += 1
                elif ci["upper"] < 1:
                    base_wins["candidate"] += 1
                else:
                    base_wins["base"] += 1
            case_reports.append(
                {
                    "id": case.id,
                    "capability": capability,
                    "median_duration_ns": medians,
                    "paired_comparisons": effects,
                    "candidate_over_base": base_ratio,
                    "candidate_over_quickjs_ng": ratio,
                }
            )
        complete = len(quickjs_ratios) == len(suite.cases)
        base_complete = len(base_ratios) == len(suite.cases)
        diagnostic = (
            math.exp(sum(math.log(ratio) for ratio in quickjs_ratios) / len(quickjs_ratios))
            if quickjs_ratios else None
        )
        base_diagnostic = (
            math.exp(sum(math.log(ratio) for ratio in base_ratios) / len(base_ratios))
            if base_ratios else None
        )
        suites.append(
            {
                "id": suite.id,
                "name": suite.name,
                "source": {
                    "repository": suite.source.repository,
                    "revision": suite.source.revision,
                },
                "reporting_rule": suite.reporting_rule,
                "case_count": len(suite.cases),
                "comparable_case_count": len(quickjs_ratios),
                "complete_comparison": complete,
                "base_comparable_case_count": len(base_ratios),
                "complete_base_comparison": base_complete,
                "official_suite_score": None,
                "diagnostic_comparable_case_geomean_ratio": diagnostic,
                "diagnostic_candidate_over_base_geomean_ratio": base_diagnostic,
                "wins": wins,
                "base_wins": base_wins,
                "cases": case_reports,
            }
        )
    return {
        "schema_version": 2,
        "artifact_type": "quickjs-external-preview-report",
        "preview_id": manifest.preview_id,
        "manifest_sha256": manifest.sha256,
        "claim_eligible": False,
        "metric": manifest.measurement.metric,
        "timer_phase_boundary": manifest.measurement.phase_boundary,
        "roles": list(EXTERNAL_ROLES),
        "blocks": manifest.measurement.blocks,
        "host": records[0]["host"],
        "binary_sha256": {
            role: next(row["binary_sha256"] for row in records if row["role"] == role)
            for role in EXTERNAL_ROLES
        },
        "analysis": {"id": "external-paired-v1", "confidence": CONFIDENCE,
                     "bootstrap_samples": BOOTSTRAP_SAMPLES, "bootstrap_seed": BOOTSTRAP_SEED,
                     "min_blocks": MIN_BLOCKS, "max_relative_half_width": MAX_RELATIVE_HALF_WIDTH},
        "suites": suites,
    }



def replay(path: Path, manifest: Manifest) -> dict[str, Any]:
    """Validate the physical plan and replay a complete raw run without binaries."""
    from dataclasses import replace
    from .external_preview import ExternalPreviewError, _sample_status
    from .performance_schema import _integer, _sha256, _unique_object, _reject_constant
    from .planning import role_orders
    from .process import ProcessResult

    raw = path.read_bytes()
    records = [json.loads(line, object_pairs_hook=_unique_object,
                          parse_constant=_reject_constant) for line in raw.splitlines()]
    if not records:
        raise ExternalPreviewError("external raw: empty run")
    blocks = _integer(records[0].get("measurement_blocks"), "external blocks", 1)
    timeout = _integer(records[0].get("timeout_seconds"), "external timeout", 1)
    manifest = replace(manifest, measurement=replace(manifest.measurement,
                                                    blocks=blocks, timeout_seconds=timeout))
    cursor = 0
    previous_end = 0
    binary_hashes: dict[str, str] = {}
    bundle_hashes: dict[tuple[str, str], str] = {}

    def take(suite, case, phase, block, order, role):
        nonlocal cursor, previous_end
        if cursor >= len(records):
            raise ExternalPreviewError("external raw: incomplete physical plan")
        row = records[cursor]
        cursor += 1
        expected = {
            "schema_version": 2, "record_type": "sample", "claim_eligible": False,
            "preview_id": manifest.preview_id, "manifest_sha256": manifest.sha256,
            "measurement_blocks": blocks, "timeout_seconds": timeout,
            "host": records[0]["host"],
            "suite_id": suite.id, "case_id": case.id, "phase": phase, "block": block,
            "order": order, "role": role, "source_revision": suite.source.revision,
            "source_files": [{"path": f.path, "sha256": f.sha256} for f in case.files],
            "metric": manifest.measurement.metric, "timer": "python.perf_counter_ns",
            "timer_phase_boundary": manifest.measurement.phase_boundary,
        }
        if any(row.get(key) != value for key, value in expected.items()):
            raise ExternalPreviewError("external raw: identity or physical plan mismatch")
        start = _integer(row.get("timer_started_ns"), "external timer start", 1)
        end = _integer(row.get("timer_finished_ns"), "external timer end", start + 1)
        duration = _integer(row.get("duration_ns"), "external duration", 1)
        if start < previous_end or end - start != duration:
            raise ExternalPreviewError("external raw: overlapping or inconsistent timers")
        previous_end = end
        digest = _sha256(row.get("binary_sha256"), "external binary")
        if binary_hashes.setdefault(role, digest) != digest:
            raise ExternalPreviewError("external raw: binary changed within run")
        bundle = _sha256(row.get("bundle_sha256"), "external bundle")
        if bundle_hashes.setdefault((suite.id, case.id), bundle) != bundle:
            raise ExternalPreviewError("external raw: bundle changed across roles")
        argv = row.get("argv")
        flag = "--script" if role == "quickjs-ng" else "--raw"
        if not isinstance(argv, list) or len(argv) != 3 or argv[1] != flag:
            raise ExternalPreviewError("external raw: incorrect shell adapter")
        fields = ProcessResult.__dataclass_fields__
        result = ProcessResult(**{key: row[key] for key in fields})
        status, error = _sample_status(result, role)
        if (status, error) != (row.get("status"), row.get("error")):
            raise ExternalPreviewError("external raw: result status does not match output")
        return status

    for si, suite in enumerate(manifest.suites):
        for ci, case in enumerate(suite.cases):
            seed = manifest.measurement.seed + si * 10000 + ci * 100
            capable = []
            for order, role in enumerate(role_orders(list(EXTERNAL_ROLES), 1, seed + 1)[0]):
                if take(suite, case, "capability", None, order, role) == "ok":
                    capable.append(role)
            if not capable:
                continue
            for block, roles in enumerate(role_orders(capable, blocks, seed)):
                for order, role in enumerate(roles):
                    take(suite, case, "measurement", block, order, role)
    if cursor != len(records):
        raise ExternalPreviewError("external raw: extra records after complete plan")
    report = _report(manifest, records)
    report["input"] = {"sha256": hashlib.sha256(raw).hexdigest(), "byte_length": len(raw)}
    return report
