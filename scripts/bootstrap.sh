#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$ROOT_DIR/scripts/lib.sh"

git -C "$ROOT_DIR" submodule update --init third_party/quickjs-ng third_party/test262

# Install git hooks from the repository. The pre-commit hook runs fast,
# change-aware checks; the pre-push hook runs the full local gate.
for hook in pre-commit pre-push; do
  HOOK_SRC="$ROOT_DIR/scripts/$hook"
  HOOK_DST="$ROOT_DIR/.git/hooks/$hook"
  if [ -f "$HOOK_SRC" ] && [ ! -e "$HOOK_DST" ]; then
    ln -sf "$HOOK_SRC" "$HOOK_DST"
    echo "Installed $hook hook."
  fi
done

if ! CARGO_BIN="$(qjs_resolve_cargo)"; then
  echo "warning: cargo not found; install Rust before running project checks" >&2
  exit 0
fi

(cd "$ROOT_DIR" && "$CARGO_BIN" fetch)
