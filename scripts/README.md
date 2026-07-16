# Script Guide

One-line index of repository scripts. `./scripts/check.sh` is the final gate
for local development. Detailed flags and operating guidance for the Test262
and gap-discovery scripts live in `docs/harness.md`; this file only says what
each script is for.

## Daily Checks

- `check.sh`: Standard project gate: formatting, clippy, workspace tests, and
  benchmark-tool tests, file-size limits, and the Test262 subset.
- `check-ci.sh`: Local parity wrapper for the GitHub Actions `check` job. It
  runs `check.sh` with serialized, split runtime tests and skips Test262
  because the workflow owns that in a dedicated job.
- `check-touched.sh`: Change-aware fast gate for AI iteration and pre-commit.
  Use `--staged --explain` before committing, or `--base <ref> --explain` to
  validate a branch slice. It runs relevant crate tests and focused Test262
  allowlist filters when touched paths imply a semantic area, plus benchmark
  unit tests when the manifest, protocol tools, or benchmark entrypoint change.
- `compare-qjs.sh`: Runs `tests/fixtures/compare-qjs/` fixtures against both
  quickjs-rust and the pinned QuickJS-NG reference.
- `find-qjsng-gaps.sh`: First agent entrypoint for conformance work. Wraps
  `test262-baseline.sh --engine both`, reports actionable gaps, and prints a
  greedy recommendation queue. See `docs/harness.md` for strategies, probe
  tuning, and replay flags.
- `test262-subset.sh`: Runs the curated Test262 allowlist
  (`tests/test262/allowlist.txt`); passing expected failures are reported as
  stale entries. `scripts/check.sh` runs the same subset with a 30-second
  per-case timeout by default to match CI; direct subset runs default to 10
  seconds unless `TEST262_CASE_TIMEOUT_SECONDS` is set.

## Conformance Measurement

- `test262-baseline.sh`: Samples or scans upstream Test262, classifying
  structural not-run cases, failures, and timeouts; `--engine both` compares
  against QuickJS-NG under its own test262 config. Set `QJS_CLI_BIN` to reuse
  a prebuilt binary across shards, or `QJS_CLI_PROFILE=release` to build an
  optimized quickjs-rust runner for timeout-sensitive scans. Set
  `TEST262_TIMEOUT_RETRIES` to retry only timeout results with the same
  per-case timeout. The generated quickjs-rust case source injects Test262
  harness files only when the case or its metadata includes require them.
- `test262-aggregate.py`: Aggregates QuickJS-NG and quickjs-rust case-result
  JSONL sets into the CI coverage summary, the schema-1 burndown entry, and an
  optional merged per-case comparison JSONL for follow-up gap selection. Used by
  the Test262 Coverage workflow; also runs locally on artifacts downloaded with
  `gh run download`.
- `test262-burndown.sh`: Appends a complete-scan entry to
  `docs/conformance/burndown.jsonl`; rejects partial or filtered scans.
- `test262-baseline-metadata.awk`: Internal metadata parser shared by the
  Test262 scripts.

## Focused Development

- `bootstrap.sh`: Initializes submodules and prefetches crates for a fresh
  checkout.
- `benchmark.sh`: Runs the versioned, externally timed benchmark manifest
  against explicit candidate/base/QuickJS-NG binaries and writes traceable raw
  JSONL. It never builds or downloads; see `docs/benchmarking.md`.
- `benchmark-report.sh`: Strictly validates one physically complete three-role
  raw plan, including durable failure states, and atomically writes a
  deterministic M3 statistics/whole-block-health report. It never emits a
  performance gate claim. Measurement and analysis manifests are independently
  versioned; use `--analysis-manifest` to select analysis policy.
- `resource-benchmark.sh`: Selects exactly one independently versioned M3
  resource lane (`fresh`, `rss`, or `size`) and writes fail-closed raw JSONL.
  It snapshots inputs but never builds, downloads, or touches QuickJS-NG.
- `resource-benchmark-report.sh`: Revalidates a complete resource physical
  plan and atomically writes a path-independent report under the independent
  resource analysis contract. Reports remain non-claim evidence through M6.
- `external-corpus-audit.sh`: Strictly validates the deny-only external-corpus
  v1 registry from any working directory. `--require-admitted <id>` consults
  only the checked-in registry and always fails; custom `--registry` input is
  structural only and cannot be combined with it. The script never downloads,
  initializes submodules, runs a corpus, or creates performance evidence.
