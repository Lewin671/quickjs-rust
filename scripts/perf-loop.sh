#!/usr/bin/env bash
# One-command performance iteration: schema 3 builds committed layout panels;
# legacy plans/base-only mode screen a working-tree candidate with optional
# function-size and typed-loop diagnostics.
# Diagnostic only; formal acceptance follows docs/performance-workflow.md.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"
export PYTHONDONTWRITEBYTECODE=1
export PYTHONPATH="$ROOT_DIR${PYTHONPATH:+:$PYTHONPATH}"

usage() {
  cat <<'EOF'
usage: scripts/perf-loop.sh (--plan <unit.json> | --base <git-ref>) [options]

  --plan <unit.json>   schema 3: committed clean candidate, full portfolio and
                       frozen layout rule; requires --output
                       legacy plan: targets/controls plus six sentinels
  --base <git-ref>     exploratory working-tree screen (no verdict)
  --case <spec>        extra screen case; repeatable (see tools.benchmark.screen)
  --pairs <n>          pairs (default 5; fixed at 5 for schema 3)
  --symbols <n>        legacy function-size list (default 15, 0 disables)
  --output <dir>      new layout-rule attempt directory (schema 3)
  --workers <n>       schema-3 case shards (1..8, default 1); must match policy
  --trace <spec>       legacy: also summarize QJS_TL_TRACE=1 for one case on a
                       perf-counters build of the working tree
  --require-quiet      refuse to screen while the load average is high

Schema 3 builds/audits six variants of each revision before timing; mandatory
panels always complete all six. Other events append variants 1..5 to the
immutable natural screen. Use the site's serialized timing queue.
Legacy base binaries are cached under
target/perf-loop/compiler-fat16-abort-v1/rev-<sha>/.
EOF
}

plan=""
base_ref=""
pairs=5
workers=1
symbols=15
trace=""
output=""
quiet=()
cases=()
while [ $# -gt 0 ]; do
  case "$1" in
    --plan) plan="${2:?}"; shift 2 ;;
    --base) base_ref="${2:?}"; shift 2 ;;
    --case) cases+=(--case "${2:?}"); shift 2 ;;
    --workers) workers="${2:?}"; shift 2 ;;
    --pairs) pairs="${2:?}"; shift 2 ;;
    --symbols) symbols="${2:?}"; shift 2 ;;
    --output) output="${2:?}"; shift 2 ;;
    --trace) trace="${2:?}"; shift 2 ;;
    --require-quiet) quiet=(--require-quiet); shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "error: unknown argument $1" >&2; usage >&2; exit 2 ;;
  esac
done
case "$workers" in
  [1-8]) ;;
  *) echo "error: --workers must be 1..8" >&2; exit 2 ;;
esac
if [ -n "$plan" ] && [ -n "$base_ref" ]; then
  echo "error: --plan fixes the base; drop --base" >&2
  exit 2
fi
if [ -z "$plan" ] && [ -z "$base_ref" ]; then
  usage >&2
  exit 2
fi

cargo_bin="$(qjs_resolve_cargo)" || { echo "error: cargo not found" >&2; exit 1; }
cd "$ROOT_DIR"

if [ -n "$plan" ] && [ "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["schema_version"])' "$plan")" = 3 ]; then
  if [ -z "$output" ]; then
    echo "error: schema-3 layout attempts require --output <new-directory>" >&2
    exit 2
  fi
  if [ "$pairs" != 5 ] || [ "${#cases[@]}" -ne 0 ] || [ -n "$trace" ] || [ -n "$base_ref" ]; then
    echo "error: schema-3 attempts use the frozen full portfolio and five pairs" >&2
    exit 2
  fi
  exec python3 -m tools.benchmark.layout_iteration --plan "$plan" --output "$output" --workers "$workers"
fi

if [ "$workers" != 1 ]; then
  echo "error: --workers requires a schema-3 layout plan" >&2
  exit 2
fi
if [ -n "$plan" ]; then
  base_ref="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["base_sha"])' "$plan")"
fi
base_sha="$(git rev-parse --verify "${base_ref}^{commit}")"

base_bin="$(qjs_build_revision "$ROOT_DIR" "$cargo_bin" "$base_sha")" || {
  echo "error: cannot build base $base_sha" >&2
  exit 1
}

echo "perf-loop: building candidate from the working tree" >&2
candidate_bin="$(python3 -m tools.benchmark.build_cache build-rust \
  --cargo "$cargo_bin" --source "$ROOT_DIR" --target-dir "$ROOT_DIR/target/perf-loop/candidate-build")"

screen_args=(--candidate "$candidate_bin" --base "$base_bin" --pairs "$pairs" --symbols "$symbols")
if [ -n "$plan" ]; then
  screen_args+=(--plan "$plan")
fi
status=0
python3 -m tools.benchmark.screen "${screen_args[@]}" ${cases[@]+"${cases[@]}"} ${quiet[@]+"${quiet[@]}"} || status=$?

if [ -n "$trace" ]; then
  echo "perf-loop: building perf-counters candidate for the trace" >&2
  "$cargo_bin" build --release -p qjs-cli --features perf-counters --target-dir target/perf-counters --quiet
  echo
  python3 -m tools.benchmark.tl_trace --binary target/perf-counters/release/qjs --case "$trace" || true
fi
exit "$status"
