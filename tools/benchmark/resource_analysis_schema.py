"""Strict analysis policy for independently versioned resource evidence."""

from __future__ import annotations

import hashlib
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .resource_schema import ResourceManifest, sha256_file


class ResourceAnalysisError(ValueError):
    """The resource analysis manifest is invalid or incompatible."""


def _reject(value: str) -> None:
    raise ResourceAnalysisError(f"resource analysis contains non-standard constant {value}")


def _unique(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ResourceAnalysisError(f"resource analysis contains duplicate key {key!r}")
        result[key] = value
    return result


def _keys(value: Any, expected: set[str], where: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        actual = set(value) if isinstance(value, dict) else set()
        raise ResourceAnalysisError(
            f"{where}: missing {sorted(expected - actual)}, unknown {sorted(actual - expected)}"
        )
    return value


def _string(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value or value.strip() != value:
        raise ResourceAnalysisError(f"{where}: expected non-empty trimmed string")
    return value


def _integer(value: Any, where: str, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        raise ResourceAnalysisError(f"{where}: expected integer >= {minimum}")
    return value


def _number(value: Any, where: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ResourceAnalysisError(f"{where}: expected number")
    result = float(value)
    if not math.isfinite(result):
        raise ResourceAnalysisError(f"{where}: expected finite number")
    return result


def _resolve(root: Path, identifier: str) -> Path:
    relative = Path(identifier)
    if relative.is_absolute() or ".." in relative.parts:
        raise ResourceAnalysisError("resource analysis.protocol.files: path escapes repository")
    path = (root / relative).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError as error:
        raise ResourceAnalysisError("resource analysis.protocol.files: path escapes repository") from error
    if not path.is_file():
        raise ResourceAnalysisError(f"resource analysis.protocol.files: missing {identifier}")
    return path


@dataclass(frozen=True)
class ResourceHealthPolicy:
    initial_blocks: int
    extension_blocks: int
    max_blocks: int
    max_relative_half_width: float
    max_invalid_block_fraction: float
    block_invalidation: str
    outlier_policy: str
    retry_policy: str


@dataclass(frozen=True)
class ResourceAnalysisManifest:
    path: Path
    sha256: str
    schema_version: int
    id: str
    compatible_schema: int
    compatible_protocol: str
    protocol_id: str
    protocol_file_ids: tuple[str, ...]
    protocol_files: tuple[Path, ...]
    protocol_sha256: str
    bootstrap_samples: int
    bootstrap_seed: int
    confidence: float
    health: ResourceHealthPolicy

    def assert_compatible(self, measurement: ResourceManifest) -> None:
        if (
            measurement.schema_version != self.compatible_schema
            or measurement.protocol_id != self.compatible_protocol
        ):
            raise ResourceAnalysisError("resource analysis is incompatible with measurement")


def load_resource_analysis(
    path: Path, measurement: ResourceManifest
) -> ResourceAnalysisManifest:
    path = path.expanduser().resolve()
    try:
        raw = path.read_bytes()
        data = json.loads(raw, object_pairs_hook=_unique, parse_constant=_reject)
    except (OSError, json.JSONDecodeError) as error:
        raise ResourceAnalysisError(f"cannot read resource analysis {path}: {error}") from error
    _keys(
        data,
        {"schema_version", "id", "compatible_measurement", "protocol", "bootstrap", "health"},
        "resource analysis",
    )
    if isinstance(data["schema_version"], bool) or data["schema_version"] != 1:
        raise ResourceAnalysisError("resource analysis.schema_version: only integer 1 is supported")
    compatible = _keys(
        data["compatible_measurement"], {"schema_version", "protocol_id"},
        "resource analysis.compatible_measurement",
    )
    compatible_schema = _integer(
        compatible["schema_version"], "resource analysis.compatible_measurement.schema_version", 1
    )
    protocol = _keys(
        data["protocol"], {"id", "files", "aggregate_sha256"}, "resource analysis.protocol"
    )
    if not isinstance(protocol["files"], list) or not protocol["files"]:
        raise ResourceAnalysisError("resource analysis.protocol.files: expected non-empty array")
    file_ids = [_string(value, "resource analysis.protocol.files[]") for value in protocol["files"]]
    if file_ids != sorted(file_ids) or len(file_ids) != len(set(file_ids)):
        raise ResourceAnalysisError("resource analysis.protocol.files: must be unique and sorted")
    root = path.parent.parent if path.parent.name == "benchmarks" else path.parent
    files = tuple(_resolve(root, identifier) for identifier in file_ids)
    digest = hashlib.sha256()
    for identifier, file_path in zip(file_ids, files):
        digest.update(identifier.encode("utf-8"))
        digest.update(b"\0")
        digest.update(bytes.fromhex(sha256_file(file_path)))
        digest.update(b"\n")
    expected_digest = _string(
        protocol["aggregate_sha256"], "resource analysis.protocol.aggregate_sha256"
    )
    if digest.hexdigest() != expected_digest:
        raise ResourceAnalysisError(
            "resource analysis.protocol.aggregate_sha256: mismatch: "
            f"expected {expected_digest}, got {digest.hexdigest()}"
        )
    bootstrap = _keys(
        data["bootstrap"], {"samples", "seed", "confidence"}, "resource analysis.bootstrap"
    )
    confidence = _number(bootstrap["confidence"], "resource analysis.bootstrap.confidence")
    if not 0 < confidence < 1:
        raise ResourceAnalysisError("resource analysis.bootstrap.confidence: outside (0,1)")
    health_data = _keys(
        data["health"],
        {"initial_blocks", "extension_blocks", "max_blocks", "max_relative_half_width",
         "max_invalid_block_fraction", "block_invalidation", "outlier_policy", "retry_policy"},
        "resource analysis.health",
    )
    health = ResourceHealthPolicy(
        initial_blocks=_integer(health_data["initial_blocks"], "resource analysis.health.initial_blocks", 1),
        extension_blocks=_integer(
            health_data["extension_blocks"], "resource analysis.health.extension_blocks", 1
        ),
        max_blocks=_integer(health_data["max_blocks"], "resource analysis.health.max_blocks", 1),
        max_relative_half_width=_number(
            health_data["max_relative_half_width"],
            "resource analysis.health.max_relative_half_width",
        ),
        max_invalid_block_fraction=_number(
            health_data["max_invalid_block_fraction"],
            "resource analysis.health.max_invalid_block_fraction",
        ),
        block_invalidation=_string(
            health_data["block_invalidation"], "resource analysis.health.block_invalidation"
        ),
        outlier_policy=_string(health_data["outlier_policy"], "resource analysis.health.outlier_policy"),
        retry_policy=_string(health_data["retry_policy"], "resource analysis.health.retry_policy"),
    )
    if (
        (health.initial_blocks, health.extension_blocks, health.max_blocks) != (30, 30, 60)
        or health.max_relative_half_width != 0.03
        or health.max_invalid_block_fraction != 0.10
        or (health.block_invalidation, health.outlier_policy, health.retry_policy)
        != ("lane-whole-block", "retain", "never")
    ):
        raise ResourceAnalysisError("resource analysis.health: frozen policy mismatch")
    result = ResourceAnalysisManifest(
        path=path, sha256=hashlib.sha256(raw).hexdigest(), schema_version=1,
        id=_string(data["id"], "resource analysis.id"),
        compatible_schema=compatible_schema,
        compatible_protocol=_string(
            compatible["protocol_id"], "resource analysis.compatible_measurement.protocol_id"
        ),
        protocol_id=_string(protocol["id"], "resource analysis.protocol.id"),
        protocol_file_ids=tuple(file_ids), protocol_files=files,
        protocol_sha256=expected_digest,
        bootstrap_samples=_integer(
            bootstrap["samples"], "resource analysis.bootstrap.samples", 1
        ),
        bootstrap_seed=_integer(bootstrap["seed"], "resource analysis.bootstrap.seed"),
        confidence=confidence, health=health,
    )
    result.assert_compatible(measurement)
    return result
