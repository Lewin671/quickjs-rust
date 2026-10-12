# T039: Fixed-scope binding instantiation

- Status: active. Activation priced; fixed-scope metadata leaf selected.
- Implementation: none. Plan frozen before runtime work; counters restored.
- Verified at: base e8b07b95 (runtime af4d0d3c), 2026-10-12; locked 4/8
  component pricing, real-case counters and exact-candidate profile receipt.
- Evidence: [component record](archive/T039-tofte-activation-components.md),
  [frozen plan](performance-units/fixed-scope-binding-instantiation.json).
- Unresolved: implementation/screen and controls; no promotion is claimed.
- Next action: implement the static binding-instantiation plan, focused
  negative tests, then one complete diagnostic screen; two-attempt budget.

## Scope

Base origin/integration/batch-1 e8b07b95c36764434c98b5431790247365150d8f;
branch agent/tofte-activation-components/codex, worktree
../quickjs-rust-worktrees/wt-quickjs-rust-tofte-activation-components-codex.
Runtime is identical to the plan/queue's af4d0d3c, whose executable is reused.
Allowed runtime paths: bytecode/{ir.rs,mod.rs,vm_frame_init.rs,vm_bindings.rs,
vm_binding_instantiation.rs,vm_binding_instantiation_tests.rs}, function/env.rs.
Documentation: this task/archive, their index rows and
 docs/design/env-model-rewrite.md (changed invariant only). Supporting assay
sources live under tasks/measurements/activation-components/.
Excluded: typed_loop/**, compact_fn/property.rs, named_property_cache.rs,
compact_fn/wide/activation.rs, class-field code, linker order and other docs.

## Selection

Compile immutable captured-slot and fixed-owned-cell facts once. For an
eligible frame, its parameter/var cell identities and liveness cannot change:
no received cells, global/sloppy bindings, lexical clearing/iteration renewal,
with, suspension, eval-deletable cells or parameter prologue. Module cell
routing keeps the fallback. Memo still checks map identity and generation;
first overlay and every remap use existing ordered publication. Ordinary
values, mapped arguments aliases, scope bypass revocation and eval-added names
remain live cells. No name-keyed snapshot, lazy function value or new tier.

The overlay group reaches 9.02% case; 27/28 memo-hit scans are removable
under the proof. Pure captured-slot classification adds separate 2.18% reach;
initial realm-membership queries for private owned cells are also redundant.
Do not add overlapping frame-construction samples to these budgets.

## Verification

Plan validation and staged gate must pass before runtime code. Full correctness,
focused Test262 selection and diagnostic screen results are recorded here at
handoff. Placement-only cycle failures do not consume an implementation attempt.
