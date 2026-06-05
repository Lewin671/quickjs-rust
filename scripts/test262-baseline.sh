#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST262_DIR="$ROOT_DIR/third_party/test262"
RUN_WITH_TIMEOUT="$ROOT_DIR/scripts/run-with-timeout.sh"
METADATA_PARSER="$ROOT_DIR/scripts/test262-baseline-metadata.awk"
CASE_TIMEOUT_SECONDS="${TEST262_CASE_TIMEOUT_SECONDS:-10}"
RUN_LIMIT="${TEST262_BASELINE_LIMIT:-50}"
FILTER_PREFIX=""
CARGO_BIN="${CARGO:-cargo}"
usage() {
  cat >&2 <<'USAGE'
usage: scripts/test262-baseline.sh [--limit N | --all] [--filter test/<prefix>]

Enumerates the upstream Test262 tree, classifies cases the current harness
cannot model yet, and executes a bounded baseline sample through qjs-cli.
USAGE
}
while [ "$#" -gt 0 ]; do
  case "$1" in
    --all)
      RUN_LIMIT="all"
      shift
      ;;
    --limit)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      RUN_LIMIT="$2"
      shift 2
      ;;
    --filter)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      FILTER_PREFIX="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 2
      ;;
  esac
done
case "$RUN_LIMIT" in
  all) ;;
  ''|*[!0-9]*)
    echo "error: --limit must be a non-negative integer or --all: $RUN_LIMIT" >&2
    exit 2
    ;;
esac
if ! command -v "$CARGO_BIN" >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
  CARGO_BIN="$HOME/.cargo/bin/cargo"
fi

if [ ! -d "$TEST262_DIR/test" ]; then
  echo "error: missing $TEST262_DIR/test; run ./scripts/bootstrap.sh first" >&2
  exit 1
fi
if [ ! -x "$RUN_WITH_TIMEOUT" ]; then
  echo "error: missing executable $RUN_WITH_TIMEOUT" >&2
  exit 1
fi
echo "building qjs-cli for baseline"
build_output="$(mktemp "${TMPDIR:-/tmp}/qjs-test262-cargo-build-XXXXXX")"
set +e
"$CARGO_BIN" build -q --message-format=json-render-diagnostics -p qjs-cli > "$build_output"
build_status=$?
set -e
if [ "$build_status" -ne 0 ]; then
  cat "$build_output" >&2
  rm -f "$build_output"
  exit "$build_status"
