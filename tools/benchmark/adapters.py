"""Black-box JavaScript shell adapters."""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from pathlib import Path

from .process import run_process
from .receipts import BuildReceipt


class AdapterError(ValueError):
    """An engine binary or adapter kind is invalid."""


@dataclass(frozen=True)
class Engine:
    role: str
    adapter_id: str
    engine_identity: str
    binary: Path
    binary_sha256: str
    receipt: BuildReceipt | None

    @property
    def provenance_status(self) -> str:
        if self.receipt is None:
            return "unverified"
        return "dirty" if self.receipt.source_dirty else "verified"

    def command(
        self,
        workload: Path,
        case_id: str,
        iterations: int,
        *,
        binary: Path | None = None,
    ) -> list[str]:
        command = [str(binary or self.binary)]
        if self.adapter_id == "qjs-rust-raw":
            command.append("--raw")
        command.extend([str(workload), case_id, str(iterations)])
        return command


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def probe_version(binary: Path) -> str | None:
    """Return bounded best-effort metadata for a disposable executable copy."""
    for flag in ("--version", "-v"):
        try:
            completed = run_process([str(binary), flag], 2)
        except OSError:
            continue
        text = (completed.stdout or completed.stderr).strip()
        if completed.exit_code == 0 and not completed.timed_out and text:
            return text[:512]
    return None


def load_engine(
    role: str,
    adapter_id: str,
    engine_identity: str,
    binary: Path,
    receipt: BuildReceipt | None = None,
) -> Engine:
    if role not in {"candidate", "base", "quickjs-ng"}:
        raise AdapterError(f"unknown role {role!r}")
    if adapter_id not in {"qjs-rust-raw", "qjs-file"}:
        raise AdapterError(f"unknown adapter id {adapter_id!r}")
    if not engine_identity or engine_identity.strip() != engine_identity:
        raise AdapterError("engine identity must be a non-empty trimmed string")
    binary = binary.expanduser().resolve()
    if not binary.is_file():
        raise AdapterError(f"{role} binary does not exist: {binary}")
    if not binary.stat().st_mode & 0o111:
        raise AdapterError(f"{role} binary is not executable: {binary}")
    return Engine(
        role=role,
        adapter_id=adapter_id,
        engine_identity=engine_identity,
        binary=binary,
        binary_sha256=_sha256(binary),
        receipt=receipt,
    )
