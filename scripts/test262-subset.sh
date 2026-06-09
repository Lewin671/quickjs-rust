#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST262_DIR="${TEST262_DIR:-$ROOT_DIR/third_party/test262}"
ALLOWLIST="${TEST262_ALLOWLIST:-$ROOT_DIR/tests/test262/allowlist.txt}"
EXPECTED_FAILURES="${TEST262_EXPECTED_FAILURES:-$ROOT_DIR/tests/test262/expected-failures.txt}"
LOCAL_CASE_DIR="${TEST262_LOCAL_CASE_DIR:-$ROOT_DIR/tests/test262}"
RUN_WITH_TIMEOUT="${RUN_WITH_TIMEOUT:-$ROOT_DIR/scripts/run-with-timeout.sh}"
METADATA_PARSER="$ROOT_DIR/scripts/test262-baseline-metadata.awk"
CASE_TIMEOUT_SECONDS="${TEST262_CASE_TIMEOUT_SECONDS:-10}"
CARGO_BIN="${CARGO:-cargo}"
if ! command -v "$CARGO_BIN" >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
  CARGO_BIN="$HOME/.cargo/bin/cargo"
fi

trim_ws() {
  local value="$1"
  value="${value#${value%%[![:space:]]*}}"
  value="${value%${value##*[![:space:]]}}"
  printf '%s' "$value"
}

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

if [ ! -f "$METADATA_PARSER" ]; then
  echo "error: missing $METADATA_PARSER" >&2
  exit 1
fi

if ! xargs -P 1 -n 1 true </dev/null >/dev/null 2>&1; then
  echo "error: xargs does not support -P; parallel Test262 subset execution is unavailable" >&2
  exit 1
fi

list_entries() {
  printf '%s\n' "$1" | tr -d '[]' | tr ',' '\n' \
    | sed 's/^[[:space:]]*//; s/[[:space:]]*$//' \
    | sed '/^$/d'
}
emit_test262_host_shim() {
  cat <<'EOF'
var $262 = {
  createRealm: function() {
    var crossRealmArray = function Array(length) {
      return arguments.length === 0 ? [] : new Array(length);
    };
    Object.defineProperty(crossRealmArray, "__quickjsRustCrossRealmArray", {
      value: true
    });
    var realmGlobal = Object.create(globalThis);
    realmGlobal.Array = crossRealmArray;
    return { global: realmGlobal };
  }
};
EOF
}

metadata_for() {
  awk -f "$METADATA_PARSER" "$1"
}

validate_includes() {
  local includes="$1"
  local include
  while IFS= read -r include; do
    if [ ! -f "$TEST262_DIR/harness/$include" ]; then
      echo "error: Test262 include does not exist: $include" >&2
      return 1
    fi
  done < <(list_entries "$includes")
}

