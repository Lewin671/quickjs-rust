"""Strict, independently versioned resource measurement manifest."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


class ResourceManifestError(ValueError):
    """The resource measurement manifest is invalid."""


def _reject_constant(value: str) -> None:
    raise ResourceManifestError(f"resource manifest contains non-standard constant {value}")


def _unique(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ResourceManifestError(f"resource manifest contains duplicate key {key!r}")
        result[key] = value
    return result


def _keys(value: Any, expected: set[str], where: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        actual = set(value) if isinstance(value, dict) else set()
        raise ResourceManifestError(
            f"{where}: missing {sorted(expected - actual)}, unknown {sorted(actual - expected)}"
        )
    return value


def _string(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value or value.strip() != value:
        raise ResourceManifestError(f"{where}: expected non-empty trimmed string")
    return value


def _integer(value: Any, where: str, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        raise ResourceManifestError(f"{where}: expected integer >= {minimum}")
    return value


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _resolve(root: Path, identifier: str, where: str) -> Path:
    relative = Path(identifier)
    if relative.is_absolute() or ".." in relative.parts:
        raise ResourceManifestError(f"{where}: path must stay inside repository")
    path = (root / relative).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError as error:
        raise ResourceManifestError(f"{where}: path escapes repository") from error
    if not path.is_file():
        raise ResourceManifestError(f"{where}: missing file {identifier}")
    return path


@dataclass(frozen=True)
class ResourceProfile:
    id: str
    platform: str
    machine: str
    rss_raw_unit: str


@dataclass(frozen=True)
class BuildRecipe:
    engine_identity: str
    build_mode: str
    toolchain: str
    target: str
    features: tuple[str, ...]
    flags: tuple[str, ...]
    lto: str
    strip: str
    allocator: str
    host_features: str


@dataclass(frozen=True)
class ResourceCase:
    id: str
    family: str
    workload: Path
    workload_id: str
    workload_sha256: str
    iterations: int
    operations: int
    checksum_factor: int
    timeout_seconds: int

    def expected_checksum(self) -> int:
        return self.iterations * self.checksum_factor


@dataclass(frozen=True)
class ResourceLane:
    id: str
    kind: str
    unit: str
    metric: str
    initial_blocks: int
    max_blocks: int
    seed: int
    case: ResourceCase | None


@dataclass(frozen=True)
class ResourceManifest:
    path: Path
    sha256: str
    schema_version: int
    series_id: str
    suite_id: str
    protocol_id: str
    protocol_file_ids: tuple[str, ...]
    protocol_files: tuple[Path, ...]
    protocol_sha256: str
    reference_identity: str
    reference_repo: str
    reference_revision: str
    profile: ResourceProfile
    build_recipes: dict[str, BuildRecipe]
    lanes: dict[str, ResourceLane]


LANE_SHAPES = {
    "binary_size/bytes": ("static", "bytes", "binary_size"),
    "fresh_process_latency/wall_ns_per_process": (
        "dynamic", "nanoseconds", "outer_wall_time",
    ),
    "peak_rss/bytes": ("dynamic", "bytes", "peak_rss"),
}


def load_resource_manifest(path: Path) -> ResourceManifest:
    path = path.expanduser().resolve()
    try:
        raw = path.read_bytes()
        data = json.loads(raw, object_pairs_hook=_unique, parse_constant=_reject_constant)
    except (OSError, json.JSONDecodeError) as error:
        raise ResourceManifestError(f"cannot read resource manifest {path}: {error}") from error
    _keys(
        data,
        {"schema_version", "series", "protocol", "reference_engine", "profile",
         "build_recipes", "lanes"},
        "resource manifest",
    )
    if isinstance(data["schema_version"], bool) or data["schema_version"] != 1:
        raise ResourceManifestError("resource manifest.schema_version: only integer 1 is supported")
    root = path.parent.parent if path.parent.name == "benchmarks" else path.parent

    series = _keys(data["series"], {"id", "suite_id"}, "resource manifest.series")
    protocol = _keys(
        data["protocol"], {"id", "files", "aggregate_sha256"}, "resource manifest.protocol"
    )
    if not isinstance(protocol["files"], list) or not protocol["files"]:
        raise ResourceManifestError("resource manifest.protocol.files: expected non-empty array")
    file_ids = [_string(value, "resource manifest.protocol.files[]") for value in protocol["files"]]
    if file_ids != sorted(file_ids) or len(file_ids) != len(set(file_ids)):
        raise ResourceManifestError("resource manifest.protocol.files: must be unique and sorted")
    files = tuple(_resolve(root, value, "resource manifest.protocol.files[]") for value in file_ids)
    digest = hashlib.sha256()
    for identifier, file_path in zip(file_ids, files):
        digest.update(identifier.encode("utf-8"))
        digest.update(b"\0")
        digest.update(bytes.fromhex(sha256_file(file_path)))
        digest.update(b"\n")
    expected_digest = _string(
        protocol["aggregate_sha256"], "resource manifest.protocol.aggregate_sha256"
    )
    if digest.hexdigest() != expected_digest:
        raise ResourceManifestError(
            "resource manifest.protocol.aggregate_sha256: mismatch: "
            f"expected {expected_digest}, got {digest.hexdigest()}"
        )

    reference = _keys(
        data["reference_engine"], {"identity", "source_repo", "revision"},
        "resource manifest.reference_engine",
    )
    revision = _string(reference["revision"], "resource manifest.reference_engine.revision")
    if len(revision) != 40 or any(character not in "0123456789abcdef" for character in revision):
        raise ResourceManifestError("resource manifest.reference_engine.revision: invalid SHA")

    profile_data = _keys(
        data["profile"], {"id", "platform", "machine", "rss_raw_unit"},
        "resource manifest.profile"
    )
    profile = ResourceProfile(**{
        field: _string(profile_data[field], f"resource manifest.profile.{field}")
        for field in ("id", "platform", "machine", "rss_raw_unit")
    })
    expected_unit = {"darwin": "bytes", "linux": "kibibytes"}.get(profile.platform)
    if expected_unit is None or profile.rss_raw_unit != expected_unit:
        raise ResourceManifestError("resource manifest.profile: unsupported platform/RSS unit pair")

    recipe_fields = {
        "engine_identity", "build_mode", "toolchain", "target", "features", "flags",
        "lto", "strip", "allocator", "host_features",
    }
    if not isinstance(data["build_recipes"], list) or not data["build_recipes"]:
        raise ResourceManifestError("resource manifest.build_recipes: expected non-empty array")
    recipes: dict[str, BuildRecipe] = {}
    for index, item in enumerate(data["build_recipes"]):
        where = f"resource manifest.build_recipes[{index}]"
        item = _keys(item, recipe_fields, where)
        identity = _string(item["engine_identity"], f"{where}.engine_identity")
        if identity in recipes:
            raise ResourceManifestError(f"{where}: duplicate identity")
        strings = {
            field: _string(item[field], f"{where}.{field}")
            for field in recipe_fields - {"engine_identity", "features", "flags"}
        }
        for field in ("features", "flags"):
            if not isinstance(item[field], list) or any(
                not isinstance(value, str) or not value for value in item[field]
            ) or len(item[field]) != len(set(item[field])):
                raise ResourceManifestError(f"{where}.{field}: expected unique strings")
        recipes[identity] = BuildRecipe(
            engine_identity=identity,
            features=tuple(item["features"]), flags=tuple(item["flags"]), **strings,
        )
    if set(recipes) != {"qjs-rust", "quickjs-ng"}:
        raise ResourceManifestError("resource manifest.build_recipes: exact identities required")

    if not isinstance(data["lanes"], list) or len(data["lanes"]) != 3:
        raise ResourceManifestError("resource manifest.lanes: exact three lanes required")
    lanes: dict[str, ResourceLane] = {}
    for index, item in enumerate(data["lanes"]):
        where = f"resource manifest.lanes[{index}]"
        item = _keys(
            item,
            {"id", "kind", "unit", "metric", "initial_blocks", "max_blocks", "seed", "case"},
            where,
        )
        lane_id = _string(item["id"], f"{where}.id")
        shape = LANE_SHAPES.get(lane_id)
        if shape is None or tuple(item[field] for field in ("kind", "unit", "metric")) != shape:
            raise ResourceManifestError(f"{where}: invalid frozen lane shape")
        initial = _integer(item["initial_blocks"], f"{where}.initial_blocks", 1)
        maximum = _integer(item["max_blocks"], f"{where}.max_blocks", 1)
        seed = _integer(item["seed"], f"{where}.seed")
        case_data = item["case"]
        case = None
        if shape[0] == "static":
            if case_data is not None or initial != 1 or maximum != 1:
                raise ResourceManifestError(f"{where}: static lane requires null case and one sample")
        else:
            if (initial, maximum) != (30, 60):
                raise ResourceManifestError(f"{where}: dynamic lane requires frozen 30/60 cohorts")
            case_data = _keys(
                case_data,
                {"id", "family", "workload", "workload_sha256", "iterations", "operations",
                 "checksum_factor", "timeout_seconds"},
                f"{where}.case",
            )
            workload_id = _string(case_data["workload"], f"{where}.case.workload")
            workload = _resolve(root, workload_id, f"{where}.case.workload")
            workload_hash = _string(case_data["workload_sha256"], f"{where}.case.workload_sha256")
            if sha256_file(workload) != workload_hash:
                raise ResourceManifestError(f"{where}.case.workload_sha256: mismatch")
            case = ResourceCase(
                id=_string(case_data["id"], f"{where}.case.id"),
                family=_string(case_data["family"], f"{where}.case.family"),
                workload=workload,
                workload_id=workload_id,
                workload_sha256=workload_hash,
                iterations=_integer(case_data["iterations"], f"{where}.case.iterations", 1),
                operations=_integer(case_data["operations"], f"{where}.case.operations", 1),
                checksum_factor=_integer(
                    case_data["checksum_factor"], f"{where}.case.checksum_factor", 1
                ),
                timeout_seconds=_integer(
                    case_data["timeout_seconds"], f"{where}.case.timeout_seconds", 1
                ),
            )
        if lane_id in lanes:
            raise ResourceManifestError(f"{where}: duplicate lane")
        if case is not None:
            expected_case = {
                "fresh_process_latency/wall_ns_per_process": (
                    "fresh_process_probe", "process", 1, 1, 7, 10,
                ),
                "peak_rss/bytes": (
                    "peak_rss_probe", "memory", 32768, 32768, 3, 10,
                ),
            }[lane_id]
            actual_case = (
                case.id, case.family, case.iterations, case.operations,
                case.checksum_factor, case.timeout_seconds,
            )
            if actual_case != expected_case:
                raise ResourceManifestError(f"{where}.case: frozen probe contract mismatch")
        lanes[lane_id] = ResourceLane(
            lane_id, shape[0], shape[1], shape[2], initial, maximum, seed, case
        )
    if set(lanes) != set(LANE_SHAPES):
        raise ResourceManifestError("resource manifest.lanes: frozen lane set mismatch")
    return ResourceManifest(
        path=path, sha256=hashlib.sha256(raw).hexdigest(), schema_version=1,
        series_id=_string(series["id"], "resource manifest.series.id"),
        suite_id=_string(series["suite_id"], "resource manifest.series.suite_id"),
        protocol_id=_string(protocol["id"], "resource manifest.protocol.id"),
        protocol_file_ids=tuple(file_ids), protocol_files=files,
        protocol_sha256=expected_digest,
        reference_identity=_string(reference["identity"], "resource manifest.reference_engine.identity"),
        reference_repo=_string(reference["source_repo"], "resource manifest.reference_engine.source_repo"),
        reference_revision=revision, profile=profile, build_recipes=recipes, lanes=lanes,
    )
