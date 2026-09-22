# T033: Wide-tier interpreter exits

## Goal

Stop charging a whole interpreter frame to every call of a body that the
wide compact tier can run except for a rare path or a loop. The tier runs
such a body until it reaches an instruction it leaves to the interpreter,
then hands over its state; the interpreter resumes at that instruction.

## Design

- Exit points (`compact_fn/wide/compile.rs`): operations in `is_exit_safe`
  (computed stores, guarded Math calls, object literals,
  `RequireObjectCoercible`, literal appends) and the backward edges of loops
  an accelerator claims or in-place fusion rewrote. Bodies whose lowering
  keeps a literal in virtual slots, and anything needing set-up at entry
  (`arguments`, closures over locals, handlers, per-iteration scopes, eval,
  `with`), still decline.
- Hand-over (`vm::resume_direct_call_bytecode`): the frame the general path
  builds for the call, plus the tier's own locals (a dead-zone marker becomes
  an uninitialized slot) and operand stack, resumed at the exit's ip. Every
  parameter has a register when a body can exit; `this` is required when any
  instruction reads it.
- Exit-heavy judgement: after 64 activations, a program that exited on three
  in four is left to the general path.
- The dispatch arm spells an exit as a call with arity `EXIT_ARGC`, so the
  driver's action type and match are unchanged (lottery rule).

Plan and evidence: `tasks/performance-units/wide-tier-interpreter-exits.json`
(screen gate PASS on base 93f98a4a; formal promotion pending).

## Acceptance Criteria

- [x] Runtime tests cover mid-expression exits, dead-zone reads past an exit,
  exits in nested activations, receivers, handlers and parameters read only
  past an exit, and loops handed to their accelerators.
- [x] `test/language` and `test/built-ins` gap scans show no new NG gaps.
- [ ] Formal promotion (batched) records a decision.

## Screen log

- `wide-tier-math-calls-and-field-thunks` (native guarded Math calls, field
  thunks without a prologue marker, `FreshIterationScope` as a no-op,
  exits only for accelerator-claimed loops), attempt 1 on base 93f98a4a:
  FAIL. raytrace-public-class-fields 0.972 (target 0.95); binary-trees 1.064
  and md5 1.033 cumulative with the exits unit. Against its own parent the
  change moved raytrace 0.988 and binary-trees 1.043, the latter a register
  allocation change in the wide dispatch loop with identical execution
  counts.
- Isolating the wide dispatch loop in its own function to stop such
  re-rolls: rejected, call-heavy cases paid the per-action boundary
  (binary-trees 1.026, md5 1.034, cdjs 1.023; sentinels 0.97-0.99).

## Next

- Run guarded Math unary calls natively instead of exiting on them.
- Admit bodies with a parameter prologue (default values) once their dead-zone
  behaviour is covered; CF traces count 179k general frames for them.
- The exit-heavy counter costs binary-trees and md5 about 2% instructions;
  look for a cheaper admission-side check.