fi
QJS_CLI_BIN="$(sed -n 's/.*"executable":"\([^"]*\)".*/\1/p' "$build_output" | tail -n 1)"
rm -f "$build_output"
target_dir="$($CARGO_BIN metadata --format-version=1 --no-deps | sed -n 's/.*\"target_directory\":\"\([^\"]*\)\".*/\1/p' | head -n 1)"
target_dir="${target_dir:-$ROOT_DIR/target}"
if [ -z "$QJS_CLI_BIN" ]; then
  QJS_CLI_BIN="$target_dir/debug/qjs"
fi

if [ ! -x "$QJS_CLI_BIN" ]; then
  echo "error: built qjs-cli binary is missing or not executable: $QJS_CLI_BIN" >&2
  exit 1
fi
metadata_for() {
  awk -f "$METADATA_PARSER" "$1"
}

skip_reason() {
  local rel="$1"
  local flags="$2"
  local includes="$3"
  local features="$4"
  local has_negative="$5"

  case "$rel" in
    *_FIXTURE.js) echo "fixture"; return ;;
    test/intl402/*|test/staging/intl402/*) echo "intl402"; return ;;
  esac

  if [ -n "$has_negative" ]; then
    echo "negative"
  elif [[ "$flags" == *module* ]]; then
    echo "module"
  elif [[ "$flags" == *async* ]]; then
    echo "async"
  elif [[ "$flags" == *raw* ]]; then
    echo "raw"
  elif [ -n "$includes" ]; then
    echo "includes"
  elif [ -n "$features" ]; then
    echo "features"
  else
    echo ""
  fi
}

make_case() {
  local source="$1"
  local output="$2"
  local flags="$3"
  {
    if [[ "$flags" == *onlyStrict* ]]; then
      printf '"use strict";\n'
    fi
    cat "$TEST262_DIR/harness/assert.js"
    printf '\n'
    cat "$TEST262_DIR/harness/sta.js"
    printf '\n'
    cat "$source"
  } > "$output"
}

run_case() {
  local file="$1"
  local flags="$2"
  local temp_dir
  local temp
  local output
  local status
  local first_line
  temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/qjs-test262-baseline-XXXXXX")"
  temp="$temp_dir/case.js"
  make_case "$file" "$temp" "$flags"
  set +e
  output="$($RUN_WITH_TIMEOUT "$CASE_TIMEOUT_SECONDS" "$QJS_CLI_BIN" "$temp" 2>&1)"
  status=$?
  set -e
  rm -rf "$temp_dir"
  if [ "$status" -eq 0 ]; then
    echo "pass"
  elif [ "$status" -eq 124 ]; then
    echo "timeout"
  else
    first_line="$(printf '%s\n' "$output" | sed -n '1p')"
    printf "fail\t%s\n" "$first_line"
  fi
}

total=0
eligible=0
run=0
pass=0
fail=0
timeout=0
skipped=0
skip_async=0
skip_features=0
skip_fixture=0
skip_includes=0
skip_intl402=0
skip_module=0
skip_negative=0
skip_raw=0

while IFS= read -r file; do
  rel="${file#"$TEST262_DIR/"}"
  if [ -n "$FILTER_PREFIX" ] && [[ "$rel" != "$FILTER_PREFIX"* ]]; then
    continue
  fi

  total=$((total + 1))
  {
    read -r flags
    read -r includes
    read -r features
    read -r has_negative
  } < <(metadata_for "$file")
  reason="$(skip_reason "$rel" "$flags" "$includes" "$features" "$has_negative")"
  if [ -n "$reason" ]; then
    skipped=$((skipped + 1))
    case "$reason" in
      async) skip_async=$((skip_async + 1)) ;;
      features) skip_features=$((skip_features + 1)) ;;
      fixture) skip_fixture=$((skip_fixture + 1)) ;;
      includes) skip_includes=$((skip_includes + 1)) ;;
      intl402) skip_intl402=$((skip_intl402 + 1)) ;;
      module) skip_module=$((skip_module + 1)) ;;
      negative) skip_negative=$((skip_negative + 1)) ;;
      raw) skip_raw=$((skip_raw + 1)) ;;
    esac
    continue
  fi

  eligible=$((eligible + 1))
  if [ "$RUN_LIMIT" != "all" ] && [ "$run" -ge "$RUN_LIMIT" ]; then
    continue
  fi

  run=$((run + 1))
  printf 'test262-baseline [%d]: %s\n' "$run" "$rel"
  result="$(run_case "$file" "$flags")"
  case "$result" in
    pass)
      pass=$((pass + 1))
      ;;
    timeout)
      timeout=$((timeout + 1))
      echo "timeout: $rel" >&2
      ;;
    fail*)
      fail=$((fail + 1))
      printf 'fail: %s\t%s\n' "$rel" "${result#fail	}" >&2
      ;;
  esac
done < <(find "$TEST262_DIR/test" -type f -name '*.js' | sort)
echo "summary:"
echo "  total: $total"
echo "  eligible: $eligible"
echo "  run: $run"
echo "  pass: $pass"
echo "  fail: $fail"
echo "  timeout: $timeout"
echo "  skipped: $skipped"
echo "  skipped.async: $skip_async"
echo "  skipped.features: $skip_features"
echo "  skipped.fixture: $skip_fixture"
echo "  skipped.includes: $skip_includes"
echo "  skipped.intl402: $skip_intl402"
echo "  skipped.module: $skip_module"
echo "  skipped.negative: $skip_negative"
echo "  skipped.raw: $skip_raw"
if [ "$fail" -ne 0 ] || [ "$timeout" -ne 0 ]; then
  exit 1
fi
