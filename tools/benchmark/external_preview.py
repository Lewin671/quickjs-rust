"""Pinned, non-claim external benchmark preview for qjs-rust and QuickJS-NG."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import random
import re
import shutil
import statistics
import sys
import tempfile
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any

from .process import ProcessResult, run_process


SENTINEL = "__QJS_EXTERNAL_OK__"
_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
_REVISION = re.compile(r"[0-9a-f]{40}\Z")
_SHA256 = re.compile(r"[0-9a-f]{64}\Z")
_GITHUB_REPOSITORY = re.compile(
    r"https://github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+?)(?:\.git)?\Z"
)


class ExternalPreviewError(ValueError):
    """The preview manifest, corpus, or measurement is invalid."""


@dataclass(frozen=True)
class SourceFile:
    path: str
    sha256: str


@dataclass(frozen=True)
class Case:
    id: str
    files: tuple[SourceFile, ...]
    harness: str
    iteration_count: int | None


@dataclass(frozen=True)
class Source:
    repository: str
    revision: str
    path_prefix: str


@dataclass(frozen=True)
class Suite:
    id: str
    name: str
    reporting_rule: str
    source: Source
    cases: tuple[Case, ...]


@dataclass(frozen=True)
class Measurement:
    blocks: int
    timeout_seconds: int
    metric: str
    phase_boundary: str
    order: str
    seed: int


@dataclass(frozen=True)
class Manifest:
    path: Path
    sha256: str
    preview_id: str
    measurement: Measurement
    suites: tuple[Suite, ...]


def _reject_constant(value: str) -> None:
    raise ExternalPreviewError(f"manifest contains non-standard numeric constant {value}")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise ExternalPreviewError(f"manifest contains duplicate key {key!r}")
        value[key] = item
    return value


def _keys(value: Any, expected: set[str], where: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ExternalPreviewError(f"{where}: expected an object")
    actual = set(value)
    if actual != expected:
        raise ExternalPreviewError(
            f"{where}: unknown or missing fields; expected {sorted(expected)}, got {sorted(actual)}"
        )
    return value


def _string(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value or value.strip() != value:
        raise ExternalPreviewError(f"{where}: expected a non-empty trimmed string")
    return value


def _integer(value: Any, where: str, minimum: int) -> int:
    if type(value) is not int or value < minimum:
        raise ExternalPreviewError(f"{where}: expected an integer >= {minimum}")
    return value


def _id(value: Any, where: str) -> str:
    result = _string(value, where)
    if not _ID.fullmatch(result):
        raise ExternalPreviewError(f"{where}: invalid stable lowercase id")
    return result


def _safe_path(value: Any, where: str, *, allow_empty: bool = False) -> str:
    if allow_empty and value == "":
        return ""
    result = _string(value, where)
    path = PurePosixPath(result)
    if path.is_absolute() or ".." in path.parts or str(path) != result:
        raise ExternalPreviewError(f"{where}: path must be normalized and repository-relative")
    return result


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_manifest(path: Path) -> Manifest:
    resolved = path.expanduser().resolve()
    try:
        raw = resolved.read_bytes()
        root = json.loads(
            raw.decode("utf-8"), object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ExternalPreviewError(f"cannot read external preview manifest: {error}") from error
    root = _keys(
        root,
        {"schema_version", "preview_id", "claim_eligible", "measurement", "suites"},
        "manifest",
    )
    if _integer(root["schema_version"], "manifest.schema_version", 1) != 1:
        raise ExternalPreviewError("manifest.schema_version: expected version 1")
    if root["claim_eligible"] is not False:
        raise ExternalPreviewError("manifest.claim_eligible: preview must remain false")
    preview_id = _string(root["preview_id"], "manifest.preview_id")
    if preview_id != "quickjs-authoritative-external-preview-v1":
        raise ExternalPreviewError("manifest.preview_id: unexpected trust-root identity")
    measurement_data = _keys(
        root["measurement"],
        {"blocks", "timeout_seconds", "metric", "phase_boundary", "order", "seed"},
        "manifest.measurement",
    )
    measurement = Measurement(
        blocks=_integer(measurement_data["blocks"], "measurement.blocks", 1),
        timeout_seconds=_integer(
            measurement_data["timeout_seconds"], "measurement.timeout_seconds", 1
        ),
        metric=_string(measurement_data["metric"], "measurement.metric"),
        phase_boundary=_string(
            measurement_data["phase_boundary"], "measurement.phase_boundary"
        ),
        order=_string(measurement_data["order"], "measurement.order"),
        seed=_integer(measurement_data["seed"], "measurement.seed", 0),
    )
    if (
        measurement.metric != "outer_process_wall_time"
        or measurement.phase_boundary != "before_process_spawn_to_wait_return"
        or measurement.order != "seeded_pair_rotation"
    ):
        raise ExternalPreviewError("manifest.measurement: unsupported protocol")

    suites_data = root["suites"]
    if not isinstance(suites_data, list) or not suites_data:
        raise ExternalPreviewError("manifest.suites: expected a non-empty array")
    suites: list[Suite] = []
    suite_ids: set[str] = set()
    for suite_index, raw_suite in enumerate(suites_data):
        where = f"manifest.suites[{suite_index}]"
        suite_data = _keys(
            raw_suite,
            {"id", "name", "reporting_rule", "source", "license_review", "cases"},
            where,
        )
        suite_id = _id(suite_data["id"], f"{where}.id")
        if suite_id in suite_ids:
            raise ExternalPreviewError("manifest.suites: duplicate suite id")
        suite_ids.add(suite_id)
        source_data = _keys(
            suite_data["source"], {"repository", "revision", "path_prefix"},
            f"{where}.source",
        )
        repository = _string(source_data["repository"], f"{where}.source.repository")
        if not _GITHUB_REPOSITORY.fullmatch(repository):
            raise ExternalPreviewError(f"{where}.source.repository: expected a GitHub HTTPS URL")
        revision = _string(source_data["revision"], f"{where}.source.revision")
        if not _REVISION.fullmatch(revision) or revision == "0" * 40:
            raise ExternalPreviewError(f"{where}.source.revision: expected a full git revision")
        license_data = _keys(
            suite_data["license_review"],
            {"status", "redistribute_source_in_artifact", "evidence_urls"},
            f"{where}.license_review",
        )
        if license_data["status"] != "execution-only":
            raise ExternalPreviewError(f"{where}.license_review.status: expected execution-only")
        if license_data["redistribute_source_in_artifact"] is not False:
            raise ExternalPreviewError(
                f"{where}.license_review: source redistribution must remain disabled"
            )
        evidence = license_data["evidence_urls"]
        if not isinstance(evidence, list) or not evidence or not all(
            isinstance(url, str) and url.startswith("https://github.com/") for url in evidence
        ):
            raise ExternalPreviewError(f"{where}.license_review.evidence_urls: invalid URLs")
        cases_data = suite_data["cases"]
        if not isinstance(cases_data, list) or not cases_data:
            raise ExternalPreviewError(f"{where}.cases: expected a non-empty array")
        cases: list[Case] = []
        case_ids: set[str] = set()
        for case_index, raw_case in enumerate(cases_data):
            case_where = f"{where}.cases[{case_index}]"
            if not isinstance(raw_case, dict):
                raise ExternalPreviewError(f"{case_where}: expected an object")
            expected = {"id", "files", "harness"}
            if raw_case.get("harness") == "jetstream-class":
                expected.add("iteration_count")
            case_data = _keys(raw_case, expected, case_where)
            case_id = _id(case_data["id"], f"{case_where}.id")
            if case_id in case_ids:
                raise ExternalPreviewError(f"{where}.cases: duplicate case id")
            case_ids.add(case_id)
            harness = _string(case_data["harness"], f"{case_where}.harness")
            if harness not in {"direct-script", "jetstream-class"}:
                raise ExternalPreviewError(f"{case_where}.harness: unsupported harness")
            files_data = case_data["files"]
            if not isinstance(files_data, list) or not files_data:
                raise ExternalPreviewError(f"{case_where}.files: expected a non-empty array")
            files: list[SourceFile] = []
            seen_paths: set[str] = set()
            for file_index, raw_file in enumerate(files_data):
                file_where = f"{case_where}.files[{file_index}]"
                file_data = _keys(raw_file, {"path", "sha256"}, file_where)
                source_path = _safe_path(file_data["path"], f"{file_where}.path")
                digest = _string(file_data["sha256"], f"{file_where}.sha256")
                if source_path in seen_paths or not _SHA256.fullmatch(digest) or digest == "0" * 64:
                    raise ExternalPreviewError(f"{file_where}: duplicate path or invalid SHA-256")
                seen_paths.add(source_path)
                files.append(SourceFile(source_path, digest))
            iteration_count = (
                _integer(case_data["iteration_count"], f"{case_where}.iteration_count", 1)
                if harness == "jetstream-class" else None
            )
            cases.append(Case(case_id, tuple(files), harness, iteration_count))
        suites.append(
            Suite(
                suite_id,
                _string(suite_data["name"], f"{where}.name"),
                _string(suite_data["reporting_rule"], f"{where}.reporting_rule"),
                Source(
                    repository,
                    revision,
                    _safe_path(
                        source_data["path_prefix"], f"{where}.source.path_prefix",
                        allow_empty=True,
                    ),
                ),
                tuple(cases),
            )
        )
    if suite_ids != {"jetstream3-js-subset", "kraken-1.1", "sunspider-1.0"}:
        raise ExternalPreviewError("manifest.suites: expected exactly the three selected suites")
    return Manifest(resolved, _sha256_bytes(raw), preview_id, measurement, tuple(suites))


def _raw_url(source: Source, source_file: SourceFile) -> str:
    match = _GITHUB_REPOSITORY.fullmatch(source.repository)
    assert match is not None
    owner, repository = match.groups()
    parts = [part for part in (source.path_prefix, source_file.path) if part]
    return (
        f"https://raw.githubusercontent.com/{owner}/{repository}/"
        f"{source.revision}/{'/'.join(parts)}"
    )


def fetch_corpora(manifest: Manifest, cache_root: Path) -> dict[str, int]:
    root = cache_root.expanduser().resolve()
    downloaded = 0
    reused = 0
    for suite in manifest.suites:
        for case in suite.cases:
            for source_file in case.files:
                target = root / suite.id / source_file.path
                if target.is_file() and _sha256_file(target) == source_file.sha256:
                    reused += 1
                    continue
                request = urllib.request.Request(
                    _raw_url(suite.source, source_file),
                    headers={"User-Agent": "quickjs-rust-external-preview/1"},
                )
                try:
                    with urllib.request.urlopen(request, timeout=30) as response:
                        content = response.read()
                except (OSError, urllib.error.URLError) as error:
                    raise ExternalPreviewError(
                        f"cannot fetch {suite.id}/{source_file.path}: {error}"
                    ) from error
                if _sha256_bytes(content) != source_file.sha256:
                    raise ExternalPreviewError(
                        f"downloaded content hash mismatch for {suite.id}/{source_file.path}"
                    )
                target.parent.mkdir(parents=True, exist_ok=True)
                temporary: Path | None = None
                try:
                    with tempfile.NamedTemporaryFile(dir=target.parent, delete=False) as handle:
                        handle.write(content)
                        handle.flush()
                        os.fsync(handle.fileno())
                        temporary = Path(handle.name)
                    os.replace(temporary, target)
                    temporary = None
                finally:
                    if temporary is not None:
                        temporary.unlink(missing_ok=True)
                downloaded += 1
    return {"downloaded": downloaded, "reused": reused}


def _bundle_source(suite: Suite, case: Case, cache_root: Path) -> str:
    pieces = [
        "/* Generated from hash-verified upstream sources; do not publish this bundle. */",
        "var __qjsExternalHostPrint = typeof print === 'function' ? print : null;",
    ]
    if case.harness == "jetstream-class":
        pieces.append(
            "if (typeof performance === 'undefined') { "
            "globalThis.performance = { now: function () { return 0; } }; }"
        )
    for source_file in case.files:
        path = cache_root / suite.id / source_file.path
        if _sha256_file(path) != source_file.sha256:
            raise ExternalPreviewError(f"cached content drift for {suite.id}/{source_file.path}")
        try:
            pieces.append(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError) as error:
            raise ExternalPreviewError(f"cannot read {suite.id}/{source_file.path}: {error}") from error
    if case.harness == "jetstream-class":
        assert case.iteration_count is not None
        pieces.append(
            "var __qjsExternalBenchmark = new Benchmark({ iterationCount: "
            f"{case.iteration_count} }});\n"
            f"for (var __qjsExternalIndex = 0; __qjsExternalIndex < {case.iteration_count}; "
            "__qjsExternalIndex++) {\n"
            "  if (typeof __qjsExternalBenchmark.prepareForNextIteration === 'function') "
            "__qjsExternalBenchmark.prepareForNextIteration();\n"
            "  __qjsExternalBenchmark.runIteration(__qjsExternalIndex);\n"
            "}\n"
            "if (typeof __qjsExternalBenchmark.validate === 'function') "
            f"__qjsExternalBenchmark.validate({case.iteration_count});"
        )
    pieces.append(
        f"if (__qjsExternalHostPrint) __qjsExternalHostPrint('{SENTINEL}');\n"
        f"'{SENTINEL}';"
    )
    return "\n;\n".join(pieces) + "\n"


def _binary(path: Path, role: str) -> tuple[Path, str]:
    resolved = path.expanduser().resolve()
    if not resolved.is_file() or not resolved.stat().st_mode & 0o111:
        raise ExternalPreviewError(f"{role} binary is not executable: {resolved}")
    return resolved, _sha256_file(resolved)


def _command(role: str, binary: Path, bundle: Path) -> list[str]:
    if role == "candidate":
        return [str(binary), "--raw", str(bundle)]
    if role == "quickjs-ng":
        return [str(binary), "--script", str(bundle)]
    raise ExternalPreviewError(f"unknown engine role {role}")


def _sample_status(result: ProcessResult) -> tuple[str, str | None]:
    if result.timed_out:
        return "timeout", "process exceeded the case timeout"
    if result.exit_code != 0:
        return "failed", f"process exited with status {result.exit_code}"
    if result.stdout_truncated or result.stderr_truncated:
        return "invalid", "process output exceeded the capture limit"
    lines = result.stdout.rstrip().splitlines()
    if not lines or lines[-1] != SENTINEL:
        return "invalid", "success sentinel was missing"
    return "ok", None


def _record(
    manifest: Manifest,
    suite: Suite,
    case: Case,
    role: str,
    binary_sha256: str,
    bundle: Path,
    phase: str,
    block: int | None,
    order: int,
    result: ProcessResult,
    argv: list[str],
) -> dict[str, Any]:
    status, error = _sample_status(result)
    return {
        "schema_version": 1,
        "record_type": "sample",
        "preview_id": manifest.preview_id,
        "manifest_sha256": manifest.sha256,
        "claim_eligible": False,
        "suite_id": suite.id,
        "case_id": case.id,
        "role": role,
        "phase": phase,
        "block": block,
        "order": order,
        "metric": manifest.measurement.metric,
        "timer": "python.perf_counter_ns",
        "timer_phase_boundary": manifest.measurement.phase_boundary,
        "duration_ns": result.duration_ns,
        "started_at": result.started_at,
        "status": status,
        "error": error,
        "exit_code": result.exit_code,
        "timed_out": result.timed_out,
        "stdout": result.stdout,
        "stderr": result.stderr,
        "stdout_truncated": result.stdout_truncated,
        "stderr_truncated": result.stderr_truncated,
        "binary_sha256": binary_sha256,
        "bundle_sha256": _sha256_file(bundle),
        "argv": argv,
        "source_revision": suite.source.revision,
        "source_files": [
            {"path": source_file.path, "sha256": source_file.sha256}
            for source_file in case.files
        ],
    }


def _atomic_write(path: Path, content: bytes) -> None:
    if path.exists():
        raise ExternalPreviewError(f"refusing to overwrite existing evidence: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
            temporary = Path(handle.name)
        os.link(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


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
        ratios: list[float] = []
        wins = {"candidate": 0, "quickjs-ng": 0, "tie": 0}
        for case in suite.cases:
            capability: dict[str, str] = {}
            medians: dict[str, int | None] = {}
            for role in ("candidate", "quickjs-ng"):
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
            ratio = None
            if medians["candidate"] is not None and medians["quickjs-ng"] is not None:
                ratio = medians["candidate"] / medians["quickjs-ng"]
                ratios.append(ratio)
                if ratio < 1:
                    wins["candidate"] += 1
                elif ratio > 1:
                    wins["quickjs-ng"] += 1
                else:
                    wins["tie"] += 1
            case_reports.append(
                {
                    "id": case.id,
                    "capability": capability,
                    "median_duration_ns": medians,
                    "candidate_over_quickjs_ng": ratio,
                }
            )
        complete = len(ratios) == len(suite.cases)
        diagnostic = math.exp(sum(math.log(ratio) for ratio in ratios) / len(ratios)) if ratios else None
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
                "comparable_case_count": len(ratios),
                "complete_comparison": complete,
                "official_suite_score": None,
                "diagnostic_comparable_case_geomean_ratio": diagnostic,
                "wins": wins,
                "cases": case_reports,
            }
        )
    return {
        "schema_version": 1,
        "artifact_type": "quickjs-external-preview-report",
        "preview_id": manifest.preview_id,
        "manifest_sha256": manifest.sha256,
        "claim_eligible": False,
        "metric": manifest.measurement.metric,
        "timer_phase_boundary": manifest.measurement.phase_boundary,
        "roles": ["candidate", "quickjs-ng"],
        "blocks": manifest.measurement.blocks,
        "suites": suites,
    }


def _markdown(report: dict[str, Any]) -> str:
    lines = [
        "## External Benchmark Preview", "",
        "> **Informational only.** These are pinned, neutral shell ports; no row is an official",
        "> JetStream, Kraken, or SunSpider score, and incomplete suites have no aggregate score.",
        "",
        "| Suite | Comparable | qjs-rust / QuickJS-NG | qjs-rust wins | QuickJS-NG wins |",
        "|---|---:|---:|---:|---:|",
    ]
    for suite in report["suites"]:
        ratio = suite["diagnostic_comparable_case_geomean_ratio"]
        ratio_text = f"{ratio:.3f}x" if ratio is not None else "—"
        lines.append(
            f"| {suite['name']} | {suite['comparable_case_count']}/{suite['case_count']} | "
            f"{ratio_text} | {suite['wins']['candidate']} | {suite['wins']['quickjs-ng']} |"
        )
    lines.extend([
        "", "### External per-case performance", "",
        "Median wall time is the outer process duration per run. Lower ratios favor qjs-rust.", "",
        "| Suite / case | qjs-rust ms/run | QuickJS-NG ms/run | Ratio | Lower wall time |",
        "|---|---:|---:|---:|---|",
    ])
    for suite in report["suites"]:
        for case in suite["cases"]:
            candidate = case["median_duration_ns"]["candidate"]
            quickjs = case["median_duration_ns"]["quickjs-ng"]
            ratio = case["candidate_over_quickjs_ng"]
            if ratio is None or candidate is None or quickjs is None:
                candidate_text = "—" if candidate is None else f"{candidate / 1_000_000:.3f}"
                quickjs_text = "—" if quickjs is None else f"{quickjs / 1_000_000:.3f}"
                ratio_text = "—"
                lower_wall_time = (
                    f"not comparable ({case['capability']['candidate']} / "
                    f"{case['capability']['quickjs-ng']})"
                )
            else:
                candidate_text = f"{candidate / 1_000_000:.3f}"
                quickjs_text = f"{quickjs / 1_000_000:.3f}"
                ratio_text = f"{ratio:.3f}x"
                lower_wall_time = (
                    "qjs-rust" if ratio < 1 else "QuickJS-NG" if ratio > 1 else "tie"
                )
            lines.append(
                f"| `{suite['id']}/{case['id']}` | {candidate_text} | {quickjs_text} | "
                f"{ratio_text} | {lower_wall_time} |"
            )
    lines.extend([
        "", "Lower ratios are faster for qjs-rust. The ratio is a diagnostic geometric mean",
        "over explicitly reported comparable cases, never a substitute for a suite score.", "",
    ])
    return "\n".join(lines)


def run_preview(
    manifest: Manifest,
    cache_root: Path,
    work_root: Path,
    output_dir: Path,
    candidate_path: Path,
    quickjs_path: Path,
    *,
    blocks: int | None = None,
    timeout_seconds: int | None = None,
) -> dict[str, Any]:
    fetch_corpora(manifest, cache_root)
    candidate, candidate_sha = _binary(candidate_path, "candidate")
    quickjs, quickjs_sha = _binary(quickjs_path, "quickjs-ng")
    source_binaries = {"candidate": candidate, "quickjs-ng": quickjs}
    binary_hashes = {"candidate": candidate_sha, "quickjs-ng": quickjs_sha}
    selected_blocks = blocks if blocks is not None else manifest.measurement.blocks
    selected_timeout = (
        timeout_seconds if timeout_seconds is not None else manifest.measurement.timeout_seconds
    )
    if selected_blocks < 1 or selected_timeout < 1:
        raise ExternalPreviewError("blocks and timeout must be positive")
    if selected_blocks != manifest.measurement.blocks:
        manifest = Manifest(
            manifest.path, manifest.sha256, manifest.preview_id,
            Measurement(
                selected_blocks, selected_timeout, manifest.measurement.metric,
                manifest.measurement.phase_boundary, manifest.measurement.order,
                manifest.measurement.seed,
            ),
            manifest.suites,
        )
    bundle_root = work_root.expanduser().resolve() / "bundles"
    snapshot_root = work_root.expanduser().resolve() / "engine-snapshots"
    if bundle_root.exists():
        shutil.rmtree(bundle_root)
    if snapshot_root.exists():
        shutil.rmtree(snapshot_root)
    bundle_root.mkdir(parents=True)
    snapshot_root.mkdir(parents=True)
    binaries: dict[str, Path] = {}
    for role, source_binary in source_binaries.items():
        snapshot = snapshot_root / role
        shutil.copyfile(source_binary, snapshot)
        snapshot.chmod(0o500)
        if _sha256_file(snapshot) != binary_hashes[role]:
            raise ExternalPreviewError(f"{role} executable snapshot hash mismatch")
        binaries[role] = snapshot
    records: list[dict[str, Any]] = []
    try:
        for suite_index, suite in enumerate(manifest.suites):
            for case_index, case in enumerate(suite.cases):
                bundle = bundle_root / suite.id / f"{case.id}.js"
                bundle.parent.mkdir(parents=True, exist_ok=True)
                bundle.write_text(_bundle_source(suite, case, cache_root), encoding="utf-8")
                capability_ok = True
                for order, role in enumerate(("candidate", "quickjs-ng")):
                    argv = _command(role, binaries[role], bundle)
                    result = run_process(argv, selected_timeout)
                    record = _record(
                        manifest, suite, case, role, binary_hashes[role], bundle,
                        "capability", None, order, result, argv,
                    )
                    records.append(record)
                    capability_ok = capability_ok and record["status"] == "ok"
                if not capability_ok:
                    continue
                for block in range(selected_blocks):
                    roles = ["candidate", "quickjs-ng"]
                    random.Random(
                        manifest.measurement.seed + suite_index * 10000 + case_index * 100 + block
                    ).shuffle(roles)
                    for order, role in enumerate(roles):
                        argv = _command(role, binaries[role], bundle)
                        result = run_process(argv, selected_timeout)
                        records.append(
                            _record(
                                manifest, suite, case, role, binary_hashes[role], bundle,
                                "measurement", block, order, result, argv,
                            )
                        )
        for role, snapshot in binaries.items():
            if _sha256_file(snapshot) != binary_hashes[role]:
                raise ExternalPreviewError(f"{role} executable snapshot changed during preview")
        report = _report(manifest, records)
        output = output_dir.expanduser().resolve()
        raw = b"".join(
            (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()
            for record in records
        )
        _atomic_write(output / "external-raw.jsonl", raw)
        _atomic_write(
            output / "external-report.json",
            (json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n").encode(),
        )
        _atomic_write(output / "external-summary.md", _markdown(report).encode())
        _atomic_write(output / "external-manifest.json", manifest.path.read_bytes())
        return report
    finally:
        shutil.rmtree(bundle_root, ignore_errors=True)
        shutil.rmtree(snapshot_root, ignore_errors=True)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=Path("benchmarks/external-preview.json"))
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("audit")
    fetch = commands.add_parser("fetch")
    fetch.add_argument("--cache-root", type=Path, required=True)
    run = commands.add_parser("run")
    run.add_argument("--cache-root", type=Path, required=True)
    run.add_argument("--work-root", type=Path, required=True)
    run.add_argument("--output-dir", type=Path, required=True)
    run.add_argument("--candidate", type=Path, required=True)
    run.add_argument("--quickjs-ng", type=Path, required=True)
    run.add_argument("--blocks", type=int)
    run.add_argument("--timeout-seconds", type=int)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        manifest = load_manifest(args.manifest)
        if args.command == "audit":
            print(json.dumps({
                "claim_eligible": False,
                "manifest_sha256": manifest.sha256,
                "preview_id": manifest.preview_id,
                "suite_count": len(manifest.suites),
                "case_count": sum(len(suite.cases) for suite in manifest.suites),
            }, sort_keys=True, separators=(",", ":")))
        elif args.command == "fetch":
            print(json.dumps(fetch_corpora(manifest, args.cache_root), sort_keys=True))
        else:
            report = run_preview(
                manifest, args.cache_root, args.work_root, args.output_dir,
                args.candidate, args.quickjs_ng,
                blocks=args.blocks, timeout_seconds=args.timeout_seconds,
            )
            print(json.dumps({
                "claim_eligible": False,
                "report": str((args.output_dir / "external-report.json").resolve()),
                "suites": len(report["suites"]),
            }, sort_keys=True))
        return 0
    except ExternalPreviewError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
