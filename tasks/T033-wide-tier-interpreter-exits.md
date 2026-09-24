# T033: Wide-tier interpreter exits

## Goal

Stop charging a whole interpreter frame to every call of a body that the
wide compact tier can run except for a rare path or a loop. The tier runs
such a body until it reaches an instruction it leaves to the interpreter,
then hands over its state; the interpreter resumes at that instruction.

## Design

- Data-only object literals are built at their exit, which always continues.
- Exit points (`compact_fn/wide/compile.rs`): operations in `is_exit_safe`
  (computed stores the tier cannot answer as a plain dense-index or
  own-data store -- it tries that first at the exit and continues --,

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

- Stack run bf07567d vs main 98f8f113 (30 blocks, cycles, quiet host;
  `target/comparison/typed-in-wide-bf07567d-30b`): typed loops against
  wide registers, slot-cached function/array/string reads, creation
  caches, inlined constructors, plain stores and literals at exits, string
  atoms, receiver-in-place `this` reads/writes, in-place local branches.
  External geomean 0.944 against the base (JetStream subset 0.933, Kraken
  0.967, SunSpider 0.935) and 1.063 against QuickJS-NG in wall time;
  fannkuch 0.527, binary-trees 0.815, bits-in-byte 0.808, cdjs 0.829.
  Controls above 1.01: xparb 1.026, unpack-code 1.023, md5 1.019. Broad
  lane flat except `array_index_of` 1.062 and `array_dynamic_read` 0.934;
  sentinels 0.87-1.005.

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

- Peephole rewrites (wide/peephole.rs, 3147d10b): CompareJump fusion,
  results stored straight into locals, discarded constants not loaded;
  hash-map runs 24% fewer wide operations (0.916 single-run cycles).
  Folding `typeof local` and `return local` in place on top: 1.001,
  closed.
- Existing global variables assigned on the tier (7477de9e): fasta 0.80.
- Measured wide costs (instruction increments against QuickJS-NG): a
  call-and-return 700-800 instructions vs 300; a cached named read about
  240 vs 50; `s = s + i` 95 vs 36. Typed loop programs run at parity with
  NG's interpreter per operation (75 cycles per iteration on the same
  8-operation loop).

- Stack run 819a6ba4 vs main 0c2b38f1 (30 blocks, cycles, quiet host;
  `target/comparison/wide-calls-819a6ba4-30b`): peephole rewrites, global
  variable stores, post-accelerator hand-back, Math.random in typed loops,
  string concatenation in place, native family dispatch, ASCII case
  mapping, array/string/number method caches, integer formatting, dense
  slice/concat/sort with default species, RegExp exec/test prelude.
  External geomean 0.951 against main (JetStream subset 0.933, Kraken
  0.939, SunSpider 0.960) and 1.0015 against QuickJS-NG in wall time;
  stanford-crypto-pbkdf2/ccm, string-validate-input 0.872,
  date-format-xparb 0.859, hash-map 0.879. Worst against main:
  math-partial-sums 1.017, access-nsieve 1.016. Sentinel
  `heterogeneous_property_read` 1.107 and broad `array_dynamic_read` 1.041
  are open.

- Rejected: fusing a typed-loop comparison with the Exit/JumpIfFalsy that
  tests it (`CompareSkip`, one new dispatch arm): corpus 1.0025,
  access-fannkuch 1.03, math-partial-sums 1.02-1.09 with fewer
  instructions -- the arm re-rolled the dispatch loop. Patch in
  /tmp/typed-compare-skip.patch at the time.
- The implicit-global loop rule (eedfb4ae) measured 0.96-0.99 on
  3d-raytrace over repeats, not the 0.90 of its first run.

- Stack run 7ea40cb2 vs main 986839a1 (30 blocks, cycles, quiet host;
  `target/comparison/builtins2-7ea40cb2`): string relational compares
  without an environment, memoized inlining facts, remembered receiver
  misses, compound member assignment on the wide tier, accelerated-loop-only
  global store declines, undeclared globals created on the tier, field
  initializer member reads. External geomean 0.995 against main (JetStream
  subset 0.991, Kraken 0.997, SunSpider 0.996) and **0.985 against
  QuickJS-NG** in wall time -- the first formal run below it.
  string-validate-input 0.928, stanford-crypto-ccm 0.950, crypto-md5 0.956,
  3d-raytrace 0.968, raytrace-public-class-fields 0.972, hash-map 0.980.
  Worst against main: math-spectral-norm 1.031, imaging-gaussian-blur
  1.023, crypto-sha1 1.020. Sentinels 0.982-0.999; broad lane flat.
  Largest remaining against NG: hash-map 2.16, tagcloud 2.03, 3d-raytrace
  1.94, ai-astar 1.82, controlflow-recursive 1.80, tofte/xparb 1.78,
  raytrace-class-fields 1.77, nbody 1.76.

- Stack run 99275d24 vs main 3daba0ec (30 blocks, cycles, quiet host;
  `target/comparison/strings3-99275d24`): lazy String-wrapper index
  properties, wide array-hole reads and the store/reload peephole, in-place
  global string appends on the wide tier, String-wrapper ToPrimitive
  without a call. External geomean 0.988 against main (SunSpider 0.978)
  and **0.971 against QuickJS-NG**; string-tagcloud 0.769,
  string-validate-input 0.825, date-format-xparb 0.895. Worst against
  main: audio-dft 1.022, stanford-crypto-sha256-iterative 1.018.
  Sentinels 0.960-1.003 (`prototype_method_call` 0.960).

