#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST262_DIR="$ROOT_DIR/third_party/test262"
QUICKJS_NG_DIR="$ROOT_DIR/third_party/quickjs-ng"
QUICKJS_NG_BIN="$QUICKJS_NG_DIR/build/qjs"
QUICKJS_NG_RUNNER="$QUICKJS_NG_DIR/build/run-test262"
RUN_WITH_TIMEOUT="$ROOT_DIR/scripts/run-with-timeout.sh"
METADATA_PARSER="$ROOT_DIR/scripts/test262-baseline-metadata.awk"
CASE_TIMEOUT_SECONDS="${TEST262_CASE_TIMEOUT_SECONDS:-10}"
RUN_LIMIT="${TEST262_BASELINE_LIMIT:-50}"
FILTER_PREFIX=""
ENGINE="quickjs-rust"
SUMMARY_JSON=""
NO_FAIL=0
SHARD_INDEX=1
SHARD_TOTAL=1
CARGO_BIN="${CARGO:-cargo}"

usage() {
  cat >&2 <<'USAGE'
usage: scripts/test262-baseline.sh [--limit N | --all] [--filter test/<prefix>] [--engine quickjs-rust|quickjs-ng|both] [--shard I/N] [--summary-json PATH] [--no-fail]
Enumerates upstream Test262 cases, classifies harness gaps, and executes a baseline sample.
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
    --engine)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      ENGINE="$2"
      shift 2
      ;;
    --shard)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      shard="$2"
      case "$shard" in
        */*)
          SHARD_INDEX="${shard%%/*}"
          SHARD_TOTAL="${shard##*/}"
          ;;
        *)
          echo "error: --shard must use I/N form: $shard" >&2
          exit 2
          ;;
      esac
      shift 2
      ;;
    --summary-json)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      SUMMARY_JSON="$2"
      shift 2
      ;;
    --no-fail)
      NO_FAIL=1
      shift
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
case "$ENGINE" in
  quickjs-rust|quickjs-ng|both) ;;
  *)
    echo "error: --engine must be quickjs-rust, quickjs-ng, or both: $ENGINE" >&2
    exit 2
    ;;
esac
case "$SHARD_INDEX:$SHARD_TOTAL" in
  *[!0-9:]*|0:*|*:0)
    echo "error: --shard must use positive integers: $SHARD_INDEX/$SHARD_TOTAL" >&2
    exit 2
    ;;
esac
if [ "$SHARD_INDEX" -gt "$SHARD_TOTAL" ]; then
  echo "error: --shard index must be <= shard total: $SHARD_INDEX/$SHARD_TOTAL" >&2
  exit 2
fi
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

needs_rust() {
  [ "$ENGINE" = "quickjs-rust" ] || [ "$ENGINE" = "both" ]
}

needs_quickjs_ng() {
  [ "$ENGINE" = "quickjs-ng" ] || [ "$ENGINE" = "both" ]
}

