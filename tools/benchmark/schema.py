"""Strict manifest loading for reproducible benchmark series."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from decimal import Decimal, DecimalException
from fractions import Fraction
from pathlib import Path
from typing import Any


class ManifestError(ValueError):
    """The manifest cannot identify a trustworthy benchmark series."""


def _reject_json_constant(value: str) -> None:
    raise ManifestError(f"manifest JSON contains non-standard numeric constant {value}")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ManifestError(f"manifest JSON contains duplicate key {key!r}")
        result[key] = value
    return result


def _keys(value: dict[str, Any], expected: set[str], where: str) -> None:
    unknown = set(value) - expected
    missing = expected - set(value)
    if unknown or missing:
        details = []
        if missing:
            details.append(f"missing {sorted(missing)}")
        if unknown:
            details.append(f"unknown {sorted(unknown)}")
        raise ManifestError(f"{where}: {', '.join(details)}")


def _string(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value or value.strip() != value:
        raise ManifestError(f"{where}: expected a non-empty trimmed string")
    return value


def _integer(value: Any, where: str, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        raise ManifestError(f"{where}: expected integer >= {minimum}")
    return value


def _bounded_fraction(
    value: Any,
    where: str,
    *,
    minimum: Decimal,
    maximum: Decimal,
    minimum_inclusive: bool,
) -> Fraction:
    if isinstance(value, bool) or not isinstance(value, (int, Decimal)):
        raise ManifestError(f"{where}: expected a JSON number")
    decimal_value = Decimal(value)
    if not decimal_value.is_finite():
        raise ManifestError(
            f"{where}: expected finite {minimum} "
            f"{'<=' if minimum_inclusive else '<'} value <= {maximum}"
        )
    minimum_valid = (
        decimal_value >= minimum if minimum_inclusive else decimal_value > minimum
    )
    if not minimum_valid or decimal_value > maximum:
        operator = "<=" if minimum_inclusive else "<"
        raise ManifestError(
            f"{where}: expected finite {minimum} {operator} value <= {maximum}"
        )
    _sign, digits, exponent = decimal_value.as_tuple()
    if len(digits) > 18 or exponent < -18:
        raise ManifestError(
            f"{where}: expected at most 18 significant digits and 18 decimal places"
        )
    try:
        return Fraction(decimal_value)
    except (OverflowError, ValueError, ZeroDivisionError) as error:
        raise ManifestError(f"{where}: invalid exact decimal value") from error


CALIBRATION_MAX_GROWTH = 16


def next_calibration_iterations(
    iterations: int, target_ns: int, duration_ns: int, max_iterations: int
) -> int:
    """Scale iterations proportionally, rounding up with bounded strict progress."""
    if not 0 < iterations < max_iterations or target_ns <= 0 or duration_ns <= 0:
        raise ValueError("invalid calibration progression inputs")
    proportional = (iterations * target_ns + duration_ns - 1) // duration_ns
    next_iterations = max(iterations + 1, proportional)
    return min(max_iterations, iterations * CALIBRATION_MAX_GROWTH, next_iterations)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


@dataclass(frozen=True)
class Profile:
    id: str
    platform: str


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
class Case:
    id: str
    family: str
    critical: bool
    workload: Path
    workload_sha256: str
    operations_per_iteration: int
    checksum_model: str
    checksum_factor: int
    initial_iterations: int
    max_iterations: int
    min_window_ms: int
    startup_max_fraction: Fraction
    calibration_safety_factor: Fraction
    warmup_runs: int
    timeout_seconds: int

    def calibration_target_ns(self, startup_ns: int) -> int:
        """Return the safety-adjusted calibration target, rounded up to integer ns."""
        eligibility_target = max(
            Fraction(self.min_window_ms * 1_000_000),
            Fraction(startup_ns) / self.startup_max_fraction,
        )
        target = eligibility_target * self.calibration_safety_factor
        return (target.numerator + target.denominator - 1) // target.denominator

    def expected_operations(self, iterations: int) -> int:
        return iterations * self.operations_per_iteration

    def expected_checksum(self, iterations: int) -> int:
        if self.checksum_model == "linear":
            return iterations * self.checksum_factor
        if self.checksum_model == "triangular":
            return iterations * (iterations + 1) // 2
        if self.checksum_model == "zero_based_triangular_plus_linear":
            return iterations * (iterations - 1) // 2 + iterations * self.checksum_factor
        raise AssertionError(f"validated unknown checksum model {self.checksum_model}")


@dataclass(frozen=True)
class Manifest:
    path: Path
    sha256: str
    schema_version: int
    series_id: str
    suite_id: str
    lane_id: str
    protocol_id: str
    protocol_file_ids: tuple[str, ...]
    protocol_files: tuple[Path, ...]
    protocol_sha256: str
    reference_identity: str
    reference_repo: str
    reference_revision: str
    profile: Profile
    build_recipes: dict[str, BuildRecipe]
    cases: tuple[Case, ...]


def _resolve_workload(root: Path, value: Any, where: str) -> Path:
    relative = Path(_string(value, where))
    if relative.is_absolute() or ".." in relative.parts:
        raise ManifestError(f"{where}: path must stay inside the repository")
    path = (root / relative).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError as error:
        raise ManifestError(f"{where}: path escapes the repository") from error
    if not path.is_file():
        raise ManifestError(f"{where}: file does not exist: {relative}")
    return path


def _protocol_sha256(root: Path, relative_paths: list[str]) -> tuple[tuple[Path, ...], str]:
    if relative_paths != sorted(relative_paths) or len(set(relative_paths)) != len(relative_paths):
        raise ManifestError("manifest.protocol.files: paths must be unique and sorted")
    digest = hashlib.sha256()
    resolved = []
    for index, relative in enumerate(relative_paths):
        path = _resolve_workload(root, relative, f"manifest.protocol.files[{index}]")
        resolved.append(path)
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(bytes.fromhex(sha256_file(path)))
        digest.update(b"\n")
    return tuple(resolved), digest.hexdigest()


def load_manifest(path: Path) -> Manifest:
    path = path.resolve()
    try:
        raw = path.read_bytes()
        data = json.loads(
            raw,
            object_pairs_hook=_unique_object,
            parse_constant=_reject_json_constant,
            parse_float=Decimal,
        )
    except ManifestError:
        raise
    except (OSError, ValueError, DecimalException) as error:
        raise ManifestError(f"cannot read manifest {path}: {error}") from error
    if not isinstance(data, dict):
        raise ManifestError("manifest: expected an object")
    _keys(
        data,
        {
            "schema_version", "series", "protocol", "reference_engine",
            "lane", "profile", "build_recipes", "cases",
        },
        "manifest",
    )
    if isinstance(data["schema_version"], bool) or data["schema_version"] != 4:
        raise ManifestError("manifest.schema_version: only integer version 4 is supported")

    series = data["series"]
    if not isinstance(series, dict):
        raise ManifestError("manifest.series: expected an object")
    _keys(series, {"id", "suite_id"}, "manifest.series")

    lane = data["lane"]
    if not isinstance(lane, dict):
        raise ManifestError("manifest.lane: expected an object")
    _keys(lane, {"id"}, "manifest.lane")
    lane_id = _string(lane["id"], "manifest.lane.id")
    if lane_id != "throughput/wall_ns_per_operation":
        raise ManifestError(
            "manifest.lane.id: version 4 requires throughput/wall_ns_per_operation"
        )

    root = path.parent.parent if path.parent.name == "benchmarks" else path.parent
    protocol = data["protocol"]
    if not isinstance(protocol, dict):
        raise ManifestError("manifest.protocol: expected an object")
    _keys(protocol, {"id", "files", "aggregate_sha256"}, "manifest.protocol")
    if not isinstance(protocol["files"], list) or not protocol["files"]:
        raise ManifestError("manifest.protocol.files: expected a non-empty array")
    protocol_paths = [_string(value, "manifest.protocol.files[]") for value in protocol["files"]]
    protocol_files, actual_protocol_hash = _protocol_sha256(root, protocol_paths)
    expected_protocol_hash = _string(
        protocol["aggregate_sha256"], "manifest.protocol.aggregate_sha256"
    )
    if actual_protocol_hash != expected_protocol_hash:
        raise ManifestError(
            "manifest.protocol.aggregate_sha256: mismatch: "
            f"expected {expected_protocol_hash}, got {actual_protocol_hash}"
        )

    reference = data["reference_engine"]
    if not isinstance(reference, dict):
        raise ManifestError("manifest.reference_engine: expected an object")
    _keys(reference, {"identity", "source_repo", "revision"}, "manifest.reference_engine")
    reference_revision = _string(reference["revision"], "manifest.reference_engine.revision")
    if len(reference_revision) != 40 or any(
        character not in "0123456789abcdef" for character in reference_revision
    ):
        raise ManifestError("manifest.reference_engine.revision: expected a full lowercase git SHA")

    profile_data = data["profile"]
    if not isinstance(profile_data, dict):
        raise ManifestError("manifest.profile: expected an object")
    profile_fields = {"id", "platform"}
    _keys(profile_data, profile_fields, "manifest.profile")
    profile = Profile(**{key: _string(profile_data[key], f"manifest.profile.{key}") for key in profile_fields})

    recipes_data = data["build_recipes"]
    if not isinstance(recipes_data, list) or not recipes_data:
        raise ManifestError("manifest.build_recipes: expected a non-empty array")
    recipe_fields = {
        "engine_identity", "build_mode", "toolchain", "target", "features", "flags",
        "lto", "strip", "allocator", "host_features",
    }
    build_recipes: dict[str, BuildRecipe] = {}
    for index, item in enumerate(recipes_data):
        where = f"manifest.build_recipes[{index}]"
        if not isinstance(item, dict):
            raise ManifestError(f"{where}: expected an object")
        _keys(item, recipe_fields, where)
        identity = _string(item["engine_identity"], f"{where}.engine_identity")
        if identity in build_recipes:
            raise ManifestError(f"{where}.engine_identity: duplicate {identity!r}")
        array_values = {}
        for field in ("features", "flags"):
            values = item[field]
            if not isinstance(values, list):
                raise ManifestError(f"{where}.{field}: expected an array")
            strings = tuple(_string(value, f"{where}.{field}[]") for value in values)
            if len(strings) != len(set(strings)):
                raise ManifestError(f"{where}.{field}: duplicate values are not allowed")
            array_values[field] = strings
        build_recipes[identity] = BuildRecipe(
            engine_identity=identity,
            build_mode=_string(item["build_mode"], f"{where}.build_mode"),
            toolchain=_string(item["toolchain"], f"{where}.toolchain"),
            target=_string(item["target"], f"{where}.target"),
            features=array_values["features"],
            flags=array_values["flags"],
            lto=_string(item["lto"], f"{where}.lto"),
            strip=_string(item["strip"], f"{where}.strip"),
            allocator=_string(item["allocator"], f"{where}.allocator"),
            host_features=_string(item["host_features"], f"{where}.host_features"),
        )
    if set(build_recipes) != {"qjs-rust", "quickjs-ng"}:
        raise ManifestError(
            "manifest.build_recipes: version 4 requires exactly qjs-rust and quickjs-ng"
        )

    cases_data = data["cases"]
    if not isinstance(cases_data, list) or not cases_data:
        raise ManifestError("manifest.cases: expected a non-empty array")
    case_fields = {
        "id", "family", "critical", "workload", "workload_sha256",
        "operations_per_iteration", "checksum", "measurement",
    }
    cases: list[Case] = []
    seen: set[str] = set()
    for index, item in enumerate(cases_data):
        where = f"manifest.cases[{index}]"
        if not isinstance(item, dict):
            raise ManifestError(f"{where}: expected an object")
        _keys(item, case_fields, where)
        case_id = _string(item["id"], f"{where}.id")
        if case_id in seen:
            raise ManifestError(f"{where}.id: duplicate {case_id!r}")
        seen.add(case_id)
        if not isinstance(item["critical"], bool):
            raise ManifestError(f"{where}.critical: expected boolean")
        workload = _resolve_workload(root, item["workload"], f"{where}.workload")
        expected_hash = _string(item["workload_sha256"], f"{where}.workload_sha256")
        if len(expected_hash) != 64 or any(character not in "0123456789abcdef" for character in expected_hash):
            raise ManifestError(f"{where}.workload_sha256: expected lowercase SHA-256")
        actual_hash = sha256_file(workload)
        if actual_hash != expected_hash:
            raise ManifestError(
                f"{where}.workload_sha256: mismatch for {workload}: expected {expected_hash}, got {actual_hash}"
            )

        checksum = item["checksum"]
        if not isinstance(checksum, dict):
            raise ManifestError(f"{where}.checksum: expected an object")
        _keys(checksum, {"model", "factor"}, f"{where}.checksum")
        checksum_model = _string(checksum["model"], f"{where}.checksum.model")
        if checksum_model not in {
            "linear", "triangular", "zero_based_triangular_plus_linear",
        }:
            raise ManifestError(f"{where}.checksum.model: unsupported model {checksum_model!r}")
        checksum_factor = _integer(checksum["factor"], f"{where}.checksum.factor")
        if checksum_model == "linear" and checksum_factor < 1:
            raise ManifestError(f"{where}.checksum.factor: linear checksums require factor >= 1")
        if checksum_model == "triangular" and checksum_factor != 1:
            raise ManifestError(f"{where}.checksum.factor: triangular checksums require factor 1")
        if checksum_model == "zero_based_triangular_plus_linear" and checksum_factor < 1:
            raise ManifestError(
                f"{where}.checksum.factor: zero-based triangular checksums require factor >= 1"
            )

        measurement = item["measurement"]
        if not isinstance(measurement, dict):
            raise ManifestError(f"{where}.measurement: expected an object")
        measurement_fields = {
            "initial_iterations", "max_iterations", "min_window_ms",
            "startup_max_fraction", "calibration_safety_factor", "warmup_runs",
            "timeout_seconds",
        }
        _keys(measurement, measurement_fields, f"{where}.measurement")
        initial = _integer(measurement["initial_iterations"], f"{where}.measurement.initial_iterations", 1)
        maximum = _integer(measurement["max_iterations"], f"{where}.measurement.max_iterations", 1)
        if maximum < initial:
            raise ManifestError(f"{where}.measurement.max_iterations: must be >= initial_iterations")
        if maximum < 2:
            raise ManifestError(
                f"{where}.measurement.max_iterations: must be >= 2 for exact N/2N diagnostics"
            )
        startup_fraction = _bounded_fraction(
            measurement["startup_max_fraction"],
            f"{where}.measurement.startup_max_fraction",
            minimum=Decimal("0"),
            maximum=Decimal("0.1"),
            minimum_inclusive=False,
        )
        calibration_safety_factor = _bounded_fraction(
            measurement["calibration_safety_factor"],
            f"{where}.measurement.calibration_safety_factor",
            minimum=Decimal("1"),
            maximum=Decimal("4"),
            minimum_inclusive=True,
        )
        cases.append(Case(
            id=case_id,
            family=_string(item["family"], f"{where}.family"),
            critical=item["critical"],
            workload=workload,
            workload_sha256=expected_hash,
            operations_per_iteration=_integer(
                item["operations_per_iteration"], f"{where}.operations_per_iteration", 1
            ),
            checksum_model=checksum_model,
            checksum_factor=checksum_factor,
            initial_iterations=initial,
            max_iterations=maximum,
            min_window_ms=_integer(measurement["min_window_ms"], f"{where}.measurement.min_window_ms", 1),
            startup_max_fraction=startup_fraction,
            calibration_safety_factor=calibration_safety_factor,
            warmup_runs=_integer(measurement["warmup_runs"], f"{where}.measurement.warmup_runs"),
            timeout_seconds=_integer(measurement["timeout_seconds"], f"{where}.measurement.timeout_seconds", 1),
        ))

    return Manifest(
        path=path,
        sha256=hashlib.sha256(raw).hexdigest(),
        schema_version=4,
        series_id=_string(series["id"], "manifest.series.id"),
        suite_id=_string(series["suite_id"], "manifest.series.suite_id"),
        lane_id=lane_id,
        protocol_id=_string(protocol["id"], "manifest.protocol.id"),
        protocol_file_ids=tuple(protocol_paths),
        protocol_files=protocol_files,
        protocol_sha256=expected_protocol_hash,
        reference_identity=_string(reference["identity"], "manifest.reference_engine.identity"),
        reference_repo=_string(reference["source_repo"], "manifest.reference_engine.source_repo"),
        reference_revision=reference_revision,
        profile=profile,
        build_recipes=build_recipes,
        cases=tuple(cases),
    )
