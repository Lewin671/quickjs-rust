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

Push the branch after it has a locally verified commit when early GitHub Actions
signal is useful:

```sh
git push -u origin agent/<task-slug>/<owner-id>
gh run list --branch agent/<task-slug>/<owner-id> --limit 1
gh run watch <run-id> --exit-status
```

Remote CI is an additional signal, not a replacement for local verification.
Do not merge or stack dependent work on a branch whose latest pushed CI run is
red or still unexplained.

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

When an owner produces a locally verified commit, they may push their feature
branch immediately to trigger branch CI while other owners continue in separate
worktrees. The main agent should poll or watch those runs with `gh`, record any
failed run URL in the handoff, and route fixes back to the same owner branch.

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
- pushed CI run URL and status, when the branch was pushed;
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
./scripts/microbench.sh
```

`scripts/test262-subset.sh` runs the curated Test262 allowlist. Allowlist
entries may point to local derived cases under `tests/test262/cases/` or pinned
upstream cases under `third_party/test262/test/`. Upstream entries are expanded
into temporary files with Test262 `assert.js`, `sta.js`, and metadata
`includes` before execution. Entries in `tests/test262/expected-failures.txt`
must also be in the allowlist and must include a reason. Expected-failure cases
may fail without failing the subset run; if one passes, the script fails and
asks for the stale entry to be removed. GitHub CI gives individual subset cases
a wider timeout than the local default to avoid false failures on shared
runners.

`scripts/test262-baseline.sh` scans upstream Test262 coverage. It can run a
bounded sample, a full scan with `--all`, a shard with `--shard I/N`, and a
quickjs-rust/QuickJS-NG comparison with `--engine both`. In that mode QuickJS-NG
config skips are applied as the shared baseline, and quickjs-rust unsupported
metadata is reported separately as a harness gap. The metadata helper supports
the inline and block-list forms used by Test262 `flags`, `includes`, and
`features` entries. Negative Test262 cases are runnable by the quickjs-rust
baseline harness; parse, early, runtime, and resolution failures are matched
against the Test262 negative metadata before being counted as expected results.
Raw Test262 cases run without injected harness files. Set `QJS_CLI_BIN` to
reuse a prebuilt quickjs-rust binary across multiple shard runs. The
`Test262 Coverage` GitHub Actions workflow runs once for each successful `CI`
commit, runs the sharded quickjs-rust scan and QuickJS-NG baseline in parallel,
uploads shard summaries, and aggregates the result into the workflow summary
without delaying the main CI workflow. The quickjs-rust scan uses 16 coverage
groups; each group runs two Test262 shards concurrently inside one runner to
reduce per-job tail latency while keeping the full 32-shard scan complete. The
workflow reuses a full QuickJS-NG baseline cache when available;
when that cache is missing, it falls back to sharded baseline jobs and saves a
full cache for later commits.

`scripts/microbench.sh` runs the repository's current QuickJS microbenchmark
subset from `tests/benchmarks/quickjs/microbench.js`. Use `--engine quickjs-ng`
or `--engine both` to compare the same subset against the pinned QuickJS-NG
reference, and pass benchmark name prefixes to narrow a run.

Do not integrate another owner branch until the target branch is green. For
pushed owner branches, also check the latest branch CI status before merging:

```sh
gh run list --branch <branch> --limit 1
gh run view <run-id> --json status,conclusion,url,jobs
```

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
