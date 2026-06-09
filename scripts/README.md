# Script Guide

Most local development should use `./scripts/check.sh` as the final gate.
The other scripts are focused tools for comparison, conformance work,
maintenance, or agent workflow isolation.

## Daily Checks

- `check.sh`: Runs the standard project gate: formatting, clippy, workspace
  tests, and file-size limits.
- `compare-qjs.sh`: Runs selected local fixtures against both quickjs-rust and
  the pinned QuickJS-NG reference.
- `find-qjsng-gaps.sh`: Runs a QuickJS-NG comparison baseline and prints the
  upstream Test262 cases where QuickJS-NG passes but quickjs-rust does not.
  Use this as the first agent entrypoint for discovering the next conformance
  gap to implement. It prints the greedy next area by default.
- `test262-subset.sh`: Runs the curated Test262 allowlist. Entries may point to
  local derived cases under `tests/test262/cases/` or pinned upstream cases
  under `third_party/test262/test/`. Upstream entries are executed with the
  standard Test262 harness files declared by their metadata. Cases listed in
  `tests/test262/expected-failures.txt` may fail, but passing expected failures
  are reported as stale entries.

## Focused Development

- `bootstrap.sh`: Initializes submodules for a fresh checkout.
- `microbench.sh`: Runs the current QuickJS microbenchmark subset, optionally
  against QuickJS-NG.
- `source-size-report.sh`: Reports large first-party source files. Use
  `--vendor` only when inspecting pinned upstream reference files.

## Test262 Exploration

- `find-qjsng-gaps.sh`: Friendly gap-discovery wrapper around
  `test262-baseline.sh --engine both`. It writes raw `summary.json`,
  `cases.jsonl`, `qjsng-pass-rust-nonpass.tsv`, and `recommendations.tsv` files
  under `target/test262-gaps/` by default, then prints a short summary, top gap
  areas, and first actionable cases. Use `--filter test/<prefix>` to focus on
  one subsystem and `--all` when a full focused scan is useful. Unfiltered
  `--all` uses a concurrent multi-shard greedy probe by default, then selects a
  next area by preferring small batches of real runtime or parser failures
  before pure harness skips; if no small engine batch is available, it recommends
  a small harness batch before falling back to a larger engine batch. Tune it
  with `--strategy fast|largest`, `--recommend-batch-cap N`, `--probe-limit N`,
  and `--probe-shards I/N[,I/N...]`; use `--probe-shard I/N` for a single very
  fast probe, and use `--exact --all` for a complete audit. Use
  `--from-report PATH` or
  `--from-latest-report` to replay a saved `cases.jsonl` and recompute the
  recommendation without rerunning Test262; add `--skip-area test/<prefix>` to
  ignore an area already being worked. Use `--no-recommend` to suppress the
  recommendation.
- `test262-baseline.sh`: Samples or scans upstream Test262 files to classify
  structural not-run cases, parser or runtime failures, and timeouts before adding
  curated local cases. Use `--engine both --all --shard I/N --summary-json PATH
  --no-fail` for asynchronous comparisons against QuickJS-NG's Test262 config.
  In that mode QuickJS-NG config skips are applied as the shared baseline, and
  quickjs-rust skips only structural harness limits such as modules, async tests,
  unsupported includes, intl402, fixtures, and known unsupported source syntax.
  Test262 `features` metadata is parsed for QuickJS-NG config alignment, but it
  does not preemptively skip quickjs-rust cases.
  Negative quickjs-rust cases must fail with the expected Test262 phase and
  error type. Set `QJS_CLI_BIN` to reuse a prebuilt quickjs-rust binary across
  multiple shard runs.
- `test262-baseline-metadata.awk`: Internal helper used by
  `test262-baseline.sh` to read Test262 metadata blocks, including inline and
  block-list `flags`, `includes`, and `features` entries.

## Agent Workflow

- `create-agent-worktree.sh`: Creates an isolated `agent/**` branch and
  worktree for parallel work.
- `validate-agent-branch.sh`: Checks that an agent branch started from the
  expected base and stayed within its allowed path boundary.

## Internal Helpers

- `check-file-size.sh`: Enforces repository file-size limits. It is called by
  `check.sh`.
- `run-with-timeout.sh`: Runs a command with a timeout. It is shared by
  comparison, benchmark, and Test262 scripts.
