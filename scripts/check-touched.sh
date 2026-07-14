#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"

MODE="staged"
BASE_REF=""
EXPLAIN=0

usage() {
  cat <<'EOF'
Usage: ./scripts/check-touched.sh [--staged | --base <ref>] [--explain]

Runs a fast, change-aware local gate for pre-commit and AI iteration. The
script always explains skipped broad checks when --explain is set; it is not a
replacement for ./scripts/check.sh before final handoff or push.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --staged)
      MODE="staged"
      BASE_REF=""
      shift
      ;;
    --base)
      if [ "$#" -lt 2 ]; then
        echo "error: --base requires a ref" >&2
        exit 2
      fi
      MODE="base"
      BASE_REF="$2"
      shift 2
      ;;
    --base=*)
      MODE="base"
      BASE_REF="${1#--base=}"
      shift
      ;;
    --explain)
      EXPLAIN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! CARGO_BIN="$(qjs_resolve_cargo)"; then
  echo "error: cargo not found; install Rust with rustup before running checks" >&2
  exit 127
fi

changed_files="$(
  if [ "$MODE" = "staged" ]; then
    git -C "$ROOT_DIR" diff --cached --name-only --diff-filter=ACMRT
  else
    git -C "$ROOT_DIR" diff --name-only --diff-filter=ACMRT "$BASE_REF"...HEAD
  fi
)"

if [ -z "$changed_files" ]; then
  echo "check-touched: no changed files for $MODE check"
  exit 0
fi

has_rust=0
has_cargo=0
has_scripts=0
has_docs_only=1
touches_runtime=0
touches_parser=0
touches_lexer=0
touches_ast=0
touches_cli=0
touches_test262_config=0
touches_benchmark=0
test262_filters=()

add_filter() {
  local filter="$1"
  local existing
  # Guard the array expansion: under `set -u`, bash 3.2 (macOS) errors on
  # "${arr[@]}" when the array is empty, which it is on the first call.
  for existing in ${test262_filters[@]+"${test262_filters[@]}"}; do
    [ "$existing" = "$filter" ] && return
  done
  test262_filters+=("$filter")
}

