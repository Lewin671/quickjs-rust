#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"
TEST262_DIR="$ROOT_DIR/third_party/test262"
QUICKJS_NG_DIR="$ROOT_DIR/third_party/quickjs-ng"
QUICKJS_NG_RUNNER="$QUICKJS_NG_DIR/build/run-test262"
RUN_WITH_TIMEOUT="$ROOT_DIR/scripts/run-with-timeout.sh"
METADATA_PARSER="$ROOT_DIR/scripts/test262-baseline-metadata.awk"
CASE_TIMEOUT_SECONDS="${TEST262_CASE_TIMEOUT_SECONDS:-10}"
TIMEOUT_RETRIES="${TEST262_TIMEOUT_RETRIES:-0}"
RUN_LIMIT="${TEST262_BASELINE_LIMIT:-50}"
FILTER_PREFIX=""
ENGINE="quickjs-rust"
SUMMARY_JSON=""
CASE_RESULTS_JSONL=""
NO_FAIL=0
STOP_AFTER_LIMIT=0
SHARD_INDEX=1
SHARD_TOTAL=1

usage() {
  cat >&2 <<'USAGE'
usage: scripts/test262-baseline.sh [--limit N | --all] [--filter test/<prefix>] [--engine quickjs-rust|quickjs-ng|both] [--shard I/N] [--summary-json PATH] [--case-results-jsonl PATH] [--stop-after-limit] [--no-fail]
Enumerates upstream Test262 cases, classifies not-run cases, and executes a baseline sample.
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
    --case-results-jsonl)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      CASE_RESULTS_JSONL="$2"
      shift 2
      ;;
    --stop-after-limit)
      STOP_AFTER_LIMIT=1
      shift
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
case "$TIMEOUT_RETRIES" in
  ''|*[!0-9]*)
    echo "error: TEST262_TIMEOUT_RETRIES must be a non-negative integer: $TIMEOUT_RETRIES" >&2
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
TEST262_BASELINE_JOBS="${TEST262_BASELINE_JOBS:-$(qjs_detect_jobs)}"
case "$TEST262_BASELINE_JOBS" in
  ''|*[!0-9]*|0)
    echo "error: TEST262_BASELINE_JOBS must be a positive integer: $TEST262_BASELINE_JOBS" >&2
    exit 2
    ;;
esac
if [ ! -d "$TEST262_DIR/test" ]; then
  echo "error: missing $TEST262_DIR/test; run ./scripts/bootstrap.sh first" >&2
  exit 1
fi
if ! xargs -P 1 -n 1 true </dev/null >/dev/null 2>&1; then
  echo "error: xargs does not support -P; parallel Test262 baseline execution is unavailable" >&2
  exit 1
fi
qjs_require_run_with_timeout

# Scratch space for the parallel case runners: the work queue plus per-case
# result, JSONL, and message files keyed by queue index.
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/qjs-test262-baseline-work-XXXXXX")"
BASELINE_RUN_ID="$(basename "$WORK_DIR")"
QUEUE_FILE="$WORK_DIR/queue"
: >"$QUEUE_FILE"
cleanup() {
  find "$TEST262_DIR/test" -name ".qjs-baseline-case-$BASELINE_RUN_ID-*" -type f -delete
  rm -rf "$WORK_DIR"
  if [ -n "${QUICKJS_NG_CONF:-}" ]; then
    rm -f "$QUICKJS_NG_CONF" "$QUICKJS_NG_FEATURES" "$QUICKJS_NG_SKIP_FEATURES" "$QUICKJS_NG_EXCLUDES"
  fi
}
trap cleanup EXIT

needs_rust() {
  [ "$ENGINE" = "quickjs-rust" ] || [ "$ENGINE" = "both" ]
}

needs_quickjs_ng() {
  [ "$ENGINE" = "quickjs-ng" ] || [ "$ENGINE" = "both" ]
}

if needs_rust; then
  if ! CARGO_BIN="$(qjs_resolve_cargo)"; then
    echo "error: cargo not found; install Rust with rustup before running the baseline" >&2
    exit 127
  fi
  QJS_CLI_BIN="$(qjs_build_cli_bin "$CARGO_BIN")"
fi

if needs_quickjs_ng; then
  qjs_ensure_quickjs_ng "$QUICKJS_NG_RUNNER"
  QUICKJS_NG_CONF="$(mktemp "${TMPDIR:-/tmp}/qjsng-test262-conf-XXXXXX")"
  QUICKJS_NG_FEATURES="$(mktemp "${TMPDIR:-/tmp}/qjsng-test262-features-XXXXXX")"
  QUICKJS_NG_SKIP_FEATURES="$(mktemp "${TMPDIR:-/tmp}/qjsng-test262-skip-features-XXXXXX")"
  QUICKJS_NG_EXCLUDES="$(mktemp "${TMPDIR:-/tmp}/qjsng-test262-excludes-XXXXXX")"
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
  # Feature membership is checked per case feature; keep the lists in shell
  # strings so the checks are in-process matches instead of grep spawns.
  QUICKJS_NG_FEATURES_SET="$(cat "$QUICKJS_NG_FEATURES")"
  QUICKJS_NG_SKIP_FEATURES_SET="$(cat "$QUICKJS_NG_SKIP_FEATURES")"
fi

