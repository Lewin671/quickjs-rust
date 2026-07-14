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
  files="$(list_files "$pattern" | filter_files "$filter_mode" "$filter_regex" | sort)"
  [ -n "$files" ] || return 0

  # One batched `wc -l` instead of one process per file; `wc` keeps argument
  # order, and xargs may emit one `total` line per batch.
  if ! printf '%s\n' "$files" | tr '\n' '\0' \
    | (cd "$ROOT_DIR" && xargs -0 wc -l) \
    | awk -v limit="$limit" -v label="$label" -v root="$ROOT_DIR/" '
        $2 == "total" { next }
        ($1 + 0) > limit {
          path = $0
          sub(/^[[:space:]]*[0-9]+[[:space:]]/, "", path)
          sub(root, "", path)
          printf "error: %s has %s lines; limit is %s for %s files\n", path, $1, limit, label > "/dev/stderr"
          bad = 1
        }
        END { exit bad }
      '; then
    status=1
  fi
}

check_limit "Rust source" 2000 '*.rs' exclude '/src/tests/'

check_limit "Rust test" 2000 '*.rs' include '^(.*/)?crates/.*/src/tests/'

check_limit "repository script" 2000 '*.sh'

check_limit "Python source" 800 '*.py' exclude '/tests/'

check_limit "Python test" 1200 '*.py' include '/tests/'

exit "$status"
