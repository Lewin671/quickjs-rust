#!/usr/bin/env bash
# Scan the typed-loop executor's pinned offset (tools/benchmark/layout_pin.py).
#
# For each offset: pin the executor there, relink until the order file is a
# fixed point, keep the binary as target/layout-scan/qjs-<offset>, and screen
# it against a base with tools.benchmark.screen on the layout canaries. The
# order file is restored afterwards; adopt an offset by setting
# DEFAULT_OFFSET in layout_pin.py and re-pinning. Diagnostic only: run it
# after editing the executor or its pinned callees, on a quiet host.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"
export PYTHONDONTWRITEBYTECODE=1
export PYTHONPATH="$ROOT_DIR${PYTHONPATH:+:$PYTHONPATH}"

usage() {
  cat <<'EOF'
usage: scripts/layout-scan.sh --offsets <o1,o2,...> [--base <qjs>] [--case <spec>]... [--pairs <n>]

  --offsets   executor offsets modulo 4 KiB, e.g. 0x0,0x200,0x400
  --base      executable to compare against (default: a copy of the current
              canonical performance build, taken before the first relink)
  --case      tools.benchmark.screen case spec; repeatable (default: the six
              sentinels, ai-astar, imaging-desaturate, access-nsieve,
              access-nbody -- the cases the executor's placement has moved)
  --pairs     screen pairs per case (default 2)
EOF
}

offsets=""
base=""
pairs=2
cases=()
while [ $# -gt 0 ]; do
  case "$1" in
    --offsets) offsets="${2:?}"; shift 2 ;;
    --base) base="${2:?}"; shift 2 ;;
    --case) cases+=("${2:?}"); shift 2 ;;
    --pairs) pairs="${2:?}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "error: unknown argument $1" >&2; usage >&2; exit 2 ;;
  esac
done
if [ -z "$offsets" ]; then
  usage >&2
  exit 2
fi
if [ ${#cases[@]} -eq 0 ]; then
  cases=(sentinel external/kraken-1.1/ai-astar external/kraken-1.1/imaging-desaturate
         external/sunspider-1.0/access-nsieve external/sunspider-1.0/access-nbody)
fi

cargo_bin="$(qjs_resolve_cargo)" || { echo "error: cargo not found" >&2; exit 1; }
cd "$ROOT_DIR"
order="$(python3 -c 'from tools.benchmark.layout_pin import DEFAULT_ORDER; print(DEFAULT_ORDER)')"
scan_dir=target/layout-scan
mkdir -p "$scan_dir"
# Use the same features, explicit host target and flags as formal measurements.
build_cli() {
  python3 -m tools.benchmark.build_cache build-rust --source "$ROOT_DIR" \
    --target-dir "$ROOT_DIR/$scan_dir/build" --cargo "$cargo_bin"
}
binary="$(build_cli)"
vm_offset="$(python3 -c 'import sys; from pathlib import Path; from tools.benchmark.layout_pin import executor_offsets; value = executor_offsets(Path(sys.argv[1]))[1]; print("none" if value is None else hex(value))' "$binary")"
if [ "$vm_offset" = none ]; then
  echo "error: no interpreter executor; use layout_pin without a VM pin" >&2
  exit 1
fi
if [ -z "$base" ]; then
  base="$scan_dir/qjs-base"
  cp "$binary" "$base"
fi
saved="$(mktemp "${TMPDIR:-/tmp}/qjs-order.XXXXXX")"
cp "$order" "$saved"
trap 'cp "$saved" "$order"; rm -f "$saved"' EXIT

case_args=()
for spec in "${cases[@]}"; do
  case_args+=(--case "$spec")
done

IFS=',' read -r -a offset_list <<< "$offsets"
for offset in "${offset_list[@]}"; do
  cp "$saved" "$order"
  # Fillers are sized from the binary they are computed on, so pin, relink
  # and pin again until the order file stops changing.
  stable=false
  for _ in 1 2 3 4; do
    before="$(shasum "$order")"
    python3 -m tools.benchmark.layout_pin --binary "$binary" --order "$order" --offset "$offset" --vm-offset "$vm_offset" >/dev/null
    binary="$(build_cli)"
    if [ "$(shasum "$order")" = "$before" ]; then
      stable=true
      break
    fi
  done
  if [ "$stable" != true ]; then
    echo "error: layout did not stabilize at $offset; no screen was run" >&2
    exit 1
  fi
  cp "$binary" "$scan_dir/qjs-$offset"
  echo "== executor at $offset: $scan_dir/qjs-$offset" >&2
  python3 -m tools.benchmark.screen --candidate "$scan_dir/qjs-$offset" --base "$base" \
    --pairs "$pairs" "${case_args[@]}"
done
