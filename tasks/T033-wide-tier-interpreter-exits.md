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
- Earlier stack runs and measurements (perf8 through perf20) are in
  `tasks/archive/T033-screen-log-early.md`.
- perf21 units (c096993c..e73471e6): a cached direct eval that writes and
  deletes no binding skips the caller's frame write-back (apply_env, 8% of
  date-format-tofte; tofte 0.917); run<Vm> pinned ahead of the wide
  executor (behind the callees it drifted with every change to the number
  of per-CGU `Value::clone` copies); dynamic-storage objects (a
  constructor's twelfth property on) read and written by shared slot
  (3d-raytrace's triangles thrashed per-object entries: 0.899), except in
  the typed executor's inlined number arms (access-nbody +7% instructions).
- Rejected (2026-09-25): incremental deopt-bindings overlay (only overlay
  cells from the first difference): tofte 1.019 -- the cost is the walk
  over the frame's locals per closure, not the hashing. `Value::clone_inline`
  (object/function/array/string arms expanded at register moves, upvalue and
  slot reads): tree walk 0.967 but 3d-raytrace 1.029, md5 1.056, instructions
  up 1-2%. Call path attribution (helpers forced out of line, wcall_e): run
  58%, Value::clone 20%, clear_window 6%, entry helpers 8% -- a trivial wide
  call+return is ~790 instructions against QuickJS-NG's ~340.
- perf22 units (76cc36cf..88e90f89): layout slots -- the executor and
  each pinned callee own a fixed-size slot padded with std filler, so a
  callee growing inside it moves nothing (identical-instruction swings had
  been churn 22%, hetero 4%); typed `get_named_object` reads a
  dynamic-storage receiver through the site's shared slot (audio-dft -8%
  instructions); a repeated prototype read proves the own miss with one
  lookup (prototype_method_call 0.93); dense `splice` in place
  (stanford-crypto-sha256 -26%, pbkdf2 -15%); and `typed_loop/forward.rs`,
  copy forwarding over the final typed program -- a copy's reader takes
  the source (site entries rewritten with it), a stored result is computed
  into the local, `ToNumeric` ahead of `Update` goes: findGraphNode 13 -> 7
  operations per iteration, ai-astar 0.78 then 0.82 again on top, nsieve
  0.88, fannkuch 0.92. Liveness counts a site's entries only where an
  operation can stop. Differential fuzz (600 random loops with type
  changes mid-loop) matches the pass-off build and V8.
- perf26 units (6f500ce6..fe533cc7, branch agent/perf-25): a `for-in`
  site remembers the prototype chain of its last ordinary target with the
  keys that chain contributes, so a loop over many objects of one shape
  enumerates own keys plus the unshadowed inherited ones without the
  shadowing hash set (a two-key `for-in` 3599 -> 1346 cycles, NG 1082); the
  wide tier's `for-in` exits answer ordinary objects without an environment.
  Closures created by a function with a direct `eval` bypass its dynamic
  scope while the scope binds none of their by-name names; the scope keeps
  the list and revokes every bypass (with the memoized call-path facts) on
  its next generation change, frames that can suspend are excluded, and at
  most 64 closures bypass one scope (`Vm::closure_scope_use`,
  `DynamicBindings::bypass`): string-tagcloud 0.784 (its JSON `walk`),
  tofte 0.979. The numeric helper's register file is 64-byte aligned
  (instruction count no longer depends on caller frame sizes) and
  `NumProgram::run` is pinned.
- Stack run fe533cc7 vs main cebad5e9 (30 blocks, cycles, loaded host;
  `target/comparison/perf26-fe533cc7-30b`): external geomean **0.995**
  against main and **0.749 against QuickJS-NG**; string-tagcloud **0.759**
  (1.31 -> 1.000 against NG), tofte 0.975, regexp-dna 0.992. Worst:
  JetStream gaussian-blur 1.057 -- the same two binaries measure 1.001 by
  min-of-5 alternation from /tmp, so this is the harness-path sensitivity
  already recorded, not code; access-nsieve 1.018. Sentinels 0.993-1.002
  (recursive_call_tree 0.993: the 10% below did not reproduce here).
- Layout (2026-09-26): recursive_call_tree ran 10% more cycles with the
  `for-in` unit at identical helper code and identical helper offset
  (`NumProgram::run` pinned where main has it): its jump tables in
  read-only data moved with other code's constants, which the order file
  cannot place. With `MallocNanoZone=0` the gap was 2%. Other sentinels
  flat. Min-of-N alternation (`/tmp/minab.sh`, `/tmp/minsent.sh`) resolved
  these while the host was loaded; the screen tool's intervals did not.
- Found (2026-09-26): 3d-raytrace's `Scene.blocked`/`intersect` loops
  compile to typed programs that decline at entry every time (`TLRUN ...
  Declined`): they write the implicit global `i`, and
  `WideLoopFrame::prepare_typed_loop_sloppy_global_write` always declines.
  Supporting it would not help this case: the loop calls
  `triangle.intersect`, not a closed-form leaf, so the program would
  deoptimize at the call (`global_store_stays_interpreted` admits the body to
  the wide tier for that reason). The loop's cost is the wide tier's own
  (1.25x NG per triangle).
- perf25 units (4d19c880..130c27c6, branch agent/perf-23): found by
  splitting each slow case into micro pieces and comparing each against
  QuickJS-NG by the N-versus-2N instruction and cycle delta. A regex literal
  builds its object directly (`new RegExp(string, string)` with `new.target`
  the constructor: 9455 -> 3301 instructions per literal, NG 1723); `new
  String(x)` likewise (`function::construct_intrinsic_directly`); ToPrimitive
  skips the `@@toPrimitive` walk on an ordinary chain that lacks it, and the
  generic `+` joins primitives in place; a Date method reads its time value
  once; a closure created by direct-eval code that resolves none of the
  eval scope's names by name drops that scope (`closure_needs_dynamic_scope`
  -- the fork only that code writes, so nothing can appear later), which
  puts xparb's format functions on the direct path; `x == null` in the wide
  tier's conditional jump without the general operator (binary-trees
  0.968). Typed loops compare against string literals without unboxing.
