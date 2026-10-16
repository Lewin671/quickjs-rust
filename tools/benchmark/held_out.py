"""Held-out acceptance lane: real third-party programs never used to select work.

The opportunity queue and every unit plan come from the pinned performance
suites (SunSpider, Kraken, the JetStream subset) and the broad/sentinel
micro-workloads. If those suites were also the only acceptance check, an
optimization tuned to their shapes would be invisible. This lane is the
independent check: unmodified, pinned third-party npm libraries, driven by
first-party scripts, measured with the same whole-process wall-time protocol
as the external lane but reported separately.

Every case runs only through the language and `print`; each driver builds its
own deterministic input, calls the library's public API, and prints one
`CHECKSUM <hex>` line. A sample is accepted only when its stdout is exactly
that checksum followed by the lane's sentinel contract, so a checksum that
differs between engines is recorded as an invalid sample, not a slow one.

`opportunity_ids` is the registry the decision tools use to refuse a plan that
names a held-out case as a target, control or profile workload: this lane
accepts work, it never selects it.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import re
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
MANIFEST_PATH = ROOT / "benchmarks/held-out-preview.json"
PREVIEW_ID = "quickjs-held-out-preview-v1"
DRIVER_PREFIX = "benchmarks/held-out/"
_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
_SHA256 = re.compile(r"[0-9a-f]{64}\Z")
_INTEGRITY = re.compile(r"(sha256|sha512)-([A-Za-z0-9+/=]+)\Z")
_REGISTRY = "https://registry.npmjs.org/"


def _reject_constant(value: str) -> None:
    raise ExternalPreviewError(f"held-out manifest contains non-standard numeric constant {value}")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ExternalPreviewError(f"held-out manifest contains duplicate key {key!r}")
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
class HeldOutCase:
    id: str
    role: str
    expected_checksum: str
    files: tuple[SourceFile, ...]
    harness: str = "direct-script"
    iteration_count: int | None = None


@dataclass(frozen=True)
class HeldOutSuite:
    id: str
    name: str
    reporting_rule: str
    package: Package
    license_review: dict[str, Any]
    source: Source
    cases: tuple[HeldOutCase, ...]


@dataclass(frozen=True)
class HeldOutManifest:
    path: Path
    sha256: str
    preview_id: str
    role: str
    measurement: Measurement
    suites: tuple[HeldOutSuite, ...]

    def opportunity_ids(self) -> frozenset[str]:
        return frozenset(
            f"held-out/{suite.id}/{case.id}"
            for suite in self.suites
            for case in suite.cases
        )


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


def _case(value: Any, where: str) -> HeldOutCase:
    data = _keys(
        value, {"id", "role", "expected_checksum", "driver", "files"}, where
    )
    if _string(data["role"], f"{where}.role") != "held-out":
        raise ExternalPreviewError(f"{where}.role: every held-out case must be marked held-out")
    driver = _keys(data["driver"], {"path", "sha256"}, f"{where}.driver")
    driver_path = _safe_path(driver["path"], f"{where}.driver.path")
    if not driver_path.startswith(DRIVER_PREFIX):
        raise ExternalPreviewError(f"{where}.driver.path: expected a {DRIVER_PREFIX} path")
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
    return HeldOutCase(
        id=_id(data["id"], f"{where}.id"),
        role="held-out",
        expected_checksum=_string(data["expected_checksum"], f"{where}.expected_checksum"),
        files=tuple(files),
    )


def load_held_out_manifest(path: Path) -> HeldOutManifest:
    resolved = path.expanduser().resolve()
    try:
        raw = resolved.read_bytes()
        root = json.loads(
            raw.decode("utf-8"), object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ExternalPreviewError(f"cannot read held-out manifest: {error}") from error
    root = _keys(
        root,
        {"schema_version", "preview_id", "claim_eligible", "role", "measurement", "suites"},
        "manifest",
    )
    if _integer(root["schema_version"], "manifest.schema_version", 1) != 1:
        raise ExternalPreviewError("manifest.schema_version: expected version 1")
    if root["claim_eligible"] is not False:
        raise ExternalPreviewError("manifest.claim_eligible: held-out lane must remain non-claim")
    if _string(root["role"], "manifest.role") != "held-out":
        raise ExternalPreviewError("manifest.role: expected held-out")
    preview_id = _string(root["preview_id"], "manifest.preview_id")
    if preview_id != PREVIEW_ID:
        raise ExternalPreviewError("manifest.preview_id: unexpected held-out identity")
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
    suites: list[HeldOutSuite] = []
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
        cases = tuple(_case(raw_case, f"{where}.cases[{index}]")
                      for index, raw_case in enumerate(cases_data))
        if len({case.id for case in cases}) != len(cases):
            raise ExternalPreviewError(f"{where}.cases: duplicate case id")
        suites.append(
            HeldOutSuite(
                id=suite_id,
                name=_string(suite_data["name"], f"{where}.name"),
                reporting_rule=_string(suite_data["reporting_rule"], f"{where}.reporting_rule"),
                package=package,
                license_review=license_review,
                source=Source(package.tarball_url, package.sha256, ""),
                cases=cases,
            )
        )
    return HeldOutManifest(
        path=resolved,
        sha256=_sha256_bytes(raw),
        preview_id=preview_id,
        role="held-out",
        measurement=measurement,
        suites=tuple(suites),
    )


def opportunity_ids(manifest_path: Path | None = None) -> frozenset[str]:
    """The frozen set of held-out opportunity ids, for the decision tools."""
    manifest = load_held_out_manifest(manifest_path or MANIFEST_PATH)
    return frozenset(
        f"held-out/{suite.id}/{case.id}"
        for suite in manifest.suites
        for case in suite.cases
    )


def _verify_integrity(integrity: str, content: bytes) -> None:
    match = _INTEGRITY.fullmatch(integrity)
    if match is None:
        raise ExternalPreviewError(f"unsupported integrity string {integrity!r}")
    algorithm, encoded = match.groups()
    digest = hashlib.new(algorithm, content).digest()
    expected = base64.b64decode(encoded)
    if digest != expected:
        raise ExternalPreviewError(f"tarball does not match its {algorithm} integrity")


def _download(url: str) -> bytes:
    request = urllib.request.Request(
        url, headers={"User-Agent": "quickjs-rust-held-out-preview/1"}
    )
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


def fetch_sources(manifest: HeldOutManifest, cache_root: Path) -> dict[str, int]:
    """Download each pinned tarball into the hash-checked cache and lay out the
    case sources where the shared bundler expects them."""
    held = (
        manifest if isinstance(manifest, HeldOutManifest)
        else load_held_out_manifest(manifest.path)
    )
    root = cache_root.expanduser().resolve()
    tarball_root = root / "_tarballs"
    downloaded = 0
    reused = 0
    for suite in held.suites:
        package = suite.package
        tarball = tarball_root / f"{package.name}-{package.version}.tgz"
        if not (tarball.is_file() and _sha256_file(tarball) == package.sha256):
            content = _download(package.tarball_url)
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
                if source_file.path.startswith(DRIVER_PREFIX):
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


def held_out_status(
    result: ProcessResult, role: str, case: HeldOutCase
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
        return "invalid", "stdout did not match the held-out checksum and sentinel contract"
    return "ok", None


def held_out_report(manifest: HeldOutManifest, records: list[dict[str, Any]]) -> dict[str, Any]:
    report = _external_report(
        manifest, records, artifact_type="quickjs-held-out-preview-report"
    )
    report["role"] = "held-out"
    by_case = {
        (suite.id, case.id): case for suite in manifest.suites for case in suite.cases
    }
    for suite in report["suites"]:
        for case in suite["cases"]:
            case["role"] = "held-out"
            case["expected_checksum"] = by_case[(suite["id"], case["id"])].expected_checksum
    return report


HELD_OUT_LANE = Lane(
    label="held-out-preview",
    artifact_prefix="held-out",
    title="Held-out Benchmark Preview",
    subtitle=(
        "These are pinned, unmodified third-party npm libraries, never used to select "
        "work; no row is an official benchmark score."
    ),
    fetch=fetch_sources,
    status=held_out_status,
    report=held_out_report,
)


def run_held_out_preview(
    manifest: HeldOutManifest,
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
        blocks=blocks, timeout_seconds=timeout_seconds, lane=HELD_OUT_LANE,
    )


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--manifest", type=Path, default=MANIFEST_PATH)
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


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        manifest = load_held_out_manifest(args.manifest)
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
            print(json.dumps(fetch_sources(manifest, args.cache_root), sort_keys=True))
        else:
            report = run_held_out_preview(
                manifest, args.cache_root, args.work_root, args.output_dir,
                args.candidate, args.base, args.quickjs_ng,
                blocks=args.blocks, timeout_seconds=args.timeout_seconds,
            )
            print(json.dumps({
                "claim_eligible": False,
                "report": str((args.output_dir / "held-out-report.json").resolve()),
                "suites": len(report["suites"]),
            }, sort_keys=True))
        return 0
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
