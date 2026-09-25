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
- Stack run 4d6c5686 vs main e3cf0b51 (30 blocks, cycles, quiet host;
  `target/comparison/perf6-4d6c5686`): prototype reads cached by slot in
  dynamic prototype storage (a prototype past a dozen methods installed no
  entry: 6,478 -> 2,425 instructions per method call through one), a write
  cache on plain named assignments, a non-cloning global-object check per
  named write, and bodies admitted whose globally-writing loop calls user
  methods (3d-raytrace's `blocked` ran 1,320 times on the general path).
  External geomean 0.994 against main and **0.900 against QuickJS-NG**;
  3d-raytrace 0.916, string-fasta 0.947, math-cordic 0.964, xparb 0.971,
  raytrace-public-class-fields 0.979. Worst against main: access-nsieve
  1.023. Sentinels 0.992-1.000.
- Measured (instructions per call, micro, ours vs QuickJS-NG): plain call
  677/273, method call 1080/441, own read ~185/80, prototype read 323/103,
  own write ~300/71 (two plain op dispatches alone ~140); entering a typed
  loop from a wide exit ~3,900; a typed iteration of `o.x += o.y * i`
  722/305. nbody enters three typed programs per `advance` call.
- Stack run 72c84fa8 vs main 1d9a5ec3 (30 blocks, cycles, quiet host;
  `target/comparison/perf7-72c84fa8`): short loops kept on the wide tier
  instead of entering their typed program, the compact tier's inlining
  proof memoized on the function, and a fixed typed-loop scalar register
  file indexed without bounds checks. External geomean 0.988 against main
  and **0.889 against QuickJS-NG**; imaging-gaussian-blur 0.944, 3d-morph
  0.952, access-nsieve 0.953, controlflow-recursive 0.956. Worst against
  main: 3d-raytrace 1.011, tofte 1.010. Sentinels 0.911-0.999
  (heterogeneous_property_read 0.911).
- Stack run 3214fdce vs main 5bcca07f (`target/comparison/perf8-3214fdce`)
  measured 0.988 against main and 0.879 against QuickJS-NG, but with the
  call sentinels +15-22% and ai-astar +18% at equal instructions: a stale
  `hot-functions.order` (see docs/performance-knowledge.md). Regenerated in
  the next commit; single-run then corpus 0.982, sentinels 0.999, ai-astar
  1.006 against the same base.
- Stack run c14c22b1 vs main 5bcca07f (30 blocks, cycles, quiet host;
  `target/comparison/perf8b-c14c22b1`): element reads without cloning the
  array, the eval overlay memo, the typed-loop operand stack rebuilt in
  place, loose string equality, and the regenerated order file. External
  geomean 0.984 against main and **0.876 against QuickJS-NG**; tofte 0.883,
  3d-raytrace 0.896, crypto-aes 0.914, bits-in-byte 0.924. Worst against
  main: string-unpack-code 1.009. Sentinels 0.995-1.005.
- Stack run 357d81c3 vs main e7c34545 (30 blocks, cycles, quiet host;
  `target/comparison/perf9-357d81c3`): helpers with loops, and numeric
  helpers on f64 registers. External geomean 0.992 against main and
  **0.869 against QuickJS-NG**; bits-in-byte 0.656 (0.997 against NG,
  from 1.52). Worst against main: imaging-gaussian-blur 1.032.
  Sentinels 0.994-0.999 except recursive_call_tree 1.040 (identical
  instructions; the grown helper interpreter's placement -- every other
  placement tried moved ai-astar or the call sentinels 18-25%).
- Stack run 59890450 vs main fb08d251 (30 blocks, cycles, quiet host;
  `target/comparison/perf10-59890450`): numeric call trees on f64
  registers. External geomean 0.993 against main and **0.866 against
  QuickJS-NG**; controlflow-recursive 0.563 (0.984 against NG, from 1.89).
  ai-astar 1.193 and the call sentinels 1.15-1.22 (recursive_call_tree
  0.966) with identical instruction counts: main fb08d251 sits in a lucky
  layout of the typed-loop executor's callees that every edit tried lost --
  only typed_loop's helper changes, the order file regenerated, one
  codegen unit (main itself: ai-astar 9270 vs 8276 M cycles), 64-byte
  function alignment (both worse). Merged on the external aggregate; the
  layout sensitivity is the open item below.
- Stack run d8a98ca3 vs main 6968f738 (30 blocks, cycles, quiet host;
  `target/comparison/perf11-d8a98ca3`): numeric call chains from wide
  calls, allocation-free plan runs. External geomean 0.993 against main and
  **0.860 against QuickJS-NG**; crypto-md5 0.785, ai-astar 0.840 and the
  call sentinels 0.84-0.93 (the layout of perf10 rolled back, identical
  instructions). Worst against main: string-unpack-code 1.047.
- Numeric plans lowered from bytecode (perf13, 2026-09-24,
  `compact_fn/numeric_plan/from_bytecode.rs`): bodies outside the compact
  tier are interpreted abstractly under number arguments -- `typeof`
  folds, string comparisons fold, unreachable cases are never lowered, and
  a reachable path the encoding cannot hold ends in `NumOp::Bail` (hand
  back). hash-map's `computeHashCode`/`equals` run on plans: hash-map
  0.913, corpus 0.9964 single-run, canaries flat. Plans may now return
  booleans (only to a root caller, never inside a call chain). Found on the
  way (fixed in e2c08594): a plan or numeric helper called with fewer
  arguments than parameters read `undefined` as a number (`f()` with
  `a === b` returned 2, QuickJS-NG 1).
- Stack run 4a8b31f3 vs main 49d8c58d (30 blocks, cycles, quiet host;
  `target/comparison/perf13-4a8b31f3`): numeric plans from bytecode, leaf
  plans on a stack array, the pinned executor address. External geomean
  0.995 against main and **0.853 against QuickJS-NG**; hash-map 0.859,
  math-cordic 0.937, controlflow-recursive 0.971. Worst against main:
  string-unpack-code 1.024; sentinels 0.94-1.04.
- Stack run 30413402 vs main 422ac19a (30 blocks, cycles, quiet host;
  `target/comparison/perf18-30413402`): the pin now names the executor's
  out-of-line dispatch loop `run<WideLoopFrame>` (it had been pinning a
  132-instruction caller), top-of-stack-only materialization for operand
  forwarding (kept `x * this.y` fused), dense element reads inline in the
  wide driver. External geomean 0.995 against main and **0.820 against
  QuickJS-NG**; 3d-raytrace 0.929, md5 0.966; worst ai-astar 1.018.
- Stack run 922e3f86 vs main e9529e66 (30 blocks, cycles, quiet host;
  `target/comparison/perf17-922e3f86`): hot named reads/writes inline in
  the wide driver, typed numeric field reads/writes inline (executor
  re-pinned at 0xe60). External geomean 0.992 against main and **0.823
  against QuickJS-NG**; access-nbody 0.825, raytrace-class-fields 0.954,
  cdjs 0.983; worst crypto-md5 1.031.
- Stack run 68edddc5 vs main c0097ea5 (30 blocks, cycles, quiet host;
  `target/comparison/perf16-68edddc5`): function properties on the name
  hasher, grandprototype cache entries, prototype hits from the hot entry.
  External geomean 0.993 against main and **0.828 against QuickJS-NG**;
  hash-map 0.847, xparb 0.971, binary-trees 0.974; worst 3d-raytrace 1.012.
- Stack run 895b06dd vs main 1bad7c18 (30 blocks, cycles, quiet host;
  `target/comparison/perf15-895b06dd`): hot named-read entry, operand
  forwarding. External geomean 0.997 against main and **0.834 against
  QuickJS-NG**; 3d-raytrace 0.952, cdjs 0.963, raytrace-class-fields
  0.966, tofte 0.969; worst sha256 1.033, regexp-dna 1.031.
- Hot named-read entry (b8ff0735): a hit walked all four cache entries
  (~150 instructions for `n.left`); the cache now remembers the entry that
  answered and checks it alone first when it is a constructor-shared slot.
  Tree-walk call 1,324 -> 1,210 instructions (QuickJS-NG 459).
- Operand forwarding (895b06dd): `Move` was the most executed wide op
  (a third of cdjs's). LoadLocal defers its copy; Binary, GetProp, plain
  GetPropNamed, Return and local-to-local stores read the local in place;
  anything else, joins and writes to the local materialize first. 3d-raytrace
  0.94, tofte 0.97. Remaining Moves are mostly call arguments. Diagnostic:
  an op histogram (`Move->next` pairs) in the perf-counters build found it;
  the patch is not kept (a format! per op).
- Stack run 8bbe675b vs main 4c378077 (30 blocks, cycles, quiet host;
  `target/comparison/perf14-8bbe675b`): pre-header typed entry, boxed
  element writes, dead completion temporaries. External geomean 0.982
  against main and **0.837 against QuickJS-NG**; hash-map 0.876, nbody
  0.899, crypto-aes 0.899; worst tofte 1.016; sentinels 0.92-1.00.
- Typed loops entered before their first iteration (perf14): an exit before
  each probed loop's header, reached only by falling into the loop, runs
  the loop's typed program from the header once that program has run a
  loop to its end from the backedge. nbody 0.910 (instructions 0.892),
  sentinels and ai-astar flat, corpus 0.985 single-run.
- Boxed element writes (0e8f1db9): hash-map's rehash stored objects with
  `newData[index] = entry`; the scalar dense write unboxed them and
  deoptimized the region on every outer iteration (98k interpreter frames
  per run). A boxed local is now written by `DenseWriteBoxed`; hash-map
  0.88. Adding the operation moved the executor's fast window to
  0xd90..0xdc0 (re-pinned at 0xda0).
- Dead completion temporaries (8bbe675b): typed regions read a function
  body's completion temporaries as `undefined` and drop their writes; the
  arms of an if/else-if chain leaving different ones no longer fail to
  join. Corpus 0.984, sentinels 0.970 single-run. ai-astar's neighbor loop
  now compiles but still deoptimizes on its first `findGraphNode` call:
  the typed tier cannot call a function that is not a closed-form leaf.
- Layout sensitivity, found (15a38453): with byte-identical code the
  typed executor's own start address decides capturing_closure_call and
  ai-astar (+18-25% outside 0xf80..0xfe0 mod 4 KiB); stack, heap,
  jump-table offsets and loop alignment measured flat. Pinned by
  `tools/benchmark/layout_pin.py` (fixed-size std filler ahead of it);
  see docs/performance-knowledge.md, "Codegen".
- Rejected (2026-09-24): the boxed typed-loop registers as a fixed array
  like the scalar file (d21a068c): 1-3% fewer instructions but +20% cycles
  on the sentinels, ai-astar and imaging-gaussian-blur, whether the file
  was its own 2 KiB box, shared one allocation with the scalar file, or
  was padded -- not 4K aliasing, not scratch allocation (1-7 fresh scratch
  per run); treated as dispatch-loop codegen. Patch /tmp/boxed-file.patch.
- Rejected (2026-09-24): regenerating `hot-functions.order` did not remove
  the layout swings (bits-in-byte +4% with identical instructions).
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

- tofte: after 7a89ea81 the remaining direct-eval cost is building the
  eval's environment (`apply_call_env`, `visible_local_entries`) and
  closure creation, not the overlay.

- Math.random from interpreted code costs 840 cycles per call (5.7x NG):
  the call runs through the generic path because the typed loop cannot
  call a stateful native.

- Admit bodies with a parameter prologue (default values) once their dead-zone
  behaviour is covered; CF traces count 179k general frames for them.
- The exit-heavy counter costs binary-trees and md5 about 2% instructions;
  look for a cheaper admission-side check.
