#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
QJS_DIR="$ROOT_DIR/third_party/quickjs-ng"
QJS_BIN="$QJS_DIR/build/qjs"
FIXTURE_DIR="${1:-$ROOT_DIR/tests/fixtures/compare-qjs}"

if [ ! -d "$QJS_DIR" ]; then
  echo "error: missing $QJS_DIR; run ./scripts/bootstrap.sh first" >&2
  exit 1
fi

if [ ! -x "$QJS_BIN" ]; then
  make -C "$QJS_DIR" all
fi

if command -v cargo >/dev/null 2>&1; then
  CARGO_BIN="cargo"
elif [ -x "$HOME/.cargo/bin/cargo" ]; then
  CARGO_BIN="$HOME/.cargo/bin/cargo"
else
  echo "error: cargo not found; install Rust with rustup before comparing" >&2
  exit 127
fi

normalize_rust_value() {
  sed -E \
    -e 's/^Number\(([0-9]+)\.0\)$/\1/' \
    -e 's/^Number\(([-]?[0-9]+(\.[0-9]+)?)\)$/\1/' \
    -e 's/^String\("(.*)"\)$/\1/' \
    -e 's/^Boolean\((true|false)\)$/\1/' \
    -e 's/^Null$/null/' \
    -e 's/^Undefined$/undefined/'
}

shopt -s nullglob
fixtures=("$FIXTURE_DIR"/*.js)

if [ "${#fixtures[@]}" -eq 0 ]; then
  echo "error: no .js fixtures found in $FIXTURE_DIR" >&2
  exit 1
fi

for fixture in "${fixtures[@]}"; do
  expression="$(tr '\n' ' ' < "$fixture")"
  rust_output="$("$CARGO_BIN" run -q -p qjs-cli -- -e "$expression" | normalize_rust_value)"
  qjs_output="$("$QJS_BIN" -e "console.log($expression)")"

  if [ "$rust_output" != "$qjs_output" ]; then
    echo "mismatch: $fixture" >&2
    echo "  quickjs-rust: $rust_output" >&2
    echo "  quickjs-ng:   $qjs_output" >&2
    exit 1
  fi

  echo "ok: $fixture => $rust_output"
done
