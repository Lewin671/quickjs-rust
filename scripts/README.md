# Script Guide

One-line index of repository scripts. `./scripts/check.sh` is the final gate
for local development. Detailed flags and operating guidance for the Test262
and gap-discovery scripts live in `docs/harness.md`; this file only says what
each script is for.

## Daily Checks

- `check.sh`: Standard project gate: formatting, clippy, workspace tests, and
  file-size limits.
- `compare-qjs.sh`: Runs `tests/fixtures/compare-qjs/` fixtures against both
  quickjs-rust and the pinned QuickJS-NG reference.
- `find-qjsng-gaps.sh`: First agent entrypoint for conformance work. Wraps
  `test262-baseline.sh --engine both`, reports actionable gaps, and prints a
  greedy recommendation queue. See `docs/harness.md` for strategies, probe
  tuning, and replay flags.
- `test262-subset.sh`: Runs the curated Test262 allowlist
  (`tests/test262/allowlist.txt`); passing expected failures are reported as
  stale entries.

## Conformance Measurement

- `test262-baseline.sh`: Samples or scans upstream Test262, classifying
  structural not-run cases, failures, and timeouts; `--engine both` compares
  against QuickJS-NG under its own test262 config. Set `QJS_CLI_BIN` to reuse
  a prebuilt binary across shards.
- `test262-aggregate.py`: Aggregates QuickJS-NG and quickjs-rust case-result
  JSONL sets into the CI coverage summary and the schema-1 burndown entry.
  Used by the Test262 Coverage workflow; also runs locally on artifacts
  downloaded with `gh run download`.
- `test262-burndown.sh`: Appends a complete-scan entry to
  `docs/conformance/burndown.jsonl`; rejects partial or filtered scans.
- `test262-baseline-metadata.awk`: Internal metadata parser shared by the
  Test262 scripts.

## Focused Development

- `bootstrap.sh`: Initializes submodules and prefetches crates for a fresh
  checkout.
- `microbench.sh`: Runs the QuickJS microbenchmark subset, optionally against
  QuickJS-NG.
- `source-size-report.sh`: Reports large first-party files; `--vendor` scans
  pinned upstream files instead.

## Agent Workflow

- `create-agent-worktree.sh`: Creates an isolated `agent/**` branch and
  worktree for one coding owner.
- `validate-agent-branch.sh`: Checks that an agent branch started from the
  expected base sha and stayed inside its path boundary.

## Internal Helpers

- `check-file-size.sh`: Enforces file-size limits; called by `check.sh`.
- `run-with-timeout.sh`: Runs a command with a timeout; shared by comparison,
  benchmark, and Test262 scripts.
