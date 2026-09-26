# T033 screen log: earlier stack runs and measurements

Moved from `tasks/T033-wide-tier-interpreter-exits.md` (Screen log) to keep
the task file under the task-file size limit. Entries are in their
original order; later entries are in the task file.

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
- Callbacks (980c95fe, 6f5388fa): `arr.forEach(function (x) { total += x;
  })` ran 9.5x slower than QuickJS-NG -- the callback assigns a captured
  variable, so the wide tier declined it (the received-cell proof was
  read-only) and every call built an interpreter `Vm`; and `call_function`
  (every native's callback path) built a compatibility frame environment
  per call. Received cells a body only reads or plainly assigns are now
  written through the cell (`cell_received_upvalue_slots`,
  `StoreUpvalueLocal`; loops keep the interpreter), and natives call direct
  leaves the interpreter's way (`call_direct_leaf_function`). forEach
  1,350M -> 595M cycles (NG 133M); string-unpack-code 0.84 single-run.
  Remaining: the per-call argument `Vec` in array iteration, the closed-form
  probes before the tiers, and the wide entry's storage swap.
- Callback and global-variable costs (perf20, cb708c9c..0f6349a2): the
  forEach-with-a-global-accumulator micro was 3.3x QuickJS-NG after the
  callback units; sampling split it into the argument Vec per call
  (`call_function_slice` passes a direct leaf a slice, 0.770), the element
  read resolving Array.prototype by name per element
  (`plain_dense_index_value`, 0.905), `LoadGlobal` hashing its name per
  read (a per-site realm-cell memo keyed on the realm table's generation,
  0.936; validate-input 0.959) and the global store cloning then re-finding
  the globalThis property (`write_existing_own_data_property_if`, 0.859;
  validate-input 0.977). Also pinned `run<Vm>` (see
  docs/performance-knowledge.md): unpinned, an unrelated edit cost
  math-partial-sums 4.7%. Corpus screen 0.993 single-run.
