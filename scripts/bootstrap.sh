#!/usr/bin/env bash
set -euo pipefail

git submodule update --init third_party/quickjs-ng third_party/test262

if command -v cargo >/dev/null 2>&1; then
  cargo fetch
elif [ -x "$HOME/.cargo/bin/cargo" ]; then
  "$HOME/.cargo/bin/cargo" fetch
else
  echo "warning: cargo not found; install Rust before running project checks" >&2
fi