case_path_for_entry() {
  local entry="$1"
  case "$entry" in
    cases/*)
      printf '%s\n' "$LOCAL_CASE_DIR/$entry"
      ;;
    test/*.js)
      printf '%s\n' "$TEST262_DIR/$entry"
      ;;
  esac
}

entry_label() {
  local entry="$1"
  case "$entry" in
    cases/*) printf 'tests/test262/%s\n' "$entry" ;;
    test/*.js) printf 'third_party/test262/%s\n' "$entry" ;;
  esac
}

validate_entry_path() {
  local entry="$1"
  case "$entry" in
    /*|*..*)
      echo "error: allowlist entry must be a relative path without '..': $entry" >&2
      return 1
      ;;
    cases/*|test/*.js)
      ;;
    *)
      echo "error: allowlist entry must start with cases/ or test/: $entry" >&2
      return 1
      ;;
  esac
}

validate_case_entry() {
  local entry="$1"
  local case_path
  local derived_from
  local flags
  local includes
  local features
  local negative_phase
  local negative_type

  validate_entry_path "$entry"
  case_path="$(case_path_for_entry "$entry")"
  if [ ! -f "$case_path" ]; then
    echo "error: allowlist entry does not exist: $(entry_label "$entry")" >&2
    return 1
  fi

  case "$entry" in
    cases/*)
      derived_from="$(sed -n 's#^// Derived from: ##p' "$case_path" | head -n 1)"
      if [ -z "$derived_from" ]; then
        echo "error: missing Test262 provenance in tests/test262/$entry" >&2
        return 1
      fi
      if [ ! -f "$TEST262_DIR/$derived_from" ]; then
        echo "error: derived Test262 source does not exist: $derived_from" >&2
        return 1
      fi
      ;;
    test/*.js)
      {
        read -r flags
        read -r includes
        read -r features
        read -r negative_phase
        read -r negative_type
      } < <(metadata_for "$case_path")
      if [[ "$flags" == *module* ]] || [[ "$flags" == *async* ]]; then
        echo "error: upstream subset entry uses unsupported flags ($flags): $entry" >&2
        return 1
      fi
      if [ -n "$negative_phase" ] || [ -n "$negative_type" ]; then
        echo "error: upstream negative Test262 entries are not supported by subset runner yet: $entry" >&2
        return 1
      fi
      validate_includes "$includes"
      ;;
  esac
}

make_upstream_case() {
  local source="$1"
  local output="$2"
  local flags="$3"
  local includes="$4"
  local include
  {
    if [[ "$flags" == *raw* ]]; then
      cat "$source"
    else
      if [[ "$flags" == *onlyStrict* ]]; then
        printf '"use strict";\n'
      fi
      cat "$TEST262_DIR/harness/assert.js"
      printf '\n'
      cat "$TEST262_DIR/harness/sta.js"
      printf '\n'
      emit_test262_host_shim
      printf '\n'
      while IFS= read -r include; do
        cat "$TEST262_DIR/harness/$include"
        printf '\n'
      done < <(list_entries "$includes")
      cat "$source"
    fi
  } >"$output"
}

allowlist_count=0
allowlist_entries=()
expected_failure_entries=()
while IFS= read -r line; do
  entry="${line%%#*}"
  entry="$(trim_ws "$entry")"
  [ -z "$entry" ] && continue

  allowlist_count=$((allowlist_count + 1))
  allowlist_entries+=("$entry")
  validate_case_entry "$entry"

done < "$ALLOWLIST"

entry_in_allowlist() {
  local wanted="$1"
  local allowlist_entry
  for allowlist_entry in "${allowlist_entries[@]}"; do
    if [ "$allowlist_entry" = "$wanted" ]; then
      return 0
    fi
  done
  return 1
}

entry_in_expected_failures() {
  local wanted="$1"
  local expected_failure_entry
  if [ "${#expected_failure_entries[@]}" -gt 0 ]; then
    for expected_failure_entry in "${expected_failure_entries[@]}"; do
      if [ "$expected_failure_entry" = "$wanted" ]; then
        return 0
      fi
    done
  fi
  return 1
}

while IFS= read -r line; do
  trimmed="$(trim_ws "$line")"
  [ -z "$trimmed" ] && continue
  case "$trimmed" in
    \#*) continue ;;
  esac

  if [[ "$line" != *"#"* ]]; then
    echo "error: expected failure is missing a reason: $line" >&2
    exit 1
  fi

  entry="${line%%#*}"
  entry="$(trim_ws "$entry")"
  if ! validate_case_entry "$entry"; then
    echo "error: expected failure entry does not exist: $(entry_label "$entry")" >&2
    exit 1
  fi
  if ! entry_in_allowlist "$entry"; then
    echo "error: expected failure entry is not in allowlist: $(entry_label "$entry")" >&2
    exit 1
  fi
  if entry_in_expected_failures "$entry"; then
    echo "error: duplicate expected failure entry: $(entry_label "$entry")" >&2
    exit 1
  fi

  expected_failure_entries+=("$entry")
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
EXPECTED_FAILURE_LIST="$RESULT_DIR/expected-failures.list"
if [ "${#expected_failure_entries[@]}" -gt 0 ]; then
  for entry in "${expected_failure_entries[@]}"; do
    printf '%s\n' "$entry"
  done
else
  true
fi >"$EXPECTED_FAILURE_LIST"

is_expected_failure() {
  local wanted="$1"
  grep -Fx -- "$wanted" "$EXPECTED_FAILURE_LIST" >/dev/null 2>&1
}

run_test262_case() {
  local current="$1"
  local entry="$2"
  local case_path
  local exec_path
  local log_path="$RESULT_DIR/$current.log"
  local status_path="$RESULT_DIR/$current.status"
  local result_path="$RESULT_DIR/$current.result"
  local flags
  local includes
  local features
  local negative_phase
  local negative_type
  local output
  local status

  printf 'test262 [%d/%d]: %s\n' "$current" "$ALLOWLIST_COUNT" "$entry"
  case_path="$(case_path_for_entry "$entry")"
  exec_path="$case_path"
  case "$entry" in
    test/*.js)
      {
        read -r flags
        read -r includes
        read -r features
        read -r negative_phase
        read -r negative_type
      } < <(metadata_for "$case_path")
      exec_path="$RESULT_DIR/$current.case.js"
      make_upstream_case "$case_path" "$exec_path" "$flags" "$includes"
      ;;
  esac
  set +e
  output="$("$RUN_WITH_TIMEOUT" "$CASE_TIMEOUT_SECONDS" "$QJS_CLI_BIN" "$exec_path" 2>&1)"
  status=$?
  set -e

  if [ "$status" -ne 0 ]; then
    if is_expected_failure "$entry"; then
      printf '%s\n' "xfail" >"$result_path"
    else
      printf '%s\n' "fail" >"$result_path"
    fi
    printf '%s\n' "$entry" >"$status_path"
    printf '%s\n' "$status" >>"$status_path"
    printf '%s\n' "$output" >"$log_path"
    return 0
  fi

  if is_expected_failure "$entry"; then
    printf '%s\n' "xpass" >"$result_path"
    printf '%s\n' "$entry" >"$status_path"
    printf '%s\n' 0 >>"$status_path"
  else
    printf '%s\n' "pass" >"$result_path"
  fi
}

export ALLOWLIST_COUNT="$allowlist_count"
export CASE_TIMEOUT_SECONDS
export EXPECTED_FAILURE_LIST
export LOCAL_CASE_DIR
export METADATA_PARSER
export QJS_CLI_BIN
export RESULT_DIR
export RUN_WITH_TIMEOUT
export TEST262_DIR
export -f is_expected_failure
export -f case_path_for_entry
export -f emit_test262_host_shim
export -f list_entries
export -f make_upstream_case
export -f metadata_for
export -f run_test262_case

set +e
for index in "${!allowlist_entries[@]}"; do
  current=$((index + 1))
  printf '%s\0%s\0' "$current" "${allowlist_entries[$index]}"
done | xargs -0 -n 2 -P "$TEST262_JOBS" bash -c 'run_test262_case "$1" "$2"' _
run_status=$?
set -e

if [ "$run_status" -ne 0 ]; then
  echo "error: Test262 subset runner failed before completing all cases" >&2
  exit "$run_status"
fi

first_status=""
first_status="$( (find "$RESULT_DIR" -name '*.result' -exec grep -l '^fail$' {} + || true) | sort | head -n 1)"

if [ -n "$first_status" ]; then
  failed_index="$(basename "$first_status" .result)"
  first_status="$RESULT_DIR/$failed_index.status"
fi

if [ -n "$first_status" ]; then
  failed_entry="$(sed -n '1p' "$first_status")"
  failed_status="$(sed -n '2p' "$first_status")"
  failed_index="$(basename "$first_status" .status)"
  failed_log="$RESULT_DIR/$failed_index.log"

  if [ "$failed_status" -eq 124 ]; then
    echo "error: Test262 case timed out after ${CASE_TIMEOUT_SECONDS}s: $(entry_label "$failed_entry")" >&2
  else
    echo "error: Test262 case failed: $(entry_label "$failed_entry")" >&2
  fi
  if [ -s "$failed_log" ]; then
    cat "$failed_log" >&2
  fi
  exit "$failed_status"
fi

first_status=""
first_status="$( (find "$RESULT_DIR" -name '*.result' -exec grep -l '^xpass$' {} + || true) | sort | head -n 1)"

if [ -n "$first_status" ]; then
  passed_index="$(basename "$first_status" .result)"
  first_status="$RESULT_DIR/$passed_index.status"
fi

if [ -n "$first_status" ]; then
  passed_entry="$(sed -n '1p' "$first_status")"
  echo "error: expected-failure case passed; remove it from tests/test262/expected-failures.txt: $(entry_label "$passed_entry")" >&2
  exit 1
fi

xfail_count="$( (find "$RESULT_DIR" -name '*.result' -exec grep -l '^xfail$' {} + || true) | wc -l | tr -d '[:space:]')"

if [ -z "$xfail_count" ]; then
  xfail_count=0
fi

if [ "$xfail_count" -gt 0 ]; then
  echo "ok: ran $allowlist_count Test262 subset cases ($xfail_count expected failures)"
else
  echo "ok: ran $allowlist_count Test262 subset cases"
fi

exit 0