metadata_for() {
  awk -f "$METADATA_PARSER" "$1"
}
skip_reason() {
  local rel="$1"
  local flags="$2"
  local includes="$3"
  case "$rel" in
    *_FIXTURE.js) echo "fixture"; return ;;
    test/intl402/*|test/staging/intl402/*) echo "intl402"; return ;;
  esac
  if needs_agent_harness "$TEST262_DIR/$rel" "$flags" "$includes"; then echo "agent"; return; fi
  if [ -n "$includes" ] && ! rust_includes_supported "$includes"; then
    echo "includes"
  else
    echo ""
  fi
}
rust_includes_supported() {
  local include
  split_entries "$1"
  for include in ${SPLIT_ENTRIES[@]+"${SPLIT_ENTRIES[@]}"}; do
    [ -f "$TEST262_DIR/harness/$include" ] || return 1
  done
}
# Splits a Test262 metadata list ("[a.js, b.js]" or "a.js, b.js") into the
# SPLIT_ENTRIES array without spawning a process. Harness file and feature
# names never contain whitespace or glob characters.
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
    return (typeof __quickjsRustEvalScript === 'function') ? __quickjsRustEvalScript(source) : (0, eval)(source);
  },
  createRealm: function() {
    var crossRealmArray = function Array() {
      return Reflect.construct(globalThis.Array, globalThis.Array.prototype.slice.call(arguments), new.target || crossRealmArray);
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
      __quickjsRustDynamicFunctionRealm = __quickjsRustRealmGlobal;
      globalThis.__quickjsRustDynamicFunctionRealm = __quickjsRustRealmGlobal;
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
        globalThis.__quickjsRustDynamicFunctionRealm = previousRealm;
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
      var previousRealm = __quickjsRustDynamicFunctionRealm;
      __quickjsRustDynamicFunctionRealm = __quickjsRustRealmGlobal;
      globalThis.__quickjsRustDynamicFunctionRealm = __quickjsRustRealmGlobal;
      var newTarget = new.target || crossRealmFunction;
      try {
        var fn = globalThis.Function.apply(null, arguments);
      } finally {
        __quickjsRustDynamicFunctionRealm = previousRealm;
        globalThis.__quickjsRustDynamicFunctionRealm = previousRealm;
      }
      Object.setPrototypeOf(fn.prototype, crossRealmObject.prototype);
      var prototype = newTarget.prototype;
      if (prototype !== null && (typeof prototype === "object" || typeof prototype === "function")) {
        Object.setPrototypeOf(fn, prototype);
      } else {
        var fallback = newTarget.__quickjsRustRealmFunctionPrototype;
        if (fallback !== undefined) {
          Object.setPrototypeOf(fn, fallback);
        }
      }
      Object.defineProperty(fn, "__quickjsRustRealmObjectPrototype", {
        value: crossRealmObject.prototype
      });
      Object.defineProperty(fn, "__quickjsRustRealmFunctionPrototype", {
        value: crossRealmFunction.prototype
      });
      Object.defineProperty(fn, "__quickjsRustRealmArrayPrototype", {
        value: crossRealmArray.prototype
      });
      Object.defineProperty(fn, "__quickjsRustRealmRegExpPrototype", {
        value: crossRealmRegExpPrototype
      });
      [
        ["Boolean", "__quickjsRustRealmBooleanPrototype"],
        ["Number", "__quickjsRustRealmNumberPrototype"],
        ["String", "__quickjsRustRealmStringPrototype"],
        ["Date", "__quickjsRustRealmDatePrototype"],
        ["Map", "__quickjsRustRealmMapPrototype"],
        ["Set", "__quickjsRustRealmSetPrototype"],
        ["WeakMap", "__quickjsRustRealmWeakMapPrototype"],
        ["WeakSet", "__quickjsRustRealmWeakSetPrototype"]
      ].forEach(function(entry) {
        Object.defineProperty(fn, entry[1], {
          value: crossRealmBuiltinPrototypes[entry[0]]
        });
      });
      [
        ["Error", "__quickjsRustRealmErrorPrototype"],
        ["EvalError", "__quickjsRustRealmEvalErrorPrototype"],
        ["RangeError", "__quickjsRustRealmRangeErrorPrototype"],
        ["ReferenceError", "__quickjsRustRealmReferenceErrorPrototype"],
        ["SyntaxError", "__quickjsRustRealmSyntaxErrorPrototype"],
        ["TypeError", "__quickjsRustRealmTypeErrorPrototype"],
        ["URIError", "__quickjsRustRealmURIErrorPrototype"],
        ["SuppressedError", "__quickjsRustRealmSuppressedErrorPrototype"]
      ].forEach(function(entry) {
        Object.defineProperty(fn, entry[1], {
          value: crossRealmNativeErrorPrototypes[entry[0]]
        });
      });
      Object.defineProperty(fn, "__quickjsRustRealmGeneratorFunctionPrototype", {
        value: crossRealmGeneratorFunction.prototype
      });
      return fn;
    };
    var __quickjsRustRealmGlobal = Object.create(globalThis);
    var crossRealmObject = function Object(value) {
      return globalThis.Object(value);
    };
    var crossRealmObjectPrototype = Object.create(globalThis.Object.prototype);
    Object.defineProperty(crossRealmObjectPrototype, "constructor", {
      value: crossRealmObject,
      writable: true,
      enumerable: false,
      configurable: true
    });
    crossRealmObject.prototype = crossRealmObjectPrototype;
    var crossRealmFunctionPrototype = function() {};
    Object.defineProperty(crossRealmFunctionPrototype, "constructor", {
      value: crossRealmFunction,
      writable: true,
      enumerable: false,
      configurable: true
    });
    crossRealmFunction.prototype = crossRealmFunctionPrototype;
    var crossRealmRegExp = function RegExp() {
      return Reflect.construct(globalThis.RegExp, globalThis.Array.prototype.slice.call(arguments), new.target || crossRealmRegExp);
    };
    var crossRealmRegExpPrototype = Object.create(globalThis.RegExp.prototype);
    Object.defineProperty(crossRealmRegExpPrototype, "constructor", {
      value: crossRealmRegExp,
      writable: true,
      enumerable: false,
      configurable: true
    });
    [
      "source",
      "flags",
      "global",
      "ignoreCase",
      "multiline",
      "dotAll",
      "unicode",
      "sticky",
      "hasIndices",
      "unicodeSets"
    ].forEach(function(name) {
      var descriptor = Object.getOwnPropertyDescriptor(globalThis.RegExp.prototype, name);
      if (!descriptor || typeof descriptor.get !== "function") return;
      Object.defineProperty(crossRealmRegExpPrototype, name, {
        get: function() {
          if (this === crossRealmRegExpPrototype) {
            if (name === "source") return "(?:)";
            if (name === "flags") return "";
            return undefined;
          }
          if (this === globalThis.RegExp.prototype) {
            throw new __quickjsRustRealmGlobal.TypeError("RegExp prototype accessor requires a RegExp receiver");
          }
          return descriptor.get.call(this);
        },
        enumerable: descriptor.enumerable,
        configurable: descriptor.configurable
      });
    });
    crossRealmRegExp.prototype = crossRealmRegExpPrototype;
    if (typeof globalThis.RegExp.escape === "function") {
      Object.defineProperty(crossRealmRegExp, "escape", {
        value: globalThis.RegExp.escape,
        writable: true,
        enumerable: false,
        configurable: true
      });
    }
    var crossRealmSymbol = function Symbol(description) {
      return globalThis.Symbol(description);
    };
    var crossRealmSymbolPrototype = Object.create(globalThis.Symbol.prototype);
    Object.defineProperty(crossRealmSymbolPrototype, "constructor", {
      value: crossRealmSymbol,
      writable: true,
      enumerable: false,
      configurable: true
    });
    crossRealmSymbol.prototype = crossRealmSymbolPrototype;
    Object.defineProperty(crossRealmSymbol, "for", {
      value: function for_(key) {
        return globalThis.Symbol.for(key);
      },
      writable: true,
      enumerable: false,
      configurable: true
    });
    Object.defineProperty(crossRealmSymbol, "keyFor", {
      value: function keyFor(symbol) {
        return globalThis.Symbol.keyFor(symbol);
      },
      writable: true,
      enumerable: false,
      configurable: true
    });
    [
      "asyncIterator",
      "hasInstance",
      "isConcatSpreadable",
      "iterator",
      "match",
      "matchAll",
      "replace",
      "search",
      "species",
      "split",
      "toPrimitive",
      "toStringTag",
      "unscopables",
      "dispose",
      "asyncDispose"
    ].forEach(function(name) {
      var descriptor = Object.getOwnPropertyDescriptor(globalThis.Symbol, name);
      if (descriptor) {
        Object.defineProperty(crossRealmSymbol, name, descriptor);
      }
    });
    var crossRealmBuiltinConstructors = {};
    var crossRealmBuiltinPrototypes = {};
    [
      "Boolean",
      "Number",
      "String",
      "Date",
      "Map",
      "Set",
      "WeakMap",
      "WeakSet"
    ].forEach(function(name) {
      var constructor = function Builtin() {
        return Reflect.construct(
          globalThis[name],
          globalThis.Array.prototype.slice.call(arguments),
          new.target || constructor
        );
      };
      var prototype = Object.create(globalThis[name].prototype);
      Object.defineProperty(prototype, "constructor", {
        value: constructor,
        writable: true,
        enumerable: false,
        configurable: true
      });
      constructor.prototype = prototype;
      if (name === "String") {
        ["toString", "valueOf"].forEach(function(method) {
          Object.defineProperty(prototype, method, {
            value: function() {
              try {
                return globalThis.String.prototype[method].call(this);
              } catch (error) {
                throw new __quickjsRustRealmGlobal.TypeError(error && error.message);
              }
            },
            writable: true,
            enumerable: false,
            configurable: true
          });
        });
      }
      crossRealmBuiltinConstructors[name] = constructor;
      crossRealmBuiltinPrototypes[name] = prototype;
    });
    var crossRealmNativeErrors = {};
    var crossRealmNativeErrorPrototypes = {};
    [
      "Error",
      "EvalError",
      "RangeError",
      "ReferenceError",
      "SyntaxError",
      "TypeError",
      "URIError",
      "SuppressedError"
    ].forEach(function(name) {
      var constructor = function NativeError() {
        return Reflect.construct(
          globalThis[name],
          globalThis.Array.prototype.slice.call(arguments),
          new.target || constructor
        );
      };
      var prototype = Object.create(globalThis[name].prototype);
      Object.defineProperty(prototype, "constructor", {
        value: constructor,
        writable: true,
        enumerable: false,
        configurable: true
      });
      constructor.prototype = prototype;
      crossRealmNativeErrors[name] = constructor;
      crossRealmNativeErrorPrototypes[name] = prototype;
    });
    __quickjsRustRealmGlobal.Object = crossRealmObject;
    __quickjsRustRealmGlobal.Array = crossRealmArray;
    __quickjsRustRealmGlobal.Function = crossRealmFunction;
    __quickjsRustRealmGlobal.RegExp = crossRealmRegExp;
    __quickjsRustRealmGlobal.Symbol = crossRealmSymbol;
    __quickjsRustRealmGlobal.Boolean = crossRealmBuiltinConstructors.Boolean;
    __quickjsRustRealmGlobal.Number = crossRealmBuiltinConstructors.Number;
    __quickjsRustRealmGlobal.String = crossRealmBuiltinConstructors.String;
    __quickjsRustRealmGlobal.Date = crossRealmBuiltinConstructors.Date;
    __quickjsRustRealmGlobal.Map = crossRealmBuiltinConstructors.Map;
    __quickjsRustRealmGlobal.Set = crossRealmBuiltinConstructors.Set;
    __quickjsRustRealmGlobal.WeakMap = crossRealmBuiltinConstructors.WeakMap;
    __quickjsRustRealmGlobal.WeakSet = crossRealmBuiltinConstructors.WeakSet;
    __quickjsRustRealmGlobal.Error = crossRealmNativeErrors.Error;
    __quickjsRustRealmGlobal.EvalError = crossRealmNativeErrors.EvalError;
    __quickjsRustRealmGlobal.RangeError = crossRealmNativeErrors.RangeError;
    __quickjsRustRealmGlobal.ReferenceError = crossRealmNativeErrors.ReferenceError;
    __quickjsRustRealmGlobal.SyntaxError = crossRealmNativeErrors.SyntaxError;
    __quickjsRustRealmGlobal.TypeError = crossRealmNativeErrors.TypeError;
    __quickjsRustRealmGlobal.URIError = crossRealmNativeErrors.URIError;
    __quickjsRustRealmGlobal.SuppressedError = crossRealmNativeErrors.SuppressedError;
    __quickjsRustRealmGlobal.globalThis = __quickjsRustRealmGlobal;
    __quickjsRustRealmGlobal.eval = function(source) {
      var value = (0, eval)(source);
      if (typeof value === "function" && value.constructor === intrinsicGeneratorFunction) {
        Object.setPrototypeOf(value, crossRealmGeneratorFunction.prototype);
        Object.setPrototypeOf(value.prototype, realmGeneratorPrototype);
      }
      return value;
    };
    return { global: __quickjsRustRealmGlobal };
  }
};
EOF
}
emit_test262_assert_fast_paths() {
  cat <<'EOF'
assert.sameValue = __quickjsRustAssertSameValue;
EOF
}
should_emit_test262_assert_fast_paths() {
  case "$1" in
    "$TEST262_DIR/test/harness/"*) return 1 ;;
    *) return 0 ;;
  esac
}
emit_quickjs_rust_case_source() {
  cat "$1"
}
harness_include_uses() {
  local includes="$1" pattern="$2" include
  split_entries "$includes"
  for include in ${SPLIT_ENTRIES[@]+"${SPLIT_ENTRIES[@]}"}; do
    grep -Eq "$pattern" "$TEST262_DIR/harness/$include" && return 0
  done
  return 1
}
needs_assert_prelude() {
  local source="$1" flags="$2" includes="$3"
  local assert_helper_pattern='(^|[^A-Za-z0-9_$])(compareArray|formatIdentityFreeValue|formatSimpleValue|isNegativeZero|isPrimitive)([^A-Za-z0-9_$]|$)'
  [[ "$flags" == *async* ]] && return 0
  grep -Eq 'assert[(]' "$source" && return 0
  grep -q 'assert[.]sameValue' "$source" && return 0
  sed 's/assert[.]sameValue//g' "$source" | grep -q 'assert[.]' && return 0
  grep -Eq "$assert_helper_pattern" "$source" && return 0
  [[ " $includes " == *" compareArray.js "* ]] && return 0
  harness_include_uses "$includes" "assert[.(]|$assert_helper_pattern"
}
needs_sta_prelude() {
  local source="$1" flags="$2" includes="$3"
  needs_assert_prelude "$source" "$flags" "$includes" && return 0
  grep -Eq 'Test262Error|[$]DONOTEVALUATE' "$source" || harness_include_uses "$includes" 'Test262Error|[$]DONOTEVALUATE'
}
needs_host_prelude() { local source="$1" includes="$2"; grep -q '[$]262' "$source" || harness_include_uses "$includes" '[$]262'; }
needs_agent_harness() {
  local source="$1" flags="$2" includes="$3"
  # A test needs the (unsupported) multi-agent / can't-block harness when it
  # drives a second agent (`$262.agent` / `atomicsHelper.js`) or requires a
  # `[[CanBlock]] == false` agent (`CanBlockIsFalse`, where `Atomics.wait` must
  # throw). The `CanBlockIsTrue` flag does NOT need it: this engine's single
  # agent can block, so standalone `Atomics.wait` returns "timed-out"/"not-equal"
  # per spec and those tests run.
  [[ "$flags" == *CanBlockIsFalse* ]] || [[ " $includes " == *" atomicsHelper.js "* ]] || grep -q '[$]262[.]agent' "$source" || harness_include_uses "$includes" '[$]262[.]agent'
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

list_test262_cases() {
  if [ -n "$FILTER_PREFIX" ]; then
    if [ -f "$TEST262_DIR/$FILTER_PREFIX" ]; then
      printf '%s\n' "$TEST262_DIR/$FILTER_PREFIX"
      return
    fi
    if [ -d "$TEST262_DIR/$FILTER_PREFIX" ]; then
      find "$TEST262_DIR/$FILTER_PREFIX" -type f -name '*.js' ! -name '.qjs-baseline-case-*.js' | sort
      return
    fi
  fi
  find "$TEST262_DIR/test" -type f -name '*.js' ! -name '.qjs-baseline-case-*.js' | sort
}

line_set_contains() {
  case $'\n'"$1"$'\n' in
    *$'\n'"$2"$'\n'*) return 0 ;;
  esac
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

  split_entries "$features"
  for feature in ${SPLIT_ENTRIES[@]+"${SPLIT_ENTRIES[@]}"}; do
    if line_set_contains "$QUICKJS_NG_SKIP_FEATURES_SET" "$feature"; then
      echo "feature"
      return
    fi
    if ! line_set_contains "$QUICKJS_NG_FEATURES_SET" "$feature"; then
      echo "unknown-feature"
      return
    fi
  done
  echo ""
}
make_case() {
  local source="$1"
  local output="$2"
  local flags="$3"
  local includes="$4"
  local include
  {
    if [[ "$flags" == *raw* ]]; then
      emit_quickjs_rust_case_source "$source"
    else
      if [[ "$flags" == *onlyStrict* ]]; then
        printf '"use strict";\n'
      fi
      if needs_assert_prelude "$source" "$flags" "$includes"; then
        cat "$TEST262_DIR/harness/assert.js"
        printf '\n'
        if should_emit_test262_assert_fast_paths "$source"; then
          emit_test262_assert_fast_paths
          printf '\n'
        fi
      fi
      needs_sta_prelude "$source" "$flags" "$includes" && { cat "$TEST262_DIR/harness/sta.js"; printf '\n'; }
      if needs_host_prelude "$source" "$includes"; then
        emit_test262_host_shim
        printf '\n'
      fi
      if [[ "$flags" == *async* ]]; then
        # Async cases report completion through $DONE; include the upstream
        # handler (like quickjs-ng's run-test262 does) so it can print the
        # AsyncTestComplete / AsyncTestFailure markers the wrapper judges by.
        cat "$TEST262_DIR/harness/doneprintHandle.js"
        printf '\n'
      fi
      split_entries "$includes"
      for include in ${SPLIT_ENTRIES[@]+"${SPLIT_ENTRIES[@]}"}; do
        cat "$TEST262_DIR/harness/$include"
        printf '\n'
      done
      emit_quickjs_rust_case_source "$source"
    fi
  } >"$output"
}
# Builds the module-scope prelude file for a module-flagged case: the same
# harness includes make_case prepends to a script, but written to a standalone
# file. The prelude is SCRIPT code (sta.js plus selective assert.js/host shim/
# $DONE handler + requested includes); the test file itself is evaluated under
# the Module goal.
# Module bodies are always strict, so onlyStrict needs no directive here.
make_module_prelude() {
  local output="$1"
  local source="$2"
  local flags="$3"
  local includes="$4"
  local include
  {
    if needs_assert_prelude "$source" "$flags" "$includes"; then
      cat "$TEST262_DIR/harness/assert.js"
      printf '\n'
      if should_emit_test262_assert_fast_paths "$source"; then
        emit_test262_assert_fast_paths
        printf '\n'
      fi
    fi
    needs_sta_prelude "$source" "$flags" "$includes" && { cat "$TEST262_DIR/harness/sta.js"; printf '\n'; }
    if needs_host_prelude "$source" "$includes"; then
      emit_test262_host_shim
      printf '\n'
    fi
    if [[ "$flags" == *async* ]]; then
      cat "$TEST262_DIR/harness/doneprintHandle.js"
      printf '\n'
    fi
    split_entries "$includes"
    for include in ${SPLIT_ENTRIES[@]+"${SPLIT_ENTRIES[@]}"}; do
      cat "$TEST262_DIR/harness/$include"
      printf '\n'
    done
  } >"$output"
}
rust_error_field() {
  local field="$1"
  local output="$2"
  printf '%s\n' "$output" | sed -n "s/^error: .*${field}=\([^ ]*\).*/\1/p" | head -n 1
}
rust_negative_matches() {
  local output="$1" phase="$2" type="$3"
  local kind actual_type
  kind="$(rust_error_field kind "$output")"
  actual_type="$(rust_error_field type "$output")"

  case "$phase" in
    parse)
      [ "$kind" = "parse" ] || return 1
      ;;
    early)
      [ "$kind" = "parse" ] || [ "$kind" = "early" ] || return 1
      ;;
    runtime)
      [ "$kind" = "runtime" ] || return 1
      ;;
    resolution)
      # Module instantiation/resolution SyntaxErrors (unresolvable imports,
      # ambiguous star exports) surface as early (link-phase) errors here.
      [ "$kind" = "parse" ] || [ "$kind" = "early" ] || [ "$kind" = "runtime" ] || return 1
      ;;
    "")
      ;;
    *)
      return 1
      ;;
  esac

  [ -z "$type" ] || [ "$actual_type" = "$type" ]
}
run_engine_case() {
  local engine="$1" temp="$2" source="$3"
  local negative_phase="${4:-}" negative_type="${5:-}" is_async="${6:-}"
  local module_prelude="${7:-}"
  local output status first_line attempt

  attempt=0
  while :; do
    set +e
    case "$engine" in
      quickjs-rust)
        if [ -n "$module_prelude" ]; then
          # Module-flagged case: run the test file under the Module goal with the
          # harness includes installed as a script prelude. The engine reads the
          # original test file so relative imports resolve against its directory.
          output="$("$RUN_WITH_TIMEOUT" "$CASE_TIMEOUT_SECONDS" "$QJS_CLI_BIN" --error-format=test262 --module --prelude "$module_prelude" "$source" 2>&1)"
        else
          output="$("$RUN_WITH_TIMEOUT" "$CASE_TIMEOUT_SECONDS" "$QJS_CLI_BIN" --error-format=test262 "$temp" 2>&1)"
        fi
        ;;
      quickjs-ng) output="$("$RUN_WITH_TIMEOUT" "$CASE_TIMEOUT_SECONDS" "$QUICKJS_NG_RUNNER" -c "$QUICKJS_NG_CONF" -t 1 -f "$source" 2>&1)" ;;
    esac
    status=$?
    set -e
    if [ "$status" -ne 124 ] || [ "$attempt" -ge "$TIMEOUT_RETRIES" ]; then
      break
    fi
    attempt=$((attempt + 1))
  done

  if [ "$status" -eq 0 ]; then
    if [ "$engine" = "quickjs-rust" ] && [ -n "$negative_phase" ]; then
      printf "fail\texpected negative %s%s\n" "$negative_phase" "${negative_type:+ $negative_type}"
      return
    fi
    # Positive async cases report completion through the $DONE channel: the
    # script and its drained jobs print exactly one marker line. Judge by that
    # marker, not just a clean exit, so a script that never calls $DONE fails.
    if [ "$engine" = "quickjs-rust" ] && [ -n "$is_async" ] && [ -z "$negative_phase" ]; then
      if printf '%s\n' "$output" | grep -q 'Test262:AsyncTestComplete'; then
        echo "pass"
      elif first_line="$(printf '%s\n' "$output" | grep -m1 'Test262:AsyncTestFailure')"; then
        printf "fail\t%s\n" "$first_line"
      else
        printf "fail\tasync case produced no \$DONE marker\n"
      fi
      return
    fi
    echo "pass"
  elif [ "$status" -eq 124 ]; then
    echo "timeout"
  elif [ "$engine" = "quickjs-rust" ] && [ -n "$negative_phase" ]; then
    if rust_negative_matches "$output" "$negative_phase" "$negative_type"; then
      echo "pass"
    else
      first_line="$(printf '%s\n' "$output" | sed -n '1p')"
      printf "fail\texpected negative %s%s, got %s\n" "$negative_phase" "${negative_type:+ $negative_type}" "$first_line"
    fi
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
    not-run) echo "not-run" ;;
    *) echo "fail" ;;
  esac
}
format_case_result() {
  printf '{"path":"%s","rust":"%s","rust_result":"%s","rust_skip":"%s","quickjs_ng":"%s","quickjs_ng_result":"%s","quickjs_ng_skip":"%s"}\n' \
    "$(json_escape "$1")" \
    "$(json_escape "$(result_kind "$2")")" \
    "$(json_escape "$(result_kind "$2")")" \
    "$(json_escape "$3")" \
    "$(json_escape "$(result_kind "$4")")" \
    "$(json_escape "$(result_kind "$4")")" \
    "$(json_escape "$5")"
}
write_case_result() {
  [ -n "$CASE_RESULTS_JSONL" ] || return 0
  format_case_result "$@" >>"$CASE_RESULTS_JSONL"
}
# Executes one queued case inside a parallel xargs worker. Every argument
# carries a leading "x" sentinel because BSD xargs drops empty NUL-separated
# fields, which would misalign the record. Results, the JSONL line, and
# diagnostic messages land under $WORK_DIR keyed by the queue index; the
# aggregation pass after the queue drains folds them into counters in order.
run_case_worker() {
  local idx="${1#x}" file="${2#x}" flags="${3#x}" rel="${4#x}" includes="${5#x}"
  local rust_skip_reason="${6#x}" qjsng_skip_reason="${7#x}"
  local negative_phase="${8#x}" negative_type="${9#x}"
  local temp_dir temp rust_result="not-run" qjsng_result="not-run" is_async=""
  local is_module="" module_prelude=""
  printf 'test262-baseline [%s]: %s\n' "$idx" "$rel"
  if [[ "$flags" == *async* ]]; then
    is_async="async"
  fi
  if [[ "$flags" == *module* ]]; then
    is_module="module"
  fi
  temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/qjs-test262-baseline-XXXXXX")"
  # Script-goal cases may use dynamic `import()` with specifiers relative to the
  # test file (e.g. `import('./x_FIXTURE.js')`). Place the combined script next
  # to the original test so those specifiers resolve against the test's own
  # directory, then remove it after the run. Module cases run the original file
  # directly under the Module goal and keep their script `temp` in temp_dir.
  local case_dir
  if [ -z "$is_module" ]; then
    case_dir="$(dirname "$file")"
    temp="$(mktemp "$case_dir/.qjs-baseline-case-$BASELINE_RUN_ID-XXXXXX")"
  else
    temp="$temp_dir/case.js"
  fi
  if [ -n "$is_module" ]; then
    # Module cases run the test file directly under the Module goal; harness
    # includes become a script prelude (see make_module_prelude). The combined
    # script `temp` is only used for the quickjs-ng leg.
    module_prelude="$temp_dir/prelude.js"
    make_module_prelude "$module_prelude" "$file" "$flags" "$includes"
  fi
  make_case "$file" "$temp" "$flags" "$includes"
  if [ "$ENGINE" = "quickjs-rust" ] || [ "$ENGINE" = "both" ]; then
    if [ -n "$rust_skip_reason" ]; then
      rust_result="skipped"
    else
      rust_result="$(run_engine_case quickjs-rust "$temp" "$file" "$negative_phase" "$negative_type" "$is_async" "$module_prelude")"
    fi
  fi
  if [ "$ENGINE" = "quickjs-ng" ] || [ "$ENGINE" = "both" ]; then
    if [ -n "$qjsng_skip_reason" ]; then
      qjsng_result="skipped"
    else
      qjsng_result="$(run_engine_case quickjs-ng "$temp" "$file")"
    fi
  fi
  rm -rf "$temp_dir"
  # A script-goal case writes its combined script into the test's own directory
  # (see above); remove it once both engine legs have run.
  if [ -z "$is_module" ] && [ -f "$temp" ]; then
    rm -f "$temp"
  fi
  printf '%s\n' "$rust_result" >"$WORK_DIR/$idx.rust"
  printf '%s\n' "$qjsng_result" >"$WORK_DIR/$idx.qjsng"
  if [ -n "$CASE_RESULTS_JSONL" ]; then
    format_case_result "$rel" "$rust_result" "$rust_skip_reason" "$qjsng_result" "$qjsng_skip_reason" >"$WORK_DIR/$idx.jsonl"
  fi
  {
    case "$rust_result" in
      skipped) echo "quickjs-rust skipped: $rel ($rust_skip_reason)" ;;
      timeout) echo "quickjs-rust timeout: $rel" ;;
      fail*) printf 'quickjs-rust fail: %s\t%s\n' "$rel" "${rust_result#fail	}" ;;
    esac
    case "$qjsng_result" in
      skipped) echo "quickjs-ng skipped: $rel ($qjsng_skip_reason)" ;;
      timeout) echo "quickjs-ng timeout: $rel" ;;
      fail*) printf 'quickjs-ng fail: %s\t%s\n' "$rel" "${qjsng_result#fail	}" ;;
    esac
  } >"$WORK_DIR/$idx.msg"
}

