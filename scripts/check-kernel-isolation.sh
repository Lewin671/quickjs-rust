#!/usr/bin/env bash
# K0 static matrix on the production target; submit through imac-bench run.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
export PYTHONPATH="$ROOT_DIR${PYTHONPATH:+:$PYTHONPATH}"
export PYTHONDONTWRITEBYTECODE=1
exec python3 -m tools.benchmark.kernel_build "$@"
