from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]


class CliTests(unittest.TestCase):
    def test_tools_package_index_is_in_benchmark_touched_scope(self) -> None:
        script = (ROOT / "scripts/check-touched.sh").read_text(encoding="utf-8")
        benchmark_case = re.search(
            r"benchmarks/\*\|.*?\)\s*\n\s*touches_benchmark=1",
            script,
            flags=re.DOTALL,
        )
        self.assertIsNotNone(benchmark_case)
        assert benchmark_case is not None
        self.assertIn("tools/__init__.py", benchmark_case.group(0))
        self.assertIn("scripts/resource-benchmark*.sh", benchmark_case.group(0))
        self.assertIn("scripts/lifecycle-bench.sh", benchmark_case.group(0))
        self.assertIn("scripts/external-corpus-audit.sh", benchmark_case.group(0))
        self.assertIn("scripts/performance-policy-audit.sh", benchmark_case.group(0))
        self.assertIn("scripts/performance-preview.sh", benchmark_case.group(0))
        self.assertIn(".github/workflows/performance-smoke.yml", benchmark_case.group(0))

    def test_benchmark_shells_are_in_syntax_gate(self) -> None:
        for script_name in ("check-touched.sh", "check.sh"):
            script = (ROOT / f"scripts/{script_name}").read_text(encoding="utf-8")
            self.assertIn("resource-benchmark.sh", script)
            self.assertIn("resource-benchmark-report.sh", script)
            self.assertIn("lifecycle-bench.sh", script)
            self.assertIn("external-corpus-audit.sh", script)
            self.assertIn("performance-policy-audit.sh", script)
            self.assertIn("performance-preview.sh", script)

    def test_benchmark_python_touched_gate_runs_size_guard_without_cargo(self) -> None:
        script = (ROOT / "scripts/check-touched.sh").read_text(encoding="utf-8")
        blocks = re.findall(
            r'if \[ "\$has_rust".*?\nfi', script, flags=re.DOTALL
        )
        self.assertGreaterEqual(len(blocks), 2)
        cargo_gate, size_gate = blocks[-2:]
        self.assertNotIn("touches_benchmark", cargo_gate)
        self.assertIn("cargo", cargo_gate.lower())
        self.assertNotIn("check-file-size.sh", cargo_gate)
        self.assertIn("touches_benchmark", size_gate)
        self.assertIn("check-file-size.sh", size_gate)
        self.assertNotIn("$CARGO_BIN", size_gate)
        self.assertNotIn(" fmt ", size_gate)
        self.assertNotIn(" clippy ", size_gate)

    def test_wrapper_runs_from_outside_repository(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.run(
                [
                    str(ROOT / "scripts/benchmark.sh"),
                    "--dry-run",
                    "--blocks", "1",
                    "--case", "plain_function_call",
                ],
                cwd=directory,
                capture_output=True,
                text=True,
                timeout=10,
                check=False,
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        plan = json.loads(result.stdout)
        self.assertEqual(plan["cases"], ["plain_function_call"])
        self.assertFalse(plan["portfolio_complete"])
        self.assertEqual(plan["protocol_id"], "quickjs-measurement-protocol-v7")
        self.assertEqual(plan["schema_version"], 4)
        self.assertEqual(plan["lane_id"], "throughput/wall_ns_per_operation")
        declarations = {item["role"]: item for item in plan["engine_declarations"]}
        self.assertEqual(declarations["candidate"]["adapter_id"], "qjs-rust-raw")
        self.assertEqual(declarations["candidate"]["engine_identity"], "qjs-rust")
        self.assertEqual(declarations["quickjs-ng"]["adapter_id"], "qjs-file")

    def test_adapter_and_identity_are_independent_and_known(self) -> None:
        result = subprocess.run(
            [
                str(ROOT / "scripts/benchmark.sh"), "--dry-run", "--blocks", "1",
                "--candidate", "/bin/echo", "--candidate-adapter", "qjs-file",
                "--candidate-identity", "qjs-rust",
            ],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        declaration = json.loads(result.stdout)["engine_declarations"][0]
        self.assertEqual(declaration["adapter_id"], "qjs-file")
        self.assertEqual(declaration["engine_identity"], "qjs-rust")

        rejected = subprocess.run(
            [
                str(ROOT / "scripts/benchmark.sh"), "--dry-run", "--blocks", "1",
                "--candidate-identity", "unknown-engine",
            ],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        self.assertEqual(rejected.returncode, 2)
        self.assertIn("unknown engine identity", rejected.stderr)

    def test_lifecycle_wrapper_isolates_quick_and_enforces_option_allowlist(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary = Path(directory)
            fake_cargo = temporary / "cargo"
            fake_cargo.write_text(
                "#!/bin/sh\n"
                "printf '%s\\n' \"$CRITERION_HOME\" > \"$FAKE_CARGO_LOG\"\n"
                "printf '%s\\n' \"$@\" >> \"$FAKE_CARGO_LOG\"\n",
                encoding="utf-8",
            )
            fake_cargo.chmod(0o700)
            environment = {
                **os.environ,
                "CARGO": str(fake_cargo),
                "CRITERION_HOME": str(temporary / "attacker-home"),
            }

            quick_log = temporary / "quick.log"
            quick = subprocess.run(
                [str(ROOT / "scripts/lifecycle-bench.sh"), "--quick", "parse/small-v1"],
                cwd=temporary,
                env={**environment, "FAKE_CARGO_LOG": str(quick_log)},
                capture_output=True,
                text=True,
                timeout=10,
                check=False,
            )
            self.assertEqual(quick.returncode, 0, quick.stderr)
            quick_lines = quick_log.read_text(encoding="utf-8").splitlines()
            self.assertEqual(quick_lines[0], str(ROOT / "target/criterion-smoke"))
            self.assertEqual(
                quick_lines[1:],
                ["bench", "-p", "qjs-runtime", "--bench", "lifecycle", "--",
                 "--quick", "parse/small-v1", "--discard-baseline"],
            )

            formal_log = temporary / "formal.log"
            formal = subprocess.run(
                [str(ROOT / "scripts/lifecycle-bench.sh"), "compile/medium-v1"],
                cwd=temporary,
                env={**environment, "FAKE_CARGO_LOG": str(formal_log)},
                capture_output=True,
                text=True,
                timeout=10,
                check=False,
            )
            self.assertEqual(formal.returncode, 0, formal.stderr)
            formal_lines = formal_log.read_text(encoding="utf-8").splitlines()
            self.assertEqual(formal_lines[0], str(ROOT / "target/criterion"))
            self.assertNotIn("--discard-baseline", formal_lines)

            allowed = (
                "--list", "--help", "--verbose", "--quiet", "--noplot",
                "--exact", "--ignored", "--color=auto", "--color=always",
                "--color=never", "--format=pretty", "--format=terse", "-v", "-n", "-h",
            )
            for index, option in enumerate(allowed):
                with self.subTest(allowed=option):
                    allowed_log = temporary / f"allowed-{index}.log"
                    result = subprocess.run(
                        [str(ROOT / "scripts/lifecycle-bench.sh"), option],
                        cwd=temporary,
                        env={**environment, "FAKE_CARGO_LOG": str(allowed_log)},
                        capture_output=True,
                        text=True,
                        timeout=10,
                        check=False,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertTrue(allowed_log.exists())

            value_flags = (
                "sample-size", "warm-up-time", "measurement-time", "nresamples",
                "noise-threshold", "confidence-level", "significance-level",
                "save-baseline", "baseline", "baseline-lenient", "load-baseline",
                "profile-time", "output-format",
            )
            attacks = [(f"--{flag}=1",) for flag in value_flags]
            attacks += [(f"--{flag}", "1") for flag in value_flags]
            attacks += [
                ("--discard-baseline",), ("--plotting-backend=gnuplot",),
                ("--plotting-backend", "gnuplot"), ("--color", "auto"),
                ("--format", "pretty"), ("--future-option",), ("-x",),
                ("-s", "name"), ("-bname",), ("-vsattacker",), ("-vbfoo",),
                ("-nvsfoo",), ("-nvbfoo",),
            ]
            for index, attack in enumerate(attacks):
                with self.subTest(attack=attack):
                    attack_log = temporary / f"attack-{index}.log"
                    result = subprocess.run(
                        [str(ROOT / "scripts/lifecycle-bench.sh"), *attack],
                        cwd=temporary,
                        env={**environment, "FAKE_CARGO_LOG": str(attack_log)},
                        capture_output=True,
                        text=True,
                        timeout=10,
                        check=False,
                    )
                    self.assertEqual(result.returncode, 2)
                    self.assertIn("policy rejects unsupported option", result.stderr)
                    self.assertFalse(attack_log.exists())

    def test_lifecycle_fixture_sentinels_and_deferred_drop_are_declared(self) -> None:
        bench = (ROOT / "crates/qjs-runtime/benches/lifecycle.rs").read_text(
            encoding="utf-8"
        )
        declarations = {
            "small-v1.js": (553, 0x834B63AD0EDE94C3),
            "medium-v1.js": (1644, 0x96DF4A20E8F19E57),
        }

        def fnv1a64(data: bytes) -> int:
            value = 0xCBF29CE484222325
            for byte in data:
                value = ((value ^ byte) * 0x100000001B3) & ((1 << 64) - 1)
            return value

        for name, (length, fingerprint) in declarations.items():
            data = (ROOT / "crates/qjs-runtime/benches/fixtures" / name).read_bytes()
            self.assertEqual(len(data), length)
            self.assertEqual(fnv1a64(data), fingerprint)
            self.assertIn(f"expected_bytes: {length:_}", bench)
            grouped = f"0x{fingerprint:016x}"
            self.assertIn(
                f"0x{grouped[2:6]}_{grouped[6:10]}_{grouped[10:14]}_{grouped[14:]}",
                bench,
            )
        self.assertEqual(bench.count("iter_with_large_drop"), 3)
        self.assertIn("black_box((script, bytecode))", bench)


if __name__ == "__main__":
    unittest.main()
