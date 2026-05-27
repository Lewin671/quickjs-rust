# Harness Runbook

This runbook defines the expected operating model for autonomous agents working
on this repository.

## Baseline

`main` is the stable integration branch. Before starting parallel work:

```sh
git switch main
git pull --ff-only
./scripts/bootstrap.sh
./scripts/check.sh
git rev-parse HEAD
```

Record the resulting commit as the task `base sha`.

## Single-Agent Task

Use a normal feature branch when only one agent is editing:

```sh
git switch -c agent/<task-slug>/<owner-id>
./scripts/bootstrap.sh
```

Keep the commit focused on one reviewable unit. Run the task-specific checks and
then `./scripts/check.sh`.

## Parallel Task

Use isolated worktrees only when ownership boundaries are clear:

```sh
./scripts/create-agent-worktree.sh <task-slug> <owner-id> <base-sha>
```

Each owner must receive:

- task id and goal;
- base sha;
- branch name;
- worktree path;
- allowed paths;
- forbidden paths;
- verification command.

Global files stay main-agent owned unless explicitly assigned:

- `Cargo.toml`
- `Cargo.lock`
- `rust-toolchain.toml`
- `.gitmodules`
- `AGENTS.md`
- `README.md`
- `docs/architecture.md`
- shared scripts and CI files

## Owner Handoff

Each coding owner must report:

- branch name;
- tip commit sha;
- base sha used;
- changed files;
- verification run;
- residual risks.

## Integration

The main agent integrates one branch at a time.

Before merging, validate branch scope:

```sh
./scripts/validate-agent-branch.sh <branch> <base-sha> <allowed-path>...
```

Then inspect and integrate:

```sh
git diff --stat <base-sha>..<branch>
git merge --no-ff <branch>
./scripts/check.sh
```

Run additional checks when relevant:

```sh
./scripts/compare-qjs.sh
./scripts/test262-subset.sh
```

Do not integrate another owner branch until the target branch is green.

## Cleanup

After a branch is merged and verified:

```sh
git worktree remove <worktree-path>
git branch -d <branch>
```

Retain failed branches or worktrees only when they are needed for diagnosis, and
record that residual state in the final report.

## Failure Handling

If validation fails, do not merge. Re-brief the owner with the out-of-scope
files or re-baseline the task.

If post-merge verification fails, stop integrating other branches. Keep the
failing result isolated until the failure is understood and the target branch is
restored or fixed.
