"""Strict governance registry for external benchmark corpora.

This module validates metadata only.  It never fetches or executes a corpus.
"""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import os
import re
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn
from urllib.parse import urlparse


class ExternalCorpusError(ValueError):
    """An external-corpus registry cannot support the requested operation."""


_GIT_SHA = re.compile(r"[0-9a-f]{40}\Z")
_SHA256 = re.compile(r"[0-9a-f]{64}\Z")
_CORPUS_ID = re.compile(r"[a-z0-9]+(?:[.-][a-z0-9]+)*\Z")
_DECISIONS = {"blocked", "excluded"}
_AUDIT_STATUS = {"pending", "passed"}
_LICENSE_STATUS = {"pending", "complete"}
_NOTICE_STATUS = {"pending", "complete", "not_required"}
_EXPECTED_CASE_STATUS = {"pending", "complete"}


def _reject_json_constant(value: str) -> None:
    raise ExternalCorpusError(
        f"external corpus registry contains non-standard numeric constant {value}"
    )


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ExternalCorpusError(
                f"external corpus registry contains duplicate key {key!r}"
            )
        result[key] = value
    return result


def _keys(value: dict[str, Any], expected: set[str], where: str) -> None:
    missing = expected - set(value)
    unknown = set(value) - expected
    if missing or unknown:
        details = []
        if missing:
            details.append(f"missing {sorted(missing)}")
        if unknown:
            details.append(f"unknown {sorted(unknown)}")
        raise ExternalCorpusError(f"{where}: {', '.join(details)}")


