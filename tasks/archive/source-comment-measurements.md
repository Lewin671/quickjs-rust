# Measurements removed from source comments on 2026-10-09

Historical record. These comment lines were in the source at `d1313d52` and
were removed when module headers and comments were limited to what a module
owns and the invariant that keeps it correct. They describe the repository
at the revisions where they were written; most carry no date or commit, so
treat each number as unverified. `git blame d1313d52 -- <file>` dates a line.

## `crates/qjs-runtime/src/bytecode/compact_fn/activation.rs`

```text
//! The compact executor removed generic dispatch and bought ~5% on the
//! recursive sentinel, which established that dispatch is not what makes a
//! recursive call slow: at ~212 ns per call against QuickJS-NG's ~35 ns,
//! roughly two thirds of the remaining time is spent building and tearing down
//! the activation itself (`tasks/archive/T021-single-vm-frame-stack-log.md`).
```

## `crates/qjs-runtime/src/bytecode/compact_fn/execute.rs`

```text
/// `drop_in_place::<Value>` stays an out-of-line call and was 22% of the
/// recursive sentinel's profile -- for registers that only ever hold numbers.
```

## `crates/qjs-runtime/src/bytecode/compact_fn/mod.rs`

```text
//! `Vm::run_current_activation` is one 24,792-instruction function whose
//! register allocator has given up: every dispatch reloads `self`, `self.ip`,
//! and the code pointer from the stack before any opcode does work. That
//! preamble is most of the per-instruction gap against QuickJS-NG, and it is
//! paid by all ninety-odd opcodes at once, so no per-opcode change can remove
//! it (`tasks/archive/T021-single-vm-frame-stack-log.md`, 2026-08-01 root cause).
```

## `crates/qjs-runtime/src/bytecode/compact_fn/numeric_plan.rs`

```text
//! A recursive numeric body -- `fib`, `ack`, `tak` -- runs on the compact
//! tier at about 900 instructions a call against QuickJS-NG's 400, most of it
//! frame machinery for `Value` registers. When every register of the body
```

## `crates/qjs-runtime/src/bytecode/compact_fn/wide/activation.rs`

```text
/// object -- and two numbers. Kept out of the shared operator helpers, whose
/// growth re-rolled this tier's dispatch codegen (crypto-md5 +2.5% cycles at
/// identical instructions).
```

## `crates/qjs-runtime/src/bytecode/compact_fn/wide/mod.rs`

```text
//! opcode added to it measurably slows the recursive bodies it exists for:
//! carrying the named-property operations in that same `match` cost the
//! recursive sentinel 20%, whether the extra arms were filtered before the
//! match, kept out of line, or packed into the same eight-byte operation
//! word. So this tier is a second executor with its own operation set, its
```

## `crates/qjs-runtime/src/bytecode/frame_program.rs`

```text
/// The loop accelerators are *not* here. Four of the six pointers a combined
/// view would carry are theirs, and a call-heavy workload reaches a backward
/// edge in a small minority of frames: 12,700,004 frames against 100,000 edges
/// on the recursion sentinel. Holding them across the dispatch loop would make
/// every frame pay register pressure for something almost none of them use, so
/// they are derived where they are needed instead.
```

## `crates/qjs-runtime/src/bytecode/frame_stack.rs`

```text
//! Every ordinary call today constructs a whole nested [`Vm`] on the Rust
//! stack -- 12,700,004 of them for a workload performing 12,700,004 calls --
//! and recurses into it. That is the cost the call-frame migration exists to
//! remove, and it is also why JavaScript recursion past roughly a thousand
//! frames aborts the process with a native stack overflow instead of throwing
//! a catchable `RangeError`.
```

## `crates/qjs-runtime/src/bytecode/named_property_cache.rs`

```text
/// field from objects built by different literals. Four costs 1-1.5% on the
/// workloads whose sites stay monomorphic -- the state is twice the size and
/// a thrashing site rewrites twice as many entries -- and is worth 18% where a
```

```text
/// can hold then allocates on every other update, which cost 4.7% on
/// prototype-dispatched reads.
```

```text
/// Every method call has this shape, and it was the one shape this cache
/// could not hold: a receiver miss cleared the whole site and walked the
/// chain again on the next read. Measured, that made a prototype-resolved
/// read cost about 20 ns more than an own-property read, against 2.5 ns
/// for QuickJS-NG.
```

