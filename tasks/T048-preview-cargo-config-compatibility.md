# T048: Performance preview across the compiler-config boundary

- Status: active.
- Implementation: present in this branch; pending integration.
- Verified at: `6d0f9b6c`, 2026-10-16; focused orchestration checks,
  fast gate and full gate passed; this finalization edits only task prose.
- Evidence: failed GitHub performance preview run `38878916859`; regression
  coverage in `tools/benchmark/tests/test_build_cache_orchestrator.py`.
- Unresolved: hosted verification pending.
- Next action: verify the repaired hosted preview, then close this task.

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