# Folds one finished case back into the serial counters, replaying its JSONL
# line and diagnostics in queue order so output stays deterministic.
aggregate_case() {
  local idx="$1"
  local rust_result="not-run" qjsng_result="not-run"
  if [ -s "$WORK_DIR/$idx.rust" ]; then
    IFS= read -r rust_result <"$WORK_DIR/$idx.rust" || true
  fi
  if [ -s "$WORK_DIR/$idx.qjsng" ]; then
    IFS= read -r qjsng_result <"$WORK_DIR/$idx.qjsng" || true
  fi
  if [ "$ENGINE" = "quickjs-rust" ] || [ "$ENGINE" = "both" ]; then
    if [ "$rust_result" = "skipped" ]; then
      rust_skipped=$((rust_skipped + 1))
    else
      count_engine_result rust "$rust_result"
    fi
  fi
  if [ "$ENGINE" = "quickjs-ng" ] || [ "$ENGINE" = "both" ]; then
    if [ "$qjsng_result" = "skipped" ]; then
      qjsng_skipped=$((qjsng_skipped + 1))
    else
      count_engine_result qjsng "$qjsng_result"
    fi
  fi
  if [ -n "$CASE_RESULTS_JSONL" ] && [ -f "$WORK_DIR/$idx.jsonl" ]; then
    cat "$WORK_DIR/$idx.jsonl" >>"$CASE_RESULTS_JSONL"
  fi
  if [ -s "$WORK_DIR/$idx.msg" ]; then
    cat "$WORK_DIR/$idx.msg" >&2
  fi

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
        skipped) qjsng_pass_rust_not_run=$((qjsng_pass_rust_not_run + 1)) ;;
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
  local value="${1//\\/\\\\}"
  printf '%s' "${value//\"/\\\"}"
}

