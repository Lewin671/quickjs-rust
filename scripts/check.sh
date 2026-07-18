#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"

if ! CARGO_BIN="$(qjs_resolve_cargo)"; then
  echo "error: cargo not found; install Rust with rustup before running checks" >&2
  exit 127
fi

qjs_check_stage() {
  local label="$1"
  shift
  local start="$SECONDS"
  echo "check: $label"
  if "$@"; then
    echo "check: ok $label ($((SECONDS - start))s)"
  else
    local status="$?"
    echo "error: failed $label after $((SECONDS - start))s" >&2
    return "$status"
  fi
}

qjs_check_runtime_core_tests() {
  local cargo_bin="$1"
  # Run the core tests in bounded, separate-process batches. A single
  # ~840-test process accumulates enough peak memory on hosted runners to be
  # killed mid-run (observed as a stall then "operation canceled" around the
  # late `tests::symbols` group); batching resets the heap per process and
  # keeps coverage identical. The heavy typed-array and weak-collection groups
  # have their own stages and are excluded here.
  local batch_size=120
  local batch=()
  while IFS= read -r test_name; do
    batch+=("$test_name")
    if [ "${#batch[@]}" -ge "$batch_size" ]; then
      "$cargo_bin" test -p qjs-runtime -- --exact "${batch[@]}" || return "$?"
      batch=()
    fi
  done < <("$cargo_bin" test -p qjs-runtime -- --list 2>/dev/null \
    | sed -n 's/: test$//p' \
    | grep -Ev '^(typed_array::|weak_maps::|weak_refs::|weak_sets::)')
  if [ "${#batch[@]}" -gt 0 ]; then
    "$cargo_bin" test -p qjs-runtime -- --exact "${batch[@]}" || return "$?"
  fi
}

qjs_check_typed_array_tests() {
  local cargo_bin="$1"
  "$cargo_bin" test -p qjs-runtime typed_array::ordering_tests
  "$cargo_bin" test -p qjs-runtime typed_array::tests::indexed_tests
  while IFS= read -r test_name; do
    case "$test_name" in
      typed_array::tests::indexed_tests::*) continue ;;
    esac
    "$cargo_bin" test -p qjs-runtime "$test_name" -- --exact
  done < <("$cargo_bin" test -p qjs-runtime typed_array::tests:: -- --list \
    | sed -n 's/: test$//p')
}

qjs_check_stage "format" "$CARGO_BIN" fmt --all -- --check
qjs_check_stage "clippy" "$CARGO_BIN" clippy --workspace --all-targets -- -D warnings
# The opt-in `agents` feature (Test262 $262.agent multi-agent harness) compiles
# extra OS-thread code paths that the default workspace build never touches.
# Lint it explicitly so the gated code cannot bitrot.
qjs_check_stage "clippy (agents)" \
  "$CARGO_BIN" clippy -p qjs-runtime -p qjs-cli --features agents --all-targets -- -D warnings
# Run the agents-feature-only unit tests (Atomics/$262.agent behavior). The
# default workspace test stage never compiles these, so exercise them here.
qjs_check_stage "agents feature tests" \
  "$CARGO_BIN" test -p qjs-runtime --features agents atomics::
if [ "${QJS_CHECK_SPLIT_RUNTIME_TESTS:-0}" = "1" ]; then
  qjs_check_stage "non-runtime crate tests" \
    "$CARGO_BIN" test -p qjs-ast -p qjs-lexer -p qjs-parser -p qjs-cli
  qjs_check_stage "qjs-runtime core tests" \
    qjs_check_runtime_core_tests "$CARGO_BIN"
  qjs_check_stage "qjs-runtime weak map tests" \
    "$CARGO_BIN" test -p qjs-runtime weak_maps::
  qjs_check_stage "qjs-runtime weak ref tests" \
    "$CARGO_BIN" test -p qjs-runtime weak_refs::
  qjs_check_stage "qjs-runtime weak set tests" \
    "$CARGO_BIN" test -p qjs-runtime weak_sets::
  qjs_check_stage "qjs-runtime typed array tests" \
    qjs_check_typed_array_tests "$CARGO_BIN"
else
  qjs_check_stage "workspace tests" "$CARGO_BIN" test --workspace
fi
qjs_check_stage "benchmark tool tests" \
  env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR" \
  python3 -m unittest discover -s "$ROOT_DIR/tools/benchmark/tests" -v
qjs_check_stage "git hook worktree isolation" \
  "$ROOT_DIR/scripts/test-git-hooks.sh"
qjs_check_stage "benchmark shell syntax" \
  bash -n \
    "$ROOT_DIR/scripts/benchmark.sh" "$ROOT_DIR/scripts/benchmark-report.sh" \
    "$ROOT_DIR/scripts/resource-benchmark.sh" \
    "$ROOT_DIR/scripts/resource-benchmark-report.sh" \
    "$ROOT_DIR/scripts/lifecycle-bench.sh" \
    "$ROOT_DIR/scripts/external-corpus-audit.sh" \
    "$ROOT_DIR/scripts/performance-policy-audit.sh" \
    "$ROOT_DIR/scripts/performance-preview.sh" \
    "$ROOT_DIR/scripts/pre-push" "$ROOT_DIR/scripts/test-git-hooks.sh"
qjs_check_stage "file-size guard" "$ROOT_DIR/scripts/check-file-size.sh"
# Run the allowlisted Test262 subset so local checks gate the same suite CI
# runs; skip with QJS_CHECK_SKIP_TEST262=1 for doc-only or scripted loops.
if [ "${QJS_CHECK_SKIP_TEST262:-0}" != "1" ]; then
  export TEST262_CASE_TIMEOUT_SECONDS="${TEST262_CASE_TIMEOUT_SECONDS:-30}"
  qjs_check_stage "Test262 subset" "$ROOT_DIR/scripts/test262-subset.sh"
fi
