"""Run-private, hash-bound copies for measured binaries and workloads."""

from __future__ import annotations

import hashlib
import os
import tempfile
from pathlib import Path


class SnapshotError(ValueError):
    """A source changed before it could be bound to its validated hash."""


class SnapshotStore:
    def __init__(self, root: Path, run_id: str):
        parent = root / "target/benchmarks/snapshots"
        parent.mkdir(parents=True, exist_ok=True)
        self._temporary = tempfile.TemporaryDirectory(prefix=f"{run_id}-", dir=parent)
        self.path = Path(self._temporary.name)
        (self.path / "engines").mkdir()
        (self.path / "version-probes").mkdir()
        (self.path / "workloads").mkdir()

    def snapshot_engine(self, role: str, source: Path, expected_sha256: str) -> Path:
        destination = self.path / "engines" / f"{role}-{expected_sha256[:16]}"
        self._copy_verified(source, destination, expected_sha256, 0o500)
        return destination

    def snapshot_version_probe(self, role: str, source: Path, expected_sha256: str) -> Path:
        """Create a disposable executable copy that measurement never uses."""
        destination = self.path / "version-probes" / f"{role}-{expected_sha256[:16]}"
        self._copy_verified(source, destination, expected_sha256, 0o500)
        return destination

    def snapshot_workload(self, source: Path, expected_sha256: str) -> Path:
        safe_name = source.name.replace(os.sep, "_")
        destination = self.path / "workloads" / f"{expected_sha256[:16]}-{safe_name}"
        self._copy_verified(source, destination, expected_sha256, 0o400)
        return destination

    def _copy_verified(
        self,
        source: Path,
        destination: Path,
        expected_sha256: str,
        mode: int,
    ) -> None:
        digest = hashlib.sha256()
        try:
            with source.open("rb") as input_handle, destination.open("xb") as output_handle:
                while True:
                    chunk = input_handle.read(1024 * 1024)
                    if not chunk:
                        break
                    output_handle.write(chunk)
                    digest.update(chunk)
            destination.chmod(mode)
        except OSError as error:
            destination.unlink(missing_ok=True)
            raise SnapshotError(f"cannot snapshot {source}: {error}") from error
        actual = digest.hexdigest()
        if actual != expected_sha256:
            destination.unlink(missing_ok=True)
            raise SnapshotError(
                f"snapshot hash mismatch for {source}: expected {expected_sha256}, got {actual}"
            )

    def cleanup(self) -> None:
        self._temporary.cleanup()
