# Script Guide

Which script do I need? One entry per script, with the document that owns its
flags and behavior. Run a script with `--help` for its exact options.

Owning documents: [harness](../docs/harness.md) for checks, Test262 and
integration; [performance workflow](../docs/performance-workflow.md) for
selecting and measuring a performance unit;
[benchmarking](../docs/benchmarking.md) for what the measurement tools and
artifacts mean.

## Gates

- `check-touched.sh`: fast, change-aware gate for a commit
  (`--staged --explain`) or a branch (`--base <ref> --explain`). Harness.
- `check.sh`: full local gate, run before handoff and by the pre-push hook.
  Harness lists its stages and prerequisites.
- `check-ci.sh`: runs `check.sh` the way the GitHub Actions `check` job does.
- `check-context.py`: document budgets, Markdown links, task resume blocks,
  and the task and plan indexes. Run by both gates.
- `check-file-size.sh`: line limits for Rust, Python, shell, and active task
  files. Run by both gates.
- `pre-commit`, `pre-push`: the git hooks `bootstrap.sh` installs.
- `test-git-hooks.sh`: checks that the hooks stay isolated to their worktree.

## Setup And Parallel Work

- `bootstrap.sh`: initializes the two pinned submodules, prefetches crates,
  installs the git hooks.
- `create-agent-worktree.sh`: creates an `agent/<task>/<owner>` branch and
  worktree and bootstraps it. Harness.
- `validate-agent-branch.sh`: checks an agent branch started from the expected
  base and stayed inside its path boundary. Harness.

## Conformance

- `compare-qjs.sh`: runs `tests/fixtures/compare-qjs/` against quickjs-rust
  and the pinned QuickJS-NG. Harness.
- `find-qjsng-gaps.sh`: first entry point for conformance work; reports cases
  QuickJS-NG passes and this engine does not, with a recommendation queue.
  Harness; `--help` for strategies and probe flags.
- `test262-subset.sh`: runs the curated allowlist
  (`tests/test262/allowlist.txt`). Harness.
- `test262-baseline.sh`: samples or scans upstream Test262 for one or both
  engines. Harness.
- `test262-aggregate.py`: merges per-case results into the coverage summary
  and a burndown entry. Harness.
- `test262-burndown.sh`: appends a complete-scan record to
  `docs/conformance/burndown.jsonl`. See
  [conformance records](../docs/conformance/README.md).
- `test262-upstream-amendments.sh`: exact metadata backports for the pinned
  Test262 revision; sourced by the Test262 scripts, not run directly.
- `test262-baseline-metadata.awk`: metadata parser shared by the Test262
  scripts; not run directly.

## Performance

- `perf-loop.sh`: inner loop for one unit: builds, runs the counter screen,
  lists symbol-size changes, optional trace. Diagnostic only. Workflow.
- `perf-compare.sh`: formal three-engine comparison of two commits on this
  host. Workflow.
- `performance-decision.sh`: builds the opportunity queue, validates a frozen
  unit plan, records a decision. Workflow.
- `layout-scan.sh`: scans the typed-loop executor's pinned code offset and
  screens each placement. Diagnostic only. Workflow.
- `external-corpus-ab.py`: amplified A/B of two binaries over the cached
  SunSpider and Kraken sources. Diagnostic only. Benchmarking.
- `external-performance-preview.sh`: audits, fetches, or runs the pinned
  external corpus preview. Benchmarking.
- `external-corpus-audit.sh`: validates the external-corpus registry.
  Benchmarking.
- `benchmark.sh`, `benchmark-report.sh`: run the versioned benchmark manifest
  against explicit binaries, then validate and report one complete run.
  Benchmarking.
- `resource-benchmark.sh`, `resource-benchmark-report.sh`: the same for the
  resource lanes (fresh start, RSS, size). Benchmarking.
- `performance-preview.sh`: the hosted, non-gating preview that CI runs.
  Benchmarking.
- `performance-policy-audit.sh`: validates the checked-in CI performance
  policy and its hashes. Benchmarking.
- `lifecycle-bench.sh`: Criterion diagnostics for the parser and compiler.
  Benchmarking.
- `microbench.sh`: legacy QuickJS microbenchmark subset; a quick probe, not
  evidence.

## Helpers

- `lib.sh`: shared shell helpers (cargo resolution, QuickJS-NG build, CLI
  build, timeout wrapper); sourced, not run.
- `run-with-timeout.sh`: runs a command with a timeout.
- `source-size-report.sh`: lists large first-party files; `--vendor` adds the
  pinned upstream files.
