# T033: Wide-tier interpreter exits

- Status: active.
- Implementation: present. The exits, the hand-over and the hand-back are in
  `crates/qjs-runtime/src/bytecode/compact_fn/wide/` and
  `crates/qjs-runtime/src/bytecode/vm/wide_resume.rs` (symbols checked at
  `d1313d52`, see Design). A rejected promotion did not remove the code.
- Verified at: `10cef53c`, 2026-09-22, Test262 CI aggregate zero gap (recorded
  under Acceptance). This file records no check for any later revision.
- Evidence: the frozen unit was formally **rejected** on batch `309a604e`
  against `93f98a4a` (see "Evidence status"). Nothing has been formally
  measured since the 2026-10-09 removal of the loop-template plans.
- Unresolved: see "Unresolved".
- Next action: none decided. The candidates recorded 2026-09-22 to
  2026-09-26 are under "Next"; re-validate any of them against a comparison
  of the current tree before acting on it.

Experiment log, oldest first: `tasks/archive/T033-screen-log.md`. Read its
"Rejected" entries before starting an experiment in this area.

## Goal

Stop charging a whole interpreter frame to every call of a body that the
wide compact tier can run except for a rare path or a loop. The tier runs
such a body until it reaches an instruction it leaves to the interpreter,
then hands over its state; the interpreter resumes at that instruction.

## Evidence status

Implementation status and evidence status are separate. The code is present;
the list below is what has and has not been measured.

- Screen gate: PASS for the plan
  `tasks/performance-units/wide-tier-interpreter-exits.json` on base
  `93f98a4a`.
- Formal promotion: **rejected**. The full decision text is the third box
  under Acceptance: batch `309a604e` against `93f98a4a`, 30 blocks, cycles,
  batched with `realm-regexp-program-cache`. Targets moved, two controls
  exceeded their ceiling, and the broad lane failed linearity because builds
  and tests ran on the host during the run.
- Follow-up unit `wide-tier-math-calls-and-field-thunks`
  (`tasks/performance-units/wide-tier-math-calls-and-field-thunks.json`):
  screen attempt 1 on base `93f98a4a` FAIL (log, 2026-09-22).
- Later work (2026-09-22 to 2026-09-26) is recorded in the log as "stack
  runs": 30-block comparisons of a branch tip against the then-current
  `main`. The last is `fe533cc7` against `cebad5e9` (2026-09-26, loaded
  host): external geomean 0.995 against main and 0.749 against QuickJS-NG.
  The log records no `decide` outcome for any stack run.
- Not measured: everything after the 2026-10-09 removal (base `7a83b568`).
  The removal entry says "not yet measured" and that every broad-lane number
  in the log predates it. The numbers above are therefore not the standing
  of the current tree.
- Every `target/comparison/` directory the log cites is a local artifact, not
  retained; regenerate before reuse.

## Design

Symbols named here were confirmed present at `d1313d52`. The behaviour was
last described on 2026-09-26 and was not re-verified statement by statement
after the 2026-10-09 removal.

- Data-only object literals are built at their exit, which always continues.
- Exit points (`compact_fn/wide/compile.rs`): operations in `is_exit_safe`
  (computed stores the tier cannot answer as a plain dense-index or
  own-data store -- it tries that first at the exit and continues --,
  `RequireObjectCoercible`, literal appends) and the backward edges of loops
  an accelerator claims or in-place fusion rewrote. Bodies whose lowering
  keeps a literal in virtual slots, and anything needing set-up at entry
  (`arguments`, closures over locals, handlers, per-iteration scopes, eval,
  `with`), still decline.
- Accelerators: a backward edge consults two, the numeric-mutation loop and
  then the typed loop (`vm_loop_dispatch.rs`).
- Hand-over (`vm::resume_direct_call_bytecode`, in `vm/wide_resume.rs`): the
  frame the general path builds for the call, plus the tier's own locals (a
  dead-zone marker becomes an uninitialized slot) and operand stack, resumed
  at the exit's ip. Every parameter has a register when a body can exit;
  `this` is required when any instruction reads it.
- Typed loops from the tier (`wide/loop_frame.rs`, `typed_loop/frame.rs`):
  at a probed backedge no other accelerator plans for, the loop's typed
  program runs against the activation's registers through the `LoopFrame`
  trait the interpreter's `Vm` also implements. A finished loop continues in
  the tier at its exit; a deoptimized one resumes in an interpreter frame
  with that program declined and hand-back armed. Such exits are not
  counted, so a function around a short loop stays on the tier.
- Exit-heavy judgement (`EXIT_JUDGEMENT_ACTIVATIONS` in `wide/mod.rs`): after
  64 activations, a program that exited on three in four is left to the
  general path.
- Hand-back (`vm/wide_resume.rs`): an unconditional backward jump exits only
  so the accelerators can claim its loop, and is probed. The exit is
  followed by the jump itself. When no accelerator claims the loop at the
  exit, no instruction runs and the tier keeps the loop; when one enters and
  deoptimizes, the interpreter frame stops at the next probed backedge whose
  accelerators decline and returns its locals and stack (a `Yield`
  completion carrying no value). Either way that backedge's exit is declined
  from then on, and the loop runs on the tier instead of generically.
- The dispatch arm spells an exit as a call with arity `EXIT_ARGC`
  (`wide/activation/exits.rs`), so the driver's action type and match are
  unchanged. An edit to the driver can change its inlining and layout; see
  `docs/performance-knowledge.md`, "Codegen".

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

## Unresolved

- No formal performance standing exists for the tree after the 2026-10-09
  removal. Broad cases the removed plans answered whole now run on the
  general tiers or the interpreter.
- The frozen unit's only formal decision is a rejection, and the code it
  measured stayed and was built upon. No later `decide` outcome is recorded
  for it.
- Nested dense plans and the general predicate scan in
  `vm_numeric_mutation_loop` are recorded (2026-10-09) as candidates for
  later consolidation into the typed loop.
- Sentinel `heterogeneous_property_read` 1.107 and broad
  `array_dynamic_read` 1.041 were recorded "open" on 2026-09-23 against the
  `819a6ba4` stack run. The log does not record them closed.

## Next

Candidates as recorded, each with the date it first entered this file (from
`git log -S`). All predate the 2026-10-09 removal and none has been
re-measured since. They are inputs to a T022 selection, not a queue.

- 2026-09-22: Admit bodies with a parameter prologue (default values) once
  their dead-zone behaviour is covered; CF traces count 179k general frames
  for them.
- 2026-09-22: The exit-heavy counter costs binary-trees and md5 about 2%
  instructions; look for a cheaper admission-side check.
- 2026-09-24: tofte: after 7a89ea81 the remaining direct-eval cost is
  building the eval's environment (`apply_call_env`,
  `visible_local_entries`) and closure creation, not the overlay.
- 2026-09-26: tofte (1.43 against NG): a formatDate call's fixed cost is 55k
  cycles against 30k -- 26% is the deopt-overlay walk over the frame's 45
  locals for each of its 28 closures, 20% frame setup, 12% creating the
  closures.
- 2026-09-26: The wide tier's per-operation cost (2-3x QuickJS-NG: method
  call, own and prototype reads) is now the common factor of hash-map, cdjs,
  3d-raytrace, binary-trees and raytrace-class-fields.

## Verification

```sh
cargo test -p qjs-runtime
./scripts/check.sh
./scripts/compare-qjs.sh
```

Performance evidence follows `docs/performance-workflow.md` and T022
(`tasks/T022-performance-priority-controller.md`).
