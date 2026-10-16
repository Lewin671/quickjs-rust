# T048: Performance preview across the compiler-config boundary

- Status: closed-landed.
- Implementation: present on main in `d001a42f`.
- Verified at: `6d0f9b6c`, 2026-10-16; focused orchestration checks,
  fast gate and full gate passed; this finalization edits only task prose.
- Evidence: failed GitHub performance preview run `38878916859`; regression
  coverage in `tools/benchmark/tests/test_build_cache_orchestrator.py`.
- Unresolved: none for this guard compatibility fix.
- Next action: none.

## Cause and correction

The adoption changed only the ordering of the two trusted Rust flags in
`.cargo/config.toml`. A push preview comparing that commit against `163be1d8`
failed its byte-for-byte source-config guard before compilation. Main CI
passed; this was a preview orchestration failure, not a measured regression.

The guard still requires the trusted config, with one narrow exception: the
exact prior spelling of the two flags is accepted. Every other byte must
match, and legacy `.cargo/config` remains rejected. No source is rewritten.
Both revisions compile with the trusted builder's `CARGO_ENCODED_RUSTFLAGS`,
which already overrides the source's flag order. Cache identities still bind
tracked source configuration, the effective environment and the build recipe.

The end-to-end orchestration regression now compares the current harness
against the prior flag order, asserts that source bytes stay unchanged and
checks the encoded compiler flags. An untrusted wrapper config remains
rejected. The hosted implementation checksum is refreshed for these changes.
Runtime sources, Cargo defaults and measurement protocols are unchanged.

## Local verification

The actual `.cargo/config.toml` bytes from `163be1d8` pass the new guard and
remain unchanged. The focused preview/orchestrator tests pass. On the iMac,
`./scripts/check-touched.sh --base d7885049 --explain` and `./scripts/check.sh`
both passed, including the 5170-case curated Test262 subset. Raw logs are local
artifacts, not retained: workspace
`outputs/imac-bench/20261016T092117Z-preview-config-compat-check/`.
The original main CI (`38878916870`) and full coverage (`38879456055`) passed;
only the preview's config guard failed. No compiler settings or runtime code
are changed by this fix.

## Hosted verification

At `d001a42fe8876a3b958ae244ad7618c76669d35d`, all three hosted workflows
completed successfully on 2026-10-16:

- [CI](https://github.com/Lewin671/quickjs-rust/actions/runs/38880424650):
  standard checks, curated Test262 and NG comparisons.
- [Test262 Coverage](https://github.com/Lewin671/quickjs-rust/actions/runs/38881104399).
- [Performance Preview](https://github.com/Lewin671/quickjs-rust/actions/runs/38880424709):
  build, both broad shards, sentinels, external programs and composed summary.

The hosted run compares against `d7885049`; the actual older `163be1d8` config
and the current/previous-order crossing are covered by the local direct check
and the end-to-end orchestration regression respectively. The successful
preview is informational, not a new performance qualification. This closure
changes only the task record and index; the compiler and runtime inputs stay
identical to the verified fix.
