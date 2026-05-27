#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: scripts/validate-agent-branch.sh <branch> <base-sha> <allowed-path>...

Validates that an agent branch descends from the recorded base sha and only
changes files under the declared path boundaries.

Examples:
  scripts/validate-agent-branch.sh agent/lexer-comments/a1 abc123 crates/qjs-lexer tests/fixtures
USAGE
}

if [ "$#" -lt 3 ]; then
  usage
  exit 2
fi

BRANCH="$1"
BASE_SHA="$2"
shift 2
ALLOWED_PATHS=("$@")

git rev-parse --verify "$BRANCH" >/dev/null
git rev-parse --verify "$BASE_SHA^{commit}" >/dev/null

if ! git merge-base --is-ancestor "$BASE_SHA" "$BRANCH"; then
  echo "error: $BASE_SHA is not an ancestor of $BRANCH" >&2
  exit 1
fi

changed_files=()
while IFS= read -r file; do
  changed_files+=("$file")
done < <(git diff --name-only "$BASE_SHA..$BRANCH")

if [ "${#changed_files[@]}" -eq 0 ]; then
  echo "error: branch has no changes relative to $BASE_SHA" >&2
  exit 1
fi

out_of_scope=()
for file in "${changed_files[@]}"; do
  in_scope=0
  for allowed in "${ALLOWED_PATHS[@]}"; do
    allowed="${allowed%/}"
    if [[ "$file" == "$allowed" || "$file" == "$allowed/"* ]]; then
      in_scope=1
      break
    fi
  done

  if [ "$in_scope" -eq 0 ]; then
    out_of_scope+=("$file")
  fi
done

if [ "${#out_of_scope[@]}" -ne 0 ]; then
  echo "error: branch changes files outside declared boundaries:" >&2
  printf '  %s\n' "${out_of_scope[@]}" >&2
  exit 1
fi

echo "ok: $BRANCH descends from $BASE_SHA"
echo "ok: changed files are within declared boundaries"
printf '%s\n' "${changed_files[@]}"
