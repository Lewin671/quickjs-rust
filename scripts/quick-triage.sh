#!/usr/bin/env bash
# Queue-only diagnostic; never a performance-unit attempt or acceptance input.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"
export PATH="$(dirname "$(qjs_resolve_cargo)"):$PATH"
export PYTHONDONTWRITEBYTECODE=1
export PYTHONPATH="$ROOT_DIR${PYTHONPATH:+:$PYTHONPATH}"
cd "$ROOT_DIR"
exec python3 -m tools.benchmark.quick_triage "$@"