add_common_filters_for_path() {
  local path="$1"
  case "$path" in
    crates/qjs-runtime/src/bytecode/*|crates/qjs-runtime/src/global.rs|crates/qjs-runtime/src/scope.rs|crates/qjs-runtime/src/tests/statements.rs)
      add_filter "test/annexB/language/eval-code/"
      add_filter "test/language/eval-code/"
      add_filter "test/language/statements/block/"
      add_filter "test/language/statements/function/"
      ;;
    crates/qjs-runtime/src/function/*|crates/qjs-runtime/src/tests/functions.rs)
      add_filter "cases/arguments-"
      add_filter "cases/call-"
      add_filter "cases/function-"
      add_filter "cases/named-function-"
      add_filter "test/language/function-code/"
      add_filter "test/language/expressions/call/"
      add_filter "test/language/statements/function/"
      ;;
    crates/qjs-runtime/src/tests/parameters.rs|crates/qjs-parser/src/*parameter*|crates/qjs-ast/src/*parameter*)
      add_filter "test/language/arguments-object/"
      add_filter "test/language/function-code/"
      ;;
    crates/qjs-parser/*|crates/qjs-lexer/*)
      add_filter "test/language/literals/"
      add_filter "test/language/statements/"
      ;;
  esac
}

while IFS= read -r path; do
  [ -z "$path" ] && continue
  case "$path" in
    *.rs) has_rust=1; has_docs_only=0 ;;
    Cargo.toml|Cargo.lock|rust-toolchain.toml) has_cargo=1; has_docs_only=0 ;;
    scripts/*|.github/*) has_scripts=1; has_docs_only=0 ;;
    tests/test262/allowlist.txt|tests/test262/expected-failures.txt|tests/test262/cases/*)
      touches_test262_config=1; has_docs_only=0 ;;
    *.md|docs/*|AGENTS.md|README.md) ;;
    *) has_docs_only=0 ;;
  esac

  case "$path" in
    crates/qjs-runtime/*) touches_runtime=1 ;;
    crates/qjs-parser/*) touches_parser=1 ;;
    crates/qjs-lexer/*) touches_lexer=1 ;;
    crates/qjs-ast/*) touches_ast=1 ;;
    crates/qjs-cli/*) touches_cli=1 ;;
    benchmarks/*|tools/__init__.py|tools/benchmark/*|scripts/benchmark*.sh|scripts/resource-benchmark*.sh|scripts/lifecycle-bench.sh|scripts/external-corpus-audit.sh|scripts/performance-policy-audit.sh|.github/workflows/performance-smoke.yml)
      touches_benchmark=1
      ;;
  esac

  add_common_filters_for_path "$path"
done <<<"$changed_files"

run_cmd() {
  echo "check-touched: $*"
  "$@"
}

if [ "$EXPLAIN" -eq 1 ]; then
  echo "check-touched: mode=$MODE"
  if [ "$MODE" = "base" ]; then
    echo "check-touched: base=$BASE_REF"
  fi
  echo "check-touched: changed files:"
  while IFS= read -r path; do
    [ -n "$path" ] && echo "  $path"
  done <<<"$changed_files"
fi

if [ "$has_docs_only" -eq 1 ]; then
  echo "check-touched: docs-only change; skipping Rust and Test262 checks"
  exit 0
fi

if [ "$has_rust" -eq 1 ] || [ "$has_cargo" -eq 1 ] || [ "$has_scripts" -eq 1 ]; then
  run_cmd "$CARGO_BIN" fmt --all -- --check
  run_cmd "$CARGO_BIN" clippy --workspace --all-targets -- -D warnings
fi

if [ "$has_rust" -eq 1 ] || [ "$has_cargo" -eq 1 ] || [ "$has_scripts" -eq 1 ] \
  || [ "$touches_benchmark" -eq 1 ]; then
  run_cmd "$ROOT_DIR/scripts/check-file-size.sh"
fi

if [ "$has_cargo" -eq 1 ]; then
  run_cmd "$CARGO_BIN" test --workspace -q
else
  [ "$touches_ast" -eq 1 ] && run_cmd "$CARGO_BIN" test -p qjs-ast -q
  [ "$touches_lexer" -eq 1 ] && run_cmd "$CARGO_BIN" test -p qjs-lexer -q
  [ "$touches_parser" -eq 1 ] && run_cmd "$CARGO_BIN" test -p qjs-parser -q
  [ "$touches_cli" -eq 1 ] && run_cmd "$CARGO_BIN" test -p qjs-cli -q
  [ "$touches_runtime" -eq 1 ] && run_cmd "$CARGO_BIN" test -p qjs-runtime -q
fi

if [ "$touches_benchmark" -eq 1 ]; then
  run_cmd env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR" \
    python3 -m unittest discover -s "$ROOT_DIR/tools/benchmark/tests" -v
  run_cmd bash -n \
    "$ROOT_DIR/scripts/benchmark.sh" "$ROOT_DIR/scripts/benchmark-report.sh" \
    "$ROOT_DIR/scripts/resource-benchmark.sh" \
    "$ROOT_DIR/scripts/resource-benchmark-report.sh" \
    "$ROOT_DIR/scripts/lifecycle-bench.sh" \
    "$ROOT_DIR/scripts/external-corpus-audit.sh" \
    "$ROOT_DIR/scripts/performance-policy-audit.sh"
fi

if [ "$touches_test262_config" -eq 1 ]; then
  export TEST262_CASE_TIMEOUT_SECONDS="${TEST262_CASE_TIMEOUT_SECONDS:-30}"
  run_cmd "$ROOT_DIR/scripts/test262-subset.sh"
elif [ "${#test262_filters[@]}" -gt 0 ]; then
  filter_args=()
  for filter in "${test262_filters[@]}"; do
    filter_args+=("--filter" "$filter")
  done
  export TEST262_CASE_TIMEOUT_SECONDS="${TEST262_CASE_TIMEOUT_SECONDS:-30}"
  run_cmd "$ROOT_DIR/scripts/test262-subset.sh" "${filter_args[@]}"
elif [ "$touches_runtime" -eq 1 ] || [ "$touches_parser" -eq 1 ] || [ "$touches_lexer" -eq 1 ]; then
  echo "check-touched: semantic engine files changed, but no focused Test262 filter matched"
  echo "check-touched: run ./scripts/check.sh before handoff or push"
fi
