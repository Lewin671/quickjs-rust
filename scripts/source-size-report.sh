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
  local mode="$2"

  printf '%s\n' "$title"
  git -C "$ROOT_DIR" ls-files -z |
    while IFS= read -r -d '' file; do
      case "$mode:$file" in
        first:third_party/*|first:target/*|first:tests/test262/cases/*)
          continue
          ;;
        vendor:third_party/*)
          ;;
        vendor:*)
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

      local lines
      lines="$(wc -l <"$ROOT_DIR/$file" | tr -d ' ')"
      printf '%8s %s\n' "$lines" "$file"
    done |
    sort -nr |
    head -n "$LIMIT"
  printf '\n'
}

report_git_files "First-party source, docs, and scripts:" first

if [ "$INCLUDE_VENDOR" -eq 1 ]; then
  report_git_files "Vendored reference files:" vendor
else
  printf 'Vendored reference files:\n'
  printf '  skipped by default; run %s [limit] --vendor to scan pinned upstream files\n' "$0"
fi
