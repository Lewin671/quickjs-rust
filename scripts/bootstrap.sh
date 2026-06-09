#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"

git -C "$ROOT_DIR" submodule update --init third_party/quickjs-ng third_party/test262

if ! CARGO_BIN="$(qjs_resolve_cargo)"; then
  echo "warning: cargo not found; install Rust before running project checks" >&2
  exit 0
fi

(cd "$ROOT_DIR" && "$CARGO_BIN" fetch)