write_summary_json() {
  [ -n "$SUMMARY_JSON" ] || return 0
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
    "raw": $skip_raw,
    "syntax": $skip_syntax
  },
  "rust_not_run": $rust_not_run,
  "quickjs_rust": {"pass": $rust_pass, "fail": $rust_fail, "timeout": $rust_timeout, "skipped": $rust_skipped},
  "quickjs_ng": {"pass": $qjsng_pass, "fail": $qjsng_fail, "timeout": $qjsng_timeout, "skipped": $qjsng_skipped},
  "comparison": {
    "both_pass": $both_pass,
    "quickjs_ng_pass_rust_nonpass": $qjsng_pass_rust_nonpass,
    "quickjs_ng_pass_rust_not_run": $qjsng_pass_rust_not_run,
    "quickjs_ng_pass_rust_fail": $qjsng_pass_rust_fail,
    "quickjs_ng_pass_rust_timeout": $qjsng_pass_rust_timeout,
    "rust_pass_quickjs_ng_nonpass": $rust_pass_qjsng_nonpass,
    "both_nonpass": $both_nonpass,
    "both_fail_or_timeout": $both_fail_or_timeout
  }
}
JSON
}

if [ -n "$CASE_RESULTS_JSONL" ]; then
  mkdir -p "$(dirname "$CASE_RESULTS_JSONL")"
  : >"$CASE_RESULTS_JSONL"
