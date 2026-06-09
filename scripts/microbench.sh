#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"
BENCH_FILE="$ROOT_DIR/tests/benchmarks/quickjs/microbench.js"
CASE_TIMEOUT_SECONDS="${MICROBENCH_TIMEOUT_SECONDS:-120}"
ENGINE="quickjs-rust"

usage() {
  cat >&2 <<'USAGE'
usage: scripts/microbench.sh [--engine quickjs-rust|quickjs-ng|both] [benchmark-prefix...]

Runs the repository's QuickJS microbenchmark subset. Benchmark prefixes are
matched against test names, for example:

  scripts/microbench.sh prop array
  scripts/microbench.sh --engine both string_to_int
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --engine)
      if [ "$#" -lt 2 ]; then
        usage
        exit 2
      fi
      ENGINE="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    -*)
      usage
      exit 2
      ;;
    *)
      break
      ;;
  esac
done

qjs_require_run_with_timeout

if ! CARGO_BIN="$(qjs_resolve_cargo)"; then
  echo "error: cargo not found; install Rust with rustup before benchmarking" >&2
  exit 127
fi

run_rust() {
  "$CARGO_BIN" build -q --release -p qjs-cli
  "$RUN_WITH_TIMEOUT" "$CASE_TIMEOUT_SECONDS" "$ROOT_DIR/target/release/qjs" --raw "$BENCH_FILE" "$@"
}

run_quickjs_ng() {
  qjs_ensure_quickjs_ng
  "$RUN_WITH_TIMEOUT" "$CASE_TIMEOUT_SECONDS" "$ROOT_DIR/third_party/quickjs-ng/build/qjs" "$BENCH_FILE" "$@"
}

case "$ENGINE" in
  quickjs-rust)
    run_rust "$@"
    ;;
  quickjs-ng)
    run_quickjs_ng "$@"
    ;;
  both)
    echo "== quickjs-rust =="
    run_rust "$@"
    echo
    echo "== quickjs-ng =="
    run_quickjs_ng "$@"
    ;;
  *)
    usage
    exit 2
    ;;
esac
