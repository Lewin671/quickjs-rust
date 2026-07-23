from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.build_cache import load_spec, ready_entry


ROOT = Path(__file__).resolve().parents[3]
CANDIDATE_SHA = "a" * 40
BASE_SHA = "b" * 40
REFERENCE_SHA = "f7830186043e4488f2998759d60a514faf07cbc9"


class BuildCacheOrchestratorTests(unittest.TestCase):
    def _executable(self, path: Path, body: str) -> None:
        path.write_text("#!/bin/sh\nset -eu\n" + body, encoding="utf-8")
        path.chmod(0o755)

    def test_miss_builds_once_shared_base_hits_and_next_run_skips_builds(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            temp = Path(name)
            harness = temp / "harness"
            base = temp / "base"
            mock_bin = temp / "bin"
            harness.mkdir()
            base.mkdir()
            mock_bin.mkdir()
            harness = harness.resolve()
            base = base.resolve()
            mock_bin = mock_bin.resolve()
            (harness / "scripts").mkdir()
            (harness / ".cargo").mkdir()
            (harness / ".cargo/config.toml").write_bytes(
                (ROOT / ".cargo/config.toml").read_bytes()
            )
            (harness / "third_party/quickjs-ng").mkdir(parents=True)
            (harness / "tools").symlink_to(ROOT / "tools", target_is_directory=True)
            (harness / "benchmarks").symlink_to(ROOT / "benchmarks", target_is_directory=True)
            preview_script = harness / "scripts/performance-preview.sh"
            preview_script.write_bytes((ROOT / "scripts/performance-preview.sh").read_bytes())
            preview_script.chmod(0o755)
            log = temp / "builds.log"
            rustflags_log = temp / "rustflags.log"

            self._executable(mock_bin / "git", f'''
case " $* " in
  *" ls-files "*) printf '100644 {"1" * 40} 0\\tCargo.toml\\0'; exit 0 ;;
  *" remote get-url origin "*) echo https://github.com/quickjs-ng/quickjs.git; exit 0 ;;
  *" status "*)
    if [ "${{TEST_DIRTY_BUILD:-0}}" = 1 ] && [ -s "$TEST_BUILD_LOG" ]; then
      case " $* " in
        *"$TEST_QUICKJS"*) ;;
        *"$TEST_HARNESS"*) echo ' M crates/qjs-runtime/src/lib.rs' ;;
      esac
    fi
    exit 0 ;;
  *" submodule "*) exit 0 ;;
  *" rev-parse HEAD "*)
    case " $* " in
      *"$TEST_QUICKJS"*) echo {REFERENCE_SHA} ;;
      *"$TEST_BASE"*) echo {BASE_SHA} ;;
      *) echo {CANDIDATE_SHA} ;;
    esac
    exit 0 ;;
esac
echo "unexpected mock git invocation: $*" >&2
exit 9
''')
            self._executable(mock_bin / "rustc", """
echo 'rustc 1.95.0 (mock)'
echo 'host: x86_64-unknown-linux-gnu'
""")
            self._executable(mock_bin / "cargo", """
if [ "${1:-}" = "-V" ]; then echo 'cargo 1.95.0 (mock)'; exit 0; fi
if [ "${1:-}" = "build" ]; then
  echo cargo >> "$TEST_BUILD_LOG"
  printf '%s' "$CARGO_ENCODED_RUSTFLAGS" > "$TEST_RUSTFLAGS_LOG"
  output="$CARGO_TARGET_DIR/$CARGO_BUILD_TARGET/release/qjs"
  mkdir -p "$(dirname "$output")"
  printf '%s\n' '#!/bin/sh' 'exit 0' > "$output"
  chmod 755 "$output"
  exit 0
fi
exit 8
""")
            self._executable(mock_bin / "cc", """
case "${1:-}" in
  --version) echo 'cc mock 1' ;;
  -dumpmachine) echo x86_64-unknown-linux-gnu ;;
  -print-prog-name=ld) echo "$TEST_MOCK_BIN/ld" ;;
  *) exit 7 ;;
esac
""")
            self._executable(mock_bin / "ld", "echo 'ld mock 1'\n")
            self._executable(mock_bin / "cmake", "echo 'cmake version mock-1'\n")
            self._executable(mock_bin / "make", """
if [ "${1:-}" = "--version" ]; then echo 'GNU Make mock-1'; exit 0; fi
source_dir=''
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-C" ]; then source_dir="$2"; shift 2; else shift; fi
done
echo make >> "$TEST_BUILD_LOG"
mkdir -p "$source_dir/build"
printf '%s\n' '#!/bin/sh' 'exit 0' > "$source_dir/build/qjs"
chmod 755 "$source_dir/build/qjs"
""")
            real_python = sys.executable
            self._executable(mock_bin / "python3", """
case " $* " in
  *" tools.benchmark.preview prepare "*) exit 42 ;;
esac
exec "$TEST_REAL_PYTHON" "$@"
""")

            environment = os.environ.copy()
            environment.update({
                "PATH": f"{mock_bin}:{environment['PATH']}",
                "TEST_BASE": str(base),
                "TEST_BUILD_LOG": str(log),
                "TEST_HARNESS": str(harness),
                "TEST_MOCK_BIN": str(mock_bin),
                "TEST_QUICKJS": str(harness / "third_party/quickjs-ng"),
                "TEST_REAL_PYTHON": real_python,
                "TEST_RUSTFLAGS_LOG": str(rustflags_log),
                "ImageOS": "ubuntu24",
                "ImageVersion": "mock-image-1",
            })
            cache_root = harness / "target/cache"

            def invoke(
                run: str,
                selected_cache: Path = cache_root,
                require_provenance: bool = True,
            ) -> tuple[subprocess.CompletedProcess[str], dict[str, object] | None]:
                output = harness / f"target/{run}/evidence"
                command = [
                    "bash", str(preview_script),
                    "--harness-mode", "main_push_head_owned_harness",
                    "--candidate-source", str(harness), "--base-source", str(base),
                    "--candidate-sha", CANDIDATE_SHA, "--base-sha", BASE_SHA,
                    "--candidate-repo", "https://github.com/example/repo.git",
                    "--base-repo", "https://github.com/example/repo.git",
                    "--output", str(output), "--build-cache-root", str(selected_cache),
                ]
                result = subprocess.run(
                    command, env=environment, capture_output=True, text=True,
                    timeout=30, check=False,
                )
                cache_file = output / "build-cache.json"
                if not require_provenance:
                    return result, None
                if not cache_file.is_file():
                    self.fail(
                        f"orchestrator did not reach cache provenance; rc={result.returncode}\n"
                        f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
                    )
                provenance = json.loads(cache_file.read_text(encoding="utf-8"))
                return result, provenance

            first, first_cache = invoke("first")
            self.assertEqual(first.returncode, 42, first.stderr)
            self.assertEqual(log.read_text(encoding="utf-8").splitlines(), ["cargo", "make"])
            self.assertEqual(
                rustflags_log.read_text(encoding="utf-8").split("\x1f"),
                ["-Ctarget-cpu=generic", "-Cllvm-args=-align-all-functions=4"],
            )
            self.assertEqual(first_cache["roles"]["candidate"]["status"], "rebuilt")
            self.assertEqual(first_cache["roles"]["base"]["status"], "hit")
            self.assertEqual(first_cache["roles"]["quickjs-ng"]["status"], "rebuilt")
            self.assertEqual(
                first_cache["roles"]["candidate"]["key_sha256"],
                first_cache["roles"]["base"]["key_sha256"],
            )
            plan_root = harness / "target/first/build-cache-plan"
            for role, namespace in (
                ("candidate", "rust"), ("base", "rust"),
                ("quickjs-ng", "quickjs-ng"),
            ):
                spec = load_spec(plan_root / f"{role}.json")
                entry = cache_root / namespace / spec["key_sha256"]
                self.assertTrue(ready_entry(entry, spec)[0], role)

            second, second_cache = invoke("second")
            self.assertEqual(second.returncode, 42, second.stderr)
            self.assertEqual(log.read_text(encoding="utf-8").splitlines(), ["cargo", "make"])
            self.assertEqual(
                {role: value["status"] for role, value in second_cache["roles"].items()},
                {"candidate": "hit", "base": "hit", "quickjs-ng": "hit"},
            )

            dirty_log = temp / "dirty-builds.log"
            dirty_cache = harness / "target/dirty-cache"
            environment["TEST_BUILD_LOG"] = str(dirty_log)
            environment["TEST_DIRTY_BUILD"] = "1"
            dirty, dirty_provenance = invoke(
                "dirty", selected_cache=dirty_cache, require_provenance=False
            )
            self.assertNotEqual(dirty.returncode, 0)
            self.assertIsNone(dirty_provenance)
            self.assertIn("dirty after build", dirty.stderr)
            self.assertEqual(dirty_log.read_text(encoding="utf-8").splitlines(), ["cargo"])
            dirty_spec = load_spec(
                harness / "target/dirty/build-cache-plan/candidate.json"
            )
            dirty_entry = dirty_cache / "rust" / dirty_spec["key_sha256"]
            self.assertFalse(dirty_entry.exists())
            self.assertFalse(ready_entry(dirty_entry, dirty_spec)[0])

            environment.pop("TEST_DIRTY_BUILD")
            (base / ".cargo").mkdir()
            (base / ".cargo/config.toml").write_text(
                "[build]\nrustc-wrapper = \"untrusted\"\n", encoding="utf-8"
            )
            mismatch, mismatch_provenance = invoke(
                "mismatched-config", require_provenance=False
            )
            self.assertNotEqual(mismatch.returncode, 0)
            self.assertIsNone(mismatch_provenance)
            self.assertIn(
                "source-local Cargo config does not match the trusted harness",
                mismatch.stderr,
            )


if __name__ == "__main__":
    unittest.main()