fi

scanned=0 total=0 configured=0 eligible=0 run=0 skipped=0
skip_async=0 skip_features=0 skip_fixture=0 skip_includes=0
skip_intl402=0 skip_module=0 skip_negative=0 skip_raw=0 skip_syntax=0
rust_not_run=0 rust_pass=0 rust_fail=0 rust_timeout=0 rust_skipped=0
qjsng_pass=0 qjsng_fail=0 qjsng_timeout=0 qjsng_skipped=0
both_pass=0 qjsng_pass_rust_nonpass=0 rust_pass_qjsng_nonpass=0
qjsng_pass_rust_not_run=0 qjsng_pass_rust_fail=0 qjsng_pass_rust_timeout=0
both_nonpass=0 both_fail_or_timeout=0

while IFS= read -r file; do
  if [ "$STOP_AFTER_LIMIT" -eq 1 ] && [ "$RUN_LIMIT" != "all" ] && [ "$run" -ge "$RUN_LIMIT" ]; then
    break
  fi

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
    read -r negative_phase
    read -r negative_type
  } < <(metadata_for "$file")
  reason="$(skip_reason "$rel" "$flags" "$includes" "$features")"
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
    write_case_result "$rel" "not-run" "$reason" "skipped" "$qjsng_reason"
    continue
  fi

  if [ "$ENGINE" = "both" ]; then
    configured=$((configured + 1))
    if [ -n "$reason" ]; then
      rust_not_run=$((rust_not_run + 1))
    else
      eligible=$((eligible + 1))
    fi
  elif [ -n "$reason" ]; then
    skipped=$((skipped + 1))
    case "$reason" in
      async) skip_async=$((skip_async + 1)) ;;
      fixture) skip_fixture=$((skip_fixture + 1)) ;;
      agent|includes) skip_includes=$((skip_includes + 1)) ;;
      intl402) skip_intl402=$((skip_intl402 + 1)) ;;
      module) skip_module=$((skip_module + 1)) ;;
      negative) skip_negative=$((skip_negative + 1)) ;;
      raw) skip_raw=$((skip_raw + 1)) ;;
      syntax) skip_syntax=$((skip_syntax + 1)) ;;
    esac
    if [ "$ENGINE" = "quickjs-rust" ]; then
      write_case_result "$rel" "skipped" "$reason" "not-run" ""
      continue
    fi
  else
    eligible=$((eligible + 1))
  fi

  if [ "$RUN_LIMIT" != "all" ] && [ "$run" -ge "$RUN_LIMIT" ]; then
    continue
  fi

  run=$((run + 1))
  printf '%s\0' "x$run" "x$file" "x$flags" "x$rel" "x$includes" \
    "x$reason" "x$qjsng_reason" "x$negative_phase" "x$negative_type" >>"$QUEUE_FILE"