def _object(value: Any, where: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ExternalCorpusError(f"{where}: expected an object")
    return value


def _array(value: Any, where: str) -> list[Any]:
    if not isinstance(value, list):
        raise ExternalCorpusError(f"{where}: expected an array")
    return value


def _string(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value or value.strip() != value:
        raise ExternalCorpusError(f"{where}: expected a non-empty trimmed string")
    return value


def _nullable_string(value: Any, where: str) -> str | None:
    if value is None:
        return None
    return _string(value, where)


def _enum(value: Any, allowed: set[str], where: str) -> str:
    text = _string(value, where)
    if text not in allowed:
        raise ExternalCorpusError(f"{where}: expected one of {sorted(allowed)}")
    return text


def _boolean(value: Any, where: str) -> bool:
    if not isinstance(value, bool):
        raise ExternalCorpusError(f"{where}: expected a boolean")
    return value


def _url(value: Any, where: str) -> str:
    text = _string(value, where)
    if any(
        character.isspace() or ord(character) < 0x20 or ord(character) == 0x7F
        for character in text
    ):
        raise ExternalCorpusError(
            f"{where}: URL cannot contain whitespace or control characters"
        )
    try:
        parsed = urlparse(text)
        hostname = parsed.hostname
        parsed.port
    except ValueError as error:
        raise ExternalCorpusError(f"{where}: malformed URL authority: {error}") from error
    if (
        parsed.scheme != "https"
        or not parsed.netloc
        or not hostname
        or parsed.username is not None
        or parsed.password is not None
        or parsed.fragment
    ):
        raise ExternalCorpusError(
            f"{where}: expected an HTTPS URL without credentials or fragment"
        )
    try:
        ipaddress.ip_address(hostname)
    except ValueError:
        try:
            ascii_hostname = hostname.encode("idna").decode("ascii")
        except UnicodeError as error:
            raise ExternalCorpusError(f"{where}: invalid hostname") from error
        labels = ascii_hostname.rstrip(".").split(".")
        if (
            len(ascii_hostname) > 253
            or any(
                not label
                or len(label) > 63
                or re.fullmatch(
                    r"[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?", label
                ) is None
                for label in labels
            )
        ):
            raise ExternalCorpusError(f"{where}: invalid hostname")
        lowered = ascii_hostname.rstrip(".").lower()
        if lowered in {"example.com", "example.net", "example.org"} or lowered.endswith(
            ".example"
        ):
            raise ExternalCorpusError(
                f"{where}: placeholder example hostname is forbidden"
            )
    return text


def _strings(value: Any, where: str, *, unique: bool = False) -> tuple[str, ...]:
    items = tuple(_string(item, f"{where}[]") for item in _array(value, where))
    if unique and len(set(items)) != len(items):
        raise ExternalCorpusError(f"{where}: values must be unique")
    return items


def _urls(value: Any, where: str) -> tuple[str, ...]:
    items = tuple(_url(item, f"{where}[]") for item in _array(value, where))
    if len(set(items)) != len(items):
        raise ExternalCorpusError(f"{where}: URLs must be unique")
    return items


@dataclass(frozen=True)
class Corpus:
    id: str
    name: str
    source_url: str
    revision: str | None
    decision: str
    intended_use: str
    license_status: str
    inventory_status: str
    inventory_entries: tuple[str, ...]
    notice_status: str
    notice_sha256: str | None
    notice_evidence_urls: tuple[str, ...]
    capability_status: str
    capability_requirements: tuple[str, ...]
    adapter: str | None
    timing_status: str
    neutral_referee: str | None
    phase_boundary: str | None
    expected_cases_status: str
    expected_cases: tuple[str, ...]
    blockers: tuple[str, ...]
    reason: str
    evidence_urls: tuple[str, ...]


@dataclass(frozen=True)
class ExternalCorpusRegistry:
    path: Path
    sha256: str
    schema_version: int
    registry_id: str
    claim_eligible: bool
    corpora: tuple[Corpus, ...]

    def by_id(self, corpus_id: str) -> Corpus | None:
        return next((corpus for corpus in self.corpora if corpus.id == corpus_id), None)


def _license(data: Any, where: str) -> tuple[str, str, tuple[str, ...], str, str | None, tuple[str, ...]]:
    item = _object(data, where)
    _keys(item, {"status", "per_file_inventory", "notice"}, where)
    status = _enum(item["status"], _LICENSE_STATUS, f"{where}.status")

    inventory = _object(item["per_file_inventory"], f"{where}.per_file_inventory")
    _keys(inventory, {"status", "entries"}, f"{where}.per_file_inventory")
    inventory_status = _enum(
        inventory["status"], _LICENSE_STATUS, f"{where}.per_file_inventory.status"
    )
    inventory_entries = _strings(
        inventory["entries"], f"{where}.per_file_inventory.entries", unique=True
    )

    notice = _object(item["notice"], f"{where}.notice")
    _keys(notice, {"status", "sha256", "evidence_urls"}, f"{where}.notice")
    notice_status = _enum(notice["status"], _NOTICE_STATUS, f"{where}.notice.status")
    notice_sha256 = _nullable_string(notice["sha256"], f"{where}.notice.sha256")
    if notice_sha256 is not None and not _SHA256.fullmatch(notice_sha256):
        raise ExternalCorpusError(f"{where}.notice.sha256: expected lowercase SHA-256")
    notice_evidence = _urls(notice["evidence_urls"], f"{where}.notice.evidence_urls")
    return (
        status, inventory_status, inventory_entries, notice_status,
        notice_sha256, notice_evidence,
    )


def _parse_corpus(data: Any, index: int) -> Corpus:
    where = f"registry.corpora[{index}]"
    item = _object(data, where)
    _keys(
        item,
        {
            "id", "name", "source", "decision", "intended_use", "license_audit",
            "capability_audit", "timing_audit", "expected_cases", "blockers",
            "reason", "evidence_urls",
        },
        where,
    )
    source = _object(item["source"], f"{where}.source")
    _keys(source, {"url", "revision"}, f"{where}.source")
    revision = _nullable_string(source["revision"], f"{where}.source.revision")
    if revision is not None and not _GIT_SHA.fullmatch(revision):
        raise ExternalCorpusError(f"{where}.source.revision: expected full lowercase git SHA")
    if revision == "0" * 40:
        raise ExternalCorpusError(f"{where}.source.revision: all-zero git SHA is forbidden")

    license_values = _license(item["license_audit"], f"{where}.license_audit")
    capability = _object(item["capability_audit"], f"{where}.capability_audit")
    _keys(capability, {"status", "requirements", "adapter"}, f"{where}.capability_audit")
    capability_status = _enum(
        capability["status"], _AUDIT_STATUS, f"{where}.capability_audit.status"
    )
    requirements = _strings(
        capability["requirements"], f"{where}.capability_audit.requirements", unique=True
    )
    adapter = _nullable_string(capability["adapter"], f"{where}.capability_audit.adapter")

    timing = _object(item["timing_audit"], f"{where}.timing_audit")
    _keys(timing, {"status", "neutral_referee", "phase_boundary"}, f"{where}.timing_audit")
    timing_status = _enum(timing["status"], _AUDIT_STATUS, f"{where}.timing_audit.status")
    neutral_referee = _nullable_string(
        timing["neutral_referee"], f"{where}.timing_audit.neutral_referee"
    )
    phase_boundary = _nullable_string(
        timing["phase_boundary"], f"{where}.timing_audit.phase_boundary"
    )

    expected = _object(item["expected_cases"], f"{where}.expected_cases")
    _keys(expected, {"status", "cases"}, f"{where}.expected_cases")
    expected_status = _enum(
        expected["status"], _EXPECTED_CASE_STATUS, f"{where}.expected_cases.status"
    )
    expected_cases = _strings(
        expected["cases"], f"{where}.expected_cases.cases", unique=True
    )

    corpus = Corpus(
        id=_string(item["id"], f"{where}.id"),
        name=_string(item["name"], f"{where}.name"),
        source_url=_url(source["url"], f"{where}.source.url"),
        revision=revision,
        decision=_enum(item["decision"], _DECISIONS, f"{where}.decision"),
        intended_use=_string(item["intended_use"], f"{where}.intended_use"),
        license_status=license_values[0],
        inventory_status=license_values[1],
        inventory_entries=license_values[2],
        notice_status=license_values[3],
        notice_sha256=license_values[4],
        notice_evidence_urls=license_values[5],
        capability_status=capability_status,
        capability_requirements=requirements,
        adapter=adapter,
        timing_status=timing_status,
        neutral_referee=neutral_referee,
        phase_boundary=phase_boundary,
        expected_cases_status=expected_status,
        expected_cases=expected_cases,
        blockers=_strings(item["blockers"], f"{where}.blockers", unique=True),
        reason=_string(item["reason"], f"{where}.reason"),
        evidence_urls=_urls(item["evidence_urls"], f"{where}.evidence_urls"),
    )
    if not _CORPUS_ID.fullmatch(corpus.id):
        raise ExternalCorpusError(f"{where}.id: expected a stable lowercase token id")
    if corpus.decision == "blocked" and corpus.revision is None:
        raise ExternalCorpusError(f"{where}.source.revision: blocked corpus requires a full pin")
    if corpus.decision == "blocked" and not corpus.blockers:
        raise ExternalCorpusError(f"{where}.blockers: blocked corpus requires at least one blocker")
    if corpus.notice_status == "pending" and corpus.notice_sha256 is not None:
        raise ExternalCorpusError(f"{where}.license_audit.notice: pending NOTICE cannot have a hash")
    if corpus.notice_status == "complete" and (
        corpus.notice_sha256 is None or not corpus.notice_evidence_urls
    ):
        raise ExternalCorpusError(
            f"{where}.license_audit.notice: complete NOTICE requires hash and evidence URL"
        )
    if corpus.notice_sha256 == "0" * 64:
        raise ExternalCorpusError(
            f"{where}.license_audit.notice.sha256: all-zero SHA-256 is forbidden"
        )
    if corpus.notice_status == "not_required" and (
        corpus.notice_sha256 is not None or not corpus.notice_evidence_urls
    ):
        raise ExternalCorpusError(
            f"{where}.license_audit.notice: not_required requires no hash and an evidence URL"
        )
    return corpus


def load_registry(path: Path) -> ExternalCorpusRegistry:
    path = path.expanduser().resolve()
    try:
        raw = path.read_bytes()
        data = json.loads(
            raw,
            object_pairs_hook=_unique_object,
            parse_constant=_reject_json_constant,
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ExternalCorpusError(f"cannot read external corpus registry {path}: {error}") from error
    root = _object(data, "registry")
    _keys(root, {"schema_version", "registry_id", "claim_eligible", "corpora"}, "registry")
    if type(root["schema_version"]) is not int or root["schema_version"] != 1:
        raise ExternalCorpusError("registry.schema_version: only integer version 1 is supported")
    registry_id = _string(root["registry_id"], "registry.registry_id")
    if registry_id != "quickjs-external-corpora-v1":
        raise ExternalCorpusError(
            "registry.registry_id: version 1 requires quickjs-external-corpora-v1"
        )
    claim_eligible = _boolean(root["claim_eligible"], "registry.claim_eligible")
    if claim_eligible:
        raise ExternalCorpusError("registry.claim_eligible: governance metadata must be false")
    corpora = tuple(
        _parse_corpus(item, index)
        for index, item in enumerate(_array(root["corpora"], "registry.corpora"))
    )
    ids = [corpus.id for corpus in corpora]
    names = [corpus.name for corpus in corpora]
    if not corpora:
        raise ExternalCorpusError("registry.corpora: expected a non-empty array")
    if ids != sorted(ids) or len(set(ids)) != len(ids):
        raise ExternalCorpusError("registry.corpora: ids must be unique and sorted")
    if len(set(names)) != len(names):
        raise ExternalCorpusError("registry.corpora: names must be unique")
    return ExternalCorpusRegistry(
        path=path,
        sha256=hashlib.sha256(raw).hexdigest(),
        schema_version=1,
        registry_id=registry_id,
        claim_eligible=claim_eligible,
        corpora=corpora,
    )


def require_admitted(registry: ExternalCorpusRegistry, corpus_id: str) -> NoReturn:
    corpus_id = _string(corpus_id, "required corpus id")
    corpus = registry.by_id(corpus_id)
    if corpus is None:
        raise ExternalCorpusError(f"unknown external corpus {corpus_id!r}")
    raise ExternalCorpusError(
        f"external corpus {corpus_id!r} is {corpus.decision}, not admitted"
    )


def registry_summary(registry: ExternalCorpusRegistry) -> dict[str, Any]:
    counts = {
        decision: sum(corpus.decision == decision for corpus in registry.corpora)
        for decision in ("blocked", "excluded")
    }
    return {
        "admitted_count": 0,
        "blocked_count": counts["blocked"],
        "claim_eligible": False,
        "corpus_count": len(registry.corpora),
        "excluded_count": counts["excluded"],
        "registry_id": registry.registry_id,
        "registry_sha256": registry.sha256,
        "schema_version": registry.schema_version,
    }


def _write_atomic(output: Path, encoded: bytes) -> None:
    output = output.expanduser().resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        raise ExternalCorpusError(f"refusing to overwrite existing output: {output}")
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
            raise ExternalCorpusError(
                f"refusing to overwrite existing output: {output}"
            ) from error
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


class _AuditParser(argparse.ArgumentParser):
    def error(self, message: str) -> None:
        raise ExternalCorpusError(f"arguments: {message}")


def _parser() -> argparse.ArgumentParser:
    parser = _AuditParser(description="validate external benchmark corpus governance")
    trust_root = parser.add_mutually_exclusive_group()
    trust_root.add_argument("--registry", type=Path)
    trust_root.add_argument("--require-admitted", metavar="ID")
    parser.add_argument("--output", type=Path)
    return parser


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    try:
        args = _parser().parse_args()
        registry_path = args.registry or root / "benchmarks/external-corpora.json"
        registry = load_registry(registry_path)
        if args.require_admitted is not None:
            require_admitted(registry, args.require_admitted)
        encoded = (
            json.dumps(registry_summary(registry), sort_keys=True, separators=(",", ":"))
            + "\n"
        ).encode()
        if args.output is None:
            sys.stdout.buffer.write(encoded)
        else:
            _write_atomic(args.output, encoded)
            print(args.output.expanduser().resolve())
        return 0
    except (ExternalCorpusError, OSError, ValueError) as error:
        payload = {"error": {"code": "external_corpus_audit_failed", "message": str(error)}}
        print(json.dumps(payload, sort_keys=True, separators=(",", ":")), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