- Stack run e34c35b4 vs main 5110c210 (30 blocks, cycles, same host with
  a transient foreign `qjs` process; `target/comparison/perf23-e34c35b4-30b`):
  external geomean **0.990** against main and **0.756 against
  QuickJS-NG**; xparb **0.677** (1.46 -> 1.016 against NG), validate-input
  0.928, binary-trees 0.968, ai-astar 0.974. Worst: crypto-md5 1.042 at
  identical instructions -- the null case added to
  `operations::eval_binary_without_env` re-partitioned codegen units and
  re-rolled the wide dispatch loop (176 bytes smaller). Moved into a
  wide-only out-of-line helper in 130c27c6: md5 1.005 against main by
  min-of-15 alternation, sentinels and canaries flat (screen, not a
  formal run). `compact_fn::numeric_plan::run` pinned as well.
- Rejected (2026-09-26): typed-loop inherited reads remembered by value for
  a dynamic-storage prototype (`Date.prototype`): prototype_method_call
  +1.25% instructions, ai-astar +3% cycles, xparb gained the same without
  it. Patch /tmp/tl-inherited.patch. Re-tried the incremental deopt overlay
  (already rejected 2026-09-25): the memo already hits; the per-closure walk
  is the cost. A discarded-value `x++` statement form in the compiler: the
  loop-plan matchers (control, numeric, mutation, predicate scan, virtual
  object) key on the six-op postfix statement shape, and the wide lowering
  already folds it where a `Pop` follows.
- Measured (2026-09-26, instructions per operation against QuickJS-NG,
  wide tier): method call `l.g()` 728 vs 343, own read `l.item` ~180 vs 22,
  `if (l === l) n++` 309 vs 132 (the `if` join keeps the postfix copy);
  calls to constant-return functions look at parity only because they are
  closed-form leaves. Date getters run 2x faster than NG's.
- perf24 units (30a93030..7d18c3ff): a numeric helper may call another
  helper of its graph (itself included) when its arguments are proven
  numbers -- arguments copied to contiguous registers above the body's, the
  callee's numeric body run under the same recursion bound, and `settle`
  dropping, to a fixed point, any body whose callee is not numeric or does
  not return a number: recursive_call_tree 716 -> 370 instructions a call,
  0.566 cycles against main. A number-only closed-form leaf whose
  parameters reach the result only through operators takes boolean and
  undefined arguments by ToNumber (sha1's `safe_add(e, w[j])` past the end
  of `w` deoptimized its block loops; neutral on time).
- Stack run 7d18c3ff vs main 83108837 (30 blocks, cycles, quiet host;
  `target/comparison/perf24-7d18c3ff`): external geomean 0.998 against
  main (0.772 against QuickJS-NG); sentinels **0.901** against main and
  **0.847 against QuickJS-NG** -- every sentinel now at or below
  QuickJS-NG (recursive_call_tree 0.682, heterogeneous_property_read
  0.998, prototype_method_call 0.996).
- Rejected (2026-09-26): global-function helper sites (flattening a global
  callee at every entry of an inner loop cost sha1 2.4% instructions, and
  its deopting call was a local closed-form one); a hand-written
  `Value::clone` (bit-test plus bit copy; churn +15% cycles at +1.5%
  instructions); borrowing the prototype chain in the creation proof
  (binary-trees +0.3% instructions: a RefCell borrow per level costs what
  the Rc upgrade did).
