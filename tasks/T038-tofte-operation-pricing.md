# T038: Tofte operation pricing

- Status: active. Brief 06 stopped after pricing and selection (step 2).
- Implementation: none. No runtime unit/plan or screen attempt; temporary
  diagnostic counters were restored byte-for-byte in the scratch worktree.
- Verified at: runtime `af4d0d3c`, branch base `8e208b8a`, 2026-10-12;
  locked nonempty 4/8 pricing, exact-binary sampling and counter readback.
- Evidence: [measurement record](archive/T038-tofte-operation-pricing.md)
  retains prices, counts, residual, commands and selection limits.
- Unresolved: 23.01% of cycle gap / 9.84% of instruction gap remains outside
  the source-character model; no independent leaf clearly reaches 10% of case.
- Next action: lead selects a staged activation/binding representation design
  or a separately priced leaf; do not reopen incremental prefix overlays.

## Scope and decision

Base `origin/integration/batch-1` was `8e208b8a8f19764e99f2a003d6114fbca9d163db`;
its runtime bytes equal measured `af4d0d3c81aad98721661689cf7224d2c17b9781`.
Branch `agent/tofte-operation-pricing/codex`; worktree
`../quickjs-rust-worktrees/wt-quickjs-rust-tofte-operation-pricing-codex`.
Committed boundary: this task, its archive record, its measurement helpers,
and its row in `tasks/README.md`. No `docs/`, runtime, property/cache/wide
activation, class-field, linker-order, benchmark or third_party changes.

The largest fixed envelope in the measurement record spans frame,
cell, closure, array and environment lifetimes. It is not one removable leaf.
The existing typed-equality admission analogue and metadata leaf budgets are below the brief's **10% of total case**
threshold; reaching that threshold by adding profile groups would double-count
or combine distinct mechanisms. Step 3 was not entered. There is no frozen
plan, no candidate, no placement failure and zero implementation attempts.

## Verification

- 188 generated micro inputs match the measured files byte-for-byte.
- Diagnostic install/restore round trip leaves `git diff --exit-code -- crates`
  clean; exact tofte stock counters and stdout match the instrumented run.
- Python compile/import/help and generation checks passed.
- Staged/full gate and branch CI results are recorded in the measurement record.

[T018](T018-broad-performance.md) owns formal standing;
[T037](T037-integration-batch-1.md) owns promotion and placement status.
