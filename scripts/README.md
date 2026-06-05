# Script Guide

Most local development should use `./scripts/check.sh` as the final gate.
The other scripts are focused tools for comparison, conformance work,
maintenance, or agent workflow isolation.

## Daily Checks

- `check.sh`: Runs the standard project gate: formatting, clippy, workspace
  tests, and file-size limits.
- `compare-qjs.sh`: Runs selected local fixtures against both quickjs-rust and
  the pinned QuickJS-NG reference.
- `test262-subset.sh`: Runs the curated local Test262-derived allowlist. Cases
  listed in `tests/test262/expected-failures.txt` may fail, but passing expected
  failures are reported as stale entries.

## Focused Development

- `bootstrap.sh`: Initializes submodules for a fresh checkout.
- `microbench.sh`: Runs the current QuickJS microbenchmark subset, optionally
  against QuickJS-NG.
- `source-size-report.sh`: Reports large first-party source files. Use
  `--vendor` only when inspecting pinned upstream reference files.

## Test262 Exploration

- `test262-baseline.sh`: Samples or scans upstream Test262 files to classify
  unsupported metadata, parser or runtime failures, and timeouts before adding
  curated local cases. Use `--engine both --all --shard I/N --summary-json PATH
  --no-fail` for asynchronous comparisons against QuickJS-NG's Test262 config.
  In that mode QuickJS-NG config skips are applied as the shared baseline, and
  quickjs-rust unsupported metadata is reported as a separate harness gap.
- `test262-baseline-metadata.awk`: Internal helper used by
  `test262-baseline.sh` to read Test262 metadata blocks.

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