if needs_rust; then
  echo "building qjs-cli for baseline"
  build_output="$(mktemp "${TMPDIR:-/tmp}/qjs-test262-cargo-build-XXXXXX")"
  set +e
  "$CARGO_BIN" build -q --message-format=json-render-diagnostics -p qjs-cli >"$build_output"
  build_status=$?
  set -e
  if [ "$build_status" -ne 0 ]; then
    cat "$build_output" >&2
    rm -f "$build_output"
    exit "$build_status"
  fi
  QJS_CLI_BIN="$(sed -n 's/.*"executable":"\([^"]*\)".*/\1/p' "$build_output" | tail -n 1)"
  rm -f "$build_output"
  target_dir="$("$CARGO_BIN" metadata --format-version=1 --no-deps \
    | sed -n 's/.*\"target_directory\":\"\([^\"]*\)\".*/\1/p' \
    | head -n 1)"
  target_dir="${target_dir:-$ROOT_DIR/target}"
  if [ -z "$QJS_CLI_BIN" ]; then
    QJS_CLI_BIN="$target_dir/debug/qjs"
  fi
  if [ ! -x "$QJS_CLI_BIN" ]; then
    echo "error: built qjs-cli binary is missing or not executable: $QJS_CLI_BIN" >&2
    exit 1
  fi
fi

if needs_quickjs_ng; then
  if [ ! -d "$QUICKJS_NG_DIR" ]; then
    echo "error: missing $QUICKJS_NG_DIR; run ./scripts/bootstrap.sh first" >&2
    exit 1
  fi
  if [ ! -x "$QUICKJS_NG_BIN" ] || [ ! -x "$QUICKJS_NG_RUNNER" ]; then
    make -C "$QUICKJS_NG_DIR" all
  fi
  QUICKJS_NG_CONF="$(mktemp "${TMPDIR:-/tmp}/qjsng-test262-conf-XXXXXX")"
  QUICKJS_NG_FEATURES="$(mktemp "${TMPDIR:-/tmp}/qjsng-test262-features-XXXXXX")"
  QUICKJS_NG_SKIP_FEATURES="$(mktemp "${TMPDIR:-/tmp}/qjsng-test262-skip-features-XXXXXX")"
  QUICKJS_NG_EXCLUDES="$(mktemp "${TMPDIR:-/tmp}/qjsng-test262-excludes-XXXXXX")"
  trap 'rm -f "$QUICKJS_NG_CONF" "$QUICKJS_NG_FEATURES" "$QUICKJS_NG_SKIP_FEATURES" "$QUICKJS_NG_EXCLUDES"' EXIT
  sed \
    -e "s#^harnessdir=.*#harnessdir=$TEST262_DIR/harness#" \
    -e "s#^testdir=.*#testdir=$TEST262_DIR/test#" \
    -e 's#^errorfile=.*#errorfile=#' \
    "$QUICKJS_NG_DIR/test262.conf" >"$QUICKJS_NG_CONF"
  awk \
    -v features="$QUICKJS_NG_FEATURES" \
    -v skip_features="$QUICKJS_NG_SKIP_FEATURES" \
    -v excludes="$QUICKJS_NG_EXCLUDES" '
      /^\[features\]/{section="features"; next}
      /^\[exclude\]/{section="exclude"; next}
      /^\[/{section=""; next}
      section=="features" {
        line=$0
        sub(/#.*/, "", line)
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", line)
        if (line == "") next
        split(line, parts, "=")
        feature=parts[1]
        gsub(/[[:space:]]+$/, "", feature)
        print feature > features
        if (line ~ /=skip([[:space:]]*$|[[:space:]]+#)/) {
          print feature > skip_features
        }
      }
      section=="exclude" {
        line=$0
        sub(/#.*/, "", line)
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", line)
        if (line == "") next
        sub(/^test262\//, "", line)
        print line > excludes
      }
    ' "$QUICKJS_NG_CONF"
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
  elif [ -n "$includes" ] && ! rust_includes_supported "$includes"; then
    echo "includes"
  elif ! rust_source_syntax_supported "$rel"; then
    echo "features"
  elif [ -n "$features" ] && ! rust_features_supported "$features" "$rel"; then
    echo "features"
  else
    echo ""
  fi
}
rust_source_syntax_supported() {
  # Some upstream files use syntax the Rust parser does not support yet.
  ! grep -Eq '(^|[^[:alnum:]_$])[0-9][0-9_]*n([^[:alnum:]_$]|$)|`([^`\\]|\\.)*\$\{' "$TEST262_DIR/$1"
}
rust_features_supported() {
  local entries; entries="$(list_entries "$1")"
  [[ "${2:-}" == test/built-ins/RegExp/prototype/Symbol.split/* ]] && entries="$(printf '%s\n' "$entries" | grep -Fxv 'Symbol.species' || true)"; [[ "${2:-}" == test/built-ins/Array/prototype/with/* ]] && entries="$(printf '%s\n' "$entries" | grep -Fxv -e 'change-array-by-copy' -e 'exponentiation' || true)"
  [ -z "$entries" ] || ! grep -Fvx -e 'Symbol' -e 'Symbol.isConcatSpreadable' -e 'Symbol.match' \
    -e 'Symbol.replace' -e 'Symbol.search' -e 'Symbol.split' -e 'Symbol.toPrimitive' \
    -e 'Reflect.construct' -e 'arrow-function' \
    -e 'array-find-from-last' -e 'Array.prototype.at' -e 'Array.prototype.includes' -e 'Array.prototype.with' -e 'RegExp.escape' -e 'String.prototype.endsWith' -e 'String.prototype.includes' -e 'String.prototype.isWellFormed' -e 'String.prototype.replaceAll' -e 'String.prototype.toWellFormed' \
    <<<"$entries" >/dev/null
}
rust_includes_supported() {
  local include
  while IFS= read -r include; do
    [ -f "$TEST262_DIR/harness/$include" ] || return 1
  done < <(list_entries "$1")
}
list_entries() {
  printf '%s\n' "$1" | tr -d '[]' | tr ',' '\n' \
    | sed 's/^[[:space:]]*//; s/[[:space:]]*$//' \
    | sed '/^$/d'
}
prefix_list_contains() {
  local rel="$1"
  local list="$2"
  local prefix
  while IFS= read -r prefix; do
    [ -n "$prefix" ] || continue
    if [[ "$rel" == "$prefix"* ]]; then
      return 0
    fi
  done <"$list"
  return 1
}

quickjs_ng_skip_reason() {
  local rel="$1"
  local features="$2"
  local feature
  case "$rel" in
    *_FIXTURE.js) echo "fixture"; return ;;
  esac
  if prefix_list_contains "$rel" "$QUICKJS_NG_EXCLUDES"; then
    echo "exclude"
    return
  fi

  while IFS= read -r feature; do
    if grep -Fx -- "$feature" "$QUICKJS_NG_SKIP_FEATURES" >/dev/null 2>&1; then
      echo "feature"
      return
    fi
    if ! grep -Fx -- "$feature" "$QUICKJS_NG_FEATURES" >/dev/null 2>&1; then
      echo "unknown-feature"
      return
    fi
  done < <(list_entries "$features")
  echo ""
}
make_case() {
  local source="$1"
  local output="$2"
  local flags="$3"
  local includes="$4"
  local include
  {
    if [[ "$flags" == *onlyStrict* ]]; then
      printf '"use strict";\n'
    fi
    cat "$TEST262_DIR/harness/assert.js"
    printf '\n'
    cat "$TEST262_DIR/harness/sta.js"
    printf '\n'
    while IFS= read -r include; do
      cat "$TEST262_DIR/harness/$include"
      printf '\n'
    done < <(list_entries "$includes")
    cat "$source"
  } >"$output"
}
run_engine_case() {
  local engine="$1"
  local temp="$2"
  local source="$3"
  local output
  local status
  local first_line

  set +e
  case "$engine" in
    quickjs-rust) output="$("$RUN_WITH_TIMEOUT" "$CASE_TIMEOUT_SECONDS" "$QJS_CLI_BIN" "$temp" 2>&1)" ;;
    quickjs-ng) output="$("$RUN_WITH_TIMEOUT" "$CASE_TIMEOUT_SECONDS" "$QUICKJS_NG_RUNNER" -c "$QUICKJS_NG_CONF" -t 1 -f "$source" 2>&1)" ;;
  esac
  status=$?
  set -e

  if [ "$status" -eq 0 ]; then
    echo "pass"
  elif [ "$status" -eq 124 ]; then
    echo "timeout"
  else
    first_line="$(printf '%s\n' "$output" | sed -n '1p')"
    printf "fail\t%s\n" "$first_line"
  fi
}
count_engine_result() {
  local prefix="$1"
  local result="$2"
  case "$result" in
    pass) eval "${prefix}_pass=\$(( ${prefix}_pass + 1 ))" ;;
    timeout) eval "${prefix}_timeout=\$(( ${prefix}_timeout + 1 ))" ;;
    fail*) eval "${prefix}_fail=\$(( ${prefix}_fail + 1 ))" ;;
  esac
}
result_kind() {
  case "$1" in
    pass) echo "pass" ;;
    timeout) echo "timeout" ;;
    skipped) echo "skipped" ;;
    *) echo "fail" ;;
  esac
}
run_case() {
  local file="$1"
  local flags="$2"
  local rel="$3"
  local includes="$4"
  local rust_skip_reason="$5"
  local qjsng_skip_reason="$6"
  local temp_dir
  local temp
  local rust_result="not-run"
  local qjsng_result="not-run"
  temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/qjs-test262-baseline-XXXXXX")"
  temp="$temp_dir/case.js"
  make_case "$file" "$temp" "$flags" "$includes"
  if [ "$ENGINE" = "quickjs-rust" ] || [ "$ENGINE" = "both" ]; then
    if [ -n "$rust_skip_reason" ]; then
      rust_result="skipped"
      rust_skipped=$((rust_skipped + 1))
    else
      rust_result="$(run_engine_case quickjs-rust "$temp" "$file")"
      count_engine_result rust "$rust_result"
    fi
  fi
  if [ "$ENGINE" = "quickjs-ng" ] || [ "$ENGINE" = "both" ]; then
    if [ -n "$qjsng_skip_reason" ]; then
      qjsng_result="skipped"
      qjsng_skipped=$((qjsng_skipped + 1))
    else
      qjsng_result="$(run_engine_case quickjs-ng "$temp" "$file")"
      count_engine_result qjsng "$qjsng_result"
    fi
  fi
  rm -rf "$temp_dir"

  case "$rust_result" in
    skipped) echo "quickjs-rust skipped: $rel ($rust_skip_reason)" >&2 ;;
    timeout) echo "quickjs-rust timeout: $rel" >&2 ;;
    fail*) printf 'quickjs-rust fail: %s\t%s\n' "$rel" "${rust_result#fail	}" >&2 ;;
  esac
  case "$qjsng_result" in
    skipped) echo "quickjs-ng skipped: $rel ($qjsng_skip_reason)" >&2 ;;
    timeout) echo "quickjs-ng timeout: $rel" >&2 ;;
    fail*) printf 'quickjs-ng fail: %s\t%s\n' "$rel" "${qjsng_result#fail	}" >&2 ;;
  esac

  if [ "$ENGINE" = "both" ]; then
    rust_kind="$(result_kind "$rust_result")"
    qjsng_kind="$(result_kind "$qjsng_result")"
    if [ "$rust_kind" = "pass" ] && [ "$qjsng_kind" = "pass" ]; then
      both_pass=$((both_pass + 1))
    elif [ "$rust_kind" = "pass" ]; then
      rust_pass_qjsng_nonpass=$((rust_pass_qjsng_nonpass + 1))
    elif [ "$qjsng_kind" = "pass" ]; then
      qjsng_pass_rust_nonpass=$((qjsng_pass_rust_nonpass + 1))
      case "$rust_kind" in
        skipped) qjsng_pass_rust_harness_gap=$((qjsng_pass_rust_harness_gap + 1)) ;;
        timeout) qjsng_pass_rust_timeout=$((qjsng_pass_rust_timeout + 1)) ;;
        fail) qjsng_pass_rust_fail=$((qjsng_pass_rust_fail + 1)) ;;
      esac
    else
      both_nonpass=$((both_nonpass + 1))
      if [ "$rust_kind" != "skipped" ] && [ "$qjsng_kind" != "skipped" ]; then
        both_fail_or_timeout=$((both_fail_or_timeout + 1))
      fi
    fi
  fi
}

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

