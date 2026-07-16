"""Fail-closed v2 policy for hosted informational evidence and future gates."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any, NoReturn
from urllib.parse import urlparse

from .analysis_schema import load_analysis_manifest
from .external_corpora import load_registry, registry_summary
from .hosted_preview import (
    BASE_MODE,
    HOSTED_BASE_REF,
    HOSTED_PUSH_REF,
    PR_INTEGRITY_SCOPE,
    PUSH_INTEGRITY_SCOPE,
    PUSH_MODE,
)
from .resource_analysis_schema import load_resource_analysis
from .resource_schema import load_resource_manifest
from .schema import load_manifest


class PerformancePolicyError(ValueError):
    """The performance policy is malformed or cannot authorize an operation."""


PROTOCOL_KEYS = (
    "resource_analysis",
    "resource_measurement",
    "throughput_analysis",
    "throughput_measurement",
)
PROTOCOL_SHAPES = {
    "throughput_measurement": (
        "benchmarks/manifest.json", "quickjs-measurement-protocol-v6"
    ),
    "throughput_analysis": (
        "benchmarks/analysis.json", "quickjs-analysis-protocol-v3"
    ),
    "resource_measurement": (
        "benchmarks/resources.json", "quickjs-resource-measurement-protocol-v1"
    ),
    "resource_analysis": (
        "benchmarks/resource-analysis.json", "quickjs-resource-analysis-protocol-v1"
    ),
}
EXPECTED_WORKFLOW_SHA256 = "609a986096d1cb5288e94288f287837cafe96e403dd83880d34d88ef37a07da7"
PREVIEW_ORCHESTRATOR = "scripts/performance-preview.sh"
PREVIEW_ROLES = ("candidate", "base", "quickjs-ng")
PREVIEW_IMPLEMENTATION_FILES = (
    ".github/actions/setup-rust/action.yml",
    ".github/workflows/performance-smoke.yml",
    "benchmarks/external-corpora.json",
    "scripts/external-corpus-audit.sh",
    "scripts/performance-policy-audit.sh",
    "scripts/performance-preview.sh",
    "tools/benchmark/build_cache.py",
    "tools/benchmark/build_cache_identity.py",
    "tools/benchmark/external_corpora.py",
    "tools/benchmark/hosted_preview.py",
    "tools/benchmark/performance_policy.py",
    "tools/benchmark/preview.py",
)
REFERENCE_ENGINE = (
    "quickjs-ng",
    "https://github.com/quickjs-ng/quickjs.git",
    "f7830186043e4488f2998759d60a514faf07cbc9",
)
AA_REQUIREMENTS = ("content_hashed", "randomized_order", "same_binary")
BASE_ARTIFACTS = ("noise_envelope", "qualified_fixed_hardware_fingerprint")
PR_ARTIFACTS = (
    "false_positive_budget",
    "noise_envelope",
    "qualified_fixed_hardware_fingerprint",
)
GATE_MINIMUMS = {"nightly": 20, "release": 20, "pr_sentinel": 30}
_SHA256 = re.compile(r"[0-9a-f]{64}\Z")


def _reject_constant(value: str) -> None:
    raise PerformancePolicyError(
        f"performance policy contains non-standard numeric constant {value}"
    )


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise PerformancePolicyError(f"performance policy contains duplicate key {key!r}")
        result[key] = value
    return result


def _object(value: Any, where: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise PerformancePolicyError(f"{where}: expected an object")
    return value


def _keys(value: dict[str, Any], expected: set[str], where: str) -> None:
    missing = expected - set(value)
    unknown = set(value) - expected
    if missing or unknown:
        details = []
        if missing:
            details.append(f"missing {sorted(missing)}")
        if unknown:
            details.append(f"unknown {sorted(unknown)}")
        raise PerformancePolicyError(f"{where}: {', '.join(details)}")


def _string(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value or value.strip() != value:
        raise PerformancePolicyError(f"{where}: expected a non-empty trimmed string")
    return value


def _boolean(value: Any, where: str) -> bool:
    if not isinstance(value, bool):
        raise PerformancePolicyError(f"{where}: expected a boolean")
    return value


def _integer(value: Any, where: str, minimum: int = 0) -> int:
    if type(value) is not int or value < minimum:
        raise PerformancePolicyError(f"{where}: expected integer >= {minimum}")
    return value


def _array(value: Any, where: str) -> list[Any]:
    if not isinstance(value, list):
        raise PerformancePolicyError(f"{where}: expected an array")
    return value


def _strings(value: Any, where: str) -> tuple[str, ...]:
    result = tuple(_string(item, f"{where}[]") for item in _array(value, where))
    if len(result) != len(set(result)):
        raise PerformancePolicyError(f"{where}: values must be unique")
    return result


def _sha256(value: Any, where: str) -> str:
    text = _string(value, where)
    if not _SHA256.fullmatch(text) or text == "0" * 64:
        raise PerformancePolicyError(f"{where}: expected non-zero lowercase SHA-256")
    return text


def _path(value: Any, where: str) -> str:
    text = _string(value, where)
    path = PurePosixPath(text)
    if (
        "\\" in text
        or path.is_absolute()
        or ".." in path.parts
        or any(part in {"", "."} for part in text.split("/"))
    ):
        raise PerformancePolicyError(f"{where}: expected a repository-relative path")
    return text


def _url(value: Any, where: str) -> str:
    text = _string(value, where)
    if any(
        character.isspace() or ord(character) < 0x20 or ord(character) == 0x7F
        for character in text
    ):
        raise PerformancePolicyError(
            f"{where}: URL cannot contain whitespace or control characters"
        )
    try:
        parsed = urlparse(text)
        hostname = parsed.hostname
        parsed.port
    except ValueError as error:
        raise PerformancePolicyError(f"{where}: malformed URL authority: {error}") from error
    if (
        parsed.scheme != "https"
        or not parsed.netloc
        or not hostname
        or parsed.username is not None
        or parsed.password is not None
        or parsed.fragment
    ):
        raise PerformancePolicyError(
            f"{where}: expected an HTTPS URL without credentials or fragment"
        )
    try:
        ipaddress.ip_address(hostname)
    except ValueError:
        try:
            ascii_hostname = hostname.encode("idna").decode("ascii")
        except UnicodeError as error:
            raise PerformancePolicyError(f"{where}: invalid hostname") from error
        labels = ascii_hostname.rstrip(".").split(".")
        if any(
            not label
            or len(label) > 63
            or re.fullmatch(
                r"[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?", label
            ) is None
            for label in labels
        ):
            raise PerformancePolicyError(f"{where}: invalid hostname")
    return text


@dataclass(frozen=True)
class ProtocolBinding:
    manifest_path: str
    protocol_id: str
    protocol_sha256: str


@dataclass(frozen=True)
class GatePolicy:
    id: str
    enabled: bool
    minimum_aa_reports: int
    required_artifacts: tuple[str, ...]


@dataclass(frozen=True)
class PerformancePolicy:
    path: Path
    sha256: str
    schema_version: int
    policy_id: str
    claim_eligible: bool
    protocols: dict[str, ProtocolBinding]
    external_registry_path: str
    external_registry_id: str
    hosted_tier: str
    hosted_orchestrator_path: str
    hosted_blocks: int
    hosted_retention_days: int
    hosted_implementation_sha256: str
    hosted_integrity_scope: str
    hosted_push_integrity_scope: str
    hosted_base_ref: str
    hosted_push_ref: str
    hosted_push_mode: str
    reference_identity: str
    reference_repo: str
    reference_revision: str
    workflow_path: str
    workflow_sha256: str
    fixed_hardware_configured: bool
    hardware_fingerprint: None
    gates: dict[str, GatePolicy]
    evidence_entries: tuple[Any, ...]


def _protocols(data: Any) -> dict[str, ProtocolBinding]:
    root = _object(data, "policy.protocols")
    _keys(root, set(PROTOCOL_KEYS), "policy.protocols")
    result = {}
    for key in PROTOCOL_KEYS:
        where = f"policy.protocols.{key}"
        item = _object(root[key], where)
        _keys(item, {"manifest_path", "protocol_id", "protocol_sha256"}, where)
        path = _path(item["manifest_path"], f"{where}.manifest_path")
        protocol_id = _string(item["protocol_id"], f"{where}.protocol_id")
        if (path, protocol_id) != PROTOCOL_SHAPES[key]:
            raise PerformancePolicyError(f"{where}: invalid frozen protocol identity")
        result[key] = ProtocolBinding(
            path, protocol_id, _sha256(item["protocol_sha256"], f"{where}.protocol_sha256")
        )
    return result


def _implementation_sha256(root: Path, files: tuple[str, ...]) -> str:
    digest = hashlib.sha256()
    for relative in files:
        path = root / relative
        try:
            content_hash = hashlib.sha256(path.read_bytes()).digest()
        except OSError as error:
            raise PerformancePolicyError(
                f"cannot hash hosted implementation file {relative}: {error}"
            ) from error
        digest.update(relative.encode())
        digest.update(b"\0")
        digest.update(content_hash)
        digest.update(b"\n")
    return digest.hexdigest()


def _protocol_hashes(data: Any, protocols: dict[str, ProtocolBinding], where: str) -> None:
    item = _object(data, where)
    _keys(item, set(PROTOCOL_KEYS), where)
    for key in PROTOCOL_KEYS:
        digest = _sha256(item[key], f"{where}.{key}")
        if digest != protocols[key].protocol_sha256:
            raise PerformancePolicyError(f"{where}.{key}: must match policy.protocols")


def _gates(data: Any, protocols: dict[str, ProtocolBinding]) -> dict[str, GatePolicy]:
    root = _object(data, "policy.gates")
    _keys(root, set(GATE_MINIMUMS), "policy.gates")
    result = {}
    for gate_id in ("nightly", "release", "pr_sentinel"):
        where = f"policy.gates.{gate_id}"
        item = _object(root[gate_id], where)
        _keys(item, {"enabled", "activation_prerequisites"}, where)
        enabled = _boolean(item["enabled"], f"{where}.enabled")
        if enabled:
            raise PerformancePolicyError(f"{where}.enabled: v2 is deny-only and requires false")
        prerequisites = _object(
            item["activation_prerequisites"], f"{where}.activation_prerequisites"
        )
        _keys(
            prerequisites,
            {
                "minimum_independent_aa_shadow_reports", "aa_shadow_requirements",
                "required_artifacts", "required_protocol_sha256",
            },
            f"{where}.activation_prerequisites",
        )
        minimum = _integer(
            prerequisites["minimum_independent_aa_shadow_reports"],
            f"{where}.activation_prerequisites.minimum_independent_aa_shadow_reports",
            1,
        )
        if minimum != GATE_MINIMUMS[gate_id]:
            raise PerformancePolicyError(f"{where}: invalid frozen A/A report minimum")
        aa_requirements = _strings(
            prerequisites["aa_shadow_requirements"],
            f"{where}.activation_prerequisites.aa_shadow_requirements",
        )
        if aa_requirements != AA_REQUIREMENTS:
            raise PerformancePolicyError(f"{where}: invalid frozen A/A requirements")
        artifacts = _strings(
            prerequisites["required_artifacts"],
            f"{where}.activation_prerequisites.required_artifacts",
        )
        expected_artifacts = PR_ARTIFACTS if gate_id == "pr_sentinel" else BASE_ARTIFACTS
        if artifacts != expected_artifacts:
            raise PerformancePolicyError(f"{where}: invalid frozen required artifacts")
        _protocol_hashes(
            prerequisites["required_protocol_sha256"],
            protocols,
            f"{where}.activation_prerequisites.required_protocol_sha256",
        )
        result[gate_id] = GatePolicy(gate_id, enabled, minimum, artifacts)
    return result


def load_policy(path: Path) -> PerformancePolicy:
    path = path.expanduser().resolve()
    try:
        raw = path.read_bytes()
        data = json.loads(
            raw, object_pairs_hook=_unique_object, parse_constant=_reject_constant
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise PerformancePolicyError(f"cannot read performance policy {path}: {error}") from error
    root = _object(data, "policy")
    _keys(
        root,
        {
            "schema_version", "policy_id", "claim_eligible", "protocols",
            "external_corpora", "reference_engine", "hosted_implementation",
            "hosted_pr", "fixed_hardware", "gates", "evidence_entries",
        },
        "policy",
    )
    if type(root["schema_version"]) is not int or root["schema_version"] != 2:
        raise PerformancePolicyError("policy.schema_version: only integer version 2 is supported")
    policy_id = _string(root["policy_id"], "policy.policy_id")
    if policy_id != "quickjs-performance-policy-v2":
        raise PerformancePolicyError(
            "policy.policy_id: version 2 requires quickjs-performance-policy-v2"
        )
    claim_eligible = _boolean(root["claim_eligible"], "policy.claim_eligible")
    if claim_eligible:
        raise PerformancePolicyError("policy.claim_eligible: v2 requires false")
    protocols = _protocols(root["protocols"])

    reference = _object(root["reference_engine"], "policy.reference_engine")
    _keys(reference, {"identity", "source_repo", "revision"}, "policy.reference_engine")
    reference_values = (
        _string(reference["identity"], "policy.reference_engine.identity"),
        _url(reference["source_repo"], "policy.reference_engine.source_repo"),
        _string(reference["revision"], "policy.reference_engine.revision"),
    )
    if reference_values != REFERENCE_ENGINE:
        raise PerformancePolicyError("policy.reference_engine: invalid frozen QuickJS-NG pin")

    implementation = _object(
        root["hosted_implementation"], "policy.hosted_implementation"
    )
    _keys(implementation, {"files", "aggregate_sha256"}, "policy.hosted_implementation")
    implementation_files = _strings(
        implementation["files"], "policy.hosted_implementation.files"
    )
    if implementation_files != PREVIEW_IMPLEMENTATION_FILES:
        raise PerformancePolicyError("policy.hosted_implementation.files: invalid frozen inventory")
    implementation_sha256 = _sha256(
        implementation["aggregate_sha256"],
        "policy.hosted_implementation.aggregate_sha256",
    )

    external = _object(root["external_corpora"], "policy.external_corpora")
    _keys(
        external,
        {"registry_path", "registry_id", "required_admitted_count", "required_claim_eligible"},
        "policy.external_corpora",
    )
    external_path = _path(external["registry_path"], "policy.external_corpora.registry_path")
    if external_path != "benchmarks/external-corpora.json":
        raise PerformancePolicyError("policy.external_corpora.registry_path: invalid frozen path")
    external_id = _string(external["registry_id"], "policy.external_corpora.registry_id")
    if external_id != "quickjs-external-corpora-v1":
        raise PerformancePolicyError("policy.external_corpora.registry_id: invalid frozen id")
    if _integer(
        external["required_admitted_count"],
        "policy.external_corpora.required_admitted_count",
    ) != 0:
        raise PerformancePolicyError("policy.external_corpora.required_admitted_count: v2 requires 0")
    if _boolean(
        external["required_claim_eligible"],
        "policy.external_corpora.required_claim_eligible",
    ):
        raise PerformancePolicyError("policy.external_corpora.required_claim_eligible: v2 requires false")

    hosted = _object(root["hosted_pr"], "policy.hosted_pr")
    _keys(
        hosted,
        {
            "tier", "evidence_class", "claim_eligible", "provider", "provider_url",
            "runner", "gate", "slowdown_threshold", "workflow_path",
            "workflow_sha256", "upload_timing_evidence", "orchestrator_path",
            "portfolio", "blocks", "roles", "artifact_retention_days", "harness",
        },
        "policy.hosted_pr",
    )
    hosted_shape = (
        _string(hosted["tier"], "policy.hosted_pr.tier"),
        _string(hosted["provider"], "policy.hosted_pr.provider"),
        _string(hosted["runner"], "policy.hosted_pr.runner"),
    )
    if hosted_shape != ("informational_preview", "github-hosted", "ubuntu-latest"):
        raise PerformancePolicyError("policy.hosted_pr: invalid frozen hosted tier")
    if _string(hosted["evidence_class"], "policy.hosted_pr.evidence_class") != "informational":
        raise PerformancePolicyError("policy.hosted_pr.evidence_class: must be informational")
    if _boolean(hosted["claim_eligible"], "policy.hosted_pr.claim_eligible"):
        raise PerformancePolicyError("policy.hosted_pr.claim_eligible: must remain false")
    provider_url = _url(hosted["provider_url"], "policy.hosted_pr.provider_url")
    if provider_url != (
        "https://docs.github.com/actions/using-github-hosted-runners/"
        "about-github-hosted-runners"
    ):
        raise PerformancePolicyError("policy.hosted_pr.provider_url: invalid frozen URL")
    workflow_path = _path(hosted["workflow_path"], "policy.hosted_pr.workflow_path")
    if workflow_path != ".github/workflows/performance-smoke.yml":
        raise PerformancePolicyError("policy.hosted_pr.workflow_path: invalid frozen path")
    workflow_sha256 = _sha256(
        hosted["workflow_sha256"], "policy.hosted_pr.workflow_sha256"
    )
    if workflow_sha256 != EXPECTED_WORKFLOW_SHA256:
        raise PerformancePolicyError(
            "policy.hosted_pr.workflow_sha256: must match the exact v2 workflow bytes"
        )
    if _boolean(hosted["gate"], "policy.hosted_pr.gate"):
        raise PerformancePolicyError("policy.hosted_pr.gate: v2 requires false")
    if hosted["slowdown_threshold"] is not None:
        raise PerformancePolicyError("policy.hosted_pr.slowdown_threshold: must remain null")
    if not _boolean(
        hosted["upload_timing_evidence"], "policy.hosted_pr.upload_timing_evidence"
    ):
        raise PerformancePolicyError("policy.hosted_pr.upload_timing_evidence: must be true")
    orchestrator = _path(hosted["orchestrator_path"], "policy.hosted_pr.orchestrator_path")
    if orchestrator != PREVIEW_ORCHESTRATOR:
        raise PerformancePolicyError("policy.hosted_pr.orchestrator_path: invalid frozen path")
    if _string(hosted["portfolio"], "policy.hosted_pr.portfolio") != "complete-broad-25-case":
        raise PerformancePolicyError("policy.hosted_pr.portfolio: invalid frozen portfolio")
    blocks = _integer(hosted["blocks"], "policy.hosted_pr.blocks", 1)
    if blocks != 3:
        raise PerformancePolicyError("policy.hosted_pr.blocks: hosted preview requires exactly 3")
    roles = _strings(hosted["roles"], "policy.hosted_pr.roles")
    if roles != PREVIEW_ROLES:
        raise PerformancePolicyError("policy.hosted_pr.roles: invalid frozen roles")
    retention = _integer(
        hosted["artifact_retention_days"], "policy.hosted_pr.artifact_retention_days", 1
    )
    if retention != 14:
        raise PerformancePolicyError("policy.hosted_pr.artifact_retention_days: must be 14")
    harness = _object(hosted["harness"], "policy.hosted_pr.harness")
    _keys(
        harness,
        {
            "pr_mode", "pr_event", "pr_integrity_scope", "candidate_role",
            "malicious_candidate_resistant", "forks_supported", "base_ref",
            "push_mode", "push_event", "push_ref", "push_integrity_scope",
            "push_comparison", "push_harness_owner", "fixed_hardware_claim_scope",
        },
        "policy.hosted_pr.harness",
    )
    harness_shape = (
        _string(harness["pr_mode"], "policy.hosted_pr.harness.pr_mode"),
        _string(harness["pr_event"], "policy.hosted_pr.harness.pr_event"),
        _string(harness["pr_integrity_scope"], "policy.hosted_pr.harness.pr_integrity_scope"),
        _string(harness["candidate_role"], "policy.hosted_pr.harness.candidate_role"),
        _boolean(
            harness["malicious_candidate_resistant"],
            "policy.hosted_pr.harness.malicious_candidate_resistant",
        ),
        _boolean(harness["forks_supported"], "policy.hosted_pr.harness.forks_supported"),
        _string(harness["base_ref"], "policy.hosted_pr.harness.base_ref"),
        _string(harness["push_mode"], "policy.hosted_pr.harness.push_mode"),
        _string(harness["push_event"], "policy.hosted_pr.harness.push_event"),
        _string(harness["push_ref"], "policy.hosted_pr.harness.push_ref"),
        _string(
            harness["push_integrity_scope"],
            "policy.hosted_pr.harness.push_integrity_scope",
        ),
        _string(harness["push_comparison"], "policy.hosted_pr.harness.push_comparison"),
        _string(harness["push_harness_owner"], "policy.hosted_pr.harness.push_harness_owner"),
        _string(
            harness["fixed_hardware_claim_scope"],
            "policy.hosted_pr.harness.fixed_hardware_claim_scope",
        ),
    )
    if harness_shape != (
        BASE_MODE, "pull_request_target", PR_INTEGRITY_SCOPE,
        "benchmark_subject_with_shared_runner_permissions", False, False,
        HOSTED_BASE_REF, PUSH_MODE, "push", HOSTED_PUSH_REF,
        PUSH_INTEGRITY_SCOPE, "event_after_candidate_vs_event_before_base",
        "event_after_candidate",
        "trusted_merged_commits_only",
    ):
        raise PerformancePolicyError("policy.hosted_pr.harness: invalid frozen integrity boundary")

    hardware = _object(root["fixed_hardware"], "policy.fixed_hardware")
    _keys(hardware, {"configured", "hardware_fingerprint"}, "policy.fixed_hardware")
    configured = _boolean(hardware["configured"], "policy.fixed_hardware.configured")
    if configured or hardware["hardware_fingerprint"] is not None:
        raise PerformancePolicyError(
            "policy.fixed_hardware: v2 requires configured=false and null fingerprint"
        )
    gates = _gates(root["gates"], protocols)
    evidence = tuple(_array(root["evidence_entries"], "policy.evidence_entries"))
    if evidence:
        raise PerformancePolicyError("policy.evidence_entries: v2 requires an empty array")
    return PerformancePolicy(
        path=path,
        sha256=hashlib.sha256(raw).hexdigest(),
        schema_version=2,
        policy_id=policy_id,
        claim_eligible=claim_eligible,
        protocols=protocols,
        external_registry_path=external_path,
        external_registry_id=external_id,
        hosted_tier="informational_preview",
        hosted_orchestrator_path=orchestrator,
        hosted_blocks=blocks,
        hosted_retention_days=retention,
        hosted_implementation_sha256=implementation_sha256,
        hosted_integrity_scope=PR_INTEGRITY_SCOPE,
        hosted_push_integrity_scope=PUSH_INTEGRITY_SCOPE,
        hosted_base_ref=HOSTED_BASE_REF,
        hosted_push_ref=HOSTED_PUSH_REF,
        hosted_push_mode=PUSH_MODE,
        reference_identity=reference_values[0],
        reference_repo=reference_values[1],
        reference_revision=reference_values[2],
        workflow_path=workflow_path,
        workflow_sha256=workflow_sha256,
        fixed_hardware_configured=configured,
        hardware_fingerprint=None,
        gates=gates,
        evidence_entries=evidence,
    )


def validate_workflow_bytes(policy: PerformancePolicy, raw: bytes) -> None:
    digest = hashlib.sha256(raw).hexdigest()
    if digest != EXPECTED_WORKFLOW_SHA256:
        raise PerformancePolicyError("policy.hosted_pr.workflow: bytes differ from exact v2 workflow")
    if digest != policy.workflow_sha256:
        raise PerformancePolicyError(
            "policy.hosted_pr.workflow_sha256: current workflow hash mismatch"
        )


def cross_check_repository(policy: PerformancePolicy, root: Path) -> None:
    root = root.resolve()
    try:
        measurement = load_manifest(root / "benchmarks/manifest.json")
        analysis = load_analysis_manifest(root / "benchmarks/analysis.json", measurement)
        resources = load_resource_manifest(root / "benchmarks/resources.json")
        resource_analysis = load_resource_analysis(
            root / "benchmarks/resource-analysis.json", resources
        )
        actual = {
            "throughput_measurement": (
                measurement.protocol_id, measurement.protocol_sha256
            ),
            "throughput_analysis": (analysis.protocol_id, analysis.protocol_sha256),
            "resource_measurement": (resources.protocol_id, resources.protocol_sha256),
            "resource_analysis": (
                resource_analysis.protocol_id, resource_analysis.protocol_sha256
            ),
        }
        for key in PROTOCOL_KEYS:
            expected = policy.protocols[key]
            if actual[key] != (expected.protocol_id, expected.protocol_sha256):
                raise PerformancePolicyError(
                    f"policy.protocols.{key}: does not match current repository protocol"
                )
        manifest_reference = (
            measurement.reference_identity,
            measurement.reference_repo,
            measurement.reference_revision,
        )
        policy_reference = (
            policy.reference_identity, policy.reference_repo, policy.reference_revision,
        )
        if manifest_reference != policy_reference:
            raise PerformancePolicyError(
                "policy.reference_engine: does not match measurement manifest"
            )
        gitlink = subprocess.run(
            ["git", "-C", str(root), "ls-tree", "HEAD", "third_party/quickjs-ng"],
            capture_output=True, text=True, timeout=10, check=False,
        )
        fields = gitlink.stdout.split()
        if gitlink.returncode != 0 or len(fields) < 3 or fields[2] != policy.reference_revision:
            raise PerformancePolicyError(
                "policy.reference_engine: does not match QuickJS-NG gitlink"
            )
        actual_implementation = _implementation_sha256(root, PREVIEW_IMPLEMENTATION_FILES)
        if actual_implementation != policy.hosted_implementation_sha256:
            raise PerformancePolicyError(
                "policy.hosted_implementation.aggregate_sha256: implementation drift"
            )
        registry = load_registry(root / policy.external_registry_path)
        external_summary = registry_summary(registry)
        if (
            registry.registry_id != policy.external_registry_id
            or external_summary["admitted_count"] != 0
            or registry.claim_eligible
        ):
            raise PerformancePolicyError(
                "policy.external_corpora: current registry must remain v1, zero-admitted, and non-claim"
            )
        validate_workflow_bytes(policy, (root / policy.workflow_path).read_bytes())
    except PerformancePolicyError:
        raise
    except (OSError, ValueError) as error:
        raise PerformancePolicyError(f"repository cross-check failed: {error}") from error


def require_gate(policy: PerformancePolicy, gate_id: str) -> NoReturn:
    gate = policy.gates.get(gate_id)
    if gate is None:
        raise PerformancePolicyError(f"unknown performance gate {gate_id!r}")
    raise PerformancePolicyError(
        f"performance gate {gate_id!r} is disabled by deny-only policy v2"
    )


def policy_summary(policy: PerformancePolicy) -> dict[str, Any]:
    return {
        "claim_eligible": False,
        "evidence_entry_count": 0,
        "external_admitted_count": 0,
        "fixed_hardware_configured": False,
        "gates": {gate_id: False for gate_id in ("nightly", "release", "pr_sentinel")},
        "hosted_pr_tier": policy.hosted_tier,
        "hosted_integrity_scope": policy.hosted_integrity_scope,
        "policy_id": policy.policy_id,
        "policy_sha256": policy.sha256,
        "schema_version": policy.schema_version,
    }


def _write_atomic(output: Path, encoded: bytes) -> None:
    output = output.expanduser().resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        raise PerformancePolicyError(f"refusing to overwrite existing output: {output}")
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb", dir=output.parent, prefix=f".{output.name}.", delete=False
        ) as handle:
            temporary = Path(handle.name)
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary, output)
        except FileExistsError as error:
            raise PerformancePolicyError(
                f"refusing to overwrite existing output: {output}"
            ) from error
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


class _PolicyParser(argparse.ArgumentParser):
    def error(self, message: str) -> None:
        raise PerformancePolicyError(f"arguments: {message}")


def _parser() -> argparse.ArgumentParser:
    parser = _PolicyParser(description="audit deny-only performance gate policy")
    trust_root = parser.add_mutually_exclusive_group()
    trust_root.add_argument("--policy", type=Path)
    trust_root.add_argument(
        "--require-gate", choices=("nightly", "release", "pr_sentinel")
    )
    parser.add_argument("--output", type=Path)
    return parser


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    try:
        args = _parser().parse_args()
        checked_in = args.policy is None
        policy_path = args.policy or root / "benchmarks/performance-policy.json"
        policy = load_policy(policy_path)
        if checked_in:
            cross_check_repository(policy, root)
        if args.require_gate is not None:
            require_gate(policy, args.require_gate)
        encoded = (
            json.dumps(policy_summary(policy), sort_keys=True, separators=(",", ":"))
            + "\n"
        ).encode()
        if args.output is None:
            sys.stdout.buffer.write(encoded)
        else:
            _write_atomic(args.output, encoded)
            print(args.output.expanduser().resolve())
        return 0
    except (PerformancePolicyError, OSError, ValueError) as error:
        payload = {
            "error": {"code": "performance_policy_audit_failed", "message": str(error)}
        }
        print(json.dumps(payload, sort_keys=True, separators=(",", ":")), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
