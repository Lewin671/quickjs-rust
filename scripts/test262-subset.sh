#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST262_DIR="$ROOT_DIR/third_party/test262"
ALLOWLIST="$ROOT_DIR/tests/test262/allowlist.txt"
EXPECTED_FAILURES="$ROOT_DIR/tests/test262/expected-failures.txt"
LOCAL_CASE_DIR="$ROOT_DIR/tests/test262"
CARGO_BIN="${CARGO:-cargo}"
if ! command -v "$CARGO_BIN" >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
  CARGO_BIN="$HOME/.cargo/bin/cargo"
fi

if [ ! -d "$TEST262_DIR" ]; then
  echo "error: missing $TEST262_DIR; run ./scripts/bootstrap.sh first" >&2
  exit 1
fi

if [ ! -f "$ALLOWLIST" ]; then
  echo "error: missing $ALLOWLIST" >&2
  exit 1
fi

if [ ! -f "$EXPECTED_FAILURES" ]; then
  echo "error: missing $EXPECTED_FAILURES" >&2
  exit 1
fi

allowlist_count=0
while IFS= read -r line; do
  entry="${line%%#*}"
  entry="$(echo "$entry" | xargs)"
  [ -z "$entry" ] && continue

  allowlist_count=$((allowlist_count + 1))
  case "$entry" in
    /*|*..*)
      echo "error: allowlist entry must be a relative path under tests/test262: $entry" >&2
      exit 1
      ;;
  esac

  case_path="$LOCAL_CASE_DIR/$entry"
  if [ ! -f "$case_path" ]; then
    echo "error: allowlist entry does not exist: tests/test262/$entry" >&2
    exit 1
  fi

  derived_from="$(sed -n 's#^// Derived from: ##p' "$case_path" | head -n 1)"
  if [ -z "$derived_from" ]; then
    echo "error: missing Test262 provenance in tests/test262/$entry" >&2
    exit 1
  fi
  if [ ! -f "$TEST262_DIR/$derived_from" ]; then
    echo "error: derived Test262 source does not exist: $derived_from" >&2
    exit 1
  fi

  "$CARGO_BIN" run -q -p qjs-cli -- "$case_path" >/dev/null
done < "$ALLOWLIST"

while IFS= read -r line; do
  trimmed="$(echo "$line" | xargs)"
  [ -z "$trimmed" ] && continue
  case "$trimmed" in
    \#*) continue ;;
  esac

  if [[ "$line" != *"#"* ]]; then
    echo "error: expected failure is missing a reason: $line" >&2
    exit 1
  fi

  entry="${line%%#*}"
  entry="$(echo "$entry" | xargs)"
  if [ ! -f "$LOCAL_CASE_DIR/$entry" ]; then
    echo "error: expected failure entry does not exist: tests/test262/$entry" >&2
    exit 1
  fi
done < "$EXPECTED_FAILURES"

if [ "$allowlist_count" -eq 0 ]; then
  echo "error: Test262 allowlist is empty; add at least one runnable subset case" >&2
  exit 1
fi

echo "ok: ran $allowlist_count Test262 subset cases"