write_summary_json() {
  [ -n "$SUMMARY_JSON" ] || return
  mkdir -p "$(dirname "$SUMMARY_JSON")"
  cat >"$SUMMARY_JSON" <<JSON
{
  "engine": "$(json_escape "$ENGINE")",
  "filter": "$(json_escape "$FILTER_PREFIX")",
  "limit": "$(json_escape "$RUN_LIMIT")",
  "shard": {"index": $SHARD_INDEX, "total": $SHARD_TOTAL},
  "total": $total,
  "configured": $configured,
  "eligible": $eligible,
  "run": $run,
  "skipped": {
    "total": $skipped,
    "async": $skip_async,
    "features": $skip_features,
    "fixture": $skip_fixture,
    "includes": $skip_includes,
    "intl402": $skip_intl402,
    "module": $skip_module,
    "negative": $skip_negative,
    "raw": $skip_raw
  },
  "rust_harness_gap": $rust_harness_gap,
  "quickjs_rust": {"pass": $rust_pass, "fail": $rust_fail, "timeout": $rust_timeout, "skipped": $rust_skipped},
  "quickjs_ng": {"pass": $qjsng_pass, "fail": $qjsng_fail, "timeout": $qjsng_timeout, "skipped": $qjsng_skipped},
  "comparison": {
    "both_pass": $both_pass,
    "quickjs_ng_pass_rust_nonpass": $qjsng_pass_rust_nonpass,
    "quickjs_ng_pass_rust_harness_gap": $qjsng_pass_rust_harness_gap,
    "quickjs_ng_pass_rust_fail": $qjsng_pass_rust_fail,
    "quickjs_ng_pass_rust_timeout": $qjsng_pass_rust_timeout,
    "rust_pass_quickjs_ng_nonpass": $rust_pass_qjsng_nonpass,
    "both_nonpass": $both_nonpass,
    "both_fail_or_timeout": $both_fail_or_timeout
  }
}
JSON
}

