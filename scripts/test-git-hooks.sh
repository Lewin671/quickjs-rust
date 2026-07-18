#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/qjs-hook-test.XXXXXX")"
trap 'rm -rf "$TEMP_ROOT"' EXIT

REPOSITORY="$TEMP_ROOT/repository"
WORKTREE="$TEMP_ROOT/worktree"
RESULT="$TEMP_ROOT/check-root"

git init -q "$REPOSITORY"
git -C "$REPOSITORY" config user.name "Hook Test"
git -C "$REPOSITORY" config user.email "hook-test@example.invalid"
mkdir -p "$REPOSITORY/scripts"
cp "$ROOT_DIR/scripts/pre-push" "$REPOSITORY/scripts/pre-push"
chmod +x "$REPOSITORY/scripts/pre-push"

cat >"$REPOSITORY/scripts/check.sh" <<'CHECK'
#!/usr/bin/env bash
set -euo pipefail
: "${QJS_HOOK_TEST_RESULT:?}"
git rev-parse --show-toplevel >"$QJS_HOOK_TEST_RESULT"
nested="$(mktemp -d "${TMPDIR:-/tmp}/qjs-hook-nested.XXXXXX")"
trap 'rm -rf "$nested"' EXIT
git init -q "$nested"
: >"$nested/tracked"
git -C "$nested" add tracked
CHECK
chmod +x "$REPOSITORY/scripts/check.sh"

git -C "$REPOSITORY" add scripts
git -C "$REPOSITORY" commit -qm "Add hook fixture"
git -C "$REPOSITORY" worktree add -qb linked "$WORKTREE"
EXPECTED_WORKTREE="$(cd "$WORKTREE" && pwd -P)"

HOOK_PATH="$(git -C "$REPOSITORY" rev-parse --path-format=absolute --git-path hooks/pre-push)"
mkdir -p "$(dirname "$HOOK_PATH")"
ln -s "$REPOSITORY/scripts/pre-push" "$HOOK_PATH"
WORKTREE_GIT_DIR="$(git -C "$WORKTREE" rev-parse --git-dir)"

(
  cd "$WORKTREE"
  GIT_DIR="$WORKTREE_GIT_DIR" QJS_HOOK_TEST_RESULT="$RESULT" "$HOOK_PATH"
)

if [ "$(cat "$RESULT")" != "$EXPECTED_WORKTREE" ]; then
  echo "error: pre-push ran check from $(cat "$RESULT"), expected $EXPECTED_WORKTREE" >&2
  exit 1
fi
if [ "$(git -C "$REPOSITORY" config --bool core.bare)" != "false" ]; then
  echo "error: pre-push leaked repository-local Git state into nested tests" >&2
  exit 1
fi

echo "ok: pre-push isolates linked worktree checks"
