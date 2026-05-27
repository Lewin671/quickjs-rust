#!/usr/bin/env bash
set -euo pipefail

git submodule update --init third_party/quickjs-ng third_party/test262

if command -v cargo >/dev/null 2>&1; then
  CARGO_BIN="cargo"
elif [ -x "$HOME/.cargo/bin/cargo" ]; then
  CARGO_BIN="$HOME/.cargo/bin/cargo"
else
  echo "warning: cargo not found; install Rust before running project checks" >&2
  exit 0
fi

"$CARGO_BIN" fetch