scanned=0 total=0 configured=0 eligible=0 run=0 skipped=0
skip_async=0 skip_features=0 skip_fixture=0 skip_includes=0
skip_intl402=0 skip_module=0 skip_negative=0 skip_raw=0
rust_harness_gap=0 rust_pass=0 rust_fail=0 rust_timeout=0 rust_skipped=0
qjsng_pass=0 qjsng_fail=0 qjsng_timeout=0 qjsng_skipped=0
both_pass=0 qjsng_pass_rust_nonpass=0 rust_pass_qjsng_nonpass=0
qjsng_pass_rust_harness_gap=0 qjsng_pass_rust_fail=0 qjsng_pass_rust_timeout=0
both_nonpass=0 both_fail_or_timeout=0

while IFS= read -r file; do
  rel="${file#"$TEST262_DIR/"}"
  if [ -n "$FILTER_PREFIX" ] && [[ "$rel" != "$FILTER_PREFIX"* ]]; then
    continue
  fi

  scanned=$((scanned + 1))
  if [ $(( (scanned - 1) % SHARD_TOTAL + 1 )) -ne "$SHARD_INDEX" ]; then
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
  qjsng_reason=""
  if needs_quickjs_ng; then
    qjsng_reason="$(quickjs_ng_skip_reason "$rel" "$features")"
  fi

  if [ "$ENGINE" = "both" ] && [ -n "$qjsng_reason" ]; then
    skipped=$((skipped + 1))
    case "$qjsng_reason" in
      exclude) skip_features=$((skip_features + 1)) ;;
      feature|unknown-feature) skip_features=$((skip_features + 1)) ;;
      fixture) skip_fixture=$((skip_fixture + 1)) ;;
    esac
    continue
  fi

  if [ "$ENGINE" = "both" ]; then
    configured=$((configured + 1))
    if [ -n "$reason" ]; then
      rust_harness_gap=$((rust_harness_gap + 1))
    else
      eligible=$((eligible + 1))
    fi
  elif [ -n "$reason" ]; then
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
    if [ "$ENGINE" = "quickjs-rust" ]; then
      continue
    fi
  else
    eligible=$((eligible + 1))
  fi

  if [ "$RUN_LIMIT" != "all" ] && [ "$run" -ge "$RUN_LIMIT" ]; then
    continue
  fi

  run=$((run + 1))
  printf 'test262-baseline [%d]: %s\n' "$run" "$rel"
  run_case "$file" "$flags" "$rel" "$includes" "$reason" "$qjsng_reason"
