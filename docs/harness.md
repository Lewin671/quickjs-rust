# Harness Runbook

How do I verify and integrate a change? `AGENTS.md` states the rules (commit
boundaries, ownership, definition of done); this file gives the mechanics,
and `scripts/README.md` indexes the scripts. Performance measurement has its
own procedure: [performance-workflow.md](performance-workflow.md).

## Local checks

Prerequisites: Rust (through `rustup`), `python3`, ripgrep (`rg`), `git`,
and `make`, `cmake` and a C compiler for the QuickJS-NG reference (built on
demand by `make -C third_party/quickjs-ng`). A fresh checkout or worktree
runs `./scripts/bootstrap.sh` once. It initialises the two top-level
submodules (network), links each git hook that is not already installed,
and runs `cargo fetch`.

- `./scripts/check-touched.sh --staged --explain`: before each commit.
  Checks selected from the staged paths.
- `./scripts/check-touched.sh --base <ref> --explain`: the same selection
  for an accumulated branch slice (`<ref>...HEAD`).
- `./scripts/check.sh`: the full gate, before handoff or push.
- `./scripts/check-ci.sh`: the hosted `check` job, for changes to CI,
  scripts or test scheduling.
- `./scripts/compare-qjs.sh`: the `tests/fixtures/compare-qjs/` fixtures
  against QuickJS-NG; not part of `check.sh`.

`check-touched.sh` selects by path: format, clippy and the file-size guard
for Rust, Cargo, script or workflow changes; the tests of each touched crate
(the whole workspace for Cargo changes); the benchmark-tool tests and
`check-unit` for benchmark files and performance plans; and Test262
allowlist slices mapped from runtime, parser and lexer paths. A change that
touches only Markdown or `docs/` exits early with no checks. When engine
files changed and no slice matched, it says so; run `check.sh`.

`check.sh` runs `cargo fmt --check`; clippy for the workspace, then with the
`agents` feature and with the `perf-counters` feature, each followed by that
feature's tests; the workspace tests; the benchmark-tool Python tests
(`tools/benchmark/tests`); `performance-decision.sh check-unit` over every
`tasks/performance-units/*.json` (listed with `rg`); the Test262 aggregate
tests (`scripts/tests`); `test-git-hooks.sh`; `bash -n` over the benchmark,
hook and check scripts; `check-file-size.sh`; and the Test262 subset. It
prints per-stage timings. Two switches change it:

- `QJS_CHECK_SKIP_TEST262=1` skips the subset stage.
- `QJS_CHECK_SPLIT_RUNTIME_TESTS=1` runs the `qjs-runtime` tests in several
  smaller processes instead of one `cargo test --workspace`.

`check-ci.sh` is `check.sh` with both switches set and
`RUST_TEST_THREADS=1`, which is what the hosted `check` job runs.

`check-file-size.sh` limits lines per file: 2000 for Rust and shell, 800 for
Python sources, 1200 for Python tests, 600 for top-level `tasks/*.md`.
`tasks/archive/` and `third_party/` are exempt.

**Git hooks.** `scripts/pre-commit` runs
`check-touched.sh --staged --explain`. `scripts/pre-push` runs the full
`check.sh` on every push. `QJS_SKIP_PRE_PUSH=1` makes the pre-push hook exit
without checking; the script states no condition for using it, and
`AGENTS.md` requires `check.sh` before a push either way. Hooks are stored in
the shared git directory, so every worktree of a clone uses them.

## Branches and worktrees

`main` is the integration branch. Before starting, update it
(`git pull --ff-only`), run `./scripts/check.sh`, and record
`git rev-parse HEAD` as the task's base sha. One agent works on a branch
named `agent/<task-slug>/<owner-id>`. Parallel owners each get a worktree:

```sh
./scripts/create-agent-worktree.sh <task-slug> <owner-id> [base-ref]
```

