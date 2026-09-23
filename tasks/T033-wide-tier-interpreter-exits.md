# T033: Wide-tier interpreter exits

## Goal

Stop charging a whole interpreter frame to every call of a body that the
wide compact tier can run except for a rare path or a loop. The tier runs
such a body until it reaches an instruction it leaves to the interpreter,
then hands over its state; the interpreter resumes at that instruction.

## Design

- Exit points (`compact_fn/wide/compile.rs`): operations in `is_exit_safe`
  (computed stores the tier cannot answer as a plain dense-index or
  own-data store -- it tries that first at the exit and continues --, object
  literals,
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
- Typed loops from the tier (`wide/loop_frame.rs`, `typed_loop/frame.rs`):
  at a probed backedge no other accelerator plans for, the loop's typed
  program runs against the activation's registers through the `LoopFrame`
  trait the interpreter's `Vm` also implements. A finished loop continues in
  the tier at its exit; a deoptimized one resumes in an interpreter frame
  with that program declined and hand-back armed. Such exits are not
  counted, so a function around a short loop stays on the tier.
- Exit-heavy judgement: after 64 activations, a program that exited on three
  in four is left to the general path.
- Hand-back (`vm/wide_resume.rs`): an unconditional backward jump exits only
  so the accelerators can claim its loop, and is probed. The exit is
  followed by the jump itself. When no accelerator claims the loop at the
  exit, no instruction runs and the tier keeps the loop; when one enters and
  deoptimizes, the interpreter frame stops at the next probed backedge whose
  accelerators decline and returns its locals and stack (a `Yield`
  completion carrying no value). Either way that backedge's exit is declined
  from then on, and the loop runs on the tier instead of generically.
- The dispatch arm spells an exit as a call with arity `EXIT_ARGC`, so the
  driver's action type and match are unchanged (lottery rule).

Plan and evidence: `tasks/performance-units/wide-tier-interpreter-exits.json`
(screen gate PASS on base 93f98a4a; formal promotion pending).

## Acceptance Criteria

- [x] Runtime tests cover mid-expression exits, dead-zone reads past an exit,
  exits in nested activations, receivers, handlers and parameters read only
  past an exit, and loops handed to their accelerators.
- [x] `test/language` and `test/built-ins` gap scans show no new NG gaps.
- [x] Formal promotion (batched) records a decision: **rejected** on
  batch 309a604e vs 93f98a4a (30 blocks, cycles; with
  `realm-regexp-program-cache`). Targets moved (hash-map 0.873, cdjs 0.927,
  string-validate-input 0.950 crossing its 0.95 target), but
  access-binary-trees read 1.074 and crypto-md5 1.032 -- the dispatch-loop
  codegen band this task already records, with the same instruction mix --
  and the broad lane failed its linearity check (base
  `property_dynamic_read` 1.37) because builds and tests ran on the host
  during the run. At 10cef53c the same two controls read 1.011 and 0.986
  against 93f98a4a in single-run cycles. Test262 at 309a604e and 10cef53c:
  CI aggregate zero gap.

- Stack run 2b799a31 vs 93f98a4a (30 blocks, cycles, quiet host, every
  lane healthy; `target/comparison/stack-2b799a31-30b`): external geomean
  0.968 against the base and 1.138 against QuickJS-NG; hash-map 0.734,
  cdjs 0.812, string-validate-input 0.856, date-format-xparb 0.868,
  date-format-tofte 0.879, 3d-raytrace 0.880. One control failed:
  string-base64 1.097, from primitive string reads on the wide tier after
  the loop hand-back; fixed in f08fb6c2 (single-run 0.70 of 2b799a31).
  Broad lane 0.997, sentinels 1.004 (worst string_key_map_churn 1.019).

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

- Hand-back, on 309a604e + the getter change (single-run cycles):
  hash-map 3.12G -> 2.78G, cdjs 3.38G -> 3.07G (both together with the
  IsHTMLDDA tag check), raytrace-public-class-fields 2.94G -> 2.87G. Not yet
  screened; hash-map's remaining generic work is `_rehash` running after its
  computed-store exit.

- Guarded Math unary calls run as one-argument method calls, whose native
  fast path answers the intrinsics without a frame (string-validate-input
  -4.7% single-run cycles; makeName/makeNumber no longer judged exit-heavy).

## Next


- Admit bodies with a parameter prologue (default values) once their dead-zone
  behaviour is covered; CF traces count 179k general frames for them.
- The exit-heavy counter costs binary-trees and md5 about 2% instructions;
  look for a cheaper admission-side check.