```text
// to build with unrelated code (1792 or 1392 bytes), and the rolled
// loop cost call-heavy property workloads 2-4% (crypto-md5, cdjs).
```

```text
/// with unrelated code, and a build that keeps it out of line is about
/// 4% slower on every property-heavy workload (3d-raytrace, access-nbody).
```

## `crates/qjs-runtime/src/bytecode/typed_loop/compile.rs`

```text
// A region used to be rejected here when more than a third of its operations
// were boxed, on the reasoning that the interpreter's own inline caches
// already answer the property protocol as well as this tier does. What that
// rule actually excluded was the whole surrounding region: a loop that keeps
// an object in a local, or reads two elements per iteration, crosses the
// ratio and then pays generic dispatch for its arithmetic, its induction
// variable, and its branches as well. The generic dispatch it fell back to is
// not free -- it reloads the frame pointer, the instruction pointer, and the
// bytecode bounds from the stack for every instruction -- so the comparison
// the rule assumed was never between two equal dispatchers.
//
// Measured on the six generic-path sentinels, admitting these regions leaves
// five unchanged (0.996-1.004) and takes the property-read sentinel to
// 0.5228 [0.5118, 0.5309]; its executed instruction count falls from 5.2M to
// 2,579 because the region now runs natively instead of declining on every
// backedge. The 40-case SunSpider/Kraken corpus that motivated the rule is
// 0.9955 with no repeatable per-case regression, including the three cases
// the rule was introduced to protect.
```

```text
// which measured a 19% regression on `regexp-dna` and 1% over
// the 40-case corpus. A dictionary loop reaches the boxed key
```

```text
// resolved one only in having no receiver, and adding a
// twenty-second arm to this dispatch loop measured 8-11% on
// `heterogeneous_property_read` with the operation never
// even reached -- the register allocator, not the work.
```

## `crates/qjs-runtime/src/bytecode/typed_loop/execute.rs`

```text
// executors run their shape far faster -- an FFT butterfly went
// 0.58 s -> 1.30 s through this tier's element operations
// (audio-fft). A dense recurrence plan is not faster than an enclosing
// typed region: fannkuch's outer loop, enclosing two, runs 1.36 vs 1.96
// against QuickJS-NG when it is declined instead. So only the special
// plans keep their enclosing region on the interpreter.
```

```text
/// call and a test: a branch on the argument mode inside the arm itself,
/// executed a few thousand times, cost ai-astar 15% by re-rolling the
/// loop's register allocation.
```

```text
// building an argument array and walking the body twice, which cost
// `imaging-darkroom` 4.1%.
```

```text
// time was a hash lookup per read (audio-dft +3% instructions). After
// the shapes: checked first, a polymorphic site's small-storage misses
// paid for it (heterogeneous_property_read +6% cycles).
```

## `crates/qjs-runtime/src/bytecode/typed_loop/helper_graph.rs`

```text
//! A call to an ordinary JavaScript function used to abort a whole loop region
//! at compile time, because the tier could only lower an intrinsic reached
//! through the `Math` global. That single gap is the terminal blocker on the
//! generic-path half of the external corpus: `imaging-darkroom`,
//! `stanford-crypto-aes`, `crypto-aes`, `access-nsieve` and `string-fasta` all
//! give up on an `Op::Call`, and `imaging-darkroom` alone dispatches 449 M
//! generic instructions with every one of its 1.9 M backedges declining.
```

```text
//! re-guarding the graph at every helper invocation was measured on this
//! workload and regressed it to 1.13x
```

```text
/// changing the layout it walks re-rolled that loop's register
/// allocation -- ai-astar and the call sentinels moved 18-25% with
/// identical instructions.
```

```text
// shadowing one cost `math-cordic` 3.7%. The site then simply has
// no graph entry, which is the state every call site was in before
// this module existed.
```

## `crates/qjs-runtime/src/bytecode/typed_loop/helper_graph/numeric.rs`

```text
//! re-checks its operands' tags and returns through an `Option`: about forty
//! instructions an operation, where QuickJS-NG's bytecode spends about
//! sixteen. `bits-in-byte`'s `bitsinbyte` -- a counting loop over a byte --
//! cost 5,800 instructions a call that way against QuickJS-NG's 1,960.
```

## `crates/qjs-runtime/src/bytecode/typed_loop/mod.rs`

