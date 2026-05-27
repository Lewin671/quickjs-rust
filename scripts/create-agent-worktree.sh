#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: scripts/create-agent-worktree.sh <task-slug> <owner-id> [base-ref]

Creates an isolated git worktree for one coding owner.

Examples:
  scripts/create-agent-worktree.sh lexer-comments a1
  scripts/create-agent-worktree.sh parser-expressions a2 main
USAGE
}

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
  usage
  exit 2
fi

TASK_SLUG="$1"
OWNER_ID="$2"
BASE_REF="${3:-HEAD}"

case "$TASK_SLUG" in
  *[!a-z0-9._-]* | "" )
    echo "error: task slug must contain only lowercase letters, numbers, dot, underscore, or dash" >&2
    exit 2
    ;;
esac

case "$OWNER_ID" in
  *[!a-z0-9._-]* | "" )
    echo "error: owner id must contain only lowercase letters, numbers, dot, underscore, or dash" >&2
    exit 2
    ;;
esac

ROOT_DIR="$(git rev-parse --show-toplevel)"
REPO_NAME="$(basename "$ROOT_DIR")"
BASE_SHA="$(git rev-parse "$BASE_REF")"
BRANCH="agent/$TASK_SLUG/$OWNER_ID"
WORKTREE_ROOT="$(dirname "$ROOT_DIR")/${REPO_NAME}-worktrees"
WORKTREE_DIR="$WORKTREE_ROOT/wt-${REPO_NAME}-${TASK_SLUG}-${OWNER_ID}"

if git show-ref --verify --quiet "refs/heads/$BRANCH"; then
  echo "error: branch already exists: $BRANCH" >&2
  exit 1
fi

if [ -e "$WORKTREE_DIR" ]; then
  echo "error: worktree path already exists: $WORKTREE_DIR" >&2
  exit 1
fi

mkdir -p "$WORKTREE_ROOT"
git worktree add -b "$BRANCH" "$WORKTREE_DIR" "$BASE_SHA"

(
  cd "$WORKTREE_DIR"
  ./scripts/bootstrap.sh
)

cat <<EOF
created worktree
  branch:   $BRANCH
  base sha: $BASE_SHA
  path:     $WORKTREE_DIR

handoff fields
  Base sha: $BASE_SHA
  Branch: $BRANCH
  Worktree: $WORKTREE_DIR
  Owner id: $OWNER_ID
EOF
