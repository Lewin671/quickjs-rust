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

qjs_check_split_runtime_tests() {
  local cargo_bin="$1"
  "$cargo_bin" test -p qjs-runtime -- --skip "typed_array::"
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
if [ "${QJS_CHECK_SPLIT_RUNTIME_TESTS:-0}" = "1" ]; then
  qjs_check_stage "non-runtime crate tests" \
    "$CARGO_BIN" test -p qjs-ast -p qjs-lexer -p qjs-parser -p qjs-cli
  qjs_check_stage "qjs-runtime split tests" \
    qjs_check_split_runtime_tests "$CARGO_BIN"
else
  qjs_check_stage "workspace tests" "$CARGO_BIN" test --workspace
fi
qjs_check_stage "file-size guard" "$ROOT_DIR/scripts/check-file-size.sh"
# Run the allowlisted Test262 subset so local checks gate the same suite CI
# runs; skip with QJS_CHECK_SKIP_TEST262=1 for doc-only or scripted loops.
if [ "${QJS_CHECK_SKIP_TEST262:-0}" != "1" ]; then
  export TEST262_CASE_TIMEOUT_SECONDS="${TEST262_CASE_TIMEOUT_SECONDS:-30}"
  qjs_check_stage "Test262 subset" "$ROOT_DIR/scripts/test262-subset.sh"
fi
