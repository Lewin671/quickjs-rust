#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"
TEST262_DIR="${TEST262_DIR:-$ROOT_DIR/third_party/test262}"
ALLOWLIST="${TEST262_ALLOWLIST:-$ROOT_DIR/tests/test262/allowlist.txt}"
EXPECTED_FAILURES="${TEST262_EXPECTED_FAILURES:-$ROOT_DIR/tests/test262/expected-failures.txt}"
LOCAL_CASE_DIR="${TEST262_LOCAL_CASE_DIR:-$ROOT_DIR/tests/test262}"
RUN_WITH_TIMEOUT="${RUN_WITH_TIMEOUT:-$ROOT_DIR/scripts/run-with-timeout.sh}"
METADATA_PARSER="$ROOT_DIR/scripts/test262-baseline-metadata.awk"
CASE_TIMEOUT_SECONDS="${TEST262_CASE_TIMEOUT_SECONDS:-10}"
if ! CARGO_BIN="$(qjs_resolve_cargo)"; then
  echo "error: cargo not found; install Rust with rustup before running the subset" >&2
  exit 127
fi

trim_ws() {
  local value="$1"
  value="${value#${value%%[![:space:]]*}}"
  value="${value%${value##*[![:space:]]}}"
  printf '%s' "$value"
}

TEST262_JOBS="${TEST262_JOBS:-$(qjs_detect_jobs)}"
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

# Splits a Test262 metadata list ("[a.js, b.js]" or "a.js, b.js") into the
# SPLIT_ENTRIES array without spawning a process. Harness file names never
# contain whitespace or glob characters.
split_entries() {
  local raw="$1"
  raw="${raw//[\[\],]/ }"
  SPLIT_ENTRIES=()
  local part
  for part in $raw; do
    SPLIT_ENTRIES+=("$part")
  done
}
emit_test262_host_shim() {
  cat <<'EOF'
var __quickjsRustDynamicFunctionRealm;
var $262 = {
  IsHTMLDDA: __quickjsRustIsHTMLDDA,
  detachArrayBuffer: __quickjsRustDetachArrayBuffer,
  evalScript: function(source) {
    return (0, eval)(source);
  },
  createRealm: function() {
    var crossRealmArray = function Array(length) {
      return arguments.length === 0 ? [] : globalThis.Array(length);
    };
    Object.defineProperty(crossRealmArray, "__quickjsRustCrossRealmArray", {
      value: true
    });
    var intrinsicGeneratorFunction = Object.getPrototypeOf(function* () {}).constructor;
    var realmGeneratorPrototype = Object.create(
      Object.getPrototypeOf((function* () {}).prototype)
    );
    var crossRealmGeneratorFunction = function GeneratorFunction() {
      var previousRealm = __quickjsRustDynamicFunctionRealm;
      __quickjsRustDynamicFunctionRealm = realmGlobal;
      try {
        var newTarget = new.target || crossRealmGeneratorFunction;
        var fn = intrinsicGeneratorFunction.apply(null, arguments);
        Object.setPrototypeOf(fn.prototype, realmGeneratorPrototype);
        var prototype = newTarget.prototype;
        if (prototype !== null && (typeof prototype === "object" || typeof prototype === "function")) {
          Object.setPrototypeOf(fn, prototype);
        } else {
          var fallback = newTarget.__quickjsRustRealmGeneratorFunctionPrototype;
          if (fallback !== undefined) {
            Object.setPrototypeOf(fn, fallback);
          }
        }
        return fn;
      } finally {
        __quickjsRustDynamicFunctionRealm = previousRealm;
      }
    };
    crossRealmGeneratorFunction.prototype = Object.create(
      Object.getPrototypeOf(function* () {})
    );
    Object.defineProperty(crossRealmGeneratorFunction.prototype, "constructor", {
      value: crossRealmGeneratorFunction,
      writable: false,
      enumerable: false,
      configurable: true
    });
    var crossRealmFunction = function Function() {
      var fn = globalThis.Function.apply(this, arguments);
      Object.defineProperty(fn, "__quickjsRustRealmArrayPrototype", {
        value: crossRealmArray.prototype
      });
      Object.defineProperty(fn, "__quickjsRustRealmGeneratorFunctionPrototype", {
        value: crossRealmGeneratorFunction.prototype
      });
      return fn;
    };
    var realmGlobal = Object.create(globalThis);
    realmGlobal.Array = crossRealmArray;
    realmGlobal.Function = crossRealmFunction;
    realmGlobal.eval = function(source) {
      var value = (0, eval)(source);
      if (typeof value === "function" && value.constructor === intrinsicGeneratorFunction) {
        Object.setPrototypeOf(value, crossRealmGeneratorFunction.prototype);
        Object.setPrototypeOf(value.prototype, realmGeneratorPrototype);
      }
      return value;
    };
    return { global: realmGlobal };
  }
};
EOF
}
emit_test262_assert_fast_paths() {
  cat <<'EOF'
assert.sameValue = __quickjsRustAssertSameValue;
EOF
}
emit_quickjs_rust_case_source() {
  sed 's/assert[.]sameValue(/__quickjsRustAssertSameValue(/g' "$1"
}
needs_test262_prelude() {
  local source="$1"
  local flags="$2"
  local includes="$3"
  if [[ "$flags" == *async* ]] || [ -n "$includes" ]; then
    return 0
  fi
  if grep -Eq '[$]262|Test262Error|assert[(]' "$source"; then
    return 0
  fi
  if sed 's/assert[.]sameValue//g' "$source" | grep -q 'assert[.]'; then
    return 0
  fi
  return 1
}