The script creates the branch at the base and a worktree at
`../<repo>-worktrees/wt-<repo>-<task-slug>-<owner-id>`, then runs
`bootstrap.sh` inside it, so it needs network access and initialises the
submodules there. It prints the handoff fields. Brief each owner with: task
and goal, base sha, branch, worktree path, allowed paths, forbidden paths,
and the verification command. Which files stay with the main agent, and when
to serialize instead of parallelizing, is defined in `AGENTS.md` ("Parallel
Agent Workflow").

Owners may build and test in parallel; `AGENTS.md` serializes performance
timing through the main agent. An owner's handoff reports branch, tip sha,
base sha, changed files, verification run, the CI run URL and status if
pushed, and residual risks.

**Integration.** The main agent integrates one branch at a time:

```sh
./scripts/validate-agent-branch.sh <branch> <base-sha> <allowed-path>...
git diff --stat <base-sha>..<branch>
git merge --no-ff <branch>
./scripts/check.sh
./scripts/compare-qjs.sh      # when the merge touches crates/qjs-runtime
```

`validate-agent-branch.sh` fails unless the branch descends from the base,
has changes, and changes only files under the allowed paths. For a pushed
branch, check its latest CI run first (see below).

**Cleanup.** `git worktree remove` refuses a worktree whose submodules are
initialised ("working trees containing submodules cannot be moved or
removed"), which is every worktree the script creates. Remove it this way:

```sh
rm -rf <worktree-path>
git worktree prune
git branch -d <branch>
```

**Failure handling.** If scope validation fails, do not merge; re-brief the
owner with the out-of-scope files, or re-baseline the task. If a check fails
after a merge, stop integrating until the target branch is fixed or restored.
Keep a failed branch or worktree only for diagnosis, and report it.

## Hosted CI

The workflow files under `.github/workflows/` are the source of truth.

- `ci.yml`: pull requests, and pushes to `main` and `agent/**`. Jobs
  `check`, `compare-qjs` and `test262-subset`.
- `test262-coverage.yml`: after each successful `CI` run, or manual. The
  full Test262 comparison and parity gate (below).
- `performance-smoke.yml`: pushes and pull requests to `main`, or manual.
  Described in `docs/benchmarking.md`.
- `release.yml`: `v*` tags.

`ci.yml` cancels an in-progress run when the same branch is pushed again, so
an older run shown as cancelled was superseded, not failed. Its `check` job
runs `check-ci.sh` and then builds the release CLI with the `agents` feature
and uploads it as `qjs-cli-release-linux`. Read results with:

```sh
gh run list --branch <branch> --limit 1
gh run view <run-id> --json status,conclusion,url,jobs
```

A red or unexplained latest run blocks integration. Green CI does not
replace the local checks.

## Test262

The language target and the pinned corpus are defined in `AGENTS.md` and
[conformance/README.md](conformance/README.md).

**Subset** (`scripts/test262-subset.sh`). Runs `tests/test262/allowlist.txt`.
An entry is a derived case under `tests/test262/cases/` or an upstream path
under `third_party/test262/test/`; upstream cases get `assert.js`, `sta.js`
and their metadata includes. Every entry in
`tests/test262/expected-failures.txt` must be allowlisted and carry a
reason; if one passes, the run fails until the stale entry is removed.
`--filter <prefix>` runs a slice. The per-case timeout is
`TEST262_CASE_TIMEOUT_SECONDS`: 10 by default, 30 under `check.sh`,
`check-touched.sh` and CI. `TEST262_JOBS` sets the worker count.

**Scan** (`scripts/test262-baseline.sh`). Enumerates the upstream corpus and
runs `--limit N` cases (default 50), everything with `--all`, a
`--filter test/<prefix>`, or a `--shard I/N`. `--engine both` also runs
QuickJS-NG and applies its config skips as the shared baseline. It runs
positive, negative, raw, async and module cases. The cases quickjs-rust does
not run are structural only:

- `_FIXTURE.js` files, which other cases import;
- `test/intl402/` and `test/staging/intl402/`;
- cases that include a harness file missing from the pinned checkout;
- three named SpiderMonkey staging stress patterns (`stress-timeout` in the
  script's `skip_reason`);
- `$262.agent` cases, unless `QJS_AGENTS=1` (below).

`QJS_CLI_BIN` reuses a prebuilt binary; otherwise the script builds one, in
release mode when `QJS_CLI_PROFILE=release`. `TEST262_CASE_TIMEOUT_SECONDS`
(default 10), `TEST262_TIMEOUT_RETRIES` (default 0; reruns only timeouts,
with the same timeout) and `TEST262_BASELINE_JOBS` (default: one per CPU)
tune a run. Load turns slow cases into timeouts: lower the job count when
scans share a machine, and re-check a timeout serially before treating it
as a gap. `--stop-after-limit` is for bounded probes, never for coverage
accounting. The harness never rewrites identifiers in a test's source,
because that changes observable name resolution.

**Agents and matching CI.** The `$262.agent` cases need OS threads, which the
engine provides only behind the cargo feature `agents`. With `QJS_AGENTS=1`
the scripts build the CLI with that feature and run those cases with
`--agent`; without it they are counted as not run. A prebuilt `QJS_CLI_BIN`
must itself have been built with `--features agents`. The hosted coverage
run uses a release binary and the settings below; `test262-coverage.yml` is
their source. A local scan comparable to it is:

```sh
QJS_AGENTS=1 QJS_CLI_PROFILE=release TEST262_TIMEOUT_RETRIES=1 \
  TEST262_CASE_TIMEOUT_SECONDS=20 ./scripts/find-qjsng-gaps.sh --exact --all
```

Use `QJS_AGENTS=1` only with `--exact` or `--filter`: the default sampled
probe builds its own CLI without the feature.

**Gap discovery** (`scripts/find-qjsng-gaps.sh`). Wraps the scan with
`--engine both`, writes the summary and per-case results under
`target/test262-gaps/`, and reports the cases QuickJS-NG passes and
quickjs-rust fails, with a ranked queue of areas. Its `--help` documents the
ranking strategies, probe sampling and tuning variables. The three modes
that matter:

- default, unfiltered `--all`: a sampled probe that recommends areas. It is
  not evidence that no gap exists.
- `--filter test/<prefix> --all`: exhaustive for one area. Run it before
  implementing and again before committing.
- `--exact --all`: the complete scan. Only this proves an exit condition or
  feeds the burndown.

`--from-report <dir>` or `--from-latest-report` recomputes a recommendation
from saved results without running anything. quickjs-rust timeouts are
listed apart from the gap list unless `--include-timeouts` is given.

**Coverage workflow.** `test262-coverage.yml` scans the commit of each
successful `CI` run, reusing that run's CLI artifact; a newer run on the
same branch cancels it. The quickjs-rust side runs as 16 groups of two
shards (32 in total); the QuickJS-NG side comes from a cache keyed on both
submodule commits and the scan scripts, or is rebuilt in 16 shards. The
`aggregate` job runs `scripts/test262-aggregate.py --require-complete-parity`
and fails when QuickJS-NG passes a case that quickjs-rust fails, times out
on, or does not run. It uploads two artifacts even then: `test262-burndown`
(one ledger line) and `test262-comparison-cases` (per-case JSONL with a
`comparison` bucket and an `actionable_gap` flag). Use the latter, not the
per-group `test262-coverage-*` artifacts, to pick follow-up areas.

**Burndown.** `scripts/test262-burndown.sh --entry <file>` appends a CI
`test262-burndown` artifact to `docs/conformance/burndown.jsonl`;
`--report <dir>` appends a local complete scan. It rejects partial or
filtered scans and a second entry for the same commit. Fields and
provenance: [conformance/README.md](conformance/README.md).

**Upstream amendments.** `scripts/test262-upstream-amendments.sh` backports
one upstream metadata fix (`tc39/test262@250f204f23a9`) to eleven TypedArray
cases: it excludes the immutable-ArrayBuffer factory, and every other
assertion still runs. Remove it when the submodule passes that commit.

**Untested harness logic.** `find-qjsng-gaps.sh` and `test262-baseline.sh`
hold real program logic (ranking, sampling, sharding, case construction) in
bash and awk with no tests of their own; only `test262-aggregate.py` has
unit tests (`scripts/tests/`).
