# Conformance Burndown

The default language target is the latest ratified ECMAScript standard:
ECMA-262 16th edition, June 2025 (ECMAScript 2025 / ES2025), anchored to the
`tc39/ecma262@es2025` specification tag. Test262 has no edition-specific stable
tag, so the pinned Test262 corpus may contain newer Stage 3+ or living-draft
material; those cases are tracked as future-work campaigns unless they are
already part of the ES2025 baseline or the pinned QuickJS-NG comparison
configuration.

`burndown.jsonl` is the append-only time series of full Test262 comparison
scans against the pinned QuickJS-NG reference. One line per recorded scan,
schema version 1:

- `recorded`: UTC date the entry was recorded.
- `commit`: the quickjs-rust commit the scan measured.
- `source`: `ci-aggregate` (Test262 Coverage workflow artifact) or
  `local-exact` (local full `--engine both` scan).
- `total` / `ng_config_skipped` / `configured`: upstream case count, cases
  skipped by QuickJS-NG's own test262 config, and the comparison baseline.
- `rust`, `ng`: per-engine pass/fail/timeout counts; `rust.not_run` counts
  remaining structural harness exclusions, such as unsupported harness includes,
  `$262.agent`/multi-agent coordination, intl402, and fixtures.
- `comparison.actionable_gap`: QuickJS-NG passes while quickjs-rust fails or
  times out. This is the primary burndown number.
- `comparison.ng_pass_rust_not_run`: QuickJS-NG passes that quickjs-rust
  cannot run yet for structural reasons. This is the second burndown number;
  it shrinks when harness exclusions are lifted.

Append entries with `scripts/test262-burndown.sh`; do not edit existing lines
except to delete a provably wrong record. Only complete, unfiltered scans may
be recorded so the series stays comparable over time.
