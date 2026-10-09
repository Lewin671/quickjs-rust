# Conformance Records

What does a conformance record establish? `burndown.jsonl` is the ledger of
complete Test262 scans compared with the pinned QuickJS-NG reference, one
line per scan of one commit ([how to run one](../harness.md)).

## What is measured

- **Target.** ECMA-262 16th edition (ES2025), anchored to
  `tc39/ecma262@es2025`; `AGENTS.md` owns that statement.
- **Corpus.** Test262 has no edition tag, so the corpus is every `.js` file
  under `test/` of the pinned `third_party/test262` commit. It can contain
  newer Stage 3+ or living-draft cases.
- **Configured coverage.** The comparison baseline is the corpus minus the
  cases the pinned QuickJS-NG configuration does not run:
  `configured = total - ng_config_skipped`. Every line so far has `total`
  53,572, `ng_config_skipped` 10,900 and `configured` 42,672.
- **`ng_config_skipped`.** `scripts/test262-baseline.sh` applies
  `third_party/quickjs-ng/test262.conf`. A case is skipped when it is a
  `_FIXTURE.js` file, when its path is in the `[exclude]` list (all of
  `test/intl402/` plus individual files), or when its metadata names a
  feature that the `[features]` section marks `=skip` or does not list.
  Skipped features include `Temporal`, `ShadowRealm`, `decorators`,
  `import-defer`, `source-phase-imports`, `regexp-modifiers`,
  `Atomics.waitAsync` and the `Intl.*` features; the config file is the full
  list. The ledger does not record how the count splits between reasons.

A record says nothing about the skipped cases: passing every configured case
is parity with QuickJS-NG's configured coverage, not full Test262.

## What a line establishes

A line describes the commit it names, with that commit's submodule pins and
harness. It does not describe `HEAD`: the newest line is as old as its
`commit`, and later commits are unmeasured until a new line is recorded.

Fields (`schema` is `1` on every line):

- `recorded`: UTC date the line was written, not the commit date.
- `commit`: the quickjs-rust commit measured. For a local scan it is the
  `--commit` argument, or the checked-out `HEAD` at recording time.
- `source`: provenance label, below.
- `total`, `ng_config_skipped`, `configured`: as above.
- `rust.pass`, `.fail`, `.timeout`, `.not_run`: quickjs-rust results over
  the configured cases; they sum to `configured`. `not_run` counts cases the
  harness did not execute, such as `$262.agent` cases without `QJS_AGENTS=1`.
- `ng.pass`, `ng.fail`, `ng.timeout`: QuickJS-NG results over the same cases.
- `comparison.both_pass`: both engines pass.
- `comparison.ng_pass_rust_fail`, `.ng_pass_rust_timeout`,
  `.ng_pass_rust_not_run`: QuickJS-NG passes; quickjs-rust fails, times out,
  or did not run the case.
- `comparison.actionable_gap`: `ng_pass_rust_fail + ng_pass_rust_timeout`.
- `comparison.rust_pass_ng_nonpass`: quickjs-rust passes, QuickJS-NG does not.

`source` labels present in the file:

- `ci-aggregate`: written by `scripts/test262-aggregate.py` in the Test262
  Coverage workflow, under the settings `test262-coverage.yml` had at that
  commit. Today that is a release binary with the `agents` feature.
- `local-exact`: the default label of `test262-burndown.sh --report`.
- `local-exact-agents`, `local-exact-release-agents`: local scans labelled
  by hand with `--source`; the script does not check a label.

## Recording

Append with `scripts/test262-burndown.sh`. `--report` accepts only complete,
unfiltered `--engine both` summaries whose case count equals the pinned
corpus; `--entry` appends a prebuilt line such as the CI artifact. Both
refuse a second line for a commit. Do not edit existing lines except to
delete a provably wrong record.