metadata_for() {
  awk -f "$METADATA_PARSER" "$1"
}

validate_includes() {
  local includes="$1"
  local include
  split_entries "$includes"
  for include in ${SPLIT_ENTRIES[@]+"${SPLIT_ENTRIES[@]}"}; do
    if [ ! -f "$TEST262_DIR/harness/$include" ]; then
      echo "error: Test262 include does not exist: $include" >&2
      return 1
    fi
  done
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

# Validates one allowlist entry and, for upstream entries, caches the parsed
# metadata under $RESULT_DIR/meta/<index> so the run phase does not parse it a
# second time. Runs in parallel via xargs.
validate_case_entry() {
  local current="$1"
  local entry="$2"
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
      metadata_for "$case_path" >"$RESULT_DIR/meta/$current"
      {
        read -r flags
        read -r includes
        read -r features
        read -r negative_phase
        read -r negative_type
      } <"$RESULT_DIR/meta/$current"
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
  if [[ "$flags" == *raw* ]]; then
    emit_quickjs_rust_case_source "$source" >"$output"
    return
  fi
  local parts=()
  if needs_test262_prelude "$source" "$flags" "$includes"; then
    parts+=("$PRELUDE_FILE")
  fi
  split_entries "$includes"
  for include in ${SPLIT_ENTRIES[@]+"${SPLIT_ENTRIES[@]}"}; do
    parts+=("$TEST262_DIR/harness/$include")
  done
  {
    if [[ "$flags" == *onlyStrict* ]]; then
      printf '"use strict";\n'
    fi
    # `awk 1` concatenates while normalizing a missing trailing newline, so
    # adjacent files never merge lines.
    if [ "${#parts[@]}" -ne 0 ]; then
      awk 1 "${parts[@]}"
    fi
    emit_quickjs_rust_case_source "$source"
  } | sed 's/assert[.]sameValue(/__quickjsRustAssertSameValue(/g' >"$output"
}

RESULT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/qjs-test262-subset-XXXXXX")"
trap 'rm -rf "$RESULT_DIR"' EXIT
mkdir -p "$RESULT_DIR/meta"

allowlist_count=0
allowlist_entries=()
while IFS= read -r line; do
  entry="${line%%#*}"
  entry="$(trim_ws "$entry")"
  [ -z "$entry" ] && continue

  allowlist_count=$((allowlist_count + 1))
  allowlist_entries+=("$entry")
done < "$ALLOWLIST"

if [ "$allowlist_count" -eq 0 ]; then
  echo "error: Test262 allowlist is empty; add at least one runnable subset case" >&2
  exit 1
fi

ALLOWLIST_FILE="$RESULT_DIR/allowlist.list"
printf '%s\n' "${allowlist_entries[@]}" >"$ALLOWLIST_FILE"

expected_failure_entries=()
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
  expected_failure_entries+=("$entry")
done < "$EXPECTED_FAILURES"

EXPECTED_FAILURE_SET=""
if [ "${#expected_failure_entries[@]}" -gt 0 ]; then
  EXPECTED_FAILURE_FILE="$RESULT_DIR/expected-failures.list"
  printf '%s\n' "${expected_failure_entries[@]}" >"$EXPECTED_FAILURE_FILE"

  # Batch membership and duplicate checks instead of per-entry array scans.
  missing="$(grep -Fxv -f "$ALLOWLIST_FILE" "$EXPECTED_FAILURE_FILE" || true)"
  if [ -n "$missing" ]; then
    while IFS= read -r entry; do
      echo "error: expected failure entry is not in allowlist: $(entry_label "$entry")" >&2
    done <<<"$missing"
    exit 1
  fi
  duplicates="$(sort "$EXPECTED_FAILURE_FILE" | uniq -d)"
  if [ -n "$duplicates" ]; then
    while IFS= read -r entry; do
      echo "error: duplicate expected failure entry: $(entry_label "$entry")" >&2
    done <<<"$duplicates"
    exit 1
  fi
  EXPECTED_FAILURE_SET="$(printf '%s\n' "${expected_failure_entries[@]}")"
fi

export LOCAL_CASE_DIR
export METADATA_PARSER
export RESULT_DIR
export TEST262_DIR
export -f case_path_for_entry
export -f entry_label
export -f metadata_for
export -f split_entries
export -f validate_case_entry
export -f validate_entry_path
export -f validate_includes

# Validate entries in parallel; expected-failure entries are a validated
# subset of the allowlist after the membership check above.
set +e
for index in "${!allowlist_entries[@]}"; do
  current=$((index + 1))
  printf '%s\0%s\0' "$current" "${allowlist_entries[$index]}"
done | xargs -0 -n 2 -P "$TEST262_JOBS" bash -c 'validate_case_entry "$1" "$2"' _
validate_status=$?
set -e
if [ "$validate_status" -ne 0 ]; then
  echo "error: Test262 allowlist validation failed" >&2
  exit 1
fi

echo "building qjs-cli for Test262 subset"
QJS_CLI_BIN="$(qjs_build_cli_bin "$CARGO_BIN")"

# Pre-concatenate the harness prelude shared by every non-raw upstream case.
PRELUDE_FILE="$RESULT_DIR/prelude.js"
{
  awk 1 "$TEST262_DIR/harness/assert.js"
  emit_test262_assert_fast_paths
  awk 1 "$TEST262_DIR/harness/sta.js"
  emit_test262_host_shim
} >"$PRELUDE_FILE"

is_expected_failure() {
  case $'\n'"$EXPECTED_FAILURE_SET"$'\n' in
    *$'\n'"$1"$'\n'*) return 0 ;;
  esac
  return 1
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
      } <"$RESULT_DIR/meta/$current"
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
export EXPECTED_FAILURE_SET
export PRELUDE_FILE
export QJS_CLI_BIN
export RUN_WITH_TIMEOUT
export -f emit_quickjs_rust_case_source
export -f is_expected_failure
export -f make_upstream_case
export -f needs_test262_prelude
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
