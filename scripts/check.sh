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
"$CARGO_BIN" test --workspace
"$ROOT_DIR/scripts/check-file-size.sh"
# Run the allowlisted Test262 subset so local checks gate the same suite CI
# runs; skip with QJS_CHECK_SKIP_TEST262=1 for doc-only or scripted loops.
if [ "${QJS_CHECK_SKIP_TEST262:-0}" != "1" ]; then
  TEST262_CASE_TIMEOUT_SECONDS="${TEST262_CASE_TIMEOUT_SECONDS:-30}" \
    "$ROOT_DIR/scripts/test262-subset.sh"
fi