done < <(list_test262_cases)

# Drain the queue with parallel workers, then fold per-case results back into
# the counters in queue order.
if [ "$run" -gt 0 ]; then
  export CASE_RESULTS_JSONL
  export CASE_TIMEOUT_SECONDS
  export ENGINE
  export QJS_CLI_BIN="${QJS_CLI_BIN:-}"
  export QUICKJS_NG_CONF="${QUICKJS_NG_CONF:-}"
  export QUICKJS_NG_RUNNER
  export RUN_WITH_TIMEOUT
  export TEST262_DIR
  export TIMEOUT_RETRIES
  export WORK_DIR
  export -f emit_test262_assert_fast_paths
  export -f should_emit_test262_assert_fast_paths
  export -f emit_test262_host_shim
  export -f emit_quickjs_rust_case_source
  export -f format_case_result
  export -f harness_include_uses
  export -f json_escape
  export -f make_case
  export -f make_module_prelude
  export -f needs_assert_prelude
  export -f needs_agent_harness
  export -f needs_host_prelude
  export -f needs_sta_prelude
  export -f result_kind
  export -f run_case_worker
  export -f run_engine_case
  export -f rust_error_field
  export -f rust_negative_matches
  export -f split_entries
  set +e
  xargs -0 -n 9 -P "$TEST262_BASELINE_JOBS" bash -c 'run_case_worker "$@"' _ <"$QUEUE_FILE"
  worker_status=$?
  set -e
  if [ "$worker_status" -ne 0 ]; then
    echo "error: Test262 baseline worker failed before completing all cases" >&2
    exit 1
  fi
  case_index=1
  while [ "$case_index" -le "$run" ]; do
    aggregate_case "$case_index"
    case_index=$((case_index + 1))
  done
fi

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
echo "  skipped.syntax: $skip_syntax"
echo "  rust.not_run: $rust_not_run"
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
exit 0
