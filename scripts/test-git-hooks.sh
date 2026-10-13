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
cp "$ROOT_DIR/scripts/pre-commit" "$REPOSITORY/scripts/pre-commit"
chmod +x "$REPOSITORY/scripts/pre-push" "$REPOSITORY/scripts/pre-commit"
: >"$REPOSITORY/scripts/lib.sh"

cat >"$REPOSITORY/scripts/check.sh" <<'CHECK'
#!/usr/bin/env bash
set -euo pipefail
: "${QJS_HOOK_TEST_RESULT:?}"
: "${QJS_HOOK_TEST_ROOT:?}"
for variable in GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY; do
  if printenv "$variable" >/dev/null; then
    echo "error: nested check inherited $variable" >&2
    exit 1
  fi
done
script_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
test "$script_root" = "$QJS_HOOK_TEST_ROOT"
git rev-parse --show-toplevel >"$QJS_HOOK_TEST_RESULT"
nested="$(mktemp -d "${TMPDIR:-/tmp}/qjs-hook-nested.XXXXXX")"
trap 'rm -rf "$nested"' EXIT
# An inherited GIT_DIR turns this into a reinitialization of the outer repo.
git init --bare -q "$nested"
CHECK
chmod +x "$REPOSITORY/scripts/check.sh"
cp "$REPOSITORY/scripts/check.sh" "$REPOSITORY/scripts/check-touched.sh"

git -C "$REPOSITORY" add scripts
git -C "$REPOSITORY" commit -qm "Add hook fixture"
git -C "$REPOSITORY" worktree add -qb linked "$WORKTREE"
EXPECTED_WORKTREE="$(cd "$WORKTREE" && pwd -P)"

WORKTREE_GIT_DIR="$(git -C "$WORKTREE" rev-parse --git-dir)"

for hook in pre-push pre-commit; do
  HOOK_PATH="$(git -C "$REPOSITORY" rev-parse --path-format=absolute --git-path "hooks/$hook")"
  mkdir -p "$(dirname "$HOOK_PATH")"
  ln -s "$REPOSITORY/scripts/$hook" "$HOOK_PATH"

  (
    cd "$WORKTREE"
    GIT_DIR="$WORKTREE_GIT_DIR" GIT_INDEX_FILE="$WORKTREE_GIT_DIR/index" \
      QJS_HOOK_TEST_RESULT="$RESULT" QJS_HOOK_TEST_ROOT="$EXPECTED_WORKTREE" "$HOOK_PATH"
  )

  if [ "$(cat "$RESULT")" != "$EXPECTED_WORKTREE" ]; then
    echo "error: $hook ran check from $(cat "$RESULT"), expected $EXPECTED_WORKTREE" >&2
    exit 1
  fi
  if [ "$(git -C "$REPOSITORY" config --bool core.bare)" != "false" ]; then
    echo "error: $hook leaked repository-local Git state into nested tests" >&2
    exit 1
  fi
  echo "ok: $hook isolates linked worktree checks"
done

# Exercise Git's real hook invocation, including its exported local variables.
: >"$WORKTREE/commit-probe"
git -C "$WORKTREE" add commit-probe
QJS_HOOK_TEST_RESULT="$RESULT" QJS_HOOK_TEST_ROOT="$EXPECTED_WORKTREE" \
  git -C "$WORKTREE" commit -qm "Exercise linked commit hook"
test "$(cat "$RESULT")" = "$EXPECTED_WORKTREE"
test "$(git -C "$REPOSITORY" config --bool core.bare)" = false
git -C "$WORKTREE" cat-file -e HEAD:commit-probe
echo "ok: a real linked worktree commit preserves repository metadata"
