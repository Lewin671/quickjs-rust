# T075 verification and cleanup

Engine source is `59eb72e511cca013cf59d6736922c9c666cb883f`; all repository changes
are diagnostic tooling/data and task documentation. Final checks below pass on the diagnostic tree, 2026-10-23 UTC.

`python3 tasks/diagnostics/t075/verify.py` passes: 90 profiles, 33,543 PC-count
rows and 7,641 mapped PCs; categories and owner envelope conserve counts.
All 90 final raw SHA-256 values and process/sampler exits were separately
read back against invocation metadata. Seven-function source-map audits pass.
The actual exported datasets, not files under target or tmp, are retained.

The first full gate passes format, clippy/feature checks, workspace tests,
benchmark tools and performance plan validation, then stops on:

```text
error: tasks/README.md: 121 lines, budget 120; split the document or move history to tasks/archive/
error: failed agent context after 0s
```

The added task row exposed the exact index budget. Reflowing the introductory
paragraph restores 120 lines without deleting any record; context now passes.
A complete final gate is rerun after all diagnostic/doc changes. This gate uses
`QJS_CHECK_SKIP_TEST262=1`: only diagnostics and prose change, so no semantic
Test262 slice is selected or conformance standing claimed.

## Final results

- `QJS_CHECK_SKIP_TEST262=1 ./scripts/check.sh`: **exit 0**. Format, clippy
  including agents/perf-counters, feature tests, all workspace tests (2,382
  runtime tests), 753 benchmark-tool tests, frozen plan validation, context,
  35 script tests, linked-worktree hooks, shell syntax and file-size guard pass.
- `./scripts/check-touched.sh --staged --explain`: **exit 0**. Selects context;
  no Rust/semantic Test262 slice applies to these paths. Python syntax,
  JSON/CSV parse checks and `verify.py` also pass.
- `git diff --cached --check`: **exit 0**. CSV export now uses explicit LF;
  its initial default CRLF was caught by the whitespace audit and corrected
  without changing counts or source maps.
- All runtime/build/third-party tracked paths are unchanged from the base.
  No origin publication or merge is performed; remote branch CI is unrun.
- Requested cleanup completes: `cargo clean` removes 5,529 files / 3.3 GiB;
  `git checkout -- target` restores the tracked fixture. The temporary local
  cross-build is also cleaned (218 files / 145.5 MiB); local pilot executable
  copies are removed. Queue evidence is retained outside the worktree.

Final full-gate log SHA-256: `2fa3977af2907fcf13522eccf2b80eb6cc564d28204d37fa24cb63699594144e`.
Raw files under tmp/output are not the durable evidence: the committed
PC counts, mappings, denominators, source audit and reports are sufficient to
read and independently recompute the conclusions.
