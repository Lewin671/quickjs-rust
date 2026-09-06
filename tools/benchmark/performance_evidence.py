"""Bind performance decisions to matching engines, inventories and raw artifacts."""
from __future__ import annotations

import json
import tempfile
from pathlib import Path
from typing import Any

from .performance_schema import (
    PerformanceDecisionError, _object, _read_json, _revision, _sha256, _ratio, _integer,
)

ROOT = Path(__file__).resolve().parents[2]
ROLES = ("candidate", "base", "quickjs-ng")
MIN_BLOCKS = 30
MAX_WIDTH = 0.03


def inventories() -> dict[str, set[str]]:
    broad = json.loads((ROOT / "benchmarks/manifest.json").read_text())
    sentinel = json.loads((ROOT / "benchmarks/generic-sentinels-manifest.json").read_text())
    external = json.loads((ROOT / "benchmarks/external-preview.json").read_text())
    return {"broad": {c["id"] for c in broad["cases"]},
            "sentinel": {c["id"] for c in sentinel["cases"]},
            **{s["id"]: {c["id"] for c in s["cases"]} for s in external["suites"]}}


def engine_identity(summary: dict) -> dict:
    engines = _object(summary.get("engines"), "summary.engines")
    if set(engines) != set(ROLES):
        raise PerformanceDecisionError("summary requires all three engine identities")
    for role, engine in engines.items():
        _revision(engine.get("source_revision"), f"summary {role} revision")
        _sha256(engine.get("binary_sha256"), f"summary {role} binary")
    pin = json.loads((ROOT / "benchmarks/manifest.json").read_text())["reference_engine"]
    if engines["quickjs-ng"]["source_revision"] != pin["revision"]:
        raise PerformanceDecisionError("summary QuickJS-NG revision differs from frozen reference")
    return engines


def validate_internal(report: dict, engines: dict, lane: str) -> list[str]:
    if report.get("schema_id") != "quickjs-benchmark-report" or report.get("schema_version") != 3:
        raise PerformanceDecisionError(f"{lane}: unsupported report schema")
    run = _object(report.get("run"), f"{lane}.run")
    rows = run.get("engines", [])
    if not isinstance(rows, list) or [r.get("role") for r in rows] != list(ROLES):
        raise PerformanceDecisionError(f"{lane}: missing ordered engine identities")
    for row in rows:
        role = row["role"]
        source = row.get("receipt", {}).get("source", {})
        if (source.get("revision") != engines[role]["source_revision"]
                or row.get("binary_sha256") != engines[role]["binary_sha256"]
                or source.get("dirty") is not False):
            raise PerformanceDecisionError(f"{lane}: engine revision/binary mismatch or dirty source: {role}")
    expected = inventories()[lane]
    comparisons = _object(report.get("comparisons"), f"{lane}.comparisons")
    reasons = []
    for comparator in ("base", "quickjs-ng"):
        section = comparisons.get(f"candidate_vs_{comparator.replace('-', '_')}")
        if not isinstance(section, dict) or set(section.get("cases", {})) != expected:
            reasons.append(f"{lane}: incomplete frozen {comparator} case inventory")
    health = report.get("health", {})
    coverage = report.get("coverage", {})
    if (health.get("input_valid") is not True
            or health.get("linearity", {}).get("status") != "pass"
            or health.get("blocks", {}).get("invalid") != 0
            or coverage.get("comparison_input_complete") is not True
            or coverage.get("physical_plan_complete") is not True
            or coverage.get("roles") != 3 or coverage.get("cases") != len(expected)):
        reasons.append(f"{lane}: incomplete or unhealthy measurement")
    return reasons


def validate_bundle(summary: dict, broad: dict, external: dict,
                    sentinel: dict | None = None) -> list[str]:
    """Identity mismatch is invalid input; missing/noisy lanes are inconclusive."""
    engines = engine_identity(summary)
    reasons = validate_internal(broad, engines, "broad")
    if external.get("schema_version") != 2 or external.get("artifact_type") != "quickjs-external-preview-report":
        raise PerformanceDecisionError("external: requires paired report schema 2; remeasure/replay legacy evidence")
    if external.get("binary_sha256") != {role: engines[role]["binary_sha256"] for role in ROLES}:
        raise PerformanceDecisionError("external: binary identities do not match summary")
    if external.get("host") != broad["run"].get("host"):
        raise PerformanceDecisionError("external: different measurement host")
    expected = inventories()
    suites = external.get("suites", [])
    ids = [s.get("id") for s in suites]
    if len(ids) != len(set(ids)) or set(ids) != set(expected) - {"broad", "sentinel"}:
        raise PerformanceDecisionError("external: wrong frozen suite inventory")
    for suite in suites:
        cases = [c.get("id") for c in suite.get("cases", [])]
        if len(cases) != len(set(cases)) or set(cases) != expected[suite["id"]]:
            reasons.append(f"external/{suite['id']}: incomplete frozen case inventory")
    if sentinel is not None:
        reasons.extend(validate_internal(sentinel, engines, "sentinel"))
        if sentinel["run"].get("host") != broad["run"].get("host"):
            raise PerformanceDecisionError("sentinel: different measurement host")
    return reasons


