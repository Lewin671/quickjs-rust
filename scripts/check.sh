#!/usr/bin/env bash
set -euo pipefail

if command -v cargo >/dev/null 2>&1; then
  CARGO_BIN="cargo"
elif [ -x "$HOME/.cargo/bin/cargo" ]; then
  CARGO_BIN="$HOME/.cargo/bin/cargo"
else
  echo "error: cargo not found; install Rust with rustup before running checks" >&2
  exit 127
fi

"$CARGO_BIN" fmt --all -- --check
"$CARGO_BIN" clippy --workspace --all-targets -- -D warnings
"$CARGO_BIN" test --workspace
"$(dirname "$0")/check-file-size.sh"
