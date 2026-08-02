# T024: General Register Execution Core

## Goal

Replace `Vm::run_current_activation` as the path ordinary synchronous function
bodies take, with a register-addressed executor whose hot loop stays small
enough for the register allocator to keep its program counter and code pointer
in machine registers.

This task exists because the alternative was measured and does not reach the
target. It is the successor to T021, whose own premise (a single-VM frame
stack) is retired, and it inherits T021's receipts.

## Why the incremental route cannot finish the job

Measured 2026-08-02 against QuickJS-NG on the 26 comparable
SunSpider/Kraken cases (`scripts/external-corpus-ab.py`):

| | start of day | end of day |
|---|---|---|
| external corpus geomean | 1.6064 | 1.4882 |
| six generic sentinels | 1.2359 | 1.0643 |

Seven admission commits landed that day and twelve attempts were rejected — a
hit rate near one in three. The best single unit took `access-fannkuch` from
2.10x to 1.21x, which moved the corpus geomean by about **1%**, because one
case in twenty-six is worth `(1.21/2.10)^(1/26)`.

From 1.4882 to below 1.0 is a further 33%. At ~1% per landed unit and one
landed unit per three attempts, that is on the order of **forty successful
units and a hundred and twenty attempts**. Continue admission work only where a
single case's ratio is embarrassing; it is not a route to parity.

Two further measured facts shape the design rather than the schedule:

- **Blockers are conjunctive.** A declining loop region needs its *whole*
  blocker set fixed. Two attempts that picked the tallest bar of a corpus-wide
  terminal-blocker histogram (`StoreGlobalSloppy` 13 regions,
  `element_assignment` 11) were implemented in full and admitted **nothing** —
  `executed_ops` came back byte-identical. Both units that did land were the
  *last* remaining blocker for some shape.
- **Additions to the existing executor are taxed ~3%.** `typed_loop::execute::run`
  is 3,816 machine instructions and sits at a register-allocation cliff. A new
  `TypedOp` arm costs 8-11% *with the operation never reached*; a loop-carried
  live value costs 16%; losing an inline on a shared helper costs 10%. Even a
  change in an unrelated module (`regexp/matcher.rs`) grew it to 4,056 and
  regressed four sentinels 2-6%. A replacement escapes this tax; an addition
  does not.

## Design constraints, each from a measurement

1. **Seed from `typed_loop`'s mixed scalar/boxed register model, not
   `compact_fn`'s all-boxed one.** Falsified directly: removing `compact_fn`'s
   backward-edge gate and adding `Op::AssignLocal` admits a plain scalar loop
   body (`compact_standalone_activations` 20,000, generic ops 780,038 ->
   320,038) and it runs **2.19x slower** — 1.09x NG before, 2.39x after.
   `compact_fn` registers are `Value`: 16 bytes, `needs_drop`, a discriminant
   test per access. Claiming a body downgrades its loops from `typed_loop`'s
   scalar `f64` registers. `compact_fn` is right for a recursive call spine and
   wrong for a loop; a general core must be right for loops.
2. **Not a repacked stack instruction stream.** A "compact generic bytecode
   core" that lowered ordinary bytecode while keeping the VM helpers was
   already built and measured A* 0.997, hash-map 1.125, dynamic-method-call
   1.621 (`tasks/T021-single-vm-frame-stack.md:2086`). That shape is closed.
3. **Zero-based registers per activation.** The single-VM frame stack worked
   mechanically — 254,001 nested activations became 2,001 window switches —
   and measured 1.2678, because every register access then paid a `base +`
   offset (T021). Each activation's registers must start at zero.
4. **Small hot loop, cold semantics out of line.** Coercion, Proxy, accessors,
   iterators, exceptions and every rare opcode behind `#[cold]
   #[inline(never)]` helpers. `#[cold]` alone was worth 6.6% on
   `math-partial-sums` when applied to one such helper.
5. **Exact side exits or pre-entry decline; never replay after observable
   work.** `TypedOp::Exit` and `TypedOp::Leave` are the existing pattern: hand
   one bytecode instruction back with the operand stack rebuilt from the
   registers, and let the interpreter run it.
6. **Staged coverage, but an eventual ordinary default** — not another narrow
   tier. A tier that claims a body it cannot run well is worse than none, per
   constraint 1.

## Slice 1 was built and reverted — read this before staging the next one

