#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST262_DIR="$ROOT_DIR/third_party/test262"
ALLOWLIST="$ROOT_DIR/tests/test262/allowlist.txt"
EXPECTED_FAILURES="$ROOT_DIR/tests/test262/expected-failures.txt"

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
  if [ ! -f "$TEST262_DIR/$entry" ]; then
    echo "error: allowlist entry does not exist in Test262: $entry" >&2
    exit 1
  fi
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
  if [ ! -f "$TEST262_DIR/$entry" ]; then
    echo "error: expected failure entry does not exist in Test262: $entry" >&2
    exit 1
  fi
done < "$EXPECTED_FAILURES"

if [ "$allowlist_count" -eq 0 ]; then
  echo "ok: Test262 allowlist is empty; no subset tests selected yet"
else
  echo "ok: validated $allowlist_count Test262 allowlist entries"
fi
