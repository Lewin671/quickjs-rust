#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIMIT="30"
INCLUDE_VENDOR=0

for arg in "$@"; do
  case "$arg" in
    --vendor)
      INCLUDE_VENDOR=1
      ;;
    ''|*[!0-9]*)
      printf 'usage: %s [limit] [--vendor]\n' "$0" >&2
      exit 2
      ;;
    *)
      LIMIT="$arg"
      ;;
  esac
done

report_git_files() {
  local title="$1"

  printf '%s\n' "$title"
  git -C "$ROOT_DIR" ls-files -z |
    while IFS= read -r -d '' file; do
      case "$file" in
        third_party/*|target/*|tests/test262/cases/*)
          continue
          ;;
      esac

      case "$file" in
        *.rs|*.toml|*.md|*.sh|*.yml|*.yaml|*.js|*.c|*.h|*.json)
          ;;
        *)
          continue
          ;;
      esac

      if [ ! -f "$ROOT_DIR/$file" ]; then
        continue
      fi

      local lines
      lines="$(wc -l <"$ROOT_DIR/$file" | tr -d ' ')"
      printf '%8s %s\n' "$lines" "$file"
    done |
    sort -nr |
    awk -v limit="$LIMIT" 'NR <= limit'
  printf '\n'
}

report_vendor_files() {
  printf 'Vendored reference files:\n'

  find "$ROOT_DIR/third_party" -mindepth 1 -maxdepth 1 -type d -print0 |
    while IFS= read -r -d '' repo; do
      local prefix
      prefix="${repo#"$ROOT_DIR/"}/"
      (
        cd "$repo"
        git ls-files -z -- \
          '*.rs' '*.toml' '*.md' '*.sh' '*.yml' '*.yaml' '*.js' '*.c' '*.h' '*.json' |
          xargs -0 wc -l
      ) |
        awk -v prefix="$prefix" '$2 != "total" { printf "%8s %s%s\n", $1, prefix, $2 }'
    done |
    sort -nr |
    awk -v limit="$LIMIT" 'NR <= limit'
  printf '\n'
}

report_git_files "First-party source, docs, and scripts:"

if [ "$INCLUDE_VENDOR" -eq 1 ]; then
  report_vendor_files
else
  printf 'Vendored reference files:\n'
  printf '  skipped by default; run %s [limit] --vendor to scan pinned upstream files\n' "$0"
fi