- `external-performance-preview.sh`: Audits, downloads, or runs the pinned
  SunSpider 1.0, Kraken 1.1, and JetStream 3 JavaScript subset preview against
  qjs-rust and QuickJS-NG. It verifies every upstream file hash, keeps source
  outside artifacts, and produces informational per-case evidence without an
  official suite score.
- `performance-policy-audit.sh`: Validates the checked-in deny-only CI policy,
  current protocol hashes, direct QuickJS-NG pin, aggregate hosted workflow /
  setup / orchestrator / renderer / admission / audit-chain hash, and zero-admitted
  external-registry state. Custom
  `--policy` input is structural only; checked-in-only `--require-gate` always
  fails for `nightly`, `release`, and `pr_sentinel` in v2. It neither calibrates
  hardware nor runs or enables a performance gate.
- `performance-preview.sh`: From a policy-selected harness, prepares the candidate
  SHA, explicit base SHA, and manifest-pinned QuickJS-NG on one shared host.
  Same-repository PRs use a base-owned `pull_request_target` harness; every
  `main` push (merge or direct) uses `github.event.after` as the head-owned
  harness/candidate and `github.event.before` as the base. A trusted manual
  dispatch from `main` uses the selected revision as both candidate and A/A
  base, then runs the full JetStream 3, Kraken, and SunSpider external preview.
  Trigger it with `gh workflow run performance-smoke.yml --ref main`. Fork PRs are
  unsupported, and push admission fail-closes on event/ref/repository/SHA
  mismatches. The script
  clears GitHub command/token channels, verifies clean sources before and after
  builds and measurement, reruns audits after measurement, emits truthful dynamic
  receipts/manifest, runs all 25 throughput cases for three blocks, and
  renders Markdown/JSON summaries. Ratios require strict 3/3 valid-block,
  non-claim, linearity-pass health. A complete hosted run whose linearity
  diagnostic fails succeeds as explicitly inconclusive, preserves its raw
  evidence, and emits no ratio direction; missing, malformed, or incomplete
  evidence still fails. Pending/failure status includes the active phase and
  remains publishable without a ratio conclusion. It is informational
  and non-gating, and is not a malicious candidate sandbox. Exact content keys
  may reuse validated final engine executables; candidate/base share one Rust
  namespace and the pinned QuickJS-NG binary normally hits. Invalid entries
  rebuild. Keys include hosted image/runtime/libc plus effective compiler and
  linker identities/environment. PR-target runs restore only; trusted `main`
  pushes and trusted manual-main runs independently revalidate and may save
  completed entries even when a later noisy measurement fails. Cache-service
  errors degrade to rebuild or
  no-save, and benchmark measurements/evidence always rerun. `build-cache.json` records
  per-role provenance.
- `lifecycle-bench.sh`: Runs the dev-only Criterion parser/compiler lifecycle
  diagnostics through public Rust APIs. Pass `--quick` for a smoke run; without
  it the frozen Criterion sampling configuration applies. Quick mode uses an
  isolated discarded baseline. A fail-closed option allowlist accepts filters
  and documented display/run flags; every other option or short cluster is
  rejected. These results do not enter the QuickJS-NG comparison protocol or a
  CI threshold.
- `microbench.sh`: Runs the legacy QuickJS microbenchmark subset, optionally
  against QuickJS-NG. Its internal millisecond timer is a quick probe, not a
  gate or claim source.
- `source-size-report.sh`: Reports large first-party files; `--vendor` scans
  pinned upstream files instead.

## Agent Workflow

- `create-agent-worktree.sh`: Creates an isolated `agent/**` branch and
  worktree for one coding owner.
- `validate-agent-branch.sh`: Checks that an agent branch started from the
  expected base sha and stayed inside its path boundary.

## Internal Helpers

- `lib.sh`: Shared helpers (cargo resolution, QuickJS-NG build, qjs-cli
  build, timeout wrapper check) sourced by the other scripts.
- `check-file-size.sh`: Enforces reviewability limits for first-party Rust,
  Python (800 source / 1200 test lines), and shell files; called by `check.sh`.
- `run-with-timeout.sh`: Runs a command with a timeout; shared by comparison,
  benchmark, and Test262 scripts.
