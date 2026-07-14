"""Strict build-receipt validation bound to one engine binary."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .schema import BuildRecipe


class ReceiptError(ValueError):
    """A build receipt is malformed or does not describe its binary."""


RECEIPT_LIMIT = 64 * 1024


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ReceiptError(f"build receipt contains duplicate key {key!r}")
        result[key] = value
    return result


def _reject_constant(value: str) -> None:
    raise ReceiptError(f"build receipt contains non-standard numeric constant {value}")


def canonical_receipt_sha256(content: dict[str, Any]) -> str:
    """Digest semantic receipt content, independent of source JSON formatting."""
    try:
        encoded = json.dumps(
            content,
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=False,
            allow_nan=False,
        ).encode("utf-8")
    except (TypeError, ValueError) as error:
        raise ReceiptError(f"build receipt cannot be canonicalized: {error}") from error
    return hashlib.sha256(encoded).hexdigest()


def _keys(value: dict[str, Any], expected: set[str], where: str) -> None:
    if set(value) != expected:
        raise ReceiptError(
            f"{where}: expected fields {sorted(expected)}, got {sorted(value)}"
        )


def _string(value: Any, where: str) -> str:
    if (
        not isinstance(value, str)
        or not value
        or value.strip() != value
        or len(value.encode("utf-8")) > 4096
    ):
        raise ReceiptError(f"{where}: expected a non-empty trimmed string")
    return value


def _strings(value: Any, where: str) -> tuple[str, ...]:
    if not isinstance(value, list):
        raise ReceiptError(f"{where}: expected an array")
    if len(value) > 128:
        raise ReceiptError(f"{where}: at most 128 values are allowed")
    result = tuple(_string(item, f"{where}[]") for item in value)
    if len(set(result)) != len(result):
        raise ReceiptError(f"{where}: duplicate values are not allowed")
    return result


@dataclass(frozen=True)
class BuildReceipt:
    path: Path
    sha256: str
    content: dict[str, Any]
    engine_identity: str
    source_repo: str
    source_revision: str
    source_dirty: bool
    profile_id: str
    binary_sha256: str


def load_receipt(
    path: Path,
    *,
    expected_binary_sha256: str,
    expected_engine_identity: str,
    expected_profile_id: str,
    expected_recipe: BuildRecipe,
    pinned_reference: tuple[str, str, str] | None = None,
) -> BuildReceipt:
    path = path.expanduser().resolve()
    try:
        if path.stat().st_size > RECEIPT_LIMIT:
            raise ReceiptError(f"build receipt exceeds {RECEIPT_LIMIT} bytes")
        raw = path.read_bytes()
        data = json.loads(
            raw, object_pairs_hook=_unique_object, parse_constant=_reject_constant
        )
    except (OSError, json.JSONDecodeError) as error:
        raise ReceiptError(f"cannot read build receipt {path}: {error}") from error
    if not isinstance(data, dict):
        raise ReceiptError("build receipt: expected an object")
    _keys(
        data,
        {"schema_version", "engine_identity", "source", "profile_id", "build", "binary_sha256"},
        "build receipt",
    )
    if isinstance(data["schema_version"], bool) or data["schema_version"] != 1:
        raise ReceiptError("build receipt.schema_version: only integer version 1 is supported")
    identity = _string(data["engine_identity"], "build receipt.engine_identity")
    if identity != expected_engine_identity:
        raise ReceiptError(
            f"build receipt.engine_identity: expected {expected_engine_identity!r}, got {identity!r}"
        )
    if expected_recipe.engine_identity != expected_engine_identity:
        raise ReceiptError("build receipt validation received the wrong identity recipe")
    profile_id = _string(data["profile_id"], "build receipt.profile_id")
    if profile_id != expected_profile_id:
        raise ReceiptError(
            f"build receipt.profile_id: expected {expected_profile_id!r}, got {profile_id!r}"
        )
    binary_sha256 = _string(data["binary_sha256"], "build receipt.binary_sha256")
    if binary_sha256 != expected_binary_sha256:
        raise ReceiptError("build receipt.binary_sha256 does not match the executable")

    source = data["source"]
    if not isinstance(source, dict):
        raise ReceiptError("build receipt.source: expected an object")
    _keys(source, {"repo", "revision", "dirty"}, "build receipt.source")
    source_repo = _string(source["repo"], "build receipt.source.repo")
    source_revision = _string(source["revision"], "build receipt.source.revision")
    if len(source_revision) != 40 or any(
        character not in "0123456789abcdef" for character in source_revision
    ):
        raise ReceiptError("build receipt.source.revision: expected a full lowercase git SHA")
    if not isinstance(source["dirty"], bool):
        raise ReceiptError("build receipt.source.dirty: expected boolean")

    build = data["build"]
    if not isinstance(build, dict):
        raise ReceiptError("build receipt.build: expected an object")
    _keys(
        build,
        {
            "build_mode", "toolchain", "target", "features", "flags",
            "lto", "strip", "allocator", "host_features",
        },
        "build receipt.build",
    )
    for field in (
        "build_mode", "toolchain", "target", "lto", "strip", "allocator", "host_features"
    ):
        _string(build[field], f"build receipt.build.{field}")
    features = _strings(build["features"], "build receipt.build.features")
    flags = _strings(build["flags"], "build receipt.build.flags")
    expected_build = {
        "build_mode": expected_recipe.build_mode,
        "toolchain": expected_recipe.toolchain,
        "target": expected_recipe.target,
        "features": expected_recipe.features,
        "flags": expected_recipe.flags,
        "lto": expected_recipe.lto,
        "strip": expected_recipe.strip,
        "allocator": expected_recipe.allocator,
        "host_features": expected_recipe.host_features,
    }
    for field, expected in expected_build.items():
        actual = {"features": features, "flags": flags}.get(field, build[field])
        if actual != expected:
            raise ReceiptError(
                f"build receipt.build.{field}: expected recipe value {expected!r}, got {actual!r}"
            )

    if pinned_reference is not None:
        pinned_identity, pinned_repo, pinned_revision = pinned_reference
        if (identity, source_repo, source_revision) != pinned_reference:
            raise ReceiptError(
                "reference receipt does not match manifest pin "
                f"{pinned_identity}@{pinned_revision} from {pinned_repo}"
            )
    return BuildReceipt(
        path=path,
        sha256=canonical_receipt_sha256(data),
        content=data,
        engine_identity=identity,
        source_repo=source_repo,
        source_revision=source_revision,
        source_dirty=source["dirty"],
        profile_id=profile_id,
        binary_sha256=binary_sha256,
    )