def comparison_index(broad: dict, external: dict, sentinel: dict | None) -> dict[str, dict]:
    result = {}
    for lane, report in (("broad", broad), ("sentinel", sentinel)):
        if report is None:
            continue
        health = report.get("health", {})
        if (health.get("input_valid") is not True
                or health.get("linearity", {}).get("status") != "pass"
                or health.get("blocks", {}).get("invalid") != 0
                or report.get("coverage", {}).get("comparison_input_complete") is not True):
            continue
        section = report.get("comparisons", {}).get("candidate_vs_base") or {}
        count = report.get("health", {}).get("blocks", {}).get("valid", 0)
        for case_id, case in section.get("cases", {}).items():
            result[f"{lane}/{case_id}"] = {**case, "valid_blocks": count}
    for suite in external.get("suites", []):
        for case in suite.get("cases", []):
            paired = case.get("paired_comparisons", {}).get("base")
            if paired is not None:
                result[f"external/{suite['id']}/{case['id']}"] = paired
    return result


def interval(row: dict, where: str) -> tuple[float, float, bool]:
    ratio = _ratio(row.get("ratio"), f"{where}.ratio")
    ci = _object(row.get("confidence_interval"), f"{where}.confidence_interval")
    lower = _ratio(ci.get("lower"), f"{where}.lower")
    upper = _ratio(ci.get("upper"), f"{where}.upper")
    if not lower <= ratio <= upper:
        raise PerformanceDecisionError(f"{where}: confidence interval does not contain estimate")
    blocks = _integer(row.get("valid_blocks"), f"{where}.valid_blocks")
    width = max(upper / ratio - 1, ratio / lower - 1)
    return lower, upper, blocks >= MIN_BLOCKS and width <= MAX_WIDTH


def load_archived_manifest(path: Path, lane: str = "broad"):
    """Keep archived bytes/hash while resolving protocol files in this checkout.

    The existing loader deliberately resolves paths relative to its manifest.
    A short-lived copy in benchmarks/ uses that public API without weakening
    path checks or altering the frozen measurement protocol.
    """
    from .schema import load_manifest
    archived, _ = _read_json(path, "archived measurement manifest")
    filename = "manifest.json" if lane == "broad" else "generic-sentinels-manifest.json"
    trusted, _ = _read_json(ROOT / "benchmarks" / filename, "trusted measurement manifest")
    semantics = lambda data: {key: value for key, value in data.items() if key not in {"profile", "build_recipes"}}
    if semantics(archived) != semantics(trusted):
        raise PerformanceDecisionError("archived measurement semantics differ from the frozen lane")
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=ROOT / "benchmarks", prefix=".hosted-preview-audit-",
                                         suffix=".json", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(path.read_bytes())
        return load_manifest(temporary)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def replay_reports(broad_path: Path, external_path: Path, sentinel_path: Path | None) -> None:
    """Reconstruct reports from colocated raw/manifest files before CLI decisions.

    Hashes alone only label a supplied file. Replay checks that the file was
    actually derived from the claimed raw evidence using the current analyzer.
    """
    from .report import build_report
    from .schema import load_manifest
    from .analysis_schema import load_analysis_manifest
    from .external_preview import load_manifest as load_external
    from .external_report import replay
    for path, prefix in ((broad_path, ""), (sentinel_path, "sentinel-")):
        if path is None:
            continue
        report, _ = _read_json(path, "internal report")
        measurement = load_archived_manifest(path.parent / f"{prefix}manifest.json",
                                             "sentinel" if prefix else "broad")
        analysis = load_analysis_manifest(ROOT / "benchmarks/analysis.json", measurement)
        rebuilt = build_report(path.parent / f"{prefix}raw.jsonl", measurement, analysis)
        if rebuilt != report:
            raise PerformanceDecisionError(f"{path.name}: report differs from raw replay")
    external, _ = _read_json(external_path, "external report")
    manifest = load_external(external_path.parent / "external-manifest.json")
    trusted = load_external(ROOT / "benchmarks/external-preview.json")
    if manifest.sha256 != trusted.sha256:
        raise PerformanceDecisionError("external manifest is not the frozen corpus contract")
    if replay(external_path.parent / "external-raw.jsonl", manifest) != external:
        raise PerformanceDecisionError("external report differs from raw replay")
