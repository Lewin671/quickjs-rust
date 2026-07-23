#!/usr/bin/env bash

# Exact metadata backports for the pinned Test262 revision. Remove an entry
# once third_party/test262 contains the cited upstream fix. These amendments
# must never weaken runtime semantics or skip an entire test file.

test262_case_excludes_immutable_typed_array_factory() {
  case "$1" in
    "$TEST262_DIR/test/built-ins/TypedArray/prototype/slice/speciesctor-return-same-buffer-with-offset.js" | \
    "$TEST262_DIR/test/built-ins/TypedArrayConstructors/internals/GetOwnProperty/BigInt/index-prop-desc.js" | \
    "$TEST262_DIR/test/built-ins/TypedArrayConstructors/internals/GetOwnProperty/index-prop-desc.js" | \
    "$TEST262_DIR/test/built-ins/TypedArrayConstructors/internals/Set/BigInt/null-tobigint.js" | \
    "$TEST262_DIR/test/built-ins/TypedArrayConstructors/internals/Set/BigInt/number-tobigint.js" | \
    "$TEST262_DIR/test/built-ins/TypedArrayConstructors/internals/Set/BigInt/string-nan-tobigint.js" | \
    "$TEST262_DIR/test/built-ins/TypedArrayConstructors/internals/Set/BigInt/symbol-tobigint.js" | \
    "$TEST262_DIR/test/built-ins/TypedArrayConstructors/internals/Set/BigInt/tonumber-value-throws.js" | \
    "$TEST262_DIR/test/built-ins/TypedArrayConstructors/internals/Set/BigInt/undefined-tobigint.js" | \
    "$TEST262_DIR/test/built-ins/TypedArrayConstructors/internals/Set/bigint-tonumber.js" | \
    "$TEST262_DIR/test/built-ins/TypedArrayConstructors/internals/Set/tonumber-value-throws.js")
      return 0
      ;;
  esac
  return 1
}

emit_test262_upstream_metadata_amendments() {
  local source="$1"
  if ! test262_case_excludes_immutable_typed_array_factory "$source"; then
    return
  fi

  # Backport tc39/test262@250f204f23a9249ff204be2baec29600faae7b75.
  # Upstream passes `["immutable"]` as the excluded constructor-argument
  # factory for these mutable-semantics cases. Filtering the helper's factory
  # list is equivalent while leaving every mutable factory and assertion live.
  cat <<'EOF'
typedArrayCtorArgFactories = typedArrayCtorArgFactories.filter(function(factory) {
  return factory !== makeImmutableArrayBuffer;
});
EOF
}
