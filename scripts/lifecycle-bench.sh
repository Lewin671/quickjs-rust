#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"

if ! CARGO_BIN="$(qjs_resolve_cargo)"; then
  echo "error: cargo not found; install Rust before running lifecycle benchmarks" >&2
  exit 1
fi

quick=0
for argument in "$@"; do
  case "$argument" in
    --quick)
      quick=1
      ;;
    --list|--help|--verbose|--quiet|--noplot|--exact|--ignored|\
    --color=auto|--color=always|--color=never|--format=pretty|--format=terse|\
    -v|-n|-h)
      ;;
    --*|-*)
      echo "error: lifecycle benchmark policy rejects unsupported option: $argument" >&2
      exit 2
      ;;
  esac
done

criterion_extra=()
if [ "$quick" -eq 1 ]; then
  export CRITERION_HOME="$ROOT_DIR/target/criterion-smoke"
  criterion_extra=(--discard-baseline)
else
  export CRITERION_HOME="$ROOT_DIR/target/criterion"
fi

cd "$ROOT_DIR"
exec "$CARGO_BIN" bench -p qjs-runtime --bench lifecycle -- \
  "$@" ${criterion_extra[@]+"${criterion_extra[@]}"}
