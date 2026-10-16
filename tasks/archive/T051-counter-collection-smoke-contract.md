# T051: Counter collection smoke-test contract

- Status: closed-landed.
- Implementation: present; test-only portable positive-counter assertion.
- Verified at: corrected test at 73e8edb6, 2026-10-16; iMac fast and
  post-integration full gates both passed.
- Evidence: exact failure, corrected contract and successful checks below.
- Unresolved: none.
- Next action: none.

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

## Verification outcome

- 20261016T121011Z-counter-smoke-fast-gate: passed change-aware fast checks
  at transport snapshot 81c4817f, including all 412 benchmark-tool tests.
  Its tree was checked equal to the index before commit 73e8edb6.
- 20261016T121214Z-counter-smoke-final-full-gate: passed full check after
  integration at 73e8edb6, including all 412 tool tests and 5170 Test262 cases.
- Final closure changes only documentation and passes context/fast checks.
  The original failed 55f76d54 gate is retained; no timing result was rerun.