done < <(find "$TEST262_DIR/test" -type f -name '*.js' | sort)

echo "summary:"
echo "  engine: $ENGINE"
echo "  shard: $SHARD_INDEX/$SHARD_TOTAL"
echo "  total: $total"
echo "  configured: $configured"
echo "  eligible: $eligible"
echo "  run: $run"
echo "  skipped: $skipped"
echo "  skipped.async: $skip_async"
echo "  skipped.features: $skip_features"
echo "  skipped.fixture: $skip_fixture"
echo "  skipped.includes: $skip_includes"
echo "  skipped.intl402: $skip_intl402"
echo "  skipped.module: $skip_module"
echo "  skipped.negative: $skip_negative"
echo "  skipped.raw: $skip_raw"
echo "  rust.harness_gap: $rust_harness_gap"
if needs_rust; then
  echo "  quickjs-rust.pass: $rust_pass"
  echo "  quickjs-rust.fail: $rust_fail"
  echo "  quickjs-rust.timeout: $rust_timeout"
  echo "  quickjs-rust.skipped: $rust_skipped"
fi
if needs_quickjs_ng; then
  echo "  quickjs-ng.pass: $qjsng_pass"
  echo "  quickjs-ng.fail: $qjsng_fail"
  echo "  quickjs-ng.timeout: $qjsng_timeout"
  echo "  quickjs-ng.skipped: $qjsng_skipped"
fi
if [ "$ENGINE" = "both" ]; then
  echo "  both.pass: $both_pass"
  echo "  quickjs-ng.pass.quickjs-rust.nonpass: $qjsng_pass_rust_nonpass"
  echo "  quickjs-rust.pass.quickjs-ng.nonpass: $rust_pass_qjsng_nonpass"
  echo "  both.nonpass: $both_nonpass"
fi

write_summary_json

if [ "$NO_FAIL" -eq 1 ]; then
  exit 0
fi
if [ "$ENGINE" = "quickjs-rust" ] && { [ "$rust_fail" -ne 0 ] || [ "$rust_timeout" -ne 0 ]; }; then
  exit 1
fi
if [ "$ENGINE" = "quickjs-ng" ] && { [ "$qjsng_fail" -ne 0 ] || [ "$qjsng_timeout" -ne 0 ]; }; then
  exit 1
fi
if [ "$ENGINE" = "both" ] && { [ "$rust_fail" -ne 0 ] || [ "$rust_timeout" -ne 0 ] || [ "$qjsng_fail" -ne 0 ] || [ "$qjsng_timeout" -ne 0 ]; }; then
  exit 1
fi
