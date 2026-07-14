#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PYTHONDONTWRITEBYTECODE=1
export PYTHONPATH="$ROOT_DIR${PYTHONPATH:+:$PYTHONPATH}"
cd "$ROOT_DIR"
exec python3 -m tools.benchmark.performance_policy "$@"
