"""Shared machinery for whole-process wall-time lanes over pinned npm tarballs.

Two lanes execute unmodified, pinned third-party npm libraries driven by
first-party scripts, and differ only in identity and in what their cases are
allowed to do:

- the **held-out** acceptance lane, whose cases never select work, and
- the **development** lane, whose cases may be profiled and may rank in the
  opportunity queue.

Both run under the external whole-process protocol: each driver builds its own
deterministic input, calls the library's public API, and prints one
`CHECKSUM <hex>` line. A sample is accepted only when its stdout is exactly that
checksum followed by the lane's sentinel contract, so a checksum that differs
between engines is an invalid sample, not a slow one. `LaneSpec` is the whole
difference between the lanes; `load_npm_manifest` refuses a case id another
registered lane already owns, so one id can never mean two programs.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import importlib
import json
import os
import re
import sys
import tarfile
import tempfile
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .external_preview import (
    EXTERNAL_ROLES,
    SENTINEL,
    ExternalPreviewError,
    Lane,
    Measurement,
    Source,
    SourceFile,
    _safe_path,
    _sha256_file,
    run_preview,
)
from .external_report import _report as _external_report
from .process import ProcessResult

ROOT = Path(__file__).resolve().parents[2]
LANE_MODULES = ("tools.benchmark.held_out", "tools.benchmark.development")
_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
_SHA256 = re.compile(r"[0-9a-f]{64}\Z")
_INTEGRITY = re.compile(r"(sha256|sha512)-([A-Za-z0-9+/=]+)\Z")
_REGISTRY = "https://registry.npmjs.org/"


@dataclass(frozen=True)
class LaneSpec:
    """Everything that makes one npm tarball lane itself.

    `key` is the lane's role, its opportunity-id prefix and its manifest role,
    so an id, a case, a manifest and a queue channel always name the same lane.
    """

    key: str
    preview_id: str
    manifest_path: Path
    driver_prefix: str
    report_artifact_type: str
    user_agent: str
    label: str
    artifact_prefix: str
    title: str
    subtitle: str


@dataclass(frozen=True)
class Package:
    name: str
    version: str
    tarball_url: str
    sha256: str
    integrity: str | None
    license: str
    license_evidence_urls: tuple[str, ...]


@dataclass(frozen=True)
class NpmCase:
    id: str
    role: str
    expected_checksum: str
    files: tuple[SourceFile, ...]
    harness: str = "direct-script"
    iteration_count: int | None = None


@dataclass(frozen=True)
class NpmSuite:
    id: str
    name: str
    reporting_rule: str
    package: Package
    license_review: dict[str, Any]
    source: Source
    cases: tuple[NpmCase, ...]


@dataclass(frozen=True)
class NpmManifest:
    path: Path
    sha256: str
    preview_id: str
    role: str
    measurement: Measurement
    suites: tuple[NpmSuite, ...]

    def opportunity_ids(self) -> frozenset[str]:
        return frozenset(
            f"{self.role}/{suite.id}/{case.id}"
            for suite in self.suites
            for case in suite.cases
        )


def _reject_constant(value: str) -> None:
    raise ExternalPreviewError(f"npm lane manifest contains non-standard numeric constant {value}")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ExternalPreviewError(f"npm lane manifest contains duplicate key {key!r}")
        result[key] = value
    return result


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


def _sha256(value: Any, where: str) -> str:
    text = _string(value, where)
    if not _SHA256.fullmatch(text) or text == "0" * 64:
        raise ExternalPreviewError(f"{where}: expected a non-zero lowercase SHA-256")
    return text


def _id(value: Any, where: str) -> str:
    text = _string(value, where)
    if not _ID.fullmatch(text):
        raise ExternalPreviewError(f"{where}: invalid stable lowercase id")
    return text


def _https(value: Any, where: str) -> str:
    text = _string(value, where)
    if not text.startswith("https://") or any(c.isspace() for c in text):
        raise ExternalPreviewError(f"{where}: expected a credential-free HTTPS URL")
    return text


def _sha256_bytes(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def _package(value: Any, where: str) -> Package:
    data = _keys(
        value,
        {"name", "version", "registry", "tarball_url", "sha256", "integrity", "license",
         "license_evidence_urls"},
        where,
    )
    if _string(data["registry"], f"{where}.registry") != "npm":
        raise ExternalPreviewError(f"{where}.registry: only npm is supported")
    tarball = _https(data["tarball_url"], f"{where}.tarball_url")
    if not tarball.startswith(_REGISTRY):
        raise ExternalPreviewError(f"{where}.tarball_url: expected the npm registry host")
    integrity = data["integrity"]
    if integrity is not None and not _INTEGRITY.fullmatch(_string(integrity, f"{where}.integrity")):
        raise ExternalPreviewError(f"{where}.integrity: expected a sha256/sha512 SRI string")
    urls = data["license_evidence_urls"]
    if not isinstance(urls, list) or not urls:
        raise ExternalPreviewError(f"{where}.license_evidence_urls: expected a non-empty array")
    return Package(
        name=_string(data["name"], f"{where}.name"),
        version=_string(data["version"], f"{where}.version"),
        tarball_url=tarball,
        sha256=_sha256(data["sha256"], f"{where}.sha256"),
        integrity=integrity,
        license=_string(data["license"], f"{where}.license"),
        license_evidence_urls=tuple(
            _https(url, f"{where}.license_evidence_urls[]") for url in urls
        ),
    )


def _license_review(value: Any, where: str) -> dict[str, Any]:
    data = _keys(value, {"status", "redistribute_source_in_artifact", "evidence_urls"}, where)
    if data["status"] != "execution-only":
        raise ExternalPreviewError(f"{where}.status: expected execution-only")
    if data["redistribute_source_in_artifact"] is not False:
        raise ExternalPreviewError(f"{where}: source redistribution must remain disabled")
    urls = data["evidence_urls"]
    if not isinstance(urls, list) or not urls:
        raise ExternalPreviewError(f"{where}.evidence_urls: expected a non-empty array")
    for url in urls:
        if not isinstance(url, str) or not url.startswith("https://"):
            raise ExternalPreviewError(f"{where}.evidence_urls: expected HTTPS URLs")
    return data


def _case(spec: LaneSpec, value: Any, where: str) -> NpmCase:
    data = _keys(value, {"id", "role", "expected_checksum", "driver", "files"}, where)
    if _string(data["role"], f"{where}.role") != spec.key:
        raise ExternalPreviewError(f"{where}.role: every {spec.key} case must be marked {spec.key}")
    driver = _keys(data["driver"], {"path", "sha256"}, f"{where}.driver")
    driver_path = _safe_path(driver["path"], f"{where}.driver.path")
    if not driver_path.startswith(spec.driver_prefix):
        raise ExternalPreviewError(f"{where}.driver.path: expected a {spec.driver_prefix} path")
    files_data = data["files"]
    if not isinstance(files_data, list) or not files_data:
        raise ExternalPreviewError(f"{where}.files: expected a non-empty array")
    files: list[SourceFile] = []
    seen: set[str] = set()
    for index, raw_file in enumerate(files_data):
        file_where = f"{where}.files[{index}]"
        file_data = _keys(raw_file, {"path", "sha256"}, file_where)
        path = _safe_path(file_data["path"], f"{file_where}.path")
        digest = _sha256(file_data["sha256"], f"{file_where}.sha256")
        if path in seen:
            raise ExternalPreviewError(f"{file_where}: duplicate file path")
        seen.add(path)
        files.append(SourceFile(path, digest))
    # The driver is concatenated after the library, so its public API exists first.
    files.append(SourceFile(driver_path, _sha256(driver["sha256"], f"{where}.driver.sha256")))
    return NpmCase(
        id=_id(data["id"], f"{where}.id"),
        role=spec.key,
        expected_checksum=_string(data["expected_checksum"], f"{where}.expected_checksum"),
        files=tuple(files),
    )


def load_npm_manifest(spec: LaneSpec, path: Path, *, check_disjoint: bool = True) -> NpmManifest:
    resolved = path.expanduser().resolve()
    try:
        raw = resolved.read_bytes()
        root = json.loads(
            raw.decode("utf-8"), object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ExternalPreviewError(f"cannot read {spec.key} manifest: {error}") from error
    root = _keys(
        root,
        {"schema_version", "preview_id", "claim_eligible", "role", "measurement", "suites"},
        "manifest",
    )
    if _integer(root["schema_version"], "manifest.schema_version", 1) != 1:
        raise ExternalPreviewError("manifest.schema_version: expected version 1")
    if root["claim_eligible"] is not False:
        raise ExternalPreviewError(f"manifest.claim_eligible: the {spec.key} lane must remain non-claim")
    if _string(root["role"], "manifest.role") != spec.key:
        raise ExternalPreviewError(f"manifest.role: expected {spec.key}")
    preview_id = _string(root["preview_id"], "manifest.preview_id")
    if preview_id != spec.preview_id:
        raise ExternalPreviewError(f"manifest.preview_id: unexpected {spec.key} identity")
    measurement_data = _keys(
        root["measurement"],
        {"blocks", "timeout_seconds", "metric", "phase_boundary", "order", "seed"},
        "manifest.measurement",
    )
    measurement = Measurement(
        blocks=_integer(measurement_data["blocks"], "measurement.blocks", 1),
        timeout_seconds=_integer(measurement_data["timeout_seconds"], "measurement.timeout_seconds", 1),
        metric=_string(measurement_data["metric"], "measurement.metric"),
        phase_boundary=_string(measurement_data["phase_boundary"], "measurement.phase_boundary"),
        order=_string(measurement_data["order"], "measurement.order"),
        seed=_integer(measurement_data["seed"], "measurement.seed", 0),
    )
    if (
        measurement.metric != "outer_process_wall_time"
        or measurement.phase_boundary != "before_process_spawn_to_wait_return"
        or measurement.order != "seeded_role_rotation"
    ):
        raise ExternalPreviewError("manifest.measurement: unsupported protocol")
    suites_data = root["suites"]
    if not isinstance(suites_data, list) or not suites_data:
        raise ExternalPreviewError("manifest.suites: expected a non-empty array")
    suites: list[NpmSuite] = []
    suite_ids: set[str] = set()
    for suite_index, raw_suite in enumerate(suites_data):
        where = f"manifest.suites[{suite_index}]"
        suite_data = _keys(
            raw_suite,
            {"id", "name", "reporting_rule", "package", "license_review", "cases"},
            where,
        )
        suite_id = _id(suite_data["id"], f"{where}.id")
        if suite_id in suite_ids:
            raise ExternalPreviewError("manifest.suites: duplicate suite id")
        suite_ids.add(suite_id)
        package = _package(suite_data["package"], f"{where}.package")
        license_review = _license_review(suite_data["license_review"], f"{where}.license_review")
        cases_data = suite_data["cases"]
        if not isinstance(cases_data, list) or not cases_data:
            raise ExternalPreviewError(f"{where}.cases: expected a non-empty array")
        cases = tuple(_case(spec, raw_case, f"{where}.cases[{index}]")
                      for index, raw_case in enumerate(cases_data))
        if len({case.id for case in cases}) != len(cases):
            raise ExternalPreviewError(f"{where}.cases: duplicate case id")
        suites.append(
            NpmSuite(
                id=suite_id,
                name=_string(suite_data["name"], f"{where}.name"),
                reporting_rule=_string(suite_data["reporting_rule"], f"{where}.reporting_rule"),
                package=package,
                license_review=license_review,
                source=Source(package.tarball_url, package.sha256, ""),
                cases=cases,
            )
        )
    manifest = NpmManifest(
        path=resolved,
        sha256=_sha256_bytes(raw),
        preview_id=preview_id,
        role=spec.key,
        measurement=measurement,
        suites=tuple(suites),
    )
    if check_disjoint:
        assert_disjoint(spec, manifest)
    return manifest


def lane_specs() -> tuple[LaneSpec, ...]:
    """Every registered npm lane, imported so the check never depends on import order."""
    return tuple(importlib.import_module(name).SPEC for name in LANE_MODULES)


def assert_disjoint(spec: LaneSpec, manifest: NpmManifest) -> None:
    """No program and no case id may belong to two lanes.

    The lanes answer different questions -- held-out acceptance never selects
    work, development cases may -- so a library pinned in both would let a
    held-out program select optimization work through its development twin, and
    a shared case id would make a queue, plan or profile receipt ambiguous about
    which program it names. Both the pinned package (by name and by tarball
    hash) and the case ids are therefore checked against every other lane.
    """
    ids = manifest.opportunity_ids()
    for other in lane_specs():
        if other.key == spec.key:
            continue
        other_manifest = load_npm_manifest(other, other.manifest_path, check_disjoint=False)
        shared_names = sorted(
            {suite.package.name for suite in manifest.suites}
            & {suite.package.name for suite in other_manifest.suites}
        )
        shared_tarballs = sorted(
            {suite.package.sha256 for suite in manifest.suites}
            & {suite.package.sha256 for suite in other_manifest.suites}
        )
        shared_ids = sorted(ids & other_manifest.opportunity_ids())
        if shared_names or shared_tarballs or shared_ids:
            raise ExternalPreviewError(
                f"the {spec.key} and {other.key} lanes must not share a program; "
                f"packages={shared_names} tarballs={shared_tarballs} case_ids={shared_ids}"
            )


def npm_opportunity_ids(spec: LaneSpec, manifest_path: Path | None = None) -> frozenset[str]:
    """The frozen case ids of one lane, for the decision tools and their tests."""
    manifest = load_npm_manifest(spec, manifest_path or spec.manifest_path)
    return manifest.opportunity_ids()


def _verify_integrity(integrity: str, content: bytes) -> None:
    match = _INTEGRITY.fullmatch(integrity)
    if match is None:
        raise ExternalPreviewError(f"unsupported integrity string {integrity!r}")
    algorithm, encoded = match.groups()
    digest = hashlib.new(algorithm, content).digest()
    expected = base64.b64decode(encoded)
    if digest != expected:
        raise ExternalPreviewError(f"tarball does not match its {algorithm} integrity")


def _download(spec: LaneSpec, url: str) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": spec.user_agent})
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            return response.read()
    except (OSError, urllib.error.URLError) as error:
        raise ExternalPreviewError(f"cannot fetch {url}: {error}") from error


def _write_cache(target: Path, content: bytes) -> None:
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


def _extract_member(tarball: Path, member_path: str) -> bytes:
    name = f"package/{member_path}"
    try:
        with tarfile.open(tarball, "r:gz") as archive:
            member = archive.getmember(name)
            if not member.isfile():
                raise ExternalPreviewError(f"{member_path}: not a regular file in the tarball")
            handle = archive.extractfile(member)
            if handle is None:
                raise ExternalPreviewError(f"{member_path}: cannot read tarball member")
            return handle.read()
    except KeyError as error:
        raise ExternalPreviewError(f"{member_path}: not present in the tarball") from error
    except tarfile.TarError as error:
        raise ExternalPreviewError(f"cannot read tarball for {member_path}: {error}") from error


def fetch_sources(spec: LaneSpec, manifest: NpmManifest, cache_root: Path) -> dict[str, int]:
    """Download each pinned tarball into the hash-checked cache and lay out the
    case sources where the shared bundler expects them."""
    root = cache_root.expanduser().resolve()
    tarball_root = root / "_tarballs"
    downloaded = 0
    reused = 0
    for suite in manifest.suites:
        package = suite.package
        tarball = tarball_root / f"{package.name}-{package.version}.tgz"
        if not (tarball.is_file() and _sha256_file(tarball) == package.sha256):
            content = _download(spec, package.tarball_url)
            if _sha256_bytes(content) != package.sha256:
                raise ExternalPreviewError(f"downloaded tarball hash mismatch for {package.name}")
            if package.integrity is not None:
                _verify_integrity(package.integrity, content)
            _write_cache(tarball, content)
            downloaded += 1
        for case in suite.cases:
            for source_file in case.files:
                target = root / suite.id / source_file.path
                if target.is_file() and _sha256_file(target) == source_file.sha256:
                    reused += 1
                    continue
                if source_file.path.startswith(spec.driver_prefix):
                    origin = ROOT / source_file.path
                    try:
                        content = origin.read_bytes()
                    except OSError as error:
                        raise ExternalPreviewError(
                            f"cannot read first-party driver {source_file.path}: {error}"
                        ) from error
                else:
                    content = _extract_member(tarball, source_file.path)
                if _sha256_bytes(content) != source_file.sha256:
                    raise ExternalPreviewError(
                        f"content hash mismatch for {suite.id}/{source_file.path}"
                    )
                _write_cache(target, content)
                downloaded += 1
    return {"downloaded": downloaded, "reused": reused}


def _sentinel_lines(role: str) -> str:
    return f"{SENTINEL}\n" if role == "quickjs-ng" else f"{SENTINEL}\n{SENTINEL}\n"


def npm_status(
    result: ProcessResult, role: str, case: NpmCase
) -> tuple[str, str | None]:
    if result.timed_out:
        return "timeout", "process exceeded the case timeout"
    if result.exit_code != 0:
        return "failed", f"process exited with status {result.exit_code}"
    if result.stdout_truncated or result.stderr_truncated:
        return "invalid", "process output exceeded the capture limit"
    if role not in EXTERNAL_ROLES:
        raise ExternalPreviewError(f"unknown engine role {role}")
    expected = f"CHECKSUM {case.expected_checksum}\n{_sentinel_lines(role)}"
    if result.stdout != expected:
        return "invalid", "stdout did not match the pinned checksum and sentinel contract"
    return "ok", None


def npm_report(spec: LaneSpec, manifest: NpmManifest, records: list[dict[str, Any]]) -> dict[str, Any]:
    report = _external_report(
        manifest, records, artifact_type=spec.report_artifact_type
    )
    report["role"] = spec.key
    by_case = {
        (suite.id, case.id): case for suite in manifest.suites for case in suite.cases
    }
    for suite in report["suites"]:
        for case in suite["cases"]:
            case["role"] = spec.key
            case["expected_checksum"] = by_case[(suite["id"], case["id"])].expected_checksum
    return report


def lane(spec: LaneSpec) -> Lane:
    return Lane(
        label=spec.label,
        artifact_prefix=spec.artifact_prefix,
        title=spec.title,
        subtitle=spec.subtitle,
        fetch=lambda manifest, cache_root: fetch_sources(spec, manifest, cache_root),
        status=npm_status,
        report=lambda manifest, records: npm_report(spec, manifest, records),
    )


def run_npm_preview(
    spec: LaneSpec,
    manifest: NpmManifest,
    cache_root: Path,
    work_root: Path,
    output_dir: Path,
    candidate_path: Path,
    base_path: Path,
    quickjs_path: Path,
    *,
    blocks: int | None = None,
    timeout_seconds: int | None = None,
) -> dict[str, Any]:
    return run_preview(
        manifest, cache_root, work_root, output_dir, candidate_path, base_path, quickjs_path,
        blocks=blocks, timeout_seconds=timeout_seconds, lane=lane(spec),
    )


def parser(spec: LaneSpec) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=f"Pinned {spec.key} benchmark lane.")
    parser.add_argument("--manifest", type=Path, default=spec.manifest_path)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("audit")
    fetch = commands.add_parser("fetch")
    fetch.add_argument("--cache-root", type=Path, required=True)
    run = commands.add_parser("run")
    run.add_argument("--cache-root", type=Path, required=True)
    run.add_argument("--work-root", type=Path, required=True)
    run.add_argument("--output-dir", type=Path, required=True)
    run.add_argument("--candidate", type=Path, required=True)
    run.add_argument("--base", type=Path, required=True)
    run.add_argument("--quickjs-ng", type=Path, required=True)
    run.add_argument("--blocks", type=int)
    run.add_argument("--timeout-seconds", type=int)
    return parser


def cli(spec: LaneSpec, argv: list[str] | None = None) -> int:
    args = parser(spec).parse_args(argv)
    try:
        manifest = load_npm_manifest(spec, args.manifest)
        if args.command == "audit":
            print(json.dumps({
                "claim_eligible": False,
                "manifest_sha256": manifest.sha256,
                "opportunity_ids": sorted(manifest.opportunity_ids()),
                "preview_id": manifest.preview_id,
                "case_count": sum(len(suite.cases) for suite in manifest.suites),
                "suite_count": len(manifest.suites),
            }, sort_keys=True, separators=(",", ":")))
        elif args.command == "fetch":
            print(json.dumps(fetch_sources(spec, manifest, args.cache_root), sort_keys=True))
        else:
            report = run_npm_preview(
                spec, manifest, args.cache_root, args.work_root, args.output_dir,
                args.candidate, args.base, args.quickjs_ng,
                blocks=args.blocks, timeout_seconds=args.timeout_seconds,
            )
            print(json.dumps({
                "claim_eligible": False,
                "report": str(
                    (args.output_dir / f"{spec.artifact_prefix}-report.json").resolve()
                ),
                "suites": len(report["suites"]),
            }, sort_keys=True))
        return 0
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
