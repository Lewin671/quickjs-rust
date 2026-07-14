from __future__ import annotations

import json
import hashlib
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.schema import ManifestError, load_manifest, sha256_file


ROOT = Path(__file__).resolve().parents[3]


class ManifestTests(unittest.TestCase):
    def test_repository_manifest_and_hashes_validate(self) -> None:
        manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        self.assertEqual(manifest.schema_version, 4)
        self.assertEqual(manifest.lane_id, "throughput/wall_ns_per_operation")
        self.assertEqual(
            [case.id for case in manifest.cases],
            [
                "plain_function_call", "method_call", "captured_read",
                "captured_write", "many_locals_call", "property_read", "array_read",
            ],
        )
        self.assertTrue(all(not Path(identifier).is_absolute() for identifier in manifest.protocol_file_ids))
        self.assertEqual(
            manifest.protocol_file_ids,
            (
                "benchmarks/workloads/core-micro.js",
                "scripts/benchmark.sh",
                "tools/__init__.py",
                "tools/benchmark/__init__.py",
                "tools/benchmark/__main__.py",
                "tools/benchmark/adapters.py",
                "tools/benchmark/planning.py",
                "tools/benchmark/process.py",
                "tools/benchmark/receipts.py",
                "tools/benchmark/records.py",
                "tools/benchmark/runner.py",
                "tools/benchmark/schema.py",
                "tools/benchmark/snapshots.py",
            ),
        )
        self.assertTrue(all(sha256_file(case.workload) == case.workload_sha256 for case in manifest.cases))

    def _temporary_manifest(self) -> tuple[tempfile.TemporaryDirectory[str], Path, dict]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        (root / "benchmarks/workloads").mkdir(parents=True)
        workload = root / "benchmarks/workloads/work.js"
        workload.write_text("0;\n", encoding="utf-8")
        relative_workload = "benchmarks/workloads/work.js"
        protocol_digest = hashlib.sha256()
        protocol_digest.update(relative_workload.encode())
        protocol_digest.update(b"\0")
        protocol_digest.update(bytes.fromhex(sha256_file(workload)))
        protocol_digest.update(b"\n")
        data = {
            "schema_version": 4,
            "series": {"id": "test", "suite_id": "test@1"},
            "lane": {"id": "throughput/wall_ns_per_operation"},
            "protocol": {
                "id": "test-protocol-v1",
                "files": [relative_workload],
                "aggregate_sha256": protocol_digest.hexdigest(),
            },
            "reference_engine": {
                "identity": "quickjs-ng",
                "source_repo": "https://example.invalid/quickjs-ng.git",
                "revision": "a" * 40,
            },
            "profile": {
                "id": "test", "platform": "test-host",
            },
            "build_recipes": [
                {
                    "engine_identity": identity,
                    "build_mode": "release",
                    "toolchain": f"{identity}-toolchain",
                    "target": "test-target",
                    "features": [],
                    "flags": [],
                    "lto": "off",
                    "strip": "none",
                    "allocator": "system",
                    "host_features": "baseline",
                }
                for identity in ("qjs-rust", "quickjs-ng")
            ],
            "cases": [{
                "id": "case", "family": "family", "critical": True,
                "workload": "benchmarks/workloads/work.js",
                "workload_sha256": sha256_file(workload),
                "operations_per_iteration": 1,
                "checksum": {"model": "linear", "factor": 1},
                "measurement": {
                    "initial_iterations": 1, "max_iterations": 2, "min_window_ms": 1,
                    "startup_max_fraction": 0.01, "warmup_runs": 0, "timeout_seconds": 1,
                },
            }],
        }
        path = root / "benchmarks/manifest.json"
        return temporary, path, data

    def test_unknown_field_fails_closed(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        data["surprise"] = True
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "unknown"):
            load_manifest(path)

    def test_duplicate_json_key_fails_closed(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        encoded = json.dumps(data)
        path.write_text(encoded.replace('{"schema_version": 4,', '{"schema_version": 4, "schema_version": 4,'), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "duplicate key"):
            load_manifest(path)

    def test_hash_mismatch_fails_closed(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        data["cases"][0]["workload_sha256"] = "0" * 64
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "mismatch"):
            load_manifest(path)

    def test_protocol_hash_mismatch_fails_closed(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        data["protocol"]["aggregate_sha256"] = "0" * 64
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "protocol.*mismatch"):
            load_manifest(path)

    def test_protocol_file_edit_requires_manifest_update(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        path.write_text(json.dumps(data), encoding="utf-8")
        (path.parent / "workloads/work.js").write_text("1;\n", encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "protocol.*mismatch"):
            load_manifest(path)

    def test_boolean_schema_version_fails_closed(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        data["schema_version"] = True
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "integer version"):
            load_manifest(path)

    def test_duplicate_id_and_escape_fail_closed(self) -> None:
        temporary, path, data = self._temporary_manifest()
        self.addCleanup(temporary.cleanup)
        data["cases"].append(dict(data["cases"][0]))
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "duplicate"):
            load_manifest(path)
        data["cases"] = [data["cases"][0]]
        data["cases"][0]["workload"] = "../outside.js"
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ManifestError, "inside"):
            load_manifest(path)


if __name__ == "__main__":
    unittest.main()