A scalar-only whole-body core (`bytecode/general_core/`: `Scalar` registers,
stack-depth-to-register lowering, `#[inline(never)]` executor, admission behind
a default-off `general-core` feature) was written, tested and measured
2026-08-02, then reverted. Three results, all of which change the staging above:

1. **The hook point is below several tiers that already claim small bodies.**
   `eval_direct_call_bytecode` is reached only for a direct-leaf call, and a
   body small enough for the closed-form evaluators -- `return a - b;` -- is
   answered before it. Worse, reachability depends on the **caller's** shape:
   the same callee reached the core 4,000 times when the caller looped and zero
   times when the caller was itself claimed by the compact tier. Any test that
   asserts on a small body is therefore fail-open, and one written that way
   passed with the entire tier deleted.
2. **A scalar-only core is ~9% slower than what it replaces**, on the one shape
   that does reach it: 0.0685s against 0.0626s with the feature off, NG 0.0595s.
   Two iterations (a 16-entry register file instead of 64, then a
   both-operands-are-numbers fast path) moved it from 0.0720 to 0.0685 and no
   further.
3. **The reason is the thing to design around.** The body it claims was already
   running its loop on `typed_loop`'s scalar registers inside a direct-leaf
   frame. Removing the frame is the core's whole advantage there, and it buys
   *less* than a less-specialized loop dispatch costs -- `typed_loop` has fused
   `Update`/`ToNumeric` forms this had no equivalent for.

**So the first slice must not be scalar-only.** Bodies made of scalars are
already well served; the core's first possible win is on bodies `typed_loop`
cannot claim at all, which means starting at boxed registers and the
representation fixpoint rather than deferring them. Stage accordingly, and
judge the core on `access-binary-trees` or `crypto-md5`, never on an arithmetic
microbenchmark.

## Scope

- Allowed paths: `crates/qjs-runtime/src/bytecode/` (a new module),
  `docs/design/`, this file.
- Forbidden paths: `third_party/`, the environment/binding model (T016),
  anything that would add an arm to `typed_loop::execute::run` or
  `Vm::run_current_activation`.
- Owner boundary: serialize on one branch. This touches the execution core.

## Acceptance Criteria

Codex's falsification gate, adopted unchanged. Judge nothing until the
mechanism gate is met, and abandon rather than widen if the timing gate fails.

- **Mechanism gate**: at least 80% of a generic-heavy case's dynamically
  executed bytecodes run in the new core. `kr/ai-astar` is no longer the right
  probe (it moved to `typed_loop`); use `ss/access-binary-trees`,
  `ss/crypto-md5` or `kr/stanford-crypto-aes`, all still generic-path.
- Then, and only then: that case at most **0.85** candidate/base, one other
  independent generic-heavy case at most **0.85**, new-core cost below **4 ns
  per executed semantic operation**, every predeclared control at most **1.03**,
  exact counters and output unchanged, Test262 zero regression.
- **If coverage is met and the case stays above 0.85, abandon that core design
  rather than widening its opcode set.** More arms raise pressure without
  fixing per-operation economics.

## Verification

```sh
./scripts/check.sh
./scripts/external-corpus-ab.py third_party/quickjs-ng/build/qjs target/release/qjs --reps 3
```

After **any** change, compare the hot executor's size against the base — a
growth there predicts a sentinel regression better than any single-case timing:

```sh
objdump -d --no-show-raw-insn target/release/qjs \
  | awk '/^([0-9a-f]+) <.*typed_loop7execute3run.*>:/{f=1;c=0;next} f&&/^$/{print c;exit} f{c++}'
```

## Notes

- Build it behind a cargo feature that is off by default until it wins. The
  default binary then stays byte-identical and pays no tax while the core is
  incomplete, which matters because constraint 1 says a half-built core is a
  regression, not a neutral.
- `executed_ops` alone does not rank cases. `kr/stanford-crypto-aes` lost
  **36%** of its dispatched instructions in `1a780181` and did not get faster
  at all (1.8629 -> 1.8583): its cost is the native work those instructions
  call, not dispatch.
- The three remaining corpus problems are distinct subsystems and this core
  addresses only the first: `access-binary-trees` 3.68 (call and frame
  construction, ~270 samples against 204 of interpretation),
  `string-tagcloud` 4.31 (allocator — malloc/free is 50% of its profile), and
  `access-nbody` 3.12 (named-property write resolution; a one-way write cache
  was built and measured 0.9513 there but 1.031/1.032 on churn and A*, a wash).
