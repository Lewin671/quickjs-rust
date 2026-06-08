#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

status=0

list_files() {
  local pattern="$1"

  if command -v rg >/dev/null 2>&1; then
    rg --files "$ROOT_DIR" -g '!third_party/**' -g '!target/**' -g "$pattern"
    return
  fi

  (
    cd "$ROOT_DIR"
    git ls-files "$pattern" ':!:third_party/**' ':!:target/**'
  )
}

filter_files() {
  local mode="$1"
  local regex="$2"

  if [ -z "$mode" ] || [ -z "$regex" ]; then
    cat
  elif command -v rg >/dev/null 2>&1; then
    if [ "$mode" = "exclude" ]; then
      rg -v "$regex" || true
    else
      rg "$regex" || true
    fi
  elif [ "$mode" = "exclude" ]; then
    grep -Ev "$regex" || true
  else
    grep -E "$regex" || true
  fi
}

check_limit() {
  local label="$1"
  local limit="$2"
  local pattern="$3"
  local filter_mode="${4:-}"
  local filter_regex="${5:-}"

  local files
  files="$(list_files "$pattern" | filter_files "$filter_mode" "$filter_regex")"

  while IFS= read -r file; do
    [ -n "$file" ] || continue
    file="${file#"$ROOT_DIR/"}"
    local lines
    lines="$(wc -l <"$ROOT_DIR/$file" | tr -d ' ')"
    if [ "$lines" -gt "$limit" ]; then
      printf 'error: %s has %s lines; limit is %s for %s files\n' "$file" "$lines" "$limit" "$label" >&2
      status=1
    fi
  done < <(printf '%s\n' "$files" | sort)
}

check_limit "Rust source" 1000 '*.rs' exclude '/src/tests/'

check_limit "Rust test" 1000 '*.rs' include '^(.*/)?crates/.*/src/tests/'

check_limit "repository script" 1000 '*.sh'

exit "$status"
