# Harness Runbook

This runbook defines the expected operating model for autonomous agents working
on this repository.

Known structural risks in this operating model are recorded in
`docs/harness-convergence-risks.md`.

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
./scripts/find-qjsng-gaps.sh --filter test/built-ins/String --limit 100
./scripts/test262-subset.sh
./scripts/microbench.sh
```

`scripts/find-qjsng-gaps.sh` is the first-choice entrypoint for discovering
behavior supported by the pinned QuickJS-NG reference but not yet supported by
quickjs-rust. It runs `scripts/test262-baseline.sh --engine both`, stores the
raw summary and case results under `target/test262-gaps/`, and prints a compact
report with the total QuickJS-NG-pass/quickjs-rust actionable gap count, the
split between runtime failures, included timeouts, excluded stress timeouts, and
quickjs-rust not-run cases, top affected areas, and the first cases to
investigate. Use
`--filter test/<prefix>` to focus the scan on one Test262 subtree and `--all`
when the focused scan should be exhaustive. It prints a greedy next area by
default. For unfiltered `--all` recommendation runs, the default is a bounded
greedy probe over `TEST262_GAP_PROBE_LIMIT` cases, currently 100, from four
shards of 16. When no explicit selection is given via `--probe-shards` or
`TEST262_GAP_PROBE_SHARDS`, the default shard set rotates one step per probe
run (state in `target/test262-gaps/.probe-rotation`), so four consecutive
probes sweep all 16 shards instead of resampling the same files. Those
probe shards run concurrently, and the report merges their case results before
ranking areas. This gives the agent broader Test262 coverage per iteration
without paying for a full audit or biasing entirely toward the first sorted
Test262 directories. After that sampled candidate queue is built, the default
global probe exact-checks the top `TEST262_GAP_VERIFY_CANDIDATES` areas,
currently 8. These focused verification runs execute concurrently, then the
script prints the final recommendation and a greedy queue from those exact
focused results. To keep the default greedy loop fast, exact verification skips
candidate subtrees with more than `TEST262_GAP_VERIFY_AREA_MAX_FILES`
JavaScript files, currently 150. Broad skipped areas remain visible in the raw
probe queue, but they do not block the verified recommendation. This costs a
few additional focused baseline runs, prevents a sampled broad area from hiding
smaller parity wins, and gives the agent several ready follow-up areas from one
global probe.
The default recommendation strategy is quickwins greedy. It prefers real
quickjs-rust engine failures when they fit in a small reviewable batch. After
that, it prefers small harness-only batches when at least one case does not
carry a hard-feature hint, because those mixed batches are often faster to
verify or clear than a broad semantic area. Harness-only batches where every
case is hard-hinted remain visible, but they rank below mixed quick wins. It
also computes `hard hints` from Test262 feature metadata, paths, and skip metadata
that usually imply larger missing features, such as async, destructuring, class,
yield, proxy/realm/species behavior, resizable or growable buffers, or Annex B
global-code semantics. The ranking weights those hints by expected breadth, so
resizable or growable buffers, async, and Annex B global-code work rank below
narrower realm/proxy/species failures when the engine-gap count is otherwise
similar. Those hints do not hide gaps; they only lower an area's default ranking
so an agent can find reviewable parity wins before getting stuck on known broad
features. Use
`--strategy fast` for the older small-batch-first behavior,
`--strategy largest` to restore largest-gap-first recommendation, or
`--recommend-batch-cap N` to tune how large a default batch may be. Use
`--recommend-queue N` or `TEST262_GAP_RECOMMEND_QUEUE` to tune how many ranked
areas the report prints for continuous or parallel follow-up work. Use
`--verify-candidates N` to tune the exact candidate follow-up,
`--verify-area-max-files N` to tune how wide a subtree may be before default
verification skips it, or `--verify-candidates 0` when the fastest possible
sampled recommendation is more useful than a verified one.
After a global probe has produced a candidate queue, use
`--from-report target/test262-gaps/<run>` or `--from-latest-report` to recompute
the recommendation from the saved `cases.jsonl` without executing Test262
again. Pair it with `--skip-area test/<prefix>` when the current top area is
already being worked or has just been rechecked, so the agent can pick the next
largest known gap immediately. This replay mode is only a planning shortcut:
focused verification still needs `--filter test/<prefix> --all`, and final
completion still needs `--exact --all`.
Use `--exact --all` when the task needs a complete report or when a probe finds
no gaps and the agent needs to prove the exit condition. Under the default
quickwins strategy, mixed harness-only areas remain first-class candidates when
they are small enough to check quickly, and broad-feature areas remain visible
in the candidate queue with their `hard` count. Stress timeouts are excluded
from the default actionable gap list so large conformance stress loops do not
hide missing behavior; use
`--include-timeouts` when performance parity is the task. Use `--probe-limit N`
and `--probe-shards I/N[,I/N...]` to tune recommendation speed versus
confidence; `--probe-shard I/N` remains as a single-shard shorthand for very
fast local checks. Use `--no-recommend` when only the raw gap report is needed.
Treat one recommended queue area as the smallest useful planning and commit
boundary unless the area is too broad to review as one change. When several
queue entries touch independent subsystems, agents may work them as separate
reviewable units after focused `--filter ... --all` verification; avoid
splitting follow-up work into one commit per individual Test262 case.

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
config skips are applied as the shared baseline. The quickjs-rust side reports
not-run cases only for structural harness limits such as modules, async tests,
unsupported harness includes, intl402, fixtures, and known unsupported source
syntax. Test262
`features` metadata is parsed for QuickJS-NG config alignment, but it does not
preemptively skip quickjs-rust cases; runnable cases produce normal pass, fail,
or timeout signal. Negative Test262 cases are runnable by the quickjs-rust
baseline harness; parse, early, runtime, and resolution failures are matched
against the Test262 negative metadata before being counted as expected results.
Raw Test262 cases run without injected harness files. `--stop-after-limit` is
reserved for bounded probe callers such as `find-qjsng-gaps.sh`; do not use it
for coverage accounting because it stops enumeration once the run limit is
reached. Set `QJS_CLI_BIN` to reuse a prebuilt quickjs-rust binary across
multiple shard runs. The
`Test262 Coverage` GitHub Actions workflow runs once for each successful `CI`
commit, runs the sharded quickjs-rust scan and QuickJS-NG baseline in parallel,
uploads shard summaries, and aggregates the result into the workflow summary
without delaying the main CI workflow. CI uploads the checked commit's
`qjs-cli` debug binary, and coverage jobs reuse that artifact instead of
rebuilding the runner binary on every shard group. The quickjs-rust scan uses
16 coverage groups; each group runs two Test262 shards concurrently inside one
runner to fit the two-core GitHub-hosted runner shape while keeping the full
32-shard scan complete. The
workflow reuses a full QuickJS-NG baseline cache when available;
when that cache is missing, it falls back to sharded baseline jobs and saves a
full cache for later commits.

`scripts/test262-burndown.sh` records the conformance burndown time series in
`docs/conformance/burndown.jsonl`. Use `--report DIR` with the output of a
complete local scan (sharded `test262-baseline.sh --all --engine both` runs or
`find-qjsng-gaps.sh --exact --all`), or `--entry FILE` with the
`test262-burndown` artifact that the Test262 Coverage workflow uploads for
each aggregated commit. The script refuses partial or filtered scans so every
entry in the series is comparable. Record an entry after each full exact scan
and when a CI aggregate provides a fresh per-commit measurement; the
`comparison.actionable_gap` and `comparison.ng_pass_rust_not_run` trends are
the convergence signal that decides when the recommendation strategy or
campaign priorities should change. See `docs/conformance/README.md` for the
schema.

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
