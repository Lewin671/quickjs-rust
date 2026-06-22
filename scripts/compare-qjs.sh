#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"
QJS_DIR="$ROOT_DIR/third_party/quickjs-ng"
QJS_BIN="$QJS_DIR/build/qjs"
FIXTURE_DIR="${1:-$ROOT_DIR/tests/fixtures/compare-qjs}"
CASE_TIMEOUT_SECONDS="${COMPARE_QJS_CASE_TIMEOUT_SECONDS:-10}"
COMPARE_QJS_JOBS="${COMPARE_QJS_JOBS:-$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 1)}"

case "$COMPARE_QJS_JOBS" in
  ''|*[!0-9]*)
    echo "error: COMPARE_QJS_JOBS must be a positive integer: $COMPARE_QJS_JOBS" >&2
    exit 2
    ;;
  0)
    echo "error: COMPARE_QJS_JOBS must be greater than zero" >&2
    exit 2
    ;;
esac

qjs_ensure_quickjs_ng
qjs_require_run_with_timeout

if ! CARGO_BIN="$(qjs_resolve_cargo)"; then
  echo "error: cargo not found; install Rust with rustup before comparing" >&2
  exit 127
fi

QJS_RUST_BIN="$(qjs_build_cli_bin "$CARGO_BIN")"

normalize_rust_value() {
  sed -E \
    -e 's/^Number\(([0-9]+)\.0\)$/\1/' \
    -e 's/^Number\(([-]?[0-9]+(\.[0-9]+)?)\)$/\1/' \
    -e 's/^String\("(.*)"\)$/\1/' \
    -e 's/^Boolean\((true|false)\)$/\1/' \
    -e 's/^Null$/null/' \
    -e 's/^Undefined$/undefined/'
}

run_fixture() {
  local index="$1"
  local fixture="$2"
  local expression rust_raw_output rust_status qjs_output qjs_status rust_output

  expression="$(tr '\n' ' ' < "$fixture")"
  set +e
  rust_raw_output="$("$RUN_WITH_TIMEOUT" "$CASE_TIMEOUT_SECONDS" "$QJS_RUST_BIN" --raw -e "$expression" 2>&1)"
  rust_status=$?
  set -e
  if [ "$rust_status" -ne 0 ]; then
    if [ "$rust_status" -eq 124 ]; then
      echo "error: quickjs-rust timed out after ${CASE_TIMEOUT_SECONDS}s: $fixture" >&2
    else
      echo "error: quickjs-rust failed for $fixture" >&2
    fi
    if [ -n "$rust_raw_output" ]; then
      echo "$rust_raw_output" >&2
    fi
    exit "$rust_status"
  fi

  set +e
  qjs_output="$("$RUN_WITH_TIMEOUT" "$CASE_TIMEOUT_SECONDS" "$QJS_BIN" -e "console.log($expression)" 2>&1)"
  qjs_status=$?
  set -e
  if [ "$qjs_status" -ne 0 ]; then
    if [ "$qjs_status" -eq 124 ]; then
      echo "error: quickjs-ng timed out after ${CASE_TIMEOUT_SECONDS}s: $fixture" >&2
    else
      echo "error: quickjs-ng failed for $fixture" >&2
    fi
    if [ -n "$qjs_output" ]; then
      echo "$qjs_output" >&2
    fi
    exit "$qjs_status"
  fi

  rust_output="$(printf '%s\n' "$rust_raw_output" | normalize_rust_value)"

  if [ "$rust_output" != "$qjs_output" ]; then
    echo "mismatch: $fixture" >&2
    echo "  quickjs-rust: $rust_output" >&2
    echo "  quickjs-ng:   $qjs_output" >&2
    exit 1
  fi

  if [ -n "${COMPARE_QJS_RESULTS_DIR:-}" ]; then
    echo "ok: $fixture => $rust_output" > "$COMPARE_QJS_RESULTS_DIR/$index.out"
  else
    echo "ok: $fixture => $rust_output"
  fi
}

shopt -s nullglob
fixtures=("$FIXTURE_DIR"/*.js)

if [ "${#fixtures[@]}" -eq 0 ]; then
  echo "error: no .js fixtures found in $FIXTURE_DIR" >&2
  exit 1
fi

if [ "$COMPARE_QJS_JOBS" -eq 1 ]; then
  for index in "${!fixtures[@]}"; do
    run_fixture "$index" "${fixtures[$index]}"
  done
  exit 0
fi

COMPARE_QJS_RESULTS_DIR="$(mktemp -d)"
export CASE_TIMEOUT_SECONDS COMPARE_QJS_RESULTS_DIR QJS_BIN QJS_RUST_BIN RUN_WITH_TIMEOUT
export -f normalize_rust_value run_fixture
trap 'rm -rf "$COMPARE_QJS_RESULTS_DIR"' EXIT

if ! {
  for index in "${!fixtures[@]}"; do
    printf '%s\0%s\0' "$index" "${fixtures[$index]}"
  done
} | xargs -0 -n 2 -P "$COMPARE_QJS_JOBS" bash -c 'run_fixture "$1" "$2"' _; then
  exit 1
fi

for index in "${!fixtures[@]}"; do
  cat "$COMPARE_QJS_RESULTS_DIR/$index.out"
done
