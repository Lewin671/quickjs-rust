# T051: Counter collection smoke-test contract

- Status: active.
- Implementation: present; test-only portable positive-counter assertion.
- Verified at: source base 55f76d54; iMac full gate job
  20261016T120645Z-typed-layout-study-final-gate exposed the old assertion.
- Evidence: exact failure and corrected contract below; verification pending.
- Unresolved: run fast and full gates on the corrected test.
- Next action: validate on iMac, integrate and close this record.

## Failure and correction

The hardware collection smoke test required cycles >100000 for a tiny Python
child. On iMac it returned 28079, while stdout and retired instructions passed.
The host is already known to return non-clock cycles; production measurement
entry points reject them through cycle_trust and judge wall time. A collector
smoke test cannot assume an absolute portable cycle count. Require a positive
readout instead, retaining the >1000000 instructions and exact stdout/exit-code
assertions. Do not change collection, trust decisions or performance gates.

The earlier T050 full gate passed at a648dc19; the final 55f76d54 gate failed
only this assertion among 412 benchmark-tool tests, before reaching Test262.
Its failure does not change any T050 wall-time result or raw measurement.

## Scope and verification

Owner: agent/vm-layout-causality/root; worktree shared workspace
worktrees/quickjs-rust/vm-layout-causality. Only this test, task and task index.
No Rust/build, collector or counter-trust implementation changes.

Fast change-aware checks before commit and full check after integration,
through the iMac queue. Keep the original failed gate artifact.
