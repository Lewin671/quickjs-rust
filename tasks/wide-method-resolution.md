# Wide method resolution

- Status: active.
- Implementation: none.
- Verified at: e54e33eb, 2026-10-12; component pricing completed before runtime changes. Frozen plan validated against the brief01 queue and exact profile receipts; check-context passed.
- Evidence: [pricing record](archive/wide-method-resolution-pricing.md); local raw counters/screens not retained.
- Unresolved: removable fraction, instruction regressions and code placement; no formal promotion authorized.
- Next action: implement duplicate hot-probe removal after this frozen-plan commit.

## Scope and selection

[The frozen plan](performance-units/wide-method-resolution.json) owns cases,
thresholds and the two-attempt budget. Base e54e33eb; branch
agent/wide-method-resolution/codex, independent of A. A was rejected with both
attempts spent and runtime reverted, recorded/pushed as 86e1a462 on
agent/own-slot-cache-hot-answers/codex before B's worktree was created.

Allowed: compact_fn/property.rs, wide/activation.rs, activation/exits.rs,
resume code and their focused tests; this task, pricing record, plan/index row.
Forbidden: typed_loop/**, vm_bindings.rs, vm_ops.rs, class-field code, order file
and layout_pin budgets. Other owners' plans and index rows are untouched.

B selects method resolution because the measured corpus-weighted lookup proxy
is larger in every target than native-loop overhead. It removes only the second
hot probe after a miss, retaining all cold ordinary/exotic eligibility and
own/prototype proofs. It does not repeat T033's rejected prototype pre-probe,
inline clone/drop, or frame-stack rewrite. Native-backedge action/view work is
priced separately and is not implemented here. Shared upvalue cells and slots
are unchanged.

All measurements use the host timing lock, reject/discard competing builds,
and use instructions and cycles. No repin is authorized. A control cycle-only
failure must be reported as placement with symbol positions and sizes, and
must not consume an attempt chasing layout.
