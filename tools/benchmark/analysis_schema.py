"""Strict loader for the independently versioned benchmark analysis contract."""

from __future__ import annotations

import hashlib
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .schema import Manifest, sha256_file


class AnalysisManifestError(ValueError):
    """The analysis manifest is malformed or incompatible."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise AnalysisManifestError(f"analysis JSON contains duplicate key {key!r}")
        result[key] = value
    return result


def _reject_constant(value: str) -> None:
    raise AnalysisManifestError(f"analysis JSON contains non-standard numeric constant {value}")


def _keys(value: dict[str, Any], expected: set[str], where: str) -> None:
    if set(value) != expected:
        raise AnalysisManifestError(
            f"{where}: missing {sorted(expected - set(value))}, unknown {sorted(set(value) - expected)}"
        )


def _string(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value or value.strip() != value:
        raise AnalysisManifestError(f"{where}: expected non-empty trimmed string")
    return value


def _integer(value: Any, where: str, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        raise AnalysisManifestError(f"{where}: expected integer >= {minimum}")
    return value


def _number(value: Any, where: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise AnalysisManifestError(f"{where}: expected number")
    result = float(value)
    if not math.isfinite(result):
        raise AnalysisManifestError(f"{where}: expected finite number")
    return result


def _resolve(root: Path, value: Any, where: str) -> tuple[str, Path]:
    identifier = _string(value, where)
    relative = Path(identifier)
    if relative.is_absolute() or ".." in relative.parts:
        raise AnalysisManifestError(f"{where}: path must stay inside repository")
    path = (root / relative).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError as error:
        raise AnalysisManifestError(f"{where}: path escapes repository") from error
    if not path.is_file():
        raise AnalysisManifestError(f"{where}: file does not exist: {identifier}")
    return identifier, path


@dataclass(frozen=True)
class HealthPolicy:
    initial_blocks: int
    extension_blocks: int
    max_blocks: int
    max_relative_half_width: float
    max_invalid_block_fraction: float
    block_invalidation: str
    outlier_policy: str
    retry_policy: str


@dataclass(frozen=True)
class AnalysisManifest:
    path: Path
    sha256: str
    schema_version: int
    id: str
    compatible_measurement_schema: int
    compatible_measurement_protocol: str
    protocol_id: str
    protocol_file_ids: tuple[str, ...]
    protocol_files: tuple[Path, ...]
    protocol_sha256: str
    bootstrap_samples: int
    bootstrap_seed: int
    confidence: float
    linearity_lower: float
    linearity_upper: float
    health: HealthPolicy

    def assert_compatible(self, measurement: Manifest) -> None:
        if (
            measurement.schema_version != self.compatible_measurement_schema
            or measurement.protocol_id != self.compatible_measurement_protocol
        ):
            raise AnalysisManifestError(
                "analysis manifest is incompatible with measurement schema/protocol"
            )


def load_analysis_manifest(path: Path, measurement: Manifest) -> AnalysisManifest:
    path = path.resolve()
    try:
        raw = path.read_bytes()
        data = json.loads(
            raw, object_pairs_hook=_unique_object, parse_constant=_reject_constant
        )
    except (OSError, json.JSONDecodeError) as error:
        raise AnalysisManifestError(f"cannot read analysis manifest {path}: {error}") from error
    if not isinstance(data, dict):
        raise AnalysisManifestError("analysis: expected object")
    _keys(
        data,
        {
            "schema_version", "id", "compatible_measurement", "protocol",
            "bootstrap", "linearity", "health",
        },
        "analysis",
    )
    if isinstance(data["schema_version"], bool) or data["schema_version"] != 2:
        raise AnalysisManifestError("analysis.schema_version: only integer version 2 is supported")
    compatible = data["compatible_measurement"]
    if not isinstance(compatible, dict):
        raise AnalysisManifestError("analysis.compatible_measurement: expected object")
    _keys(compatible, {"schema_version", "protocol_id"}, "analysis.compatible_measurement")
    compatible_schema = _integer(
        compatible["schema_version"], "analysis.compatible_measurement.schema_version", 1
    )

    root = path.parent.parent if path.parent.name == "benchmarks" else path.parent
    protocol = data["protocol"]
    if not isinstance(protocol, dict):
        raise AnalysisManifestError("analysis.protocol: expected object")
    _keys(protocol, {"id", "files", "aggregate_sha256"}, "analysis.protocol")
    if not isinstance(protocol["files"], list) or not protocol["files"]:
        raise AnalysisManifestError("analysis.protocol.files: expected non-empty array")
    resolved = [
        _resolve(root, value, f"analysis.protocol.files[{index}]")
        for index, value in enumerate(protocol["files"])
    ]
    file_ids = [identifier for identifier, _path in resolved]
    if file_ids != sorted(file_ids) or len(file_ids) != len(set(file_ids)):
        raise AnalysisManifestError("analysis.protocol.files: paths must be unique and sorted")
    digest = hashlib.sha256()
    for identifier, file_path in resolved:
        digest.update(identifier.encode("utf-8"))
        digest.update(b"\0")
        digest.update(bytes.fromhex(sha256_file(file_path)))
        digest.update(b"\n")
    expected_hash = _string(protocol["aggregate_sha256"], "analysis.protocol.aggregate_sha256")
    if digest.hexdigest() != expected_hash:
        raise AnalysisManifestError(
            f"analysis.protocol.aggregate_sha256: mismatch: expected {expected_hash}, got {digest.hexdigest()}"
        )

    bootstrap = data["bootstrap"]
    if not isinstance(bootstrap, dict):
        raise AnalysisManifestError("analysis.bootstrap: expected object")
    _keys(bootstrap, {"samples", "seed", "confidence"}, "analysis.bootstrap")
    confidence = _number(bootstrap["confidence"], "analysis.bootstrap.confidence")
    if not 0 < confidence < 1:
        raise AnalysisManifestError("analysis.bootstrap.confidence: expected 0 < value < 1")
    linearity = data["linearity"]
    if not isinstance(linearity, dict):
        raise AnalysisManifestError("analysis.linearity: expected object")
    _keys(linearity, {"normalized_per_op_lower", "normalized_per_op_upper"}, "analysis.linearity")
    lower = _number(linearity["normalized_per_op_lower"], "analysis.linearity.normalized_per_op_lower")
    upper = _number(linearity["normalized_per_op_upper"], "analysis.linearity.normalized_per_op_upper")
    if not 0 < lower <= 1 <= upper or lower == upper:
        raise AnalysisManifestError("analysis.linearity: invalid normalized per-op bounds")

    health = data["health"]
    if not isinstance(health, dict):
        raise AnalysisManifestError("analysis.health: expected object")
    health_fields = {
        "initial_blocks", "extension_blocks", "max_blocks",
        "max_relative_half_width", "max_invalid_block_fraction",
        "block_invalidation", "outlier_policy", "retry_policy",
    }
    _keys(health, health_fields, "analysis.health")
    initial_blocks = _integer(health["initial_blocks"], "analysis.health.initial_blocks", 1)
    extension_blocks = _integer(
        health["extension_blocks"], "analysis.health.extension_blocks", 1
    )
    max_blocks = _integer(health["max_blocks"], "analysis.health.max_blocks", 1)
    width = _number(
        health["max_relative_half_width"], "analysis.health.max_relative_half_width"
    )
    invalid_fraction = _number(
        health["max_invalid_block_fraction"],
        "analysis.health.max_invalid_block_fraction",
    )
    policy = HealthPolicy(
        initial_blocks=initial_blocks,
        extension_blocks=extension_blocks,
        max_blocks=max_blocks,
        max_relative_half_width=width,
        max_invalid_block_fraction=invalid_fraction,
        block_invalidation=_string(
            health["block_invalidation"], "analysis.health.block_invalidation"
        ),
        outlier_policy=_string(health["outlier_policy"], "analysis.health.outlier_policy"),
        retry_policy=_string(health["retry_policy"], "analysis.health.retry_policy"),
    )
    if (
        policy.initial_blocks != 30
        or policy.extension_blocks != 30
        or policy.max_blocks != 60
        or policy.initial_blocks + policy.extension_blocks != policy.max_blocks
        or policy.max_relative_half_width != 0.03
        or policy.max_invalid_block_fraction != 0.10
        or policy.block_invalidation != "portfolio-whole-block"
        or policy.outlier_policy != "retain"
        or policy.retry_policy != "never"
    ):
        raise AnalysisManifestError("analysis.health: unsupported version 2 policy")

    result = AnalysisManifest(
        path=path,
        sha256=hashlib.sha256(raw).hexdigest(),
        schema_version=2,
        id=_string(data["id"], "analysis.id"),
        compatible_measurement_schema=compatible_schema,
        compatible_measurement_protocol=_string(
            compatible["protocol_id"], "analysis.compatible_measurement.protocol_id"
        ),
        protocol_id=_string(protocol["id"], "analysis.protocol.id"),
        protocol_file_ids=tuple(file_ids),
        protocol_files=tuple(file_path for _identifier, file_path in resolved),
        protocol_sha256=expected_hash,
        bootstrap_samples=_integer(bootstrap["samples"], "analysis.bootstrap.samples", 1),
        bootstrap_seed=_integer(bootstrap["seed"], "analysis.bootstrap.seed"),
        confidence=confidence,
        linearity_lower=lower,
        linearity_upper=upper,
        health=policy,
    )
    result.assert_compatible(measurement)
    return result
