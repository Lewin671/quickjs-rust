#!/usr/bin/env bash
# One-command formal three-engine comparison on this host: build candidate and
# base at exact commits in clean worktrees (cached per commit), verify the
# pinned QuickJS-NG build, write truthful build receipts, and run
# `python3 -m tools.benchmark.compare`. The output bundle feeds
# `performance-decision.sh queue` and `decide` (docs/performance-workflow.md).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"
export PYTHONDONTWRITEBYTECODE=1
export PYTHONPATH="$ROOT_DIR${PYTHONPATH:+:$PYTHONPATH}"

usage() {
  cat <<'EOF'
usage: scripts/perf-compare.sh --base <git-ref> --output-dir <dir> [options]

  --base <git-ref>        comparison base (a plan's base_sha for decisions)
  --candidate <git-ref>   candidate commit (default HEAD); must be committed
  --blocks <3|30|60>      measurement blocks (default 30; 3 is diagnostic only)
  --output-dir <dir>      new directory for the evidence bundle
  --quickjs-ng <path>     pinned QuickJS-NG executable
                          (default third_party/quickjs-ng/build/qjs)

Build the reference once with:
  cmake -S third_party/quickjs-ng -B third_party/quickjs-ng/build \
    -DCMAKE_BUILD_TYPE=Release -DBUILD_QJS_LIBC=ON
  cmake --build third_party/quickjs-ng/build --target qjs
EOF
}

base_ref=""
candidate_ref="HEAD"
blocks=30
output_dir=""
ng_bin="$ROOT_DIR/third_party/quickjs-ng/build/qjs"
while [ $# -gt 0 ]; do
  case "$1" in
    --base) base_ref="${2:?}"; shift 2 ;;
    --candidate) candidate_ref="${2:?}"; shift 2 ;;
    --blocks) blocks="${2:?}"; shift 2 ;;
    --output-dir) output_dir="${2:?}"; shift 2 ;;
    --quickjs-ng) ng_bin="${2:?}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "error: unknown argument $1" >&2; usage >&2; exit 2 ;;
  esac
done
if [ -z "$base_ref" ] || [ -z "$output_dir" ]; then
  usage >&2
  exit 2
fi
if [ -e "$output_dir" ]; then
  echo "error: $output_dir already exists; evidence directories are never reused" >&2
  exit 2
fi
if [ ! -x "$ng_bin" ]; then
  echo "error: no QuickJS-NG executable at $ng_bin" >&2
  usage >&2
  exit 1
fi

cargo_bin="$(qjs_resolve_cargo)" || { echo "error: cargo not found" >&2; exit 1; }
cd "$ROOT_DIR"
base_sha="$(git rev-parse --verify "${base_ref}^{commit}")"
candidate_sha="$(git rev-parse --verify "${candidate_ref}^{commit}")"
base_bin="$(qjs_build_revision "$ROOT_DIR" "$cargo_bin" "$base_sha")"
candidate_bin="$(qjs_build_revision "$ROOT_DIR" "$cargo_bin" "$candidate_sha")"
ng_sha="$(python3 -c 'import json; print(json.load(open("benchmarks/manifest.json"))["reference_engine"]["revision"])')"

receipts="$(mktemp -d "${TMPDIR:-/tmp}/qjs-perf-compare-receipts.XXXXXX")"
trap 'rm -rf "$receipts"' EXIT
python3 -m tools.benchmark.local_receipt --identity qjs-rust --revision "$candidate_sha" \
  --binary "$candidate_bin" --output "$receipts/candidate.json" >/dev/null
python3 -m tools.benchmark.local_receipt --identity qjs-rust --revision "$base_sha" \
  --binary "$base_bin" --output "$receipts/base.json" >/dev/null
python3 -m tools.benchmark.local_receipt --identity quickjs-ng --revision "$ng_sha" \
  --binary "$ng_bin" --output "$receipts/quickjs-ng.json" >/dev/null

echo "perf-compare: candidate $candidate_sha vs base $base_sha, $blocks blocks" >&2
python3 -m tools.benchmark.compare \
  --candidate "$candidate_bin" --candidate-receipt "$receipts/candidate.json" \
  --base "$base_bin" --base-receipt "$receipts/base.json" \
  --quickjs-ng "$ng_bin" --quickjs-ng-receipt "$receipts/quickjs-ng.json" \
  --blocks "$blocks" --output-dir "$output_dir"
