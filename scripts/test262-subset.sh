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

FILTER_PREFIXES=()
while [ "$#" -gt 0 ]; do
  case "$1" in
    --filter)
      if [ "$#" -lt 2 ]; then
        echo "error: --filter requires an allowlist prefix" >&2
        exit 2
      fi
      FILTER_PREFIXES+=("$2")
      shift 2
      ;;
    --filter=*)
      FILTER_PREFIXES+=("${1#--filter=}")
      shift
      ;;
    -h|--help)
      cat <<'EOF'
Usage: ./scripts/test262-subset.sh [--filter <allowlist-prefix>]...

Runs the curated Test262 allowlist. When one or more --filter values are
provided, only allowlist entries with a matching path prefix are validated and
executed.
EOF
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

trim_ws() {
  local value="$1"
  value="${value#${value%%[![:space:]]*}}"
  value="${value%${value##*[![:space:]]}}"
  printf '%s' "$value"
}

matches_filter_prefixes() {
  local entry="$1"
  local prefix
  if [ "${#FILTER_PREFIXES[@]}" -eq 0 ]; then
    return 0
  fi
  for prefix in "${FILTER_PREFIXES[@]}"; do
    case "$entry" in
      "$prefix"*) return 0 ;;
    esac
  done
  return 1
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
    return (typeof __quickjsRustEvalScript === 'function')
      ? __quickjsRustEvalScript(source)
      : (0, eval)(source);
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
      if (new.target && new.target !== crossRealmObject) {
        var prototype = new.target.prototype;
        if (prototype === null || (typeof prototype !== "object" && typeof prototype !== "function")) {
          prototype = new.target.__quickjsRustRealmObjectPrototype || crossRealmObject.prototype;
        }
        return globalThis.Object.create(prototype);
      }
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
    Object.defineProperty(crossRealmFunctionPrototype, "apply", {
      value: function apply(thisArg, argArray) {
        if (typeof this !== "function") {
          throw new __quickjsRustRealmGlobal.TypeError("Function.prototype.apply target is not callable");
        }
        if (argArray !== null && argArray !== undefined) {
          var argType = typeof argArray;
          if (argType !== "object" && argType !== "function") {
            throw new __quickjsRustRealmGlobal.TypeError("Function.prototype.apply argument list must be an object");
          }
        }
        return globalThis.Function.prototype.apply.call(this, thisArg, argArray);
      },
      writable: true,
      enumerable: false,
      configurable: true
    });
    Object.defineProperty(crossRealmFunctionPrototype, "bind", {
      value: function bind() {
        if (typeof this !== "function") {
          throw new __quickjsRustRealmGlobal.TypeError("Function.prototype.bind target is not callable");
        }
        return globalThis.Function.prototype.bind.apply(this, arguments);
      },
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
emit_quickjs_rust_case_source() {
  cat "$1"
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
  if grep -q 'assert[.]sameValue' "$source"; then
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
  } >"$output"
}

RESULT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/qjs-test262-subset-XXXXXX")"
trap 'rm -rf "$RESULT_DIR"' EXIT
mkdir -p "$RESULT_DIR/meta"

allowlist_count=0
unfiltered_allowlist_count=0
allowlist_entries=()
while IFS= read -r line; do
  entry="${line%%#*}"
  entry="$(trim_ws "$entry")"
  [ -z "$entry" ] && continue
  unfiltered_allowlist_count=$((unfiltered_allowlist_count + 1))
  if ! matches_filter_prefixes "$entry"; then
    continue
  fi

  allowlist_count=$((allowlist_count + 1))
  allowlist_entries+=("$entry")
done < "$ALLOWLIST"

if [ "$allowlist_count" -eq 0 ]; then
  if [ "${#FILTER_PREFIXES[@]}" -gt 0 ]; then
    echo "error: Test262 allowlist filters selected no cases: ${FILTER_PREFIXES[*]}" >&2
  else
    echo "error: Test262 allowlist is empty; add at least one runnable subset case" >&2
  fi
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
  if ! matches_filter_prefixes "$entry"; then
    continue
  fi
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
if [ "${#FILTER_PREFIXES[@]}" -gt 0 ]; then
  echo "test262: selected $allowlist_count/$unfiltered_allowlist_count allowlist cases with filters: ${FILTER_PREFIXES[*]}"
fi
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
