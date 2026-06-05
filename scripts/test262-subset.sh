#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST262_DIR="$ROOT_DIR/third_party/test262"
ALLOWLIST="$ROOT_DIR/tests/test262/allowlist.txt"
EXPECTED_FAILURES="$ROOT_DIR/tests/test262/expected-failures.txt"
LOCAL_CASE_DIR="$ROOT_DIR/tests/test262"
RUN_WITH_TIMEOUT="$ROOT_DIR/scripts/run-with-timeout.sh"
CASE_TIMEOUT_SECONDS="${TEST262_CASE_TIMEOUT_SECONDS:-10}"
CARGO_BIN="${CARGO:-cargo}"
if ! command -v "$CARGO_BIN" >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
  CARGO_BIN="$HOME/.cargo/bin/cargo"
fi

detect_jobs() {
  local jobs
  jobs=""
  if command -v getconf >/dev/null 2>&1; then
    jobs="$(getconf _NPROCESSORS_ONLN 2>/dev/null || true)"
  fi
  case "$jobs" in
    ''|*[!0-9]*|0)
      if command -v sysctl >/dev/null 2>&1; then
        jobs="$(sysctl -n hw.ncpu 2>/dev/null || true)"
      fi
      ;;
  esac
  case "$jobs" in
    ''|*[!0-9]*|0) jobs=1 ;;
  esac
  echo "$jobs"
}

TEST262_JOBS="${TEST262_JOBS:-$(detect_jobs)}"
case "$TEST262_JOBS" in
  ''|*[!0-9]*)
    echo "error: TEST262_JOBS must be a positive integer: $TEST262_JOBS" >&2
    exit 2
    ;;
  0)
    echo "error: TEST262_JOBS must be greater than zero" >&2
    exit 2
    ;;
esac

if [ ! -d "$TEST262_DIR" ]; then
  echo "error: missing $TEST262_DIR; run ./scripts/bootstrap.sh first" >&2
  exit 1
fi

if [ ! -f "$ALLOWLIST" ]; then
  echo "error: missing $ALLOWLIST" >&2
  exit 1
fi

if [ ! -f "$EXPECTED_FAILURES" ]; then
  echo "error: missing $EXPECTED_FAILURES" >&2
  exit 1
fi

if [ ! -x "$RUN_WITH_TIMEOUT" ]; then
  echo "error: missing executable $RUN_WITH_TIMEOUT" >&2
  exit 1
fi

if ! xargs -P 1 -n 1 true </dev/null >/dev/null 2>&1; then
  echo "error: xargs does not support -P; parallel Test262 subset execution is unavailable" >&2
  exit 1
fi

allowlist_count=0
allowlist_entries=()
while IFS= read -r line; do
  entry="${line%%#*}"
  entry="$(echo "$entry" | xargs)"
  [ -z "$entry" ] && continue

  allowlist_count=$((allowlist_count + 1))
  allowlist_entries+=("$entry")
  case "$entry" in
    /*|*..*)
      echo "error: allowlist entry must be a relative path under tests/test262: $entry" >&2
      exit 1
      ;;
  esac

  case_path="$LOCAL_CASE_DIR/$entry"
  if [ ! -f "$case_path" ]; then
    echo "error: allowlist entry does not exist: tests/test262/$entry" >&2
    exit 1
  fi

  derived_from="$(sed -n 's#^// Derived from: ##p' "$case_path" | head -n 1)"
  if [ -z "$derived_from" ]; then
    echo "error: missing Test262 provenance in tests/test262/$entry" >&2
    exit 1
  fi
  if [ ! -f "$TEST262_DIR/$derived_from" ]; then
    echo "error: derived Test262 source does not exist: $derived_from" >&2
    exit 1
  fi

done < "$ALLOWLIST"

while IFS= read -r line; do
  trimmed="$(echo "$line" | xargs)"
  [ -z "$trimmed" ] && continue
  case "$trimmed" in
    \#*) continue ;;
  esac

  if [[ "$line" != *"#"* ]]; then
    echo "error: expected failure is missing a reason: $line" >&2
    exit 1
  fi

  entry="${line%%#*}"
  entry="$(echo "$entry" | xargs)"
  if [ ! -f "$LOCAL_CASE_DIR/$entry" ]; then
    echo "error: expected failure entry does not exist: tests/test262/$entry" >&2
    exit 1
  fi
done < "$EXPECTED_FAILURES"

if [ "$allowlist_count" -eq 0 ]; then
  echo "error: Test262 allowlist is empty; add at least one runnable subset case" >&2
  exit 1
fi

echo "building qjs-cli for Test262 subset"
"$CARGO_BIN" build -q -p qjs-cli

target_dir="$("$CARGO_BIN" metadata --format-version=1 --no-deps \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p' \
  | head -n 1)"
if [ -z "$target_dir" ]; then
  target_dir="$ROOT_DIR/target"
fi

QJS_CLI_BIN="$target_dir/debug/qjs"
if [ ! -x "$QJS_CLI_BIN" ]; then
  echo "error: built qjs-cli binary is missing or not executable: $QJS_CLI_BIN" >&2
  exit 1
fi

RESULT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/qjs-test262-subset-XXXXXX")"
trap 'rm -rf "$RESULT_DIR"' EXIT

run_test262_case() {
  local current="$1"
  local entry="$2"
  local case_path="$LOCAL_CASE_DIR/$entry"
  local log_path="$RESULT_DIR/$current.log"
  local status_path="$RESULT_DIR/$current.status"
  local output
  local status

  printf 'test262 [%d/%d]: %s\n' "$current" "$ALLOWLIST_COUNT" "$entry"
  set +e
  output="$("$RUN_WITH_TIMEOUT" "$CASE_TIMEOUT_SECONDS" "$QJS_CLI_BIN" "$case_path" 2>&1)"
  status=$?
  set -e

  if [ "$status" -ne 0 ]; then
    printf '%s\n' "$entry" >"$status_path"
    printf '%s\n' "$status" >>"$status_path"
    printf '%s\n' "$output" >"$log_path"
    exit "$status"
  fi
}

export ALLOWLIST_COUNT="$allowlist_count"
export CASE_TIMEOUT_SECONDS
export LOCAL_CASE_DIR
export QJS_CLI_BIN
export RESULT_DIR
export RUN_WITH_TIMEOUT
export -f run_test262_case

set +e
for index in "${!allowlist_entries[@]}"; do
  current=$((index + 1))
  printf '%s\0%s\0' "$current" "${allowlist_entries[$index]}"
done | xargs -0 -n 2 -P "$TEST262_JOBS" bash -c 'run_test262_case "$1" "$2"' _
run_status=$?
set -e

if [ "$run_status" -ne 0 ]; then
  first_status="$(find "$RESULT_DIR" -name '*.status' -print | sort | head -n 1)"
  if [ -z "$first_status" ]; then
    echo "error: Test262 subset failed before recording the failing case" >&2
    exit "$run_status"
  fi

  failed_entry="$(sed -n '1p' "$first_status")"
  failed_status="$(sed -n '2p' "$first_status")"
  failed_index="$(basename "$first_status" .status)"
  failed_log="$RESULT_DIR/$failed_index.log"

  if [ "$failed_status" -eq 124 ]; then
    echo "error: Test262 case timed out after ${CASE_TIMEOUT_SECONDS}s: tests/test262/$failed_entry" >&2
  else
    echo "error: Test262 case failed: tests/test262/$failed_entry" >&2
  fi
  if [ -s "$failed_log" ]; then
    cat "$failed_log" >&2
  fi
  exit "$failed_status"
fi

echo "ok: ran $allowlist_count Test262 subset cases"