- perf23 units (c88cbb06..04e0e7aa): the front end measured 2-3x
  QuickJS-NG (`tools.benchmark.front_end`; 1-8% of many cases' totals):
  binary operators by precedence climbing (imaging-darkroom's parse -21%
  cycles), the compiler's scope tables on the name hasher (cdjs parse
  -10%), canonical UTF-8 copied straight through (cdjs -9% instructions),
  borrowed name sets in bytecode finalization. A parser bug found on the
  way: `in` inside brackets of a for initializer was rejected. Layout:
  math_binary kept out of the typed executor (a codegen-unit shift had
  inlined it: nsieve +4% at equal instructions), boxed_equality and
  get_named pinned, executor re-scanned to 0x200.
- Stack run 3babb930 vs main f2b21ab6 (30 blocks, cycles, quiet host;
  `target/comparison/perf23-3babb930`): external geomean **0.994** against
  main and **0.772 against QuickJS-NG**; gaussian-blur (JetStream) 0.952,
  imaging-darkroom 0.963, tinderbox 0.965, desaturate 0.971. Worst:
  ai-astar 1.023 (byte-identical binaries at two paths differ by 1-2%
  there), regexp-dna 1.015; sentinels 1.003. Slowest against QuickJS-NG:
  xparb 1.46, tofte 1.41, validate-input 1.31, binary-trees 1.31.
- Stack run 88e90f89 vs main 8d7c580e (30 blocks, cycles, quiet host;
  `target/comparison/perf22-88e90f89`): perf21's and perf22's units.
  External geomean **0.962** against main and **0.782 against
  QuickJS-NG**; ai-astar 0.648, stanford-crypto-sha256 0.716, pbkdf2 0.863,
  audio-dft 0.864, nsieve 0.873. Sentinels 0.947 against main, 0.938
  against QuickJS-NG (recursive_call_tree 1.21 the one clearly behind).
  Worst against main: crypto-md5 1.038, raytrace-class-fields 1.026, cdjs
  1.022, binary-trees 1.019 at equal instructions -- `NamedPropertyCache::
  probe` had come out rolled (1392 bytes, 1792 unrolled); unrolled by hand
  in 4e77b832, those read 0.996-1.001 against main. Slowest against
  QuickJS-NG: xparb 1.49, tofte 1.42, binary-trees 1.34, cdjs 1.34,
  3d-raytrace 1.33, tagcloud 1.32, validate-input 1.31, hash-map 1.31.
- Stack run 0f6349a2 vs main 553d0ba4 (30 blocks, cycles, quiet host;
  `target/comparison/perf20-0f6349a2`): the units above. External geomean
  0.995 against main and **0.797 against QuickJS-NG**; validate-input
  0.944, fasta 0.952, cordic 0.971, 3d-raytrace 0.975. Worst against main:
  imaging-gaussian-blur 1.013; sentinels 0.996-1.003. Slowest against
  QuickJS-NG: tofte 1.55, xparb 1.52, ai-astar 1.47, 3d-raytrace 1.46.
- Stack run 6f5388fa vs main 422ac19a (30 blocks, cycles, quiet host;
  `target/comparison/perf19-6f5388fa`): perf18's stack plus the callback
  units. External geomean 0.991 against main and **0.801 against
  QuickJS-NG**; string-unpack-code 0.872, 3d-raytrace 0.927, crypto-md5
  0.965. Worst against main: regexp-dna 1.015; sentinels 0.986-1.004.
  Slowest against QuickJS-NG: tofte 1.55, xparb 1.54, 3d-raytrace 1.50,
  ai-astar 1.47, validate-input 1.42, tagcloud 1.35, binary-trees 1.34.
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

- Fixed in de8accb1 (predated this task): a typed-loop named read took an
  own accessor for a miss and returned the prototype's remembered value, so
  `ps[5].get()` called the prototype method past an own getter.

## Next

- tofte: after 7a89ea81 the remaining direct-eval cost is building the
  eval's environment (`apply_call_env`, `visible_local_entries`) and
  closure creation, not the overlay.

- tofte (1.43 against NG): a formatDate call's fixed cost is 55k cycles
  against 30k -- 26% is the deopt-overlay walk over the frame's 45 locals
  for each of its 28 closures, 20% frame setup, 12% creating the closures.
- The wide tier's per-operation cost (2-3x QuickJS-NG: method call, own
  and prototype reads) is now the common factor of hash-map, cdjs,
  3d-raytrace, binary-trees and raytrace-class-fields.

- Admit bodies with a parameter prologue (default values) once their dead-zone
  behaviour is covered; CF traces count 179k general frames for them.
- The exit-heavy counter costs binary-trees and md5 about 2% instructions;
  look for a cheaper admission-side check.
