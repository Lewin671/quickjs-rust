#!/usr/bin/env bash
# One-command inner loop for performance work: build a cached base executable
# and the working-tree candidate, run the hardware-counter screen, list the
# largest function-size changes, and optionally summarize a typed-loop trace.
# Diagnostic only; formal acceptance follows docs/performance-workflow.md.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"
export PYTHONDONTWRITEBYTECODE=1
export PYTHONPATH="$ROOT_DIR${PYTHONPATH:+:$PYTHONPATH}"

usage() {
  cat <<'EOF'
usage: scripts/perf-loop.sh (--plan <unit.json> | --base <git-ref>) [options]

  --plan <unit.json>   screen the plan's fast_gate targets/controls plus the
                       six sentinels against its base_sha and print PASS/FAIL
  --base <git-ref>     exploratory: screen against this revision (no verdict)
  --case <spec>        extra screen case; repeatable (see tools.benchmark.screen)
  --pairs <n>          screen pairs (default 5)
  --symbols <n>        function-size changes to list (default 15, 0 disables)
  --trace <spec>       also summarize QJS_TL_TRACE=1 for one case on a
                       perf-counters build of the working tree
  --require-quiet      refuse to screen while the load average is high

The base executable is cached per commit under target/perf-loop/.
EOF
}

plan=""
base_ref=""
pairs=5
symbols=15
trace=""
quiet=()
cases=()
while [ $# -gt 0 ]; do
  case "$1" in
    --plan) plan="${2:?}"; shift 2 ;;
    --base) base_ref="${2:?}"; shift 2 ;;
    --case) cases+=(--case "${2:?}"); shift 2 ;;
    --pairs) pairs="${2:?}"; shift 2 ;;
    --symbols) symbols="${2:?}"; shift 2 ;;
    --trace) trace="${2:?}"; shift 2 ;;
    --require-quiet) quiet=(--require-quiet); shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "error: unknown argument $1" >&2; usage >&2; exit 2 ;;
  esac
done
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

if [ -n "$plan" ]; then
  base_ref="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["base_sha"])' "$plan")"
fi
base_sha="$(git rev-parse --verify "${base_ref}^{commit}")"

cache_dir="$ROOT_DIR/target/perf-loop"
base_bin="$cache_dir/base-$base_sha/qjs"
if [ ! -x "$base_bin" ]; then
  echo "perf-loop: building base $base_sha" >&2
  worktree="$(mktemp -d "${TMPDIR:-/tmp}/qjs-perf-loop-base.XXXXXX")"
  rmdir "$worktree"
  git worktree add --detach --quiet "$worktree" "$base_sha"
  trap 'git -C "$ROOT_DIR" worktree remove --force "$worktree" 2>/dev/null || true' EXIT
  (cd "$worktree" && "$cargo_bin" build --release -p qjs-cli --target-dir "$cache_dir/build-base" --quiet)
  mkdir -p "$(dirname "$base_bin")"
  cp "$cache_dir/build-base/release/qjs" "$base_bin.tmp"
  mv "$base_bin.tmp" "$base_bin"
  git worktree remove --force "$worktree"
  trap - EXIT
fi

echo "perf-loop: building candidate from the working tree" >&2
"$cargo_bin" build --release -p qjs-cli --quiet
candidate_bin="$ROOT_DIR/target/release/qjs"

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