```text
//! The specialized loop tiers each match one exact opcode sequence, so a loop
//! that computes the same thing in a different shape — an `if`/`else` in the
//! body, an extra temporary, a different operator order — runs on the general
//! interpreter at roughly 4.5ns per opcode. This module accepts *any* loop
//! region built from a whitelist of opcodes, compiles it once into a register
//! program, and runs that program with no operand stack, no `Value` boxing for
//! arithmetic, and no per-opcode dispatch through the main match.
```

```text
/// one out-of-line call: the dispatch loop's register allocation re-rolls
/// on any growth of an arm, and cost ai-astar 15% twice while this was
/// being added.
```

```text
/// The one-way `caches` entry is not enough on a polymorphic site:
/// `heterogeneous_property_read` rotates three distinct object-literal shapes
/// through one read, so it missed every iteration and fell back to resolving
/// the name -- a hash lookup and a `memcmp` per access, which the profile
/// charged 27% of that sentinel to. Shape identity is an `Rc` pointer
/// comparison, so scanning a few is far cheaper than resolving the name.
```

```text
/// keeps their entry one pointer instead of three. Measured, not assumed:
/// unboxed costs `prototype_method_call` 3.9% against 2.6% boxed, and that
/// sentinel never reaches the shape path.
```

```text
/// Two reasons. Adding a sixth vector to the scratch cost
/// `prototype_method_call` 3.6%: that sentinel declines this tier on
/// essentially every backedge, and a declined backedge still moves the
/// scratch twice. And a site's shapes are a property of the site, not of
```

```text
/// program. Threading it through `execute` instead measured 16% on
/// `heterogeneous_property_read`: one more loop-carried live value costs
/// this executor every opcode, whether or not the value is used.
```

## `crates/qjs-runtime/src/bytecode/vm.rs`

```text
// registers, so none of `FrameState`'s 704 bytes would be read. Admission
```

```text
/// `FrameState` is built and dropped once per general-path call, and measured
/// per-call cost tracks its size: twelve extra empty fields cost an ordinary
/// call about 18%. Everything here is reached only by `try`/`finally`,
```

## `crates/qjs-runtime/src/bytecode/vm/general_ops.rs`

```text
//! out of line precisely *because* they are large. Disassembled, the single
//! 25,020-instruction dispatch function spilled `self`, the code pointer and
//! the length across every dispatch, so all ~90 opcodes paid an eight-memory-op
//! preamble before doing any work. An opcode that already costs a property
//! lookup or a call absorbs one extra call; an opcode that costs four
//! instructions cannot.
```

## `crates/qjs-runtime/src/bytecode/vm/rare_ops.rs`

```text
//! `Vm::run_current_activation` disassembled as a single 25,020-instruction
//! function with a 4.3 KB stack frame and 335 distinct spill slots: the
//! register allocator had given up, so every dispatch reloaded `self`, the code
//! pointer and the code length *from the stack* before any opcode did any work.
//! That preamble is paid by the opcodes a benchmark actually executes, and it
//! is caused by the ones it never executes -- `NewFunction`, `TypeofGlobal`,
//! the `super` family and the generator suspensions each inline a large body
//! into the same function and compete for the same registers.
```

## `crates/qjs-runtime/src/bytecode/vm_bindings.rs`

```text
// Building the `PREFIX + name` marker for each local did: on
// `string-tagcloud`, whose source evals a JSON payload, those `format!`
// calls were 11% of the profile's allocator traffic, all of it to
// discover an empty set. The bindings are the smaller side and already
// carry the marker, so strip the prefix from them instead.
```

## `crates/qjs-runtime/src/bytecode/vm_frame_init.rs`

```text
/// hashed every one (date-format-tofte spent 15% there).
```

## `crates/qjs-runtime/src/bytecode/vm_numeric_leaf.rs`

```text
/// with unrelated edits elsewhere in the crate, and the inlined copy grew
/// the executor by 176 bytes and re-rolled its code (access-nsieve +4%,
/// heterogeneous_property_read +5% cycles at equal instructions).
```

## `crates/qjs-runtime/src/value/object/slot_reads.rs`

```text
// executor, whose hot arm grew and re-rolled its register allocation
// with a dynamic-storage case (access-nbody +7% instructions). A
// dynamic object takes the executor's general read instead.
```