- Stack run 520a5734 vs main 9f13933a (30 blocks, cycles, quiet host;
  `target/comparison/json-parse-520a5734`): JSON.stringify copying
  unescaped runs, operator-token variant checks, typed-loop global writes
  by slot (dynamic property storage now slot-addressed), nested literals
  parsed once (the parser tried every `[`/`{` as a destructuring pattern
  first -- exponential in nesting depth), for-in layers read from storage,
  the cheap dynamic-realm predicate. External geomean 0.965 against main
  and **0.938 against QuickJS-NG**; json-stringify-tinderbox 0.551,
  math-partial-sums 0.771 (now 0.991 against NG), jetstream
  stanford-crypto-aes 0.783, string-tagcloud 0.885. Worst against main:
  xparb 1.021; sentinel `string_key_map_churn` 1.020 (dictionary churn pays
  the slot index's upkeep on removal).

- Stack run 745de05c vs main b3179386 (30 blocks, cycles, quiet host;
  `target/comparison/calls4-745de05c`): wide call/return trims (packed
  inlining facts, object receivers as `this`, slimmer frames, empty-register
  skip; 1007 -> 894 instructions per call), strings and JSON text built
  without per-value temporaries, repeated JSON.parse keys shared, RegExp
  lastIndex written in place. External geomean 0.984 against main and
  **0.922 against QuickJS-NG**; json-stringify-tinderbox 0.730,
  json-parse-financial 0.831, crypto-md5 0.953, binary-trees 0.957, cdjs
  0.960, hash-map 0.970. Worst against main: access-nsieve 1.017.
  Sentinels 0.9995-1.008.
- Stack run 26c1ba7a vs main b62e9326 (30 blocks, cycles, quiet host;
  `target/comparison/perf5-26c1ba7a`): number-only callees evaluated on
  typed-loop argument numbers, number-only leaves in register form,
  typed-loop invariant reads hoisted to loop entry, home-object methods
  and base-class `new` inlined on the wide tier. External geomean 0.980
  against main and **0.901 against QuickJS-NG**; bits-in-byte 0.701,
  raytrace-public-class-fields 0.743, spectral-norm 0.869, ai-astar 0.874,
  sha1 0.893. Worst against main: imaging-gaussian-blur 1.033,
  imaging-desaturate 1.031. The sentinels regressed 2-3%
  (`prototype_method_call` 1.031); bisected to e1f94e22, whose inline
  number-only path made the compiler emit the typed loop's boxed argument
  array drop out of line. Fixed in 1d7564cd (evaluation out of line,
  argument array `ManuallyDrop`): sentinels 0.963 against main
  (`polymorphic_call_site` 0.915), corpus 0.999 single-run.
- Rejected: a proven-receiver prototype read ahead of the own-entry probe
  (method read 333 -> 268 instructions on a micro) -- hash-map 1.052,
  raytrace-class-fields 1.045, cdjs 1.041 in cycles with slightly more
  instructions: their sites rarely repeat a receiver. Patch
  /tmp/proven-prototype.patch.

- Rejected (single-run, no measurable gain; 2026-09-23):
  - Running closures a direct `eval` made (`Function::deopt_bindings`) on
    the wide tier over their named environment: admitted and ran (xparb's
    format functions, tofte's nested formatters), but xparb 1.007 and tofte
    0.993 -- their time is in the natives and conversions they call, not in
    the interpreter frame.
  - Proving a typed-loop receiver's own miss once per receiver for inherited
    reads (three name lookups per `this.method` read): 489 -> 302
    instructions on a micro, corpus 0.996, sentinels 1.005
    (`prototype_method_call` 1.014). Patch /tmp/typed-receiver-miss.patch.
  - Skipping already-empty registers in the wide return's window clear:
    -19 instructions per call (of 1005; QuickJS-NG 364), corpus 0.998.
  - Keeping a wide loop native instead of entering its typed program: 2.3x
    slower on bits-in-byte, 3.4x on ai-astar -- the typed tier's per-entry
    cost is worth paying even for 8-iteration loops.
- Measured (instructions): a wide method call and return costs about 1005
  instructions against QuickJS-NG's 364 (`this.f(k)` one level deeper).
  Clearing the returning window is 72 of them.

## Found, then fixed

- Fixed in 03bcf182 (all predated this task, interpreter paths):
  a sloppy or strict assignment to an accessor global now calls its setter
  (strict without a setter throws); `delete globalThis.x` and
  `Reflect.deleteProperty` unbind the mirroring realm binding; `Reflect.set`
  and `Object.assign` on the global object update it; and a `var` declared
  by a direct eval in a sloppy function shadows the global for the
  function's later assignments (they wrote the global before).

## Next

- Math.random from interpreted code costs 840 cycles per call (5.7x NG):
  the call runs through the generic path because the typed loop cannot
  call a stateful native.

- Admit bodies with a parameter prologue (default values) once their dead-zone
  behaviour is covered; CF traces count 179k general frames for them.
- The exit-heavy counter costs binary-trees and md5 about 2% instructions;
  look for a cheaper admission-side check.
