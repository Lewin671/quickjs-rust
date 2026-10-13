# T042: Consume prepared read-only eval environments once

- Status: active.
- Implementation: none; architecture and exact-evidence plan validated.
- Verified at: base `06f0ecd3`, 2026-10-12; original and top-level-repeated
  exact-binary receipts verified; `validate-unit --profile-root` passed.
- Evidence: [plan](performance-units/prepared-cached-readonly-eval-owned-execution.json)
  binds the T040 queue and six receipts. Raw files are workspace-local,
  not retained; repeated-source profiles are diagnostic, not formal timings.
- Unresolved: implementation, semantic/provenance tests, screens and promotion.
- Next action: implement the owned read-only route in an isolated checkout,
  retaining caller projection construction and every non-admitted fallback.

## Scope and target architecture

The current cached direct-eval caller already obtains an immutable blueprint
and an independent owned projection. It then discards the blueprint handle,
enters native eval, repeats context/cache work, clones the projection, and
clones it again into the evaluator. The proposed route consumes that existing
projection once in the same evaluator; it never moves the caller's own env.

Admission is a direct call to the current realm's stable eval intrinsic,
string argument, cache hit with the original parse-context key, declaration-
free/closure-free/template-free blueprint, no binding writes or deletes,
nested eval, with or suspension, and no parameter/dynamic-realm context.
All other cases retain the native chain. Global eval rebinding cannot change
intrinsic authority; store a weak identity once per realm, with conservative
fallback for unregistered synthetic realms and foreign/bound/Proxy callees.

Read-only bytecode can call side-effecting functions. Current canonical cells,
objects, realm and private/module context remain live. The caller refreshes
realm-backed slots on success and error, then uses its existing result/throw
handler. The ordinary evaluator initializes `this` and creates fresh runtime
values. The proof removes redundant ownership layers, not actual execution.

Implementation stages within this leaf:

1. Register stable intrinsic identity and introduce prepared owned execution;
   do not change projection scanning or FrameBindings storage.
2. Prove hit/fallback behavior and semantics, then use the plan's bounded
   screens and complete formal promotion. No stage alone is a speed claim.

Owner paths: `bytecode/vm_call.rs`, `global.rs`, `function/env.rs`, their
existing eval-cache/direct-eval tests, and necessary test/perf counter hooks
under `crates/qjs-runtime/src/`. The lead owns plans, docs and order files.
No AST, dependency, general call ABI, COW, projection-slot cache or writes
optimization belongs to this unit.

## Profile interpretation

Original tofte call-lifecycle envelopes are 44/116, 53/102 and 45/103.
Thirty-two top-level source copies yield 1266/3300, 1299/3296 and 1238/3294;
their native-eval subsets are 640, 701 and 699, with clone subsets 115, 106
and 109. These nested numbers are never added or called wholly removable.
The larger diagnostic distinguishes repeated environment ownership from a
small slot-index scan; the original samples independently show the same path.

## Required coverage and verification

Confirm cold fallback and warmed prepared hits; changing caller values,
different callers sharing a source, prior eval-added names, callbacks mutating
cells/realm and callbacks throwing; strict mode, TDZ and writes/deletes;
declarations, parameters, with and nested eval fallback; stable intrinsic
rebinding, foreign/bound/Proxy and dynamic-realm fallback; extra arguments,
RegExp identity, arguments, private/super/new.target and module context.
Loop tests have in-script guard counters.

Follow [the performance workflow](../docs/performance-workflow.md). The frozen
plan owns thresholds and attempts. Record every screen, complete external and
broad results, sentinels and exact-candidate Test262 evidence before promotion.
