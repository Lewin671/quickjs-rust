#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"

if ! CARGO_BIN="$(qjs_resolve_cargo)"; then
  echo "error: cargo not found; install Rust with rustup before running checks" >&2
  exit 127
fi

"$CARGO_BIN" fmt --all -- --check
"$CARGO_BIN" clippy --workspace --all-targets -- -D warnings
if [ "${QJS_CHECK_SPLIT_RUNTIME_TESTS:-0}" = "1" ]; then
  "$CARGO_BIN" test -p qjs-ast -p qjs-lexer -p qjs-parser -p qjs-cli
  "$CARGO_BIN" test -p qjs-runtime -- --skip "typed_array::"
  "$CARGO_BIN" test -p qjs-runtime typed_array::ordering_tests
  "$CARGO_BIN" test -p qjs-runtime typed_array::tests::indexed_tests
  while IFS= read -r test_name; do
    case "$test_name" in
      typed_array::tests::indexed_tests::*) continue ;;
    esac
    "$CARGO_BIN" test -p qjs-runtime "$test_name" -- --exact
  done < <("$CARGO_BIN" test -p qjs-runtime typed_array::tests:: -- --list \
    | sed -n 's/: test$//p')
else
  "$CARGO_BIN" test --workspace
fi
"$ROOT_DIR/scripts/check-file-size.sh"
# Run the allowlisted Test262 subset so local checks gate the same suite CI
# runs; skip with QJS_CHECK_SKIP_TEST262=1 for doc-only or scripted loops.
if [ "${QJS_CHECK_SKIP_TEST262:-0}" != "1" ]; then
  TEST262_CASE_TIMEOUT_SECONDS="${TEST262_CASE_TIMEOUT_SECONDS:-30}" \
    "$ROOT_DIR/scripts/test262-subset.sh"
fi
