# T021: Single-VM Frame Stack And Compact Execution Core

## Goal

Remove recursive per-call VM construction from ordinary synchronous bytecode
calls, then compact the same execution core into register-oriented or
superinstruction dispatch. This is the next structural T018 unit: it must
produce general external wins while preserving the current zero-gap Test262
baseline. It is a foundation for the final every-case `<= 0.50x` QuickJS-NG
contract, not permission to specialize benchmark identities or loop shapes.

## Current Evidence

Trusted-main Performance Preview run `29865188694` at
`b8d0c2385128ad823a18488326a64299cffc3b2a` reports:

- broad overall candidate/QuickJS-NG `0.3322x`, but the apparent aggregate win
  hides `top_level_function_call 9.9898x`, `dynamic_method_call 7.0351x`,
  `array_write 4.7977x`, `array_allocation 3.3963x`, and
  `object_allocation 2.6901x`;
- critical families still above the final per-family boundary are allocation
  `2.6066x`, array `0.6048x`, and string `0.5402x`;
- external candidate/QuickJS-NG is `8.160x` on JetStream (5/5 comparable),
  `4.912x` on Kraken (7/14 comparable), and `7.649x` on SunSpider (26/26
  comparable), with zero qjs-rust wins. The two focused call gates are
  JetStream `hash-map 9.0627x` and SunSpider
  `controlflow-recursive 10.7814x`;
- a local `sample(1)` profile of JetStream `hash-map` attributes about 29% of
  samples to `Vm::run_completion`, about 17% to allocation/free, and about 7%
  to `Value` clone/drop. Per-call `Vm`, `CallEnv`, locals, and operand-buffer
  construction remain visible.

The matching Test262 Coverage run `29865445340` is the correctness floor:
42,672 configured cases passed, with zero failures, timeouts, not-run cases, or
actionable QuickJS-NG gaps. No performance unit may lower that result.

## Architectural Decision

Use one execution semantics and this order:

1. collect the existing VM's per-invocation fields into an owned `FrameState`;
2. trampoline only the existing direct-leaf eligible ordinary synchronous
   bytecode calls on `Vec<FrameState>`;
3. replace dispatch inside that same executor with compact register operations
   and/or superinstructions;
4. attack remaining object/array/string/RegExp/JSON costs from independent
   external profiles.

Do not build a separate numeric/control VM. A second VM would duplicate
completion, exception, binding, call, and deoptimization semantics and would
turn every future Test262 fix into a two-engine maintenance problem. A JIT is
also out of scope while the repository forbids FFI and lacks a stable compact
IR, safepoints, and deoptimization state.

## Scope

- Allowed paths for R1/R2: `crates/qjs-runtime/src/bytecode/`, the minimum
  `crates/qjs-runtime/src/function/` call preparation needed to reuse existing
  direct-call slots, and focused runtime tests.
- Allowed paths for later compact dispatch: the existing bytecode IR/compiler
  and the same executor; each slice gets its own measured commit.
- Forbidden paths: `third_party/`, benchmark identity/path/checksum branches,
  an independent second VM, widening `is_direct_leaf_function` in the same
  commit as trampoline routing, or weakening Test262/benchmark coverage.
- No new dependency, `unsafe`, FFI, or platform-specific executable-memory
  work in R1/R2.
- Global docs, task files, Cargo files, and benchmark protocol files remain
  integration-owner only.

## Parallel Assignment

- Each coding owner starts from an exact recorded `main` SHA in an isolated
  `agent/<task-slug>/<owner-id>` worktree.
- R1 and R2 serialize because both own VM state and call control flow.
- Independent read-only profiling and completed-CI artifact analysis may run
  concurrently. After local verification, push promptly and continue the next
  isolated unit while hosted CI runs asynchronously.
- Integration owner: main agent.

## R1: Owned Frame State

Move the existing per-invocation fields (`bytecode`, instruction pointer,
operand stack, locals/upvalues, `CallEnv`, try/pending completion state,
`with`, disposal, realm/module host, and caches) behind one owned frame value.
Keep all existing opcode handlers and all existing recursive call routing.
Avoid a self-referential frame: operand storage owns its `Vec<Value>`, while a
borrowed root bytecode or shared nested bytecode owner supplies recycling and
instruction access.

Acceptance:

- no observable behavior or call eligibility change;
- focused runtime tests, full `qjs-runtime`, Test262 slices, `compare-qjs.sh`,
  and `check.sh` pass;
- no focused external case is above `1.03x` candidate/base and the complete
  broad call family is not above `1.02x` candidate/base;
- if the state move itself regresses those bounds, fix it before R2.

## R2: Direct-Leaf Trampoline

Route only functions already accepted by `is_direct_leaf_function` through the
explicit frame stack. Numeric-leaf execution keeps its current priority.
Native, Proxy, bound, constructor/super, generator/async, direct-eval, `with`,
closure-producing, and every guard-false call retains the old path.

Each frame owns an independent operand `Vec<Value>` so the parent operand stack
and `TryFrame.stack_depth` remain frame-relative. Entering a child must release
any borrow of the parent opcode before swapping frames. A true child return
runs the child's own `finally` handling before restoring the parent; an
unhandled `RuntimeError` must be offered to each restored parent's existing
catch/finally machinery without rebuilding the thrown value.

Focused coverage includes:

- 10,000-level eligible recursion without Rust-stack growth;
- parent operand preservation such as `10 + leaf(3) * 2`;
- zero/one/two/three-argument order and direct-call slot behavior;
- multi-frame thrown-value identity and caller/callee `finally` ordering;
- repeated frame-buffer reuse without retained values;
- explicit fallback tests for eval, `with`, closures, generators, async, class
  construction, Proxy, and bound functions.

Promotion requires all of the following:

- JetStream `hash-map` and SunSpider `controlflow-recursive` each `<= 0.90x`
  candidate/base;
- broad `top_level_function_call <= 0.80x` candidate/base and complete call
  family `<= 0.90x` candidate/base;
- eligible-call profiles no longer show per-call `Vm` construction, fresh
  operand/local vectors, or a full recursive bytecode-evaluation entry;
- no external case exceeds `1.03x` candidate/base, no critical family exceeds
  `1.02x`, and the Test262 correctness floor remains unchanged.

If both external call cases fail to improve by at least 10%, stop extending
the fast-path predicate. Retain a non-regressing explicit-frame foundation and
move directly to compact dispatch in the same executor.

### 2026-07-21 R2 result: correct foundation, not promoted

R2 implemented the explicit single-VM frame scheduler on the existing
`is_direct_leaf_function` boundary, including ordinary calls plus named,
computed, and indexed direct getters. Focused coverage exercises 10,000-deep
ordinary recursion and getter chains, zero-through-three argument ordering,
parent operand preservation, thrown-value identity and `finally` ordering,
fallback call kinds, and bounded frame-buffer reuse. The final branch passed
1,433 `qjs-runtime` tests, Clippy, and the file-size guard before its complete
repository gate.

The first mailbox-based candidate was rejected immediately: against exact base
`a9a752ec9f589e15017745a1a0cd7306a8ee304e`, its three-case screen measured
`plain_function_call 0.99939x`, `top_level_function_call 1.06164x`, and
`dynamic_method_call 1.15123x` candidate/base. Code-generation inspection then
found two general costs rather than a semantic or eligibility problem:

- the ordinary opcode backedge repeatedly decoded `BytecodeOwner` and reloaded
  the VM pointer;
- zero/one-argument numeric hits built and moved the scheduler's 56-byte owned
  argument payload even though no child frame would be installed.

The retained B+C repair resolves `BytecodeOwner` once, isolates the dispatch
loop behind a stable bytecode pointer, probes numeric leaves through borrowed
argument slices, and constructs owned arguments plus the call mailbox only on
a miss. Its complete 27/27-eligible screen produced:

| Case/scope | Candidate / base |
| --- | ---: |
| `plain_function_call` | 1.00500x |
| `top_level_function_call` | 0.99456x |
| `dynamic_method_call` | 1.08721x |
| three-case geometric mean | 1.02810x |

Candidate binary SHA-256 was
`34624b735c8afbb13b9393f324a30ec9a2514a4e66ba9c6bf5286f6b59513b48`;
raw JSONL SHA-256 was
`8ac94a3ecb118a7843e862d283627144d3c4acf8a8b8cb60dddf70b93e512d3c`.
Because the dynamic case and aggregate still exceed the `1.03x`/`1.02x`
guardrails, the protocol stopped before external or full-portfolio measurement
and R2 was not integrated into `main`.

The retained experimental branch is
`agent/direct-leaf-trampoline/hotpath-r2` at
`2685051b4e70bd0ed5359661ece718c702464409`. It passed 11/11 focused
direct-leaf tests, 1,433/1,433 `qjs-runtime` tests, the touched gate including
116/116 selected Test262 cases, the complete repository gate including
5,141/5,141 Test262 subset cases, and all 205 QuickJS-NG comparison fixtures.
Its pushed CI run is `29898252281`; CI remains asynchronous evidence and did
not delay the dependent R3 branch.

A final attempt to merge the scheduler signal into the existing `Completion`
enum was rejected without another benchmark: release machine code kept the
same 98,508-byte inner loop, `0xeb0` stack frame, backedge pointer reloads, and
instruction-for-instruction `GetPropNamed`/`CallResolved` hot paths. This is
negative evidence, not an optimization result. The correctness-tested R2
branch remains an experimental base for R3; promotion now requires compact
dispatch to pay back its remaining ordinary-dispatch cost.

## R3: Compact Dispatch In The Same Executor

Introduce a deterministic, fixed-width register or superinstruction form
inside the existing executor. Lower only complete, prevalidated functions; an
unsupported function falls back before executing observable work. Expand
coverage in measured semantic families, with differential tests for `NaN`,
`-0`, Infinity, BigInt errors, short-circuiting, TDZ, exceptions, and calls.

Do not close T021 merely because a subset improves. R3 remains open until its
benefit generalizes across at least two independent external suites and the
next dominant shared runtime cost is outside dispatch/call-frame mechanics.

### 2026-07-22 R3b result: isolated dispatch works, total call cost still dominates

The first R3 slice is preserved on `agent/compact-dispatch/r3b-executor` as
prevalidated compact IR `0a11a5354971e62ec1ba62b03a24db575af93e6d` plus the
same-VM compact executor `ec79ca72f657e3f4dd8f6cbdb4b31b21eaf8bb0b`, based
on final R2 `2685051b4e70bd0ed5359661ece718c702464409`. Only
scheduler-created direct-leaf child frames select compact execution; roots,
standalone VMs, and incompletely lowered functions remain ordinary before any
observable instruction. The branch passed 31/31 focused compact/trampoline
tests, 1,453/1,453 runtime tests, 65/65 touched Test262 cases, and the complete
5,141/5,141 Test262 subset gate. CI run `29898665752` was queued
asynchronously.

An independent release-code review confirmed that the dispatch loops are
actually isolated. The ordinary loop is 89,364 bytes with a 3,904-byte total
stack frame, down from final R2's 98,508 bytes and 3,920 bytes. The compact loop
is a separate 5,380-byte symbol with a 992-byte total stack frame; an explicit
scheduler branch calls one loop or the other. Candidate binary SHA-256 is
`3d827fe05181836e562bbdbc2e3108277352474f99f2550263c9aad61f11486d`.

The mechanism did not yet pay back whole-call overhead. An eight-case call
screen against exact final R2 measured a `1.01525x` geometric mean, with
`dynamic_method_call 1.06613x`, `top_level_function_call 1.04304x`, and
`closure_allocation_call 1.03269x`; the other five cases ranged from
`0.99400x` to `0.99906x`. Raw JSONL SHA-256 is
`3a06852477256fd797eff4b6c2fe3906488c2414b50072d7d6655c13a521c486`.

The focused external gate confirmed that this is a structural ceiling, not a
codegen failure: JetStream `hash-map` was `0.98745x`, Kraken
`json-parse-financial` was `1.01211x`, and SunSpider
`controlflow-recursive` was only `0.97103x` candidate/R2. Their
candidate/QuickJS-NG ratios were `8.39617x`, `0.77808x`, and `7.39349x`.
External report/raw SHA-256 values are
`d626f82950e43386674b06ec6fac5bbe08c21611231aa58fc4c0975c5e6c1205` and
`d93b84c1c362e29900feafe3a4b61404dd41a2432e4efbf0d7185a22f1591864`.
R3b is therefore not promoted to `main`; the next slice must remove shared
frame construction/call staging cost or broaden dispatch across the measured
property/allocation operations, then repeat the same fail-fast gates.

### 2026-07-22 R3c result: recursion improves, ordinary calls regress

The follow-up `agent/compact-frame-setup/frame-r3c` experiment boxed active
frames, recycled bounded frame/buffer storage, reset direct-leaf children in
place, and read immutable loop plans from bytecode-owned `OnceCell`s instead
of cloning them into every frame. It passed 1,453 runtime tests and the complete
5,141-case Test262 subset, but remains local negative evidence and was not
pushed or integrated.

The clean external screen showed that the structural change was real:
SunSpider `controlflow-recursive` improved to `0.85103x` candidate/R3b,
JetStream `hash-map` to `0.93357x`, and Kraken `json-parse-financial` was
neutral at `0.99819x`. The same clean eight-case call screen nevertheless
measured a `1.02156x` geometric mean. Seven cases were between `0.99685x` and
`1.01760x`, but `top_level_function_call` regressed to `1.14024x`, failing both
the per-case and aggregate gates. The complete 507-sample run had no
eligibility failures; raw JSONL SHA-256 was
`45a1b0e4e478f91fb1e9ea4c28db5daf99980239283fa67da08dade3b05b44f9`.
R3c is therefore not a promotion candidate. Its recursive-frame result may
inform a later general executor rewrite, but the current reset/recycling layer
cannot be stacked onto rejected R2/R3b merely to improve one external case.

## Final Acceptance

- T018's strict contract is met: every broad and pinned external case is
  runnable and `<= 0.50x` QuickJS-NG, with every suite/family aggregate also
  `<= 0.50x`.
- Two independent complete benchmark runs confirm the result.
- The current configured Test262 inventory remains zero-failure, and focused
  tests cover every changed semantic boundary.
- `./scripts/check.sh` and `./scripts/compare-qjs.sh` pass.

## Verification

```sh
cargo test -p qjs-runtime
./scripts/test262-subset.sh
./scripts/compare-qjs.sh
./scripts/check.sh
```

Fast broad call-path diagnostics are deliberately not formal portfolio claims:

```sh
QJS_FRAME_GATE_DIR="$(mktemp -d /tmp/qjs-frame-gate.XXXXXX)"
./scripts/benchmark.sh \
  --candidate target/release/qjs \
  --base /tmp/qjs-frame-gate-base \
  --quickjs-ng third_party/quickjs-ng/build/qjs \
  --filter call --blocks 5 --seed 20250713 \
  --output "$QJS_FRAME_GATE_DIR/call.jsonl"
```

Snapshot `/tmp/qjs-frame-gate-base` from the exact base commit before building
the candidate. A focused raw file is valid smoke evidence but cannot be passed
to `benchmark-report.sh`: the formal validator intentionally requires all 25
manifest cases, complete coverage, and verified three-role receipts.

For a low-latency independent external gate, derive a temporary manifest that
keeps JetStream `hash-map`, Kraken `json-parse-financial`, and SunSpider
`controlflow-recursive`, then run the existing hash-verified adapter. All three
suites remain present because the preview manifest validator deliberately
rejects partial suite identities:

```sh
QJS_FRAME_EXT_DIR="$(mktemp -d /tmp/qjs-frame-ext.XXXXXX)"
jq '
  .suites |= map(
    if .id == "jetstream3-js-subset" then
      .cases |= map(select(.id == "hash-map"))
    elif .id == "kraken-1.1" then
      .cases |= map(select(.id == "json-parse-financial"))
    elif .id == "sunspider-1.0" then
      .cases |= map(select(.id == "controlflow-recursive"))
    else . end
  )
' benchmarks/external-preview.json > "$QJS_FRAME_EXT_DIR/manifest.json"
./scripts/external-performance-preview.sh \
  --manifest "$QJS_FRAME_EXT_DIR/manifest.json" run \
  --cache-root target/benchmarks/external-cache \
  --work-root "$QJS_FRAME_EXT_DIR/work" \
  --output-dir "$QJS_FRAME_EXT_DIR/result" \
  --candidate target/release/qjs \
  --base /tmp/qjs-frame-gate-base \
  --quickjs-ng third_party/quickjs-ng/build/qjs \
  --blocks 3 --timeout-seconds 15
jq '.suites[] | {id, cases: [.cases[] | {
  id, candidate_over_base, candidate_over_quickjs_ng, capability
}]}' "$QJS_FRAME_EXT_DIR/result/external-report.json"
```

Only complete, healthy same-host portfolio runs may update the formal T018
score. Local gates decide whether to push; completed hosted artifacts confirm
the unit asynchronously and never block work on the next isolated slice.

## Notes

The first correctness prerequisite is stable Realm intrinsic prototype
identity. Object and array literals must not follow a later reassignment of the
global `Object` or `Array` binding; land that regression fix before extracting
VM frames so the structural rewrite starts from correct semantics.

### 2026-07-24 dispatch-cost measurement and the codegen-cliff constraint

An instruction-level profile of the release binary (`sample` plus `objdump`
against the sampled offsets) attributes the ordinary dispatch loop's self time
to two instructions:

- about 46% to the jump-table indirect branch that selects an opcode handler,
- about 14% to the reload of `self.ip`, which is written to memory at the end
  of one instruction and read back at the start of the next.

Three negative results follow from that shape and should not be repeated:

- `Op` is 96 bytes, but padding it to 168 bytes changed nothing measurable.
  Instruction-stream size is not a bottleneck; do not spend time shrinking it.
- The loop sits on a codegen cliff. Adding a single `unreachable!()` match arm
  cost about 17%. `lto = "fat"`, `codegen-units = 1`, and outlining the two
  largest cold arms (`NewFunction`, `TypeofGlobal`) each made it slower.
- One new superinstruction is a net loss. `BinaryLocals`, fusing
  `LoadLocal; LoadLocal; Binary` — the most frequent triple in real code at
  8-10% of executed opcode pairs — measured a 28-case local corpus 2.2%
  slower, uniformly. Fusion has to arrive as a batch large enough to pay for
  the extra dispatch arm, or after the dispatch itself is restructured.

Removing work in the *compiler* does pay, and fusions added inside
`virtual_object::lower` do not disturb the loop-plan matchers, because plans
compile from `bytecode.code` while the VM runs the lowered stream and lowering
preserves instruction offsets. That is the cheap place to experiment.

The loop-plan matchers are worth keeping. Reducing `jump_with_loop_plans` to a
plain `self.ip = target` measured the same corpus 9.7% slower, with
`bitops-nsieve-bits` 5.3x slower and `access-fannkuch` 1.3x slower. They are a
loop-specialization tier, not benchmark fitting. Their cost is brittleness:
every codegen change must also make them shape-tolerant, which is roughly an
hour of work across the five matcher sites when done with running cursors and
optional prologue/suffix helpers rather than fixed offsets.

### 2026-07-24 addendum: opcode-count levers and how to measure them

Two further negative results, both from interleaved A/B runs (both binaries
built first, then alternated per case, min of five to seven rounds):

- Adding any variant to `Op` costs about 2-3%, including a variant that is
  never constructed or executed. The discriminant is niche-encoded inside the
  first field, so the variant count perturbs the encoding every fetch pays for.
  `#[repr(u8)]` recovers most of that but is not itself a measurable win. A
  superinstruction needing a new opcode therefore starts about 2% behind;
  `BinaryLocals`, fusing the most frequent triple in real code, never caught up.
- Moving 54 cold opcodes out of the dispatch `match` behind one
  `#[inline(never)]` fallback measured neutral. Arm count is not the lever.

One real gap was found and closed instead. `virtual_object::lower` returned the
original instruction stream whenever scalar replacement found no candidate,
which also skipped every shared-dispatch superinstruction, so an ordinary
function never received one. The compare-and-branch fusion now runs for every
analyzable function.

Local single-shot timings drift 3-5% between builds on the development host,
which is larger than most candidate effects and produced two contradictory
verdicts on one change. Interleaved A/B is the minimum bar for a local
decision; the hosted preview protocol remains the authority.

### 2026-07-25 the next structural target: per-call environment materialization

Profiling the worst external ratio (`date-format-tofte`, a builtin- and
string-heavy workload) attributes about 40% of its time to `malloc`/`free`,
about 17% to `Vm::frame_call_env` plus `CallEnv::apply_env` plus
`FrameBindings::insert`, and about 12% to `memcmp`.

The cause is `Vm::call_env`. Calling a **native builtin** takes the
`self.current_env()` branch, which materializes every visible frame local into
a name-keyed compatibility frame -- one `String` allocation per local, appended
to a `Vec<(String, FrameBindingValue)>` that `insert` scans linearly with
`memcmp` -- and then writes the whole thing back through `apply_env`. Every
`Math.floor`, `charAt`, `push`, or `getTime` pays it.

Two attempts to remove it were measured and rejected:

- Routing every native call through `realm_env()` breaks direct `eval`, which
  does read the caller's scope through this path (27 tests).
- Gating that on the frame's compile-time `contains_direct_eval` /
  `contains_with` / deopt-bindings properties passes all 1,802 tests, the
  5,159-case Test262 subset, and all 218 QuickJS-NG fixtures, but measured
  +1.9% overall: `string-base64` +41%, `crypto-md5` +18%, against
  `crypto-aes` -18%. Something on the builtin path depends on the snapshot
  beyond name resolution; find it before retrying.

The promising direction is instead to make the snapshot itself cheap rather
than to skip it: `Local::name` is a `String` cloned per local per
materialization, so making binding names `Rc<str>` end to end turns the
dominant allocation into a refcount bump. That touches `Local`, `FrameBindings`,
and about 63 `insert`/`insert_frame_cell` call sites, and it is the highest
expected-value unit currently identified.

### 2026-07-25 `Rc<str>` binding names: migrated, measured, rejected

The `Rc<str>` binding-name unit named above was implemented end to end:
`Local::name`, `Bytecode::local_slots`, `FrameBindings`, and the `CallEnv`
`insert` / `insert_frame` / `insert_frame_cell` signatures, with the roughly
one hundred call sites converted from compiler diagnostics. All 1,802
`qjs-runtime` tests passed.

It measured **15% slower** on interleaved A/B: `crypto-aes` +73%,
`date-format-tofte` +75%, `3d-cube` +36%, and every remaining case between
+1% and +21%. Nothing got faster.

Two things went wrong, and a retry must address both:

- The mechanical conversion inserted about forty `Rc::from(<literal>)` calls,
  many of them in `function/call.rs` on the ordinary call path. `Rc::from` of
  a `&str` allocates and copies exactly like `String::from`, so those sites
  kept their allocation and added an indirection. Interned statics, not
  per-call construction, are what a shared name needs.
- The premise itself is unconfirmed. Skipping the frame snapshot entirely for
  native calls also regressed (+1.9%, `string-base64` +41%), so the model that
  says "the snapshot's `String` clones dominate" does not predict either
  result. Establish *why* the builtin path depends on the snapshot, with a
  targeted experiment, before rebuilding around it.

### 2026-07-25 where the remaining external gap actually lives

Measured against QuickJS-NG on four call shapes, to locate the 3.8x:

| shape | qjs-rust | QuickJS-NG | ratio |
| --- | ---: | ---: | ---: |
| leaf numeric call in a counted loop | 0.05s | 0.89s | **0.06x** |
| function returning an object literal | 1.74s | 0.89s | 1.96x |
| recursion (`fib`) | 0.76s | 0.08s | 9.4x |
| method call returning `this.v` | 4.40s | 0.45s | 9.8x |

The gap is not uniform interpreter overhead. Shapes the numeric-leaf and
loop-plan fast paths accept are already far ahead of the reference; the ~10x
sits entirely in calls those paths decline — method calls and recursion —
which is exactly the profile of the worst external cases (`access-nbody`,
`raytrace-public-class-fields`, `controlflow-recursive`, `date-format-tofte`).

A `sample` profile of the method-call case attributes about 23% to call setup
and teardown (`call_direct_leaf_function`, `eval_function_bytecode`,
`Vm::new_with_globals_*`, dropping `CallEnv`), about 13% to the allocator, and
25% to the dispatch loop. Note that on this platform `free` dominates the
allocator cost and itself calls `mach_absolute_time`.

Pooling the frame's slot vector on the bytecode, mirroring the existing
operand-stack pool, was implemented and measured **0.8% slower** (method call
+3.4%, recursion +4.5%): the `Rc<RefCell<Option<Vec<_>>>>` handshake costs
more than the platform allocator's thread-cached small allocation it replaces.
`local_upvalues` already avoids its allocation for direct leaf calls.

So the call-setup cost is not one allocation to remove; it is the total work of
building `Vm`/`FrameState`/`CallEnv` per call. That is the structure the
original T021 R1/R2 aimed at, and it remains the only identified route to the
10x shapes.

### 2026-07-25 declining loop plans were the dominant cost of calls in loops

Profiling the 9.8x method-call shape found that its largest non-dispatch cost
was not the call at all. A backward edge whose instruction range matches a
compiled numeric loop plan ran that plan's full preparation every iteration --
write targets for counter, accumulator, and result slots, the forbidden-cell
and forbidden-realm-write sets derived from them, and a prepared copy of every
term, plus a deep clone of the plan itself -- and a loop containing a call
fails that preparation every time, because a call is not an admissible term.

Two commits removed it:

- `04bbb420` shares the plan's term vector (`Rc<[NumericLoopTerm]>`) so the
  per-backedge plan copy stops allocating. Running the plan through a borrow
  of the bytecode instead was tried and measured 14% *slower* on a
  read-modify-write property loop: the executor then reloads the plan's fields
  across the `&mut Vm` writes in the loop it drives.
- `432a707c` retires a plan in a frame after it declines three times. The
  retry budget covers a plan that only becomes admissible once the loop's
  values settle; a fresh invocation starts fresh. Plans are accelerators, so
  retirement is unobservable.

A method call in a counted loop went from 9.8x to **6.4x** against the
reference (4.40s to 2.80s, ng 0.44s). Extending retirement to the mutation and
control loop plans was tried and reverted: net neutral, and
`math-spectral-norm` regressed 4.5% reproducibly because its mutation plan
becomes admissible only after more than three iterations.

After this, the same shape's profile is roughly 33% `Vm`/`CallEnv`
construction and teardown and the rest dispatch, with the allocator no longer
in the top twelve symbols. That is T021 R1/R2 territory again, now without the
plan-preparation noise on top of it.

### 2026-07-25 frame size is the next measurable per-call cost

With the plan-preparation overhead removed, a recursion profile attributes
about 15% to `Vm::new_with_globals_*`, about 13% to the allocator, and about
4% to `_platform_memmove` charged directly to `eval_function_bytecode`. That
last one is the frame itself: `FrameState` is **704 bytes**, of which
`CallEnv` is 192, and a call constructs it, moves it into `Vm`, moves it back
out through `into_frame`, and then moves four fields into
`FunctionBytecodeResult`.

Marking the constructor `#[inline]` so the frame is built in place was tried
and measured 0.9% *slower* — it bloats the caller more than it saves. Caching
the direct-call authoritative-slot mask on the bytecode (for a direct leaf
call `CallEnv::slot_is_authoritative` provably does not depend on the name, so
the mask is a bytecode constant) was also tried and measured 0.3% slower: the
guard conditions cost as much as the loop over a handful of locals.

What is left is to make the frame smaller rather than to move it differently.
The concrete unit: `CallEnv` inherits seven `Rc` fields unchanged from its
parent on an ordinary call (`global_lexical_bindings`, `global_lexical_values`,
`immutable_lexical_bindings`, `catch_bindings`, `direct_eval_var_conflicts`,
`module_host`, and for non-module code `module_imports`). Grouping them behind
one `Rc<InheritedEnv>` would cut `CallEnv` by roughly a third and replace seven
reference-count pairs per call with one. `module_imports` is a per-frame
parameter and needs care.

### 2026-07-25 grouping `CallEnv`'s inherited handles: measured, rejected

The unit named above was implemented. Two shapes were tried:

- Grouping all six inherited handles is not possible without a per-call
  allocation: a direct leaf frame deliberately *resets* `catch_bindings` and
  `direct_eval_var_conflicts` to the realm's shared empty set, so putting them
  in the shared group forces the group to be rebuilt for every call. The
  existing test `direct_leaf_frames_share_empty_metadata_until_mutation`
  catches exactly this.
- Grouping only the four handles every frame does inherit unchanged
  (`global_lexical_bindings`, `global_lexical_values`,
  `immutable_lexical_bindings`, `module_host`) compiles, passes all 1,804
  tests, and shrinks `CallEnv` from 192 to 168 bytes and `FrameState` from 704
  to 688. It measured **2.8% slower**: `access-binary-trees` +6.6%, recursion
  +7.4%, `t3` +10.8%. Twenty-four bytes off a frame copy does not pay for an
  extra indirection on every read of those handles.

Frame size is therefore not the lever either. Every attempt to reduce the
per-call cost by rearranging the frame -- pooling its slot storage, inlining
its constructor, caching its authority mask, shrinking its environment -- has
now measured neutral or worse. What remains is the count of operations a call
performs, not how they are laid out, which is the R1/R2 trampoline design.

### 2026-07-25 direct-call eligibility audit: five real defects, one non-defect

Profiling the shapes the external suites are built from led to the richest
vein of the campaign: functions the slot-seeded direct call path declined for
a reason that did not apply to them. Each was a defect rather than a tuning
knob, and each is measured against QuickJS-NG on an isolated shape.

| Excluded by | Why it did not apply | Commit | Shape result |
| --- | --- | --- | ---: |
| `immutable_env_binding` | every class member carried the class inner name, used or not | `7cb3e6e4` | -46.6% (13.8x -> 7.7x) |
| `has_name_binding` | every named function expression carried its own name, used or not | `72fc4947` | -44.5% (12.0x -> 6.7x) |
| named `new.target` binding | it needed no name at all | `b5e2e06d` | -10.7% |
| `creates_closures` | only a closure that *captures* needs frame cells | `1521cba4` | -41.8% (2.33x -> 1.36x) |
| `params.is_simple()` | a default initializer fills the parameter's own slot | `21b97f07` | -50.9% (12.8x -> 5.9x) |

The reference test in every case is "does the body actually use the name", and
**slot existence is not that test** -- the compiler declares the slot
regardless, so `Bytecode::uses_local_binding` scans the instructions. Each fix
keeps the exclusion whenever the body could reach the name by a route the
compiler does not record: a capturing closure, a direct `eval`, or a `with`.

`needs_arguments_object` was audited and is **not** actionable: reading
`arguments` costs 3.9x here against 3.8x in the reference, so the penalty is
proportionate and the ratio is inherited from the base method shape.

A prototype-chain read cache was also built and rejected. Method lookup on a
prototype was genuinely uncached -- `try_cached_get_string` cleared the cache
and re-walked the chain on every access -- but caching the immediate
prototype's slot measured 0.7% slower overall: the added entry kind costs the
cache scan more than the one-level walk it replaces.

### 2026-07-25 correction: the frame copy was removable after all

Four attempts to reduce the per-call frame cost by changing how the frame is
moved -- pooling its slot storage, inlining its constructor, caching its
authority mask, grouping its environment handles -- all measured neutral or
worse, and the earlier entries here concluded that frame layout was not the
lever.

That conclusion was wrong about the cost, right about those mechanisms.
`eval_function_bytecode` bound the whole `FrameState` as a local before moving
four fields into the result, and a 704-byte binding materializes in the
caller's own stack frame. Destructuring those four fields directly out of the
VM (`b390cf40`) never creates the temporary at all: 1.3% geometric mean over
21 cases, with a plain method call 5.5%, a class method 6.0%, recursion 5.0%,
and no case outside the noise band.

The transferable lesson: a measured failure refutes a *mechanism*, not the
*cost* it was aimed at. The remaining per-call frame costs -- `Vm::new_*`
returning by value, `CallEnv`'s seven reference-count pairs -- should be
re-attacked the same way, by removing the materialization rather than by
rearranging it.

### 2026-07-25 measurement hygiene: always rebuild the base

Marking `vm::eval_function_bytecode` `#[inline]`, so the caller's `CallEnv`
flows into the frame instead of crossing two non-inlined boundaries, looked
like a 5-8% win against remembered numbers. Against a base rebuilt from the
current `main` it measured **1.1% slower** and was reverted: the remembered
numbers predated the previous commit, which had already taken that gain.

Rebuild the base binary from `HEAD` for every A/B. Comparing against numbers
from an earlier build attributes the previous commit's gain to the current
experiment.

### 2026-07-26 direct-call result-only exit

The slot-seeded direct-call path has already proved that it cannot write
compatibility bindings back to its caller. It nevertheless returned a full
`FunctionBytecodeResult`, moving the completed frame's surviving fields only
for each direct caller to immediately take its `value`. A dedicated
`eval_direct_call_bytecode` entry now keeps the completed VM local, drops its
frame in place, and returns only `Result<Value, RuntimeError>`. General calls
retain the existing result object because they still need its environment and
slot state. This neither widens direct-call eligibility nor changes opcode
execution.

Focused coverage includes the existing zero-through-three-argument and
receiver direct-call cases plus a new thrown-object identity test, proving the
result-only exit still reaches the caller's catch with the original value.

The complete five-block local broad diagnostic compared final candidate release
SHA-256 `80e02eb574965bf0a586b5a609fdc1fffbd822ad997a7d392697285fdf12c8a5`
against exact base SHA-256
`5a8ade05243e0e6450b8691018d9cc541503b628742012fb5fe6401e5101d94e`
and pinned QuickJS-NG SHA-256
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
All **375/375** formal rows were eligible and all **600/600** linearity rows
were `ok`. Candidate/base was **0.99577x** geometrically; the largest movement
was `local_read` at 1.00732x. `dynamic_method_call` improved to **0.95785x**
of base and `function_call_two_args` to **0.90678x**, while the complete
portfolio reached 0.09303x of QuickJS-NG. The raw JSONL SHA-256 is
`bf7b0f0ac02873b4219d8f8d08748ee879d062e7d5916959a57f1c8604c0dedc`.

A separate three-block external screen retained one independent case from each
suite: JetStream `hash-map` measured 0.99968x of base, Kraken `ai-astar`
0.98993x, and SunSpider `controlflow-recursive` 0.99949x. Its report and raw
SHA-256 values are
`e0271f8850411efb50cf77829201a71a5a6e0d5e4160f7f079075f31fdacf0c6`
and `1a5ac748635523772f82be3ccee839d6f838b62e29b1d1a3e23d2c88a6bf7b44`.
These focused external rows are non-claim diagnostics; they establish no
material regression but do not close T018's full external acceptance gate or
its 2x-versus-QuickJS-NG objective.

A second independent three-block screen substituted SunSpider
`date-format-tofte`, the native-call-heavy holdout, and measured it at 0.99063x
of base; the same run measured `hash-map` at 1.00302x and `ai-astar` at
0.99748x. Its report and raw SHA-256 values are
`98af5e1ddd0e3fb0fa334b6b062de12994d6b531341406589f7460fe8c79135d`
and `53f1f930b12d075b9d0ad82a382ac7f57460352eabac9877e4621482a69aca2f`.

### 2026-07-26 rejected realm execution-view construction

`Vm::realm_env()` normally builds an empty ordinary function frame for every
native call, coercion hook, and property operation. A candidate replaced that
with the already-minimal direct-leaf frame shape while preserving the caller's
module-import routing. It removed inherited catch/eval conflict maps,
private-name state, and the direct-eval with-stack from this realm-only view;
a focused environment-contract test and compilation both passed.

The mechanism was nevertheless rejected and reverted. A three-block,
six-case diagnostic using candidate SHA-256
`0adb7cd1eb998ada432da1f4a79b04973d20d7f656cbcaf92a260b3ddb11b6db`
against exact base `80e02eb574965bf0a586b5a609fdc1fffbd822ad997a7d392697285fdf12c8a5`
gave no material benefit: `math_abs` 1.00016x, `array_index_of` 1.00066x,
`string_slice` 1.00025x, `property_read` 1.00127x,
`dynamic_method_call` 1.00092x, and `function_call_two_args` 0.99818x of
base. All 54 formal and 144 linearity records were `ok`; raw JSONL SHA-256 is
`9efe510bda7ada2cde573b562c325cbf469da5a36903cdcfa0542e1ee1c66cb5`.
The savings in empty-metadata reference-count traffic do not repay the new
construction route, so this is not a general-performance improvement.

### 2026-07-26 rejected function private-environment demand cache

Every ordinary user-function call currently asks its function and then its
home object for a private environment, even when the body cannot access a
private name. A candidate classified bytecode once per function and skipped
that cold auxiliary-state lookup unless the body has a private operation,
direct `eval`, or a nested function/class that could capture lexical private
state. General frames still explicitly cleared inherited private state. The
classifier unit test and all 42 private-name runtime tests passed.

The mechanism was nevertheless neutral and was reverted. Candidate release
SHA-256 `22608bde72aa6938b3ef9b1fe17dd5b3b2efe90b2162f0e2b1d470861b50e6b8`
was compared against an exact rebuild of pushed `2a4c9670`, SHA-256
`80e02eb574965bf0a586b5a609fdc1fffbd822ad997a7d392697285fdf12c8a5`, and
pinned QuickJS-NG SHA-256
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
Six independent three-block diagnostics measured candidate/base at
`function_call_two_args` 0.99845x, `dynamic_method_call` 1.00022x,
`property_read` 1.00121x, `array_index_of` 0.99999x, `string_slice` 0.99909x,
and `math_abs` 1.00169x: a 1.00011x geometric mean. All 54 formal and 144
linearity records were `ok`; the segmented raw JSONL SHA-256 values were
`0765f604cfc860e9533e18dbd15c807c244c28b400291473efd2138a4a4378cc`,
`a1ba643ed56aad65c270bdb9e10ef1d65d3e8ad275cf273007d153bd773d3f3d`,
`c2f53eeb1ab06be4f9343248612bb2978cf8d06dbf16299466cc79341bdf54be`,
`3b18e4f651fba6b5ed780b01eb5174f89926b95c1887adec80370f13f408358c`,
`7d113dffb3d59b0a50a466026386aa9d263da78fe142ea230f9044d17a4cad71`, and
`e86955b84d0813bdd10de8a16dce68f3856a132354dcf700b38bc9f7f922cee3`.
Avoiding an empty cold lookup does not pay for an extra per-function cache
probe, so this does not advance the general-performance campaign.

### 2026-07-26 rejected truncated direct captured-upvalue storage

Direct leaf calls with received lexical upvalues rebuilt a
`Vec<Option<Upvalue>>` through every local slot, including an all-`None` tail
after the last capture. A candidate separated sloppy-global routes from the
already indexed received-upvalue slots and, when no module import or sloppy
fallback was present, constructed only the prefix through the final received
slot. Reads past that prefix already use bounds-checked `get` and mean no cell;
direct-leaf eligibility excludes direct eval, `with`, and capturing closures,
which are the routes that could later install a cell in the omitted tail.

The focused storage-shape and direct-leaf semantic tests passed. On an isolated
captured-property-call shape with six later ordinary locals, it removed roughly
3% of retired instructions (about 26.87B versus 27.70B across interleaved
3M-call runs), so it received a complete broad check rather than being
discarded on the microbenchmark alone.

The candidate was rejected and reverted after that check. Candidate release
SHA-256 `f97d3c3c2afee9c3ca1e6d04caa05838f1a5272b116386386937386dc8039c7a`
was compared with exact base SHA-256
`80e02eb574965bf0a586b5a609fdc1fffbd822ad997a7d392697285fdf12c8a5` and
pinned QuickJS-NG SHA-256
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
The complete five-block raw diagnostic has SHA-256
`8579e0197eb0d629af06eb2dd9de3a02f3ad734e703a30b386c2c7cb29e1b153`:
all **375/375** formal records were eligible and `ok`, all **600/600**
linearity records were `ok`, and `run_end=complete`. Its provenance is local
and unverified because these dirty-worktree binaries have no clean build
receipts, so `benchmark-report.sh` correctly refused to turn it into a formal
report; the raw result remains a non-claim diagnostic. Median-per-case
geometric candidate/base was **1.000014x**, with
`closure_allocation_call` regressing to **1.01483x**. Candidate/QuickJS-NG was
0.09277x, unchanged in campaign terms.

An independent three-block external screen gave the tempting but insufficient
local signal: JetStream `hash-map` 0.98558x of base, Kraken `ai-astar`
1.01203x, and SunSpider `controlflow-recursive` 0.98009x. Its report/raw
SHA-256 values are
`f2660dac6e0790ecdb417a530ca9d7102d28cec3bf44d8eec6962b03b59d11d2` and
`319b035dd6d95c6dcd01b1dd850db0501722a13447fe7d93f34b23726d01fa61`.
The broad neutral result and closure-allocation regression show that shortening
this transient vector is not a general runtime improvement; do not retry it
without a design that removes work across captured and closure-creation calls.

### 2026-07-26 bounded direct-eval compilation blueprints

The updated `date-format-tofte` profile showed a different repeatable cost:
the formatter dynamically evaluates small expression-only strings such as
`Y()` on every format character. Each direct eval previously parsed and
compiled that same string again, then rebuilt the caller's complete visible
name set even when the eval had no declarations to instantiate.

`RealmState` now owns a FIFO-bounded cache (32 entries, sources at most 4 KiB)
keyed by the direct-eval source and the full `EvalParseContext`. An entry holds
immutable bytecode behind `Rc`; every invocation still creates a distinct VM
frame and runtime values. The cache rejects hoisted and lexical declarations,
nested function/class construction, and tagged-template objects, which are the
bytecode features with per-evaluation declaration or identity state. A second
fast path skips the caller-wide visible-name snapshot only for those reusable
blueprints that also contain neither nested direct eval nor `with`; a write is
instead checked against the original caller environment by its exact name.
All other evals retain the prior declaration, dynamic-scope, and writeback
path.

Focused runtime coverage verifies cache reuse across changed caller values,
fresh object identity for separate eval invocations, declaration-bearing
source bypass, and repeated local writeback. The complete 1,885-test runtime
suite, the 5,159-case Test262 subset, and all QuickJS-NG comparison fixtures
passed after the final fast path.

The fixed candidate release SHA-256
`89992e1bffd4089d908a88b63a17352849548f0c1cef23189407552fcf09d7f6`
was screened against exact base
`80e02eb574965bf0a586b5a609fdc1fffbd822ad997a7d392697285fdf12c8a5` and
pinned QuickJS-NG
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
The five-block external screen measured `date-format-tofte` at **0.83328x**
of base (220.62 ms versus 264.76 ms), while `cdjs` was 0.99550x,
`hash-map` 0.98753x, `audio-oscillator` 0.99764x, and
`controlflow-recursive` 1.00077x. Its report/raw SHA-256 values are
`13bc0cf375b3dcf90fcfae4e49c4498de079bb8d2bf3d0c54fa4b3974a60598e` and
`9cc90c2286e0251fa4a46a4ce9e840fac4bd7fd1683e74b580fa62b05b0c68cc`.

The complete three-block local broad-v2 diagnostic produced all 225 formal
records and 600 linearity records with `ok` status and `run_end=complete`.
Candidate/base geometric mean was 1.00068x; the largest movement was
`closure_allocation_call` at 1.01155x. Candidate/QuickJS-NG geometric mean
was 0.09283x. Raw SHA-256:
`2c52a42e1f4cd7631a2ea8666bc74cbb71fcb5762a28df70a9153b238d701cd2`.
These are local receipt-free diagnostics, not a claim that T018's external
2x objective is closed: even the improved date row remains 4.83x QuickJS-NG.

### 2026-07-26 paired named compound-store cache

The compiler now gives a named compound assignment or update expression one
shared `NamedPropertyCache` for its `GetPropNamed` and paired `SetPropNamed`.
Plain named assignment intentionally carries no store cache. On a validated
ordinary own-data-property hit, an exact read entry promotes to an
object/layout-checked slot; polymorphic constructor-built receivers can use a
slot only after the exact interned property name is rechecked. Each write still
checks the live descriptor's writability. Accessors, read-only properties,
prototype writes, Proxies, globals, symbols, and exotic storage all miss and
retain the existing `[[Set]]` route.

Focused coverage changes an already cached receiver into an accessor and
proves the getter/setter remain observable. The compiler test proves only the
four compound/update stores in its fixture receive a paired cache. The full
1,886-test runtime suite, 5,159-case Test262 subset, and all QuickJS-NG
comparison fixtures passed.

The candidate release SHA-256
`90462460b159a52cdf9fa9395233d62fc76f6cd2b41aacd22c413fd60ba4dd2a`
was measured against exact base
`80e02eb574965bf0a586b5a609fdc1fffbd822ad997a7d392697285fdf12c8a5`
and pinned QuickJS-NG
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
The complete three-block local broad diagnostic recorded all **225/225**
eligible formal rows, **600/600** `ok` linearity rows, and 75/75 aggregated
linearity checks within the 0.85--1.15 bound. Candidate/base was **0.999406x**
(95% bootstrap CI 0.998813--1.001298), so the broad portfolio is neutral;
candidate/QuickJS-NG was 0.092572x. Raw JSONL SHA-256:
`219e0356f5a2b4b77d68fdd694069c7c2e74a19d7c5e9515433f6bfd22d5568e`.
This receipt-free local run is diagnostic only, not a formal comparison claim.

A full three-block pinned external preview made 44/45 candidate/base rows
comparable. Its diagnostic geometric candidate/base ratios were 1.003x across
JetStream's five rows, 0.998x across 13 comparable Kraken rows, and 0.993x
across all 26 SunSpider rows. Most importantly, the independent SunSpider
`access-nbody` workload, whose loop repeatedly performs named compound field
writes, measured **0.945x** of base (73.023 ms versus 77.281 ms), matching the
repeated local result. No suite score or 2x claim follows from the preview;
the external report/raw SHA-256 values are
`3dc1e36c76eaf203de1b47e943291f25666e867eb46e579e0481b454649f464e` and
`2fc152942572ddd9a56c7b7f1e5f311b035623479e5ae8f4eb0717c280a62877`.

### 2026-07-26 direct `this` own-data leaf methods

The remaining method-call gap includes a genuinely general leaf shape that the
numeric plans intentionally decline: `function () { return this.value; }`.
Such a body previously constructed a child VM even when its receiver already
had an ordinary own data property. Bytecode construction now recognizes only
the exact `FunctionPrologueEnd`, `LoadGlobal("this")`, `GetPropNamed`,
`Return` prefix and retains a clone of that instruction's existing
`NamedPropertyCache`. A direct-leaf invocation skips the VM only for an object
receiver whose live own property is ordinary data; cache hits still validate
the receiver/layout revisions. Accessors, inherited properties, Proxies,
exotics, and primitive receivers all decline to the unchanged VM path.

The plan is classified while immutable bytecode is built, rather than lazily
at a call site. This matters for ordinary sloppy calls: their implicit global
`this` is an object even when their body is not a property reader. An initial
lazy-probe experiment made `captured_write` 1.02347x of base in a complete
three-block screen; after construction-time classification, an independent
five-block `captured_write` screen was **1.000357x** of base (raw SHA-256
`6d3ea0701f91ce05e58e99a4986e4d560d71e69a5d310049411c0d6d0d9cdf38`).

Focused coverage proves ordinary values refresh, an accessor installed after a
cache hit still runs once, inherited accessors and Proxies preserve their
observable `[[Get]]` path, and primitive `this` retains ordinary boxing. The
initial plan unit test distinguished the plain read from `return this.value +
1`; the later numeric-property plan below admits that arithmetic form only
under additional live Number guards. The complete 1,888-test runtime suite,
5,159-case Test262 subset, and all QuickJS-NG comparison fixtures passed.

The fixed candidate release SHA-256
`77e3ae4663ee65145aa0909fbed2ae5cbf522fc53f4e6bf8b83e64d4289bdfa8` was
compared with exact parent base
`90462460b159a52cdf9fa9395233d62fc76f6cd2b41aacd22c413fd60ba4dd2a` and
pinned QuickJS-NG
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
As a shape diagnostic only, three 10-million-call runs of the direct method
fell from 3.62--3.63 seconds of user time to 2.26--2.27 seconds, about
**1.60x**. The complete three-block broad-v2 diagnostic recorded all
**225/225** formal samples eligible and `ok`, all **600/600** linearity rows
`ok`, and all 300 paired linearity checks within the 0.85--1.15 bound.
Candidate/base was **0.999752x** geometrically (call family 1.001461x; worst
case `array_dynamic_read` 1.007617x), and candidate/QuickJS-NG was 0.092767x.
Raw SHA-256: `0dd7ea935903a88e2784aeac4e2899437d41a1e0a4ed9dbfcf0f8282f09b2774`.
The no-receipt local raw file remains diagnostic only; strict reporting
correctly refuses a formal claim for provenance rather than a missing physical
measurement.

The matching full three-block external preview had 5/5 comparable JetStream
rows at 0.998x candidate/base, 13/14 comparable Kraken rows at 1.001x, and
26/26 SunSpider rows at 1.001x. `imaging-gaussian-blur` timed out on both base
and candidate while QuickJS-NG completed, so it is explicitly not compared.
The largest comparable candidate/base ratio was 1.022713x; `hash-map` was
0.993442x, `controlflow-recursive` 0.992378x, and `access-nbody` 0.989131x.
This is a safe general improvement, not closure of the 2x campaign: the
targeted `hash-map` case remains 5.316x QuickJS-NG. The receipt-free external
report/raw SHA-256 values are
`ee4fd1f560421ce242f8575de57f6d3955ae99a3bc9432a3bafbb3fd49dc7fe3` and
`1fb35ff5689f8adc3b63323cb536ece5b3ce6315e8bbad5dbc9b0ffb24c04e7e`.

### 2026-07-26 direct numeric own-data property leaf expressions

Real vector and geometry methods often do more than return one field, for
example `this.x * other.x + this.y * other.y + this.z * other.z`. The existing
numeric leaf plan deliberately rejects `GetPropNamed`, so every such direct
method still built a child VM even when all operands were ordinary numeric data
fields. This slice extends the already eager bytecode-owned property-leaf
classification, not direct-call eligibility or the VM's semantics.

The new numeric variant accepts only a straight-line prefix beginning at
`FunctionPrologueEnd`: `this` field reads, static field reads from simple
parameter local slots, number literals, numeric-result binary operations, and
one return. It reuses each original `NamedPropertyCache`. At execution, every
receiver/argument must be an object with a live ordinary own **Number** data
property. Accessors, inherited values, Proxies/exotics, primitives, BigInts,
strings, missing fields, unsupported operations, stores, calls, and control
flow all decline before user-observable work and use the unchanged direct-leaf
VM. This makes a partial guarded read safe: the only reads before fallback are
ordinary own data reads, which cannot invoke user code.

Focused tests cover plan formation for the six-field dot product and direct
execution, refreshed data fields, an accessor installed after cache warming,
an inherited accessor, a Proxy argument trap, string and BigInt addition
fallback, primitive `this` boxing, negative-zero multiplication, and a
duplicate formal parameter. The latter exposed that eager plan construction
had observed the deduplicated local-slot list before `new_function` restored
the authoritative parameter-position list; construction now supplies that
list first, so `function (other, other)` resolves `other` to the final
argument as required. The focused runtime test and the complete 1,891-test
runtime suite passed.

The candidate release SHA-256
`f10358f13f4782520c2b6fee21e4097418f89c404597220d3892d3c73fc91251` was
compared with exact parent-base SHA-256
`77e3ae4663ee65145aa0909fbed2ae5cbf522fc53f4e6bf8b83e64d4289bdfa8` and
pinned QuickJS-NG SHA-256
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
For a 10-million-call vector dot-product diagnostic, candidate user time was
2.47--2.49 seconds versus base 4.87--4.88 seconds, a repeatable **1.96x**
local improvement. QuickJS-NG took 0.75 seconds, so this remains about 3.3x
slower than the reference and is not a campaign-completion claim.

The complete three-block broad-v2 diagnostic recorded **225/225** eligible
formal samples, **600/600** `ok` linearity samples, and all **300/300** paired
linearity ratios within the 0.85--1.15 bound. Candidate/base geometric mean
was **1.002488x** (worst `math_abs` 1.020903x; call family 1.003022x);
candidate/QuickJS-NG was 0.092866x. The raw JSONL SHA-256 is
`eb84c950d0cfc6ef197966b07c5d9e6402f88637fbde53555cd46365aea777a4`.
This dirty, receipt-free run is diagnostic only; strict reporting correctly
refuses a formal claim because its provenance is unverified.

The matching full external preview retained all 5/5 JetStream rows and 26/26
SunSpider rows, plus 13/14 Kraken rows; `imaging-gaussian-blur` timed out for
both qjs-rust roles while QuickJS-NG completed, so it remains explicitly
noncomparable. Candidate/base diagnostic geometric ratios were 0.996636x for
JetStream, 1.000964x for comparable Kraken, and 1.005532x for SunSpider. The
field-heavy CDJS and `raytrace-public-class-fields` rows improved independently
to 0.995382x and 0.962834x of base; the surrounding suites remain neutral, as
expected for a narrow general fast path. Candidate/QuickJS-NG remains 2.669004x
for JetStream and 2.446380x for SunSpider, so this slice advances a shared
property/call cost without closing the 2x target. External report/raw SHA-256
values are `86a1c91e7a2c96035a3a51f1f6ca6b73f37265dd0f8e60f00ddb349ae75bd8bc`
and `766c868c171187d67ce2ae92cbf9eda28d1323e7e7307a6c39ad0329d8dac48a`.

### 2026-07-27 rejected direct-leaf tail-frame reuse

A fresh profile of the queue's first-ranked SunSpider
`controlflow-recursive` opportunity still showed direct-leaf `Vm` entry in
the recursive chain. A narrowly guarded candidate reused the active frame only
when the callee had the same bytecode, the next instruction was exactly
`Return`, and no try/finally, disposable scope, or pending abrupt completion
remained. Existing numeric/property leaf plans retained priority; all other
calls fell back unchanged. Focused tests covered 10,000 recursive calls,
receiver/parameter transfer, and active-finally fallback.

The mechanism was rejected and reverted after its first fast screen. On a
50-times wrapper of the hash-verified upstream source (wrapper SHA-256
`09e7ab8190fcba58055963f3a6f84f126e99add8ae5332424723a48aab8962d9`), three
alternating local runs measured candidate real times of 3.45, 3.44, and 3.45
seconds against 3.41, 3.43, and 3.41 seconds for the exact-base binary. The
median is **1.012x candidate/base**, failing the predeclared `<= 0.90x` target
gate. The dirty candidate binary SHA-256 was
`48b5743d012fb57398a7994d7b6fc04020e02379f8a509a4d1562e805e9cde58`; this
is a local diagnostic, not a portfolio claim.

The route avoids Rust-stack growth but leaves the expensive direct-call
environment/frame construction in place and adds a tail probe to every
direct-leaf call. Do not retry it by caching or reshaping that probe; a future
attempt must remove a different shared cost and start from a fresh profile.

### 2026-07-27 rejected same-function direct-leaf frame stack

A second, distinct mechanism targeted all exact same-`Function` direct-leaf
recursion rather than only tail calls. It retained independently initialized
parameter, receiver, operand, and try state in a `Vec<FrameState>` owned by
one VM, then delivered return values and errors through the original caller
boundaries. Different functions retained the existing nested-VM route. The
fresh profile was `/tmp/qjs-profile-controlflow-refresh.sample` (SHA-256
`1807dd1c5291b25b036ffd8e89598fdf48aad32778172d484754d74a1065440c`), whose
dominant chain repeatedly entered `Vm::run_completion` through the direct-leaf
call path.

The candidate passed a focused 10,000-level non-tail recursion test plus
caller-operand, 1/2/3 parameter, receiver, thrown-value identity, nested
catch/finally ordering, and different-function fallback checks. It was still
rejected after its sole predeclared fast screen: on the same 50-times,
hash-verified upstream wrapper (SHA-256
`09e7ab8190fcba58055963f3a6f84f126e99add8ae5332424723a48aab8962d9`), three
complete alternating pairs measured base real times of 3.34, 3.32, and 3.32
seconds against candidate times of 3.67, 3.67, and 3.67 seconds. The medians
are **1.105x candidate/base**, a 10.5% regression rather than the required
`<= 0.90x` target. Candidate and exact-base release SHA-256 values were
`0f4dd32fa4623e1643909c419b2c1ae35c00bf2260491e7b96f391e12db3e2ed` and
`22ac7687531e8b7044ddefa6844d6af70a4f4cea2ea347f01932a8e44166143c`.

Do not refine this stack scheduler or reuse its return/error transfer shape:
it removes nested Rust calls but adds enough frame movement and dispatch that
the measured hot path regresses. Any successor must target a different shared
cost and begin with new profile evidence.

### 2026-07-27 rejected constructor receiver-write leaf

The rank-3 `raytrace-public-class-fields` and rank-4 `hash-map` profiles both
showed constructors whose bodies repeatedly perform statically named
`this.field = parameter-or-primitive` writes through a general function frame
and VM. A prevalidated plan accepted only complete straight-line base or
ordinary constructor bodies with that exact shape. Before its first write it
preflighted the entire receiver/prototype chain, declining accessors, Proxies,
non-writable descriptors, non-extensible receivers, derived constructors,
explicit returns, `super`, eval, and closures. That preserves field
initialization order and avoids partially observable fallback.

The candidate passed focused classifier and no-partial-write tests, constructor
field-order/setter/`Reflect.construct` fallback tests, `cargo check -p
qjs-runtime`, and 37/37 curated Test262 class cases using the candidate release
binary. Its SHA-256 was
`6fd8b53069cc22326ded4d47fae918c6559c74b22b625146405fd8a72e04ce0c`; the
exact base binary was
`22ac7687531e8b7044ddefa6844d6af70a4f4cea2ea347f01932a8e44166143c`.

The predeclared dual-target fast gate nevertheless failed. The three-block
hash-verified external screen (report SHA-256
`29038eb94918e61419f0b650da17b9e6395558e626dab1263f07db384e96620f`, raw
SHA-256 `cefd5716b9d4735420d005c2d6fca1ad03f39e191c8d28bc1c1f8d5965b8d626`)
measured `raytrace-public-class-fields` at **0.852996x** candidate/base but
`hash-map` at only **0.986041x**, missing the frozen `<= 0.90x` target. The
independent Kraken `ai-astar` and SunSpider `controlflow-recursive` sentinels
were 1.005862x and 0.959686x respectively. The implementation and its tests
were reverted immediately; only the plan and this negative evidence remain.

Do not refine this constructor-only write plan by widening literal forms or
relaxing its Set preflight. It is a useful raytrace-specific signal but not the
shared two-workload improvement its declared gate required. A successor must
identify a different cost that is current-profiled in both targets.

### 2026-07-27 rejected streaming simple-RegExp quantifier boundaries

Fresh profiles of rank-6 `string-tagcloud` and rank-14
`string-validate-input` shared the RegExp matcher path through
`match_pattern_first`, `repeat_atom`, and `simple_atom_boundaries`. The
candidate avoided allocating the temporary boundary vector only for
non-Unicode, single-code-unit, capture-free quantified atoms. It scanned the
contiguous run once and replayed greedy or lazy continuation candidates by
index; Unicode, surrogate-pair, multi-code-unit, and captured atom shapes
retained the existing boundary-vector path. This was source-independent
matcher work with no workload-name, input-length, checksum, or source-path
condition.

The focused `regexp::matcher` suite passed 42/42 tests, including greedy and
lazy backtracking, captures, lookaround, Unicode surrogate behavior, and long
repetition coverage. Both exact external sources also completed successfully.
The candidate release SHA-256 was
`93b73dbce06d26ebc6150095b77df5a719d03108ddba951ab6327b2ba72f7366`; after
revert the release binary exactly matched the base SHA-256
`22ac7687531e8b7044ddefa6844d6af70a4f4cea2ea347f01932a8e44166143c`.

Its single predeclared fast screen used one warm-up plus seven alternating
candidate/base process samples per exact upstream workload. `string-tagcloud`
improved only to **0.954032x** candidate/base (202,932,166 ns median versus
212,710,042 ns), and `string-validate-input` regressed to **1.020318x**
(75,609,125 ns versus 74,103,500 ns). Both miss the frozen `<= 0.90x` target
gate, so the matcher change was reverted immediately. The profile receipts are
`/tmp/qjs-profile-tagcloud-current-20260727.sample` (SHA-256
`1accdced86069af4e9c8cbf1b1f8ddf2e9b802a853d62713c44c17cc071dbfb6`) and
`/tmp/qjs-profile-string-validate-input-current-20260727.sample` (SHA-256
`94b1bbaa20b544d2c64635bc33ef418e93ee98c300f9f5198cd23e1188e053f0`).

Do not retry this boundary-vector removal by broadening its atom classifier or
by adding another streaming probe. Its one material local movement is below
the campaign gate and its paired target regresses. A future RegExp unit must
identify a different current-profiled cost, such as construction, matching, or
result materialization, shared by its chosen targets.

### 2026-07-28 rejected static property-store cache diagnostic

A current-main A* profile showed named-property cache work beside the shared
dispatch and direct-leaf call chains. The diagnostic therefore gave every
ordinary `obj.name = value` site a cache and allowed a small-object slot cache
to validate separately allocated but textually equal property keys. The latter
preserved its full slot, accessor, and writability checks; focused coverage
proved cross-object read/write reuse, and all 1,900 `qjs-runtime` unit tests
passed.

The first alternating exact-source screen rejected it immediately. The
candidate built from main `6fa6ecde464489b5e2de5947da6f492ad40abd99` had
SHA-256 `020bd036fa7d6e2831dff19129f1a30dbf349ceb36cabd23093e40923d391bd9`;
the exact-main base binary had SHA-256
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`.
The hash-verified A* adapter source had SHA-256
`a3653c77773ce2b424301835021957b26119240810f43d5434d98fd88d7a416c`.
Its first candidate/base pair was 11.76 s versus 10.83 s, or **1.086x
candidate/base**. This was an early diagnostic rather than a performance-unit
claim, so no broad or promotion run was started and the implementation was
reverted before a second pair.

Do not retry this by merely changing cache cardinality, key comparison, or
which static stores allocate a cache. The extra cache state and per-store
probe outweigh the avoided lookup in the selected top-ranked workload. A
successor must begin with a fresh profile and remove a different shared cost.

### 2026-07-27 retained ordinary `for-in` key-list cache: local fast gate only

The next unopened external opportunity was SunSpider `string-fasta`. A fresh
profile of the exact base attributes 242 of 1,457 samples (about 16.6%) to
`EnumerateKeys`, `own_property_names`, descriptor queries, and their temporary
key-list allocation. The receipt is
`/tmp/qjs-profile-string-fasta-current-20260727.sample` (SHA-256
`5e8ff803689022a0d9b802aef92e564bda5d64efae321998f4b39bda654e6e89`).
Current profiles of the other ranked `for-in` sources did not share that cost,
so this is deliberately a one-target, one-attempt unit rather than a claim of
cross-workload campaign progress. Its frozen plan is
`tasks/performance-units/for-in-ordinary-key-cache.json`.

The implementation retains a hidden `ArrayRef` per `EnumerateKeys` bytecode
site only after a canonical enumeration of an all-`ObjectRef` chain. A reuse
requires every live link to retain its exact identity, prototype link, and
own-layout revision; arrays, functions, Proxies, typed arrays, module
namespaces, symbol primitives, and unsupported chains use the canonical path.
The existing live per-key descriptor check remains inside the loop, so a
deletion while an already-started loop is running is still observable. Focused
coverage mutates own and inherited enumerable layouts, replaces a prototype,
and deletes a later key from inside the loop.

Against exact-base release binary SHA-256
`22ac7687531e8b7044ddefa6844d6af70a4f4cea2ea347f01932a8e44166143c`, the
candidate binary SHA-256
`4066181c59f5c3bc5d68f15b3cee4f3446dca0e90c3b40db27e96932e3680205` passed
the single declared fast gate. Seven alternating exact-source process samples
had medians of 119,263,500 ns candidate and 286,959,125 ns base, or
**0.415611x candidate/base**. The independent `string-tagcloud`, `regexp-dna`,
and `string-base64` controls were 1.000758x, 1.002190x, and 0.995233x;
single-case broad diagnostic controls were 1.000753x (`local_read`) and
1.000574x (`object_allocation`). The latter raw diagnostic is not a complete
broad report and is not promotion evidence. A candidate profile at
`/tmp/qjs-profile-string-fasta-for-in-cache-20260727.sample` (SHA-256
`24fbbfe3fcf82fc95d4e4dff25632189286c12a0a6b6672019e4c3dfd8e0f916`) no
longer shows the prior `enumerable_keys`/`own_property_names` hot chain.

The focused runtime `for_in_` suite passed 10/10 tests, the targeted Test262
slice passed 3/3, `check.sh` passed including 5,160/5,160 curated Test262
cases, and `compare-qjs.sh` passed. A one-block three-engine external smoke
completed for all 45 fixed cases: 44 were fully comparable and
`imaging-gaussian-blur` timed out for both qjs-rust roles while QuickJS-NG
completed. Its SunSpider `string-fasta` row was 0.418045x candidate/base.
The report explicitly marks `claim_eligible: false` because it has one block
and incomplete Kraken comparability; report/raw SHA-256 values are
`dbaea6091d5766dd0be13d612d29a00c9f4ad010aa5f19d2daa5f73480d4d3b2` and
`6f82929b2246dd8373f09ed00d19d563ec86fed09f321e778fe9ebdaf45f7f68`.

Retain this as a local, general engine candidate, not a T021 or 2x-campaign
promotion. Promotion still requires the plan's complete broad and external
evidence plus a zero-gap exact Test262 scan from an exact committed candidate.

### 2026-07-28 rejected compact direct-call dispatcher

The refreshed exact-`6cac3f50` profile of rank-1 SunSpider
`controlflow-recursive` still rooted 713 of 816 main-thread samples in
`Vm::run_completion`. The frozen plan
`tasks/performance-units/compact-hot-dispatch.json` (SHA-256
`c0cf73dd23c22279e04f6fd5ca8fd13e5c5e6836f0a48cf80472e99cb093ba13`)
therefore compiled an all-or-nothing smaller instruction stream for direct-call
function bytecode containing only constants, local loads/stores, binary
operations, calls, returns, and forward branches. It had no source name,
workload name, iteration count, or type admission. Backedges, virtual-object
views, dynamic scope, `try`, disposal, generators, and unsupported bytecode
retained the original dispatcher.

The focused plan/admission/coercion tests passed, as did the 5,160-case curated
Test262 subset and all QuickJS-NG comparison fixtures. The candidate release
binary SHA-256 was
`005e7a408a082d1477f983ddb822b7b39736b8ff53e9b932f81b0eac5a9da2e7`; the
exact-base binary SHA-256 was
`d7ecaed330745fb257f9286bdb661f0dd489426578daaf026a33f1019c624987`.
On two alternating runs of the 100-times hash-verified pinned-source wrapper
(wrapper SHA-256
`52ecb05f622d41dd35db1d476f1f5c46d16e252efa72946516b5396c33f56261`), base
CPU times were 6.79 s and 6.80 s while candidate times were 6.51 s and 6.50 s.
Their medians are 6.795 s and 6.505 s, or **0.9573x candidate/base**. That is
about a 4.3% improvement, well short of the frozen `<= 0.90x` target gate, so
the dirty-worktree timing is negative diagnostic evidence rather than a
promotion claim.

The implementation and its focused tests were reverted immediately. Do not
retry this exact compact-stream design by widening its opcode set or changing
its admission guards: reducing only the outer `Op` match does not remove enough
of the direct-call, local-load, binary, and value-management cost. A successor
must start from a fresh cross-workload profile of one of those remaining shared
costs.

### 2026-07-28 rejected scalar self-recursive numeric call cluster

The next rank-1 attempt began from the same exact-`6cac3f50` controlflow
profile, but inspected the compiled `ack`, `fib`, and `tak` bodies before
implementation. Each body contains only primitive numeric constants and
parameters, arithmetic/comparisons, branches, returns, and fixed-arity calls
through its captured self binding. The frozen plan
`tasks/performance-units/numeric-recursive-call-cluster.json` (SHA-256
`ed076b7227201cde161b7f8ccf6f26ca42f28567fb0febeb1a15f4121fb46d59`)
therefore used a bytecode-complete scalar frame stack only after verifying the
live self-upvalue identity and Number arguments. Every other opcode, capture,
argument, or depth kept the ordinary direct-leaf VM before user-observable
work. Focused coverage included all three source shapes, signed zero, live
self-binding replacement, string coercion fallback, and rejected mutation or
non-self calls.

The mechanism did remove the intended target cost. Against exact base binary
SHA-256 `d7ecaed330745fb257f9286bdb661f0dd489426578daaf026a33f1019c624987`,
the final candidate binary SHA-256
`61fb8763a5aa01feda5b19179d86ca1a13b836b72fd4c779d842f065a836499b` had
two alternating 100-times pinned-source runs with base CPU times 6.91 s and
6.94 s and candidate times 1.55 s and 1.55 s: **0.2238x
candidate/base**. The hash-verified wrapper SHA-256 is
`52ecb05f622d41dd35db1d476f1f5c46d16e252efa72946516b5396c33f56261`.

It nevertheless fails its frozen control gate. Two final alternating A*
samples gave base CPU times 9.94 s and 9.60 s versus candidate 10.24 s and
10.05 s, whose medians are 9.77 s and 10.145 s, or **1.0384x
candidate/base**, above the plan's `<= 1.03x` ceiling. Single exact controls
were neutral-to-positive for `hash-map` (1.62/1.68 s = 0.9643x) and
`raytrace-public-class-fields` (2.45/2.53 s = 0.9684x), but they cannot offset
the failed A* gate. The candidate A* profile receipt is
`/tmp/qjs-profile-ai-astar-numeric-recursive.ankPLX/ai-astar.sample` (SHA-256
`4320f2a222c297861420b274f58ead01e1d9d60fe86b0c5bcc36c9ca7b4ea792`) and
remains rooted in the generic VM/direct-leaf chain.

The scalar implementation and its tests were reverted immediately; no runtime
code is retained. Do not retry this unit by moving its admission checks,
raising the depth limit, or broadening its opcode subset. The one-attempt plan
closed when a non-target direct-call workload regressed, so the next unit must
remove a different current-profiled shared cost.

### 2026-07-28 rejected direct-leaf `CallEnv` cold-state refactor

Four fresh current-main profiles showed the same ordinary direct-call chain in
the top four external opportunities: `Vm::run_completion` followed by
`call_direct_leaf_function` accounted for 87% of controlflow-recursive samples,
99.8% of hash-map and raytrace-public-class-fields samples, and 96.7% of
ai-astar samples. The frozen one-attempt plan
`tasks/performance-units/direct-leaf-frame-cold-state.json` (SHA-256
`22efae97826e42c388114d6e0f93619d993e4d1ea0d175dd6ab4c6265b00efe6`)
therefore moved constructor, dynamic-scope, private-name, direct-eval, and
module-live state from the inline `CallEnv` representation into an on-demand
copy-on-write block. Ordinary direct leaves began with no cold block; every
dynamic or exceptional operation continued through the same `CallEnv` API.

The candidate passed the focused environment tests and all 1,899
`qjs-runtime` unit tests. Its release executable SHA-256 was
`1bb84cf3202c31a88a474b71f470832ca68f441b8a3c43623834adab5ef9aedf`; the
runtime-identical current-main base was
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`.
The fixed six-case external manifest SHA-256 was
`b1e6f158e55bd94967d783d8b488d8af9757eec1164b00c339cc23a72015e36c`.

Its three-block, three-role Latin-square external gate rejected the mechanism:
the four frozen targets were **0.978910x** hash-map, **1.005638x**
raytrace-public-class-fields, **0.975448x** ai-astar, and **0.970403x**
controlflow-recursive candidate/base, all missing the required `<= 0.95x`
target gate. Independent controls were within their regression ceiling
(`date-format-tofte` **1.002767x**, `string-tagcloud` **0.975388x**), but they
cannot compensate for a failed target gate. The raw receipt SHA-256 is
`a09d096d79581f458f6240f43fb325c578ed1bae3fb029390b99a6fad971bfe0` and the
report SHA-256 is
`e3c92e9aebe933f6e38c31f2737abfb22b8bac5b958fb51f85e8f471e38629c9`.

The runtime implementation and its focused tests were completely reverted;
the partial external fast gate failed before broad controls, complete external
coverage, or Test262 promotion work was warranted. Do not retry this exact
cold-block representation by repacking fields, changing the box placement, or
splitting out a smaller subset of the same state: the intended shared direct
call path did not produce a material cross-workload win, and class-field
raytrace regressed. A successor must profile and remove a different shared
cost.

### 2026-07-28 rejected immediate object-value slot read cache

Fresh profiles of rank-2 `raytrace-public-class-fields` and rank-3 `ai-astar`
showed ordinary named-property cache update work under their hot direct-call
chains. The frozen one-attempt plan
`tasks/performance-units/immediate-object-slot-read-cache.json` (SHA-256
`53a7862abafafa94439326495402b634cc9d71c28b5017d3e3c2e30e475c1a9d`)
therefore made the existing `OwnSlot` representation available on the first
object-valued read of a compact ordinary receiver. It retained the current
receiver-identity and layout-revision checks; accessors, exotics, shaped and
dynamic storage, structural changes, and non-object values retained the
unchanged cache path. Focused tests proved that an ordinary value write reads
the live replacement from the slot and that the weak receiver entry does not
keep an object alive.

The candidate passed the focused named-property cache suite. Its release
binary SHA-256 was
`d8370a54beaacd85ed8bfc63085ff8891e9b8a9db82225693f8c943701cebdbf`; the
runtime-identical base was
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`.
The raytrace wrapper was hash-verified at
`824daa5582289787f6e25a200892a1d6bdfa682afe9ce76a30e520dc7e03528c`.
Three alternating exact-source pairs measured candidate/base ratios of
**0.995951x**, **1.000000x**, and **1.000000x** (candidate/base real seconds
2.46/2.47, 2.44/2.44, and 2.43/2.43), for a median of **1.000000x** rather
than the frozen `<= 0.97x` target. A supplementary fixed-source A* diagnostic
also moved in the wrong direction: 10.23 s candidate versus 10.05 s base, or
**1.017910x**. The A* wrapper SHA-256 was
`a3653c77773ce2b424301835021957b26119240810f43d5434d98fd88d7a416c`.

The runtime implementation and its focused tests were reverted immediately;
the raytrace target failed before broad controls, complete external coverage,
or Test262 promotion work was warranted. Do not retry this exact first-read
promotion by changing only its value-category guard or `Exact`/`OwnSlot`
selection order: the avoided weak-value cache work is below the current
cross-workload materiality threshold. A successor must profile a distinct
shared object or call cost.

### 2026-07-28 rejected shared instance-field key installation

The next rank-two raytrace unit froze
`tasks/performance-units/shared-instance-field-key-install.json` (SHA-256
`87355449a8b18384d62e7edbfeb438500df546d60fb52d22fa90a3fcce475e08`)
against the exact `6cac3f50` queue. Its fresh raytrace receipt showed 117
`initialize_instance_fields`, 78 `define_property_on_value_key`, and 60
`ObjectRef::define_property` frames among 1,550 main-thread samples. The
candidate shared each resolved public-instance string key with the target
object and combined the ordinary absent-property check with insertion. It
retained the generic path for symbols, Proxies, non-object receivers,
TypedArrays, module namespaces, non-extensible receivers, and existing own
properties; focused tests covered both an earlier initializer defining a later
field and a receiver made non-extensible before its next field.

The class-heavy target did improve materially. Candidate binary SHA-256
`ecb78e1fb4abf24903433293916db12a881f5e080ed3a66e5cc00ddbb788050e`
versus base SHA-256
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`
produced the same output SHA-256
`f5bc4f369844bf414bcaa550808d7e5406037ad4da77bde3ae30fcaa7701bdfc`
on all six pinned raytrace wrapper runs (wrapper SHA-256
`824daa5582289787f6e25a200892a1d6bdfa682afe9ce76a30e520dc7e03528c`).
The three alternating candidate/base pairs were 1.91/2.42 s, 1.90/2.42 s,
and 1.97/2.45 s: a median **0.789256x candidate/base**, well past the frozen
`<= 0.97x` target. A*, hash-map, and controlflow diagnostics stayed within the
non-target ceiling: A* was 9.65/9.74 s, 9.65/10.18 s, and 9.44/10.16 s;
hash-map was 1.62/1.59 s, 1.62/1.59 s, and 1.62/1.60 s; and controlflow was
6.29/6.31 s. Their wrapper SHA-256 values are
`a3653c77773ce2b424301835021957b26119240810f43d5434d98fd88d7a416c`,
`aa98bc1975d8824840df5c31a397b5dab27d514e1854ff5681ceea8ec4bf2c20`, and
`52ecb05f622d41dd35db1d476f1f5c46d16e252efa72946516b5396c33f56261`.

It nevertheless fails the frozen object-allocation control. The focused
two-role runner independently normalized candidate/base to about **1.097x**
(median 719,083,333 ns / 12,612,198 operations versus 719,461,667 ns /
13,840,373 operations). Fixed-work manual confirmation used the same
13,840,373 iterations and identical output SHA-256
`e4f3cf3bb850e4e733486d922740880c221c11505fa0710b8de260482c04b98b`:
candidate/base was 0.77/0.71 s, 0.78/0.71 s, and 0.77/0.71 s, or a median
**1.084507x**, above the unit's `<= 1.03x` control ceiling. The runtime code
and focused tests were reverted; complete external coverage and Test262
promotion were therefore not run. Do not retry this exact shared-key plus
single-borrow layout by changing field guards or accepting the allocation
regression. A successor must first isolate the general allocation regression
and remove it structurally.

### 2026-07-28 rejected dense-index numeric leaf

The frozen one-attempt plan
`tasks/performance-units/dense-index-numeric-leaf.json` (SHA-256
`11365e3011774f32f031fc3b664c35b148847783bb81df2e76d6d2d87e8c06e4`)
targeted rank-7 SunSpider `3d-raytrace` after the higher-ranked direct-call,
field-cache, and allocation candidates had either been rejected or profiled as
different costs. A fresh exact-current profile
`/tmp/qjs-current-3d-raytrace-profile.sample` (SHA-256
`5e18053f743296ba821bba030b66e76429987999ce9d4019749501d29af933a0`)
showed 1,105 main-thread samples rooted in `Vm::run_completion` through
`call_direct_leaf_function`. The pinned source repeatedly calls ordinary
array-parameter vector helpers such as `sqrLengthVector(self)`, whose fixed
numeric element reads currently execute through the child VM.

The candidate preclassified only complete straight-line numeric leaves with
fixed parameter-array indices and live present dense Number elements. Holes,
indexed descriptors, proxies, TypedArrays, non-Number elements, dynamic
indices, `this`, calls, stores, and control flow retained the original VM
before observable work. Focused plan and execution tests covered ordinary
arrays, duplicate formals, holes with inherited getters, own indexed getters,
proxies, string fallback, and signed zero; they passed before timing.

The release candidate SHA-256
`5117a1205e978d28252908d215ba9189d8ec39fc73208ab234d91c0393744c28`
was measured against the runtime-identical base SHA-256
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`.
The three-block, hash-verified, three-suite diagnostic manifest SHA-256 was
`1f536f44bf73c8ac9ecc09fea46c19c44d6627beaa26b12365ad9a03d0a2ee4f`;
the report/raw SHA-256 values were
`0c2b85ea0e4019e57f1c206097526e0ffe34df52dc6fad80cb0bf494fe89d04e` and
`15dcd3634e8e26502961a31796e8570f752d8547eedd41402823bdd6a290bbd2`.
`3d-raytrace` moved only from 86.955 ms to 85.779 ms, or **0.986477x**
candidate/base, missing the frozen `<= 0.93x` target. The independent A*
control regressed from 9.700 s to 10.625 s, or **1.095380x**, exceeding the
`<= 1.03x` control ceiling; class-field raytrace was neutral at 0.999009x.

The runtime implementation and its focused tests were completely reverted;
no complete portfolio or Test262 promotion run was warranted. Do not retry
this direct dense-index leaf route by widening indexed forms, accepting more
array/value classes, moving its probe to the `this` plan, or changing only
its guards. A successor must begin from a distinct current-profiled shared
array or call cost.

### 2026-07-28 rejected shared-slot promotion compaction

The rank-11 `access-nbody` profile found 98 `NamedPropertyCache::get` frames
among 1,466 main-thread samples. Its `Body` objects use the same seven compact
fields repeatedly, and cache inspection showed that the second receiver
promotes a site to `SharedSlot` while leaving an older `Exact` entry ahead of
it. The frozen one-attempt plan
`tasks/performance-units/shared-slot-promotion-compaction.json` (SHA-256
`d966e0295c4aa8afd17850a641071cb8ac74cf29462d88eddc055466190cb97c`)
therefore compacted that promotion to one front-positioned shared key/slot
entry. The candidate retained the existing per-hit interned-key, slot,
accessor, descriptor, and generic-fallback checks; focused cache tests also
proved that a nonmatching ordinary receiver can still occupy the second cache
entry and that paired writes return the current shared-slot value.

The release candidate SHA-256
`65d0b7049d6eec4609e14560b19e3ae167339d6e7443f847db57ef32d4d8374c`
was measured against runtime-identical base
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`.
The fixed six-case, three-suite manifest SHA-256 was
`a7bdd19bcb3101197adf94ea9ad70868d3aae6632abf8122a1d8649542c2acce`;
the raw/report SHA-256 values were
`24d808f3b3def646352f4962fbad68cf08fd7a4f5ac27f44572ca3490006ae8a` and
`2a301b1a2219beab2966e12f65751b1ee6867ef847cd3d839040f2fb774483c1`.
The target improved only from 73.538 ms to 72.690 ms, or **0.988474x**
candidate/base, missing the frozen `<= 0.95x` gate. The independent A*
control regressed from 9.795 s to 10.798 s, or **1.102369x**, exceeding the
`<= 1.03x` ceiling; hash-map, class-field raytrace, 3d-raytrace, and
controlflow-recursive measured 1.012362x, 0.974786x, 0.991758x, and 0.998676x
respectively.

The runtime implementation and focused tests were completely reverted before
any broad, complete-external, or Test262 promotion run. Do not retry this
cache-entry ordering or compaction mechanism by changing the placement,
retaining one identity entry, or varying cache cardinality: the avoided probe
is below the materiality gate and the independent control regressed. A
successor must profile a different shared property, numeric, or call cost.

### 2026-07-28 rejected primitive String prototype read cache

The rank-12 `string-base64` profile showed repeated primitive method lookup:
182 of 1,479 main-thread samples were in `Vm::try_direct_get_string`, with the
same direct-leaf chain also retaining `primitive_prototype_env`, String
constructor/prototype resolution, and property-table reads. The frozen
one-attempt plan
`tasks/performance-units/primitive-string-prototype-read-cache.json` (SHA-256
`afc7845c05e422e064ebc06cf3ce5dbd9b226011bc036d90aaabe7d067931e53`)
therefore cached only ordinary own data properties from the current
`String.prototype`, with weak values plus constructor/prototype mutation
guards. Focused checks covered accessors, global `String` replacement,
prototype replacement, and string-own `length` precedence.

The profile's 48-source concatenation was used only to lengthen the sample
interval and is not timing evidence. Exact-source process timing did not
reproduce the initial apparent speedup: ten independent `string-base64`
processes took 0.74 s for the candidate versus 0.77 s for the exact base, or
**0.961039x candidate/base**, missing the frozen `<= 0.95x` target gate. More
importantly, the three-block broad control run rejected the mechanism even
after moving the cache off the generic bytecode-cache layout: candidate binary
SHA-256 `eef360a3497b86492fc4f796fa23e44d9e60dad7ae49984b436bad60d4ce1402`
versus runtime-identical base
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`
measured `dynamic_method_call` **1.000809x** and `object_allocation`
**1.083295x** candidate/base. The latter exceeds the unit's `<= 1.03x`
control ceiling; raw diagnostic SHA-256 is
`3abd92f66232db8b1906c0e27dd0e563a95232c7606aac7293360448e2f46241`.

The runtime implementation and its focused tests were fully reverted before
complete external coverage or Test262 promotion work. Do not retry this
per-site or constructor-side primitive-prototype cache by changing cache
placement, weak-value representation, or helper inlining: the repeated lookup
is real, but the guard and dispatch cost does not meet the current general
performance gates. A successor must remove a different current-profiled
shared cost.

### 2026-07-28 accepted typed-loop scratch reuse

The rank-14 `bitops-bits-in-byte` profile captured 897 inclusive
`try_run_typed_loop` samples out of 3,707 while the hot `bitsinbyte` function
ran 89,600 times. Executor inspection found that each already-admitted typed
loop rebuilt its register, boxed-value, receiver, inline-cache, and optional
sloppy-global vectors. The frozen one-attempt unit
`tasks/performance-units/typed-loop-scratch-reuse.json` (SHA-256
`33b59921bf381ccaa3be6bec8ec01995b483a1c78bd8dc645825137677b81a3f`)
therefore gives every `TypedLoopProgram` a lazily-created, single-entry scratch
pool. It clears every vector before reuse and after exit; recursive entry gets
fresh independent storage, and only one completed bundle is retained. No
benchmark source, source-path condition, iteration count, or workload-specific
execution route was added.

The focused `typed_loop_scratch_pool_reuses_only_cleared_storage` test proves
lazy creation, cleared reuse, retained capacity, non-aliasing nested entry, and
the one-bundle cap. `cargo test -p qjs-runtime typed_loop --lib` passed 12
tests, and `cargo clippy -p qjs-runtime --lib --tests -- -D warnings` passed.
The final candidate binary SHA-256
`8b86c312da742f89a8a0b4710f5d5e0409f1e4f86e90a3f6b4bcf0699dffe6cb` was
compared with the runtime-identical base SHA-256
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`.

The complete three-block external report/raw SHA-256 values are
`55739fdb2a6cdb7ed4b3f3ce209856cb5aac7ec177a64c2598c76ca59218c21e` and
`81d282355495ff26548409b11387983006d56b327e00b7a979150baf8cd1ef38`.
All 45 cases completed for candidate, base, and QuickJS-NG with a 60-second
per-case limit. `bitops-bits-in-byte` improved from 83.824 ms to 76.297 ms,
or **0.910205x candidate/base**, clearing the `<= 0.95x` target. Its
candidate/QuickJS-NG ratio remains **2.859752x**, so this is not a claim that
the overall two-times-QuickJS-NG goal has been reached. Suite candidate/base
geomeans were **0.990006x** (JetStream subset), **0.996615x** (Kraken), and
**0.989342x** (SunSpider); no individual external case exceeded the 1.03x
regression ceiling.

The final three-block fast broad controls (raw SHA-256
`aa43bcffcbd7ed027448814beb01cc6f3309df2b9df4454d90e0b3ba9a2d590e`) were
`array_write` 0.99898x, `dynamic_method_call` 0.99432x,
`plain_function_call` 1.00160x, and `object_allocation` 1.01489x. The first
full 25-case long-run raw record (SHA-256
`35c9eea609dfd90a2f3b8c2584318f4f42871f006f28a3be1965ea999cdd5129`) showed
apparent regressions for `math_abs` 1.067x, `array_index_of` 1.0885x, and
`array_dynamic_read` 1.0965x. Those observations were not discarded:
five-block isolated rechecks (SHA-256
`4668fc169f9e92ef9f4d6f5a7939ef6b033d85ff7e9319baa723659f4ac377c6`) instead
measured 1.00571x, 0.96501x, and 0.93875x respectively, while
`dynamic_method_call` and `plain_function_call` were 0.99059x and 1.00246x.
The contradiction is consistent with long-run frequency drift rather than a
reproducible regression; both raw artifacts remain recorded rather than being
selectively hidden.

`./scripts/test262-subset.sh` passed all 5,160 selected cases,
`./scripts/compare-qjs.sh` passed, and `./scripts/check.sh` passed. This is an
accepted general allocation reduction, but only an incremental campaign step:
the performance queue must continue from a newly profiled, unclosed shared
cost rather than treating this target-local win as completion.

### 2026-07-28 accepted cached direct-eval selected bindings

The rank-five `sunspider-1.0/date-format-tofte` profile attributed 802 of
1,209 samples to `native_global_eval` and its caller-environment construction
path. On an exact hit in the existing direct-eval compilation cache, the old
path still materialized every visible caller binding before evaluation. The
frozen unit `tasks/performance-units/direct-eval-selected-bindings.json`
(SHA-256 `e420347b498df54eaaaeec240129e8be38fc301b8776422cb71412f4b89bc9c8`)
instead builds that cached call environment from static global references,
`this`, and the cache entry's selected outer names. Declarations, nested direct
eval, `with`, cache misses, and all other dynamic cases retain the complete
caller-environment path. The selected cells preserve their prior lexical order
and deoptimized shared-cell identity; no source-path condition, benchmark
condition, or target-only execution route was added.

The focused
`cached_direct_eval_keeps_only_referenced_outer_bindings` test covers selected
reads and writes, `this`, `arguments`, cache hits, and a binding first created
by declaration-bearing eval. It passed along with all 1,900 runtime tests and
the 65 directly relevant Test262 cases selected by the touched-file gate.
`./scripts/compare-qjs.sh` passed. The exact candidate binary SHA-256
`a048b921e4402a7368e42e15649d3d200088b8347049af02962807c4cf0a3d0b`
was compared with the exact base binary SHA-256
`776de42b79bb7200f372584b705a234d222405c10440e968bf961bc0d49a68ed`.

The three-block formal broad report SHA-256
`e7b0c67a131ab9aacef8a529a4819f93d9c4d2dc0253c57b62bfcae51baa1fe3`
completed all 25 cases and measured 0.9998x candidate/base overall. Its
pre-registered controls were `dynamic_method_call` 1.00098x,
`plain_function_call` 1.00102x, and `object_allocation` 0.99363x, all under
the 1.03x regression ceiling. A matching three-block 60-second complete
external run (report/raw SHA-256
`aade22a0399424592e04ed637f9afd8bb3262ae8959a7c119ba3bf30eef5bf43` and
`d67a1997049041183fd525f56e077a577047d6da45c49c3181426121661ec5f6`)
completed all 45 cases for candidate, base, and QuickJS-NG. The target fell
from 218.261 ms to 140.269 ms, or **0.642666x candidate/base**, clearing the
frozen `<= 0.90x` target; the external controls were `controlflow` 0.99590x,
`raytrace` 1.00742x, `A*` 1.00427x, `hash` 1.00467x, and `tagcloud` 0.98782x.

The complete-external fast decision retained the unit. The corresponding
promotion decision remains inconclusive solely because no exact project-wide,
zero-gap Test262 burndown exists; this is not a claim of global conformance or
of reaching the overall two-times-QuickJS-NG goal. It is an accepted general
cached-evaluation reduction under the existing runtime, curated Test262, and
QuickJS-NG comparison gates; the next unit must again start from a current
profiled, unclosed shared cost.

### 2026-07-28 rejected numeric-property `Math` unary leaf

The refreshed exact-current queue put JetStream class-field raytrace and
Kraken A* at ranks two and three by candidate/QuickJS-NG ratio. Their current
profiles shared a distinct unclosed cost: 1,947 raytrace samples and 7,362 A*
samples passed through `call_direct_leaf_function`; raytrace repeatedly calls
`Vector.magnitude()` with `Math.sqrt`, while A* repeatedly calls its Manhattan
heuristic with `Math.abs`. The frozen one-attempt unit
`numeric-property-math-unary-leaf` (plan SHA-256
`6ef7bc28e6bc8cb6363abd68d51a455b784811e36cfbab147d71bac37b200d71`)
therefore extended the existing numeric own-data property leaf to admit
straight-line numeric local temporaries and receiver-preserving unary `Math`
calls. It still declined dynamic realms, replaced/accessor `Math` methods,
accessors/proxies/non-number fields, captured locals, and all other calls.

The implementation's focused plan and observable-fallback tests passed, as
did all 1,902 runtime tests. It then failed the predeclared fast gate before
staging: the release candidate binary SHA-256
`d9f40b7dafa5724f8137ad55d8903d6fa2e392abad2adbac70d09b0fc554df6c`
was alternated with the runtime-identical `2159444f` base binary SHA-256
`de0b888d2ac6d7352959471a9defcdc16cafbc1aa72949a20b931e2a418b24df` on
the hash-verified upstream bundles. Class-field raytrace was only 2.37 s
candidate versus 2.44 s base, or **0.971311x candidate/base**, missing its
`<= 0.95x` target. More importantly, A* regressed from 9.45 s base to 10.79 s
candidate, or **1.141799x candidate/base**. The following raytrace pair was
again only 2.37 s versus 2.42 s, or 0.979339x. The first complete alternating
round was already decisive, so the remaining timing process was stopped; no
complete-external, broad, Test262, or promotion run was warranted.

The implementation, tests, and uncommitted plan were fully reverted before
any staging or push. Do not retry this direct-property `Math` route by
changing its local-register representation, cache position, or fallback
guards: per-call intrinsic revalidation and plan dispatch outweigh the child
VM avoided on A*, while raytrace misses materiality. A successor must start
from a different shared, current-profiled cost rather than widening this
admission path.

### 2026-07-28 rejected direct-leaf lazy loop-plan state

Fresh exact-current profiles of rank-one `controlflow-recursive` and rank-four
`hash-map` both showed `Vm::new_with_globals_upvalues_with_stack_and_direct_call_slots`
as a shared top-of-stack cost: 171 of 2,173 and 141 of 2,173 samples,
respectively. The frozen one-attempt unit
`tasks/performance-units/direct-leaf-lazy-loop-plan-state.json` (SHA-256
`d868a524a3c2fa1d13245c4b71fcb7a025b1634abffb4ddf620aaf8a7af41cf8`)
therefore classified bytecode with no backward unconditional jump and supplied
empty numeric, control, typed, and numeric-mutation loop-plan slices to its
VM frame and virtual-object lowering. Every body with a backward `Op::Jump`
retained the exact old plan route. Focused coverage proved that a no-backedge
body did not materialize any loop-plan `OnceCell`, and eight direct-leaf
receiver, argument, exception, and fallback tests passed.

The first target's two alternating exact-source timings rejected the unit
before it reached the second target. The candidate release binary SHA-256
`54b05426c8111b729562a7937eb06ed4698a42472d06f4e7b850baf3ee5ee313` was
compared with the exact current-main base SHA-256
`2b36815789e28f95d445728ba858913413452c6bcdc3744b4c5568b7a1751bcb` on a
100-times wrapper of the official `controlflow-recursive` source. Base times
were 6.407385 s and 6.413233 s; candidate times were 6.454553 s and
6.462816 s. Their medians are 6.410309 s and 6.458685 s, or
**1.007547x candidate/base**, rather than the frozen `<= 0.95x` target.

The implementation and focused test were reverted immediately. Do not retry
this by changing the cached backedge test, covering more plan kinds, or
delaying only another subset of plan lookups: the construction probes are a
real shared cost but not material enough for the stated general gate. A future
unit must remove a different current-profiled cost inside direct-call setup or
the execution core.

### 2026-07-28 rejected ASCII string literal span copy

The current queue's rank-five SunSpider string-tagcloud case had a diagnostic
profile with 1,194 of 2,173 top-of-stack samples in Lexer::string, plus 462 in
Lexer::advance and 374 in push_js_scalar. The frozen one-attempt unit
tasks/performance-units/ascii-string-literal-span-copy.json (plan SHA-256
96b0898780677e82a803c3150210f8ff60265d544565f6341871c7f7f7868b7c)
therefore copied only contiguous ASCII string spans, stopping before quotes,
backslashes, CR, LF, and non-ASCII source so existing escape, diagnostic, and
runtime WTF-16 paths retained all special cases. The similarly sized
string-unpack-code profile did not show this cost, so the plan deliberately
claimed one target rather than a general benchmark-family win.

The focused lexer suite passed all 65 tests, including an ASCII span around an
escape and Unicode scalar and a runtime-WTF-16 sentinel boundary. Candidate
and base also produced matching exit status and output on the hash-verified
upstream string-tagcloud, string-unpack-code, regexp-dna, and string-base64
sources. The release candidate binary SHA-256
5175f622ac02e361f56c8c641fdfcc2f3a487bee1840c19090dfb46b0db0d82f was
then alternated seven times with the exact-current runtime base SHA-256
2b36815789e28f95d445728ba858913413452c6bcdc3744b4c5568b7a1751bcb on the
unwrapped, official string-tagcloud source. Base times were 0.204893,
0.203606, 0.204366, 0.207729, 0.209302, 0.204618, and 0.207952 seconds;
candidate times were 0.211515, 0.205109, 0.206226, 0.206185, 0.205841,
0.203567, and 0.205915 seconds. The medians are 0.204893 and 0.205915
seconds, or **1.004988x candidate/base**, not the frozen <=0.85x target.

The profile's repeated qjs -i input was used only to locate lexical work and
did not establish material end-to-end benefit on the exact workload. The
implementation and focused tests were reverted before staging; no broad,
complete-external, Test262, or promotion run was warranted. Do not retry this
same contiguous-ASCII-copy route through a different byte scan, inlining, or
preallocation tweak. A successor must begin with a distinct current-profiled
cost whose exact-source timing supports a material general opportunity.

### 2026-07-28 accepted Number-only numeric direct-leaf program

The refreshed queue put SunSpider \`crypto-md5\` and \`crypto-sha1\` at ranks
14 and 19. Independent profiles of both exact-current workloads attributed
71/477 and 52/387 main-thread samples respectively to
\`try_eval_numeric_leaf\`, below direct calls to the same \`safe_add\` shape:
straight-line Number arguments, local temporary stores, and bitwise
arithmetic. The frozen one-attempt unit
\`tasks/performance-units/number-only-numeric-leaf-program.json\` (SHA-256
\`41ae87b208008cc96c2f5220439b5260b2713495f54d9cd8841ffc91eb20627a\`)
therefore precompiles only that general bytecode subset into scalar \`f64\`
locals and an operand stack. It never observes benchmark identities, source
paths, input values beyond the Number type, or iteration counts.

The compiler declines before observable work when any parameter is missing or
not a Number, an upvalue is received, or the body contains coercion,
comparison, boolean/update/stack manipulation, control flow, calls, property
access, or an unsupported local operation. The general direct-leaf executor
remains the fallback. The scalar binary helper matches the established Number
fast path for arithmetic, remainder, exponentiation, shifts, and bitwise
operations; focused coverage compares NaN, signed-zero, infinity, conversion,
and shift edge cases bit-for-bit and verifies both local-temporary admission
and non-Number fallback.

The release candidate SHA-256
\`38b7e4b85b195b0f00e142d1020b968556dc3c947f84a3b47603ec608d79a6f6\`
was compared with runtime-identical current-main base
\`2b36815789e28f95d445728ba858913413452c6bcdc3744b4c5568b7a1751bcb\`.
The complete three-block external report/raw SHA-256 values are
\`794519642a48ef43b2173957fa6f17b59bb69f4ca7e74063fb7068e671c00e48\` and
\`eb0d6576349f83097417d37c9d8a452f70d83700b7aec84323ce7d28c17cb6fb\`.
All 45 scheduled sources were exercised for candidate, base, and QuickJS-NG;
44 candidate/base rows were comparable. The two frozen targets improved from
38.713 ms to 33.138 ms (**0.855988x candidate/base**) for MD5 and from
36.050 ms to 31.690 ms (**0.879036x**) for SHA-1. Their remaining
candidate/QuickJS-NG ratios are **2.540928x** and **2.388396x**, so this is
not a claim that the final every-case \`<= 0.50x\` objective has been met.
The 44-row external candidate/base geometric mean is **0.981460x**; the
largest regression is \`string-unpack-code\` at **1.028481x**, below the
unit's 1.03x ceiling. Predeclared external controls range from 0.904092x
(A*) through 0.997786x (class-field raytrace), with hash-map at 0.993325x,
AES at 0.992278x, controlflow-recursive at 0.995826x, and the two SunSpider
controls at 0.988733x and 0.984448x.

The complete 25-case, three-block broad raw record has SHA-256
\`423235031b99462f9a8da47db5e688629a95c6ab415457dbf2c2638652be96d6\`.
Its candidate/base geometric mean is **0.996728x**, its largest individual
regression is \`closure_allocation_call\` at **1.020195x**, and the frozen
controls are \`dynamic_method_call\` 0.948492x,
\`function_call_two_args\` 0.998202x, \`plain_function_call\` 1.000794x,
and \`math_abs\` 1.002326x. This local broad raw record is explicitly
provenance-unverified because the candidate was built before its source commit;
it is conservative pre-merge regression evidence, not a claim-grade report.

\`cargo test -p qjs-runtime --lib bytecode::vm_numeric_leaf --no-fail-fast\`
passed all 9 focused tests, \`./scripts/test262-subset.sh\` passed all 5,160
selected cases, and \`./scripts/compare-qjs.sh\` passed. The complete
\`./scripts/check.sh\` gate also passed before the commit was pushed.
Promotion remains deliberately inconclusive until an exact candidate-commit
zero-gap Test262 burndown and a fully comparable external report are available.
This is an accepted general direct-leaf reduction under the current local
runtime, curated Test262, and QuickJS-NG comparison gates; the next unit must
again begin from a current shared-cost profile.

### 2026-07-28 Number-only leaf plan layout correction and exact revalidation

The preceding acceptance was invalidated by a fresh exact-current external
refresh: the original `8bf379e2` layout regressed Kraken `ai-astar` to
`1.114135x` candidate/base. Two structural corrections were then rejected
before promotion. Commit `20d00510` moved the scalar program into an
optional member of every `NumericLeafPlan`; this recovered A* but grew the
shared plan and regressed `array_allocation` to `1.091967x` (95% CI
`1.073655..1.094184`). Commit `dc020566` boxed the program in the
shortcut enum instead; it restored allocation to `1.002494x` but extended
the common shortcut dispatch and regressed `math_abs` to `1.041533x`
(95% CI `1.038962..1.042656`). Neither intermediary is a retained
performance result.

The retained layout is commit `f0b3ea878a7dc34bf8aa3ab0822d854ce316396f`.
It represents scalar leaves as the dedicated `NumericLeafPlan::NumberOnly`
variant, while ordinary leaves remain in the `General` payload and retain
the pre-existing `NumericLeafShortcut` variants. Thus no ordinary plan
carries optional scalar vectors and no ordinary shortcut executes a scalar
variant arm. The scalar executor is still a general bytecode-subset mechanism;
it accepts only Number arguments and falls back to the full VM before
observable work for coercive values.

The exact hosted recipe compared candidate binary SHA-256
`a8d6cf8aa94bef1f459627f41c0ee6d1a96aa68a884476f6a3f4e7f0c486f25f`
at `f0b3ea87` against base `0c31864b` (binary SHA-256
`a048b921e4402a7368e42e15649d3d200088b8347049af02962807c4cf0a3d0b`).
The three-block broad run has 225/225 valid records and a candidate/base
geometric mean of **0.996442x**; no case exceeds 1.03x. The two prior
holdouts are `math_abs` **0.997592x** (CI `0.983775..1.013537`) and
`array_allocation` **0.997826x** (CI `0.991410..1.004104`). Its raw
and report SHA-256 values are
`ec3eb61e9c7c34240bcdb1265f9057d98c3839868d438a85d07a47d3103270ea` and
`c4e0431a30a8c77c1fe5976c6c8f23bcb2f365bec15995b2b12ee38c70a3f8de`.

The matching external run reports 44 comparable candidate/base cases with a
**0.980431x** geometric mean and no regression above 1.03x; `ai-astar` is
**0.999048x**, `crypto-md5` **0.851911x**, and `crypto-sha1`
**0.835176x**. `kraken-1.1/imaging-gaussian-blur` is explicitly excluded:
both candidate and base hit the same 15-second capability timeout, while
QuickJS-NG completed, so it is not silently treated as a win or a comparison.
The external raw and report SHA-256 values are
`43c9ce92702cb5dbfe6153cc3e8539bdf190a12585cddbadc8570b2164ad9947` and
`ab3c0b8e69cb594d7c89e37a299b9830caf21598fadd0f8610d5a52a5f2fceca`.
This is informational same-host evidence, not a claim that the every-case
`<= 0.50x` QuickJS-NG objective is complete.

`cargo test -p qjs-runtime --lib` passed 1,903 tests, the staged
`check-touched` gate passed with its 65 affected Test262 cases, and the
full `./scripts/check.sh` and `./scripts/compare-qjs.sh` gates completed
for this retained commit. The next performance unit must be selected from the
refreshed external queue rather than extrapolated from this local improvement.

### 2026-07-28 rejected prepared RegExp atom metadata

The exact `af4a65a1` queue put `string-tagcloud` fourth and `regexp-dna`
seventh. Fresh profiles of both workloads showed repeated immutable matcher
work: DNA's collapsed stacks included `atom_end`, `atom_capture_indices`,
`simple_atom_matcher`, and `quantifier`, while tagcloud independently showed
the same chain under prepared native global replacement. The frozen one-attempt
plan `tasks/performance-units/prepared-regexp-atom-metadata.json` (plan SHA-256
`085aeb3b27acf4cd0be8294ba46ccbdaffe67e6a5cff1d30b3075430be49ae33`)
therefore cached only top-level simple class and escape atom metadata in a
prepared matcher. It retained the existing repetition-boundary vector and all
group, assertion, backreference, and generic fallback paths; this was distinct
from the earlier rejected boundary-streaming removal.

Focused matcher tests passed before timing. The final release candidate
(`635246fc70239e4dea7832f8945a96ae5186f6050706f68ebf228cb170024dc1`)
was compared with the exact `af4a65a1` base
(`d1cfd7429e2400704202b93f1b0bc514cd12cdcc30d6d096e54e84239c9b8a41`)
in a complete one-block external screen. `regexp-dna` improved to
**0.942225x candidate/base**, but the independent `string-tagcloud` target
reached only **0.974216x**, missing the frozen `<= 0.95x` dual-target gate.
Declared external controls stayed within the `<= 1.03x` ceiling: validate
input 1.011899x, base64 0.999849x, controlflow 0.991042x, A* 0.982583x,
and hash-map 1.009209x. The external raw/report SHA-256 values are
`b0617088bc72e84b8746284375291156dbc23fc5a6a2fb956045490fffb8f120` and
`c34486650bb95731fe889a56f503aabb37624a617f8c026f1a01239bba0ad2b0`.

The implementation and focused test were reverted immediately; broad
controls, Test262 promotion, and a commit were not warranted after the target
failure. Do not retry this same atom-metadata layout by changing cache breadth,
its storage representation, or the literal guard: it produces a real DNA gain
but not a material cross-workload RegExp win. A successor must remove a
different currently profiled shared regexp or string cost.

### 2026-07-28 rejected pure numeric call graph leaf

The refreshed exact `af4a65a1` queue ranked `kraken-1.1/imaging-darkroom`
eighth. A current sample attributed 3,496 of 3,662 main-thread stacks to the
repeated direct-leaf path through `FastGain`, `FastBias`, `FastLog2`, and pure
`Math` calls. The frozen one-attempt plan
`tasks/performance-units/pure-numeric-call-graph-leaf.json` (plan SHA-256
`6ded39882d940c75a7e5bf330413e16f1f52caa8324954d9e947026a401e55bc`)
therefore admitted only acyclic, prevalidated Number-only helper graphs with
forward branches, live lexical captures, and own-data pure `Math` intrinsics.
It rejected every coercive, dynamic-realm, accessor, loop, cycle, write, and
unsupported path before scalar execution. The cold graph payload was boxed so
ordinary `NumericLeafPlan` layout did not grow.

Focused tests covered both branch results against the ordinary VM, replacement
of `Math` intrinsics, and coercive-argument fallback; the complete local
Test262 subset passed all **5,160** cases. The final release candidate binary
SHA-256 `58b3a2b07511a3a0ed88e9653b1f32cc37b7a4ccbff6ad5983196829c37dd775`
was then compared with the exact `af4a65a1` base binary SHA-256
`d1cfd7429e2400704202b93f1b0bc514cd12cdcc30d6d096e54e84239c9b8a41` in a
complete one-block external screen. The target regressed to
**1.131718x candidate/base**, missing the frozen `<= 0.85x` gate; `ai-astar`
also reached 1.060260x, and `imaging-gaussian-blur` remained an explicit
capability timeout. The external raw/report SHA-256 values are
`b35fc0e8baa2fcb3e47d9044c04035ecf0ddfd2adad75b19357e93defe73d671` and
`b569987b97903b43c4d8193ea033af9c410867f8902b7f34ac693ace46ff0dd3`.

The implementation was reverted without a runtime commit. Do not retry this
per-invocation recursive graph preparation by changing its staging or storage
inside this unit: the evidence shows that avoiding child VMs does not pay for
the guards and preparation at this call depth. A future call-graph proposal
needs a distinct current profile and a new frozen plan.

### 2026-07-29 rejected exact direct-leaf tail-frame reset

The refreshed queue still ranked SunSpider `controlflow-recursive` first. Its
current profile placed 585 sampled stacks in `call_direct_leaf_function` and
570 in `Vm::new_with_globals_upvalues_with_stack_and_direct_call_slots`, so
the frozen one-attempt unit
`tasks/performance-units/direct-leaf-tail-frame-reset.json` targeted a
different cost from the prior tail probe: for an exact same-`Function` direct
leaf call in syntactic tail position, it re-seeded the active VM frame instead
of constructing the child `CallEnv`, `Vm`, and `FrameState` at all.

The candidate was deliberately narrow. It retained the ordinary path unless
the following opcode was `Return`, the callee identity exactly matched the
active direct leaf, no receiver normalization, dynamic realm, virtual value,
active try/finally/disposal scope, pending completion, or dynamic scope was
present, and the only discarded statement-completion operands were
`undefined`. On a hit it rebuilt parameter and hoisted slots, direct local
upvalues, binding masks, virtual-object selection, loop state, and literal
prototype caches exactly as a fresh direct frame would. Focused coverage
confirmed 2,048 nested self-tail calls and manual release checks returned
`100000` for deep recursion, `0:xxxx` through `try/finally`, `3` through a
receiver-sensitive method, and `37` after rebinding the recursive function.
The complete `./scripts/test262-subset.sh` passed all **5,160** curated cases.

The target fast screen nevertheless missed its predeclared `<= 0.90x`
candidate/base gate. The hash-verified 100-times upstream controlflow wrapper
(`52ecb05f622d41dd35db1d476f1f5c46d16e252efa72946516b5396c33f56261`)
ran in alternating pairs: base/candidate `6.54/6.23`, candidate/base
`6.14/6.53`, and base/candidate `6.53/6.15` seconds. Median candidate/base is
**0.941807x** (6.15s / 6.53s): a real 5.8% gain, but below the required 10%
threshold. The candidate and exact runtime-identical base binary SHA-256
values were `52acf6ff52d785d55352883c532ac10b80f9ccbd0b03821b5e425c00541ecc51`
and `d7600fa3516b718782a48e24b64f3337a79e39a62f5cc3c632d99eed5d8e54c3`.

The runtime implementation was reverted immediately; A*, hash-map, broad,
and complete external controls were intentionally not run after the target
fast gate failed. Do not retry direct-leaf tail-frame reset by broadening its
operand guard, changing its retained function handle, or reshaping its reset
sequence: eliminating child construction on this tail-only slice is still too
narrow. A successor must begin from a fresh shared-cost profile and remove a
different cost that also covers non-tail direct calls or another queue-ranked
workload.
### 2026-07-29 rejected capture-free ordered RegExp program after full screen

The current queue ranked `string-tagcloud` fourth and `regexp-dna` seventh.
Fresh profiles tied both to one different shared cost: Tagcloud placed 1,655
of 4,319 main-thread samples under `PreparedRegexp::match_input`, while DNA
placed 3,785 of 4,000 there. The frozen one-attempt unit
`tasks/performance-units/capture-free-regexp-program.json` (plan SHA-256
`f2de0fa42cc2be45f7bb484978ace127a8596468b412b220dd850c79f0efc527`)
therefore compiles the capture-free subset once into an ordered explicit
backtracking program. It predecodes literals, classes, escapes, anchors,
word boundaries, non-capturing groups, alternatives, and greedy or lazy
quantifiers, then reuses its choice stack across candidate starts. Captures,
named groups, backreferences, lookarounds, unsupported syntax, and nullable
unbounded loops decline before observable matching to the existing matcher.

The exact upstream sources passed on both binaries with matching `Null` and
`Undefined` output. Seven alternating direct-process samples per target
compared candidate binary SHA-256
`7c1c93f6f18dc037a4ab0603c57199f31c3ebacb2f55bdae6cd879bdf1c202a8`
against the runtime-identical base SHA-256
`d7600fa3516b718782a48e24b64f3337a79e39a62f5cc3c632d99eed5d8e54c3`.
The median `string-tagcloud` time was 181,433,416 ns versus 211,674,708 ns
(**0.857133x** candidate/base), and `regexp-dna` was 114,044,583 ns versus
440,470,917 ns (**0.258915x**). Both clear the frozen `<= 0.90x` fast gate.
The declared controls stayed below the `1.03x` ceiling: `string-validate-input`
0.986646x, `string-base64` 0.987798x, `controlflow-recursive` 0.998348x,
`ai-astar` 1.012124x, and `hash-map` 1.002831x.

Focused matcher coverage passed 44 tests (including direct comparison against
the unchanged generic fallback across the supported syntax), the full runtime library passed
1,911 tests, clippy passed with warnings denied, and the curated Test262
subset passed all 5,160 cases. A deterministic differential script covering
alternation priority, lazy repeats, Unicode, multiline anchors, global exec,
replace, captures, lookarounds, and nullable-loop fallback produced the same
1,836-byte output from candidate and base (SHA-256
`f028d864b2ca55911384d44003490d08bc2f20db1e0a2d75630685aa5c48b642`).

The retained fast screen was not sufficient for promotion. A complete
three-block broad portfolio produced 75 valid linearity checks; its
candidate/base geometric mean was 0.982189x, but its allocation family was
1.065956x. The complete three-block external manifest then preserved the
large intended wins -- `string-tagcloud` **0.846015x** and `regexp-dna`
**0.266699x** candidate/base -- but failed two frozen independent controls:
`string-base64` was **1.051479x** and `controlflow-recursive` was
**1.034165x**, both above the 1.03x ceiling. The other declared controls
were hash-map 0.992385x, A* 0.987877x, and string-validate-input 0.962097x.

The deterministic decision, bound to the exact source queue
`9a7ab99bc827c9b4d42a00baaf1d042f25324e30c91f0993fdf71078c63d3758`,
therefore returned `rejected`. Its preview-summary, broad-report, and
external-report SHA-256 values are
`6086b156294f1487df2525b562427fe4bff28b9c7336659081aaa7608402b83f`,
`1d3369f26f0e42dbd090233377ac6e3219d961a78d0c7b4b4a5314adb34ff6f9`,
and `a00ae1b4374e9a143f037eeb694352819369bdf447b9a72cc92208d35d1c36a8`.
The runtime implementation and its dedicated tests are reverted; the frozen
plan remains as negative evidence. Do not retry the same compiled-program
layout by broadening syntax admission, changing choice-stack storage, or
tuning the fallback boundary: the target wins do not offset its independently
measured control regressions. A successor needs a new profile of a different
shared RegExp or allocation cost.

### 2026-07-29 rejected typed-loop numeric-object-field scalarization

The exact `930eaeb8` opportunity queue ranked Kraken `ai-astar` second and
SunSpider `access-nbody` seventh. Their fresh current-binary profiles shared
named property work but not an already-retained implementation route: A* had
291 samples in `NamedPropertyCache::get` out of 2,391, while N-body had 202
out of 1,448. The frozen one-attempt unit
`tasks/performance-units/typed-loop-numeric-object-fields.json` (plan
SHA-256 `35ba5c7c4dfbbc8963289fd178f5d9a5cf614da52fcdee40a04e54d68a2a2451`)
therefore extended the existing typed-loop dataflow rather than a workload
special case: an unshared numeric named read stayed scalar, a scalar named
write avoided boxing, and a dense local Array element became a guarded boxed
ordinary receiver only when a later property operation needed one.

Focused typed-loop coverage passed 14 tests, including numeric field reads and
writes, Array `length`, dense object receivers, accessor fallback, and
read-only-write fallback. `cargo fmt --check`, clippy with warnings denied,
and the full runtime library passed (1,914 tests). The exact candidate release
binary SHA-256 was
`8e1079b7b288113d3d5a68ea4f2a8e78de4c1d39edce899e4347fccc026c78b3`; it
completed the unmodified upstream A* and N-body sources with their
`__QJS_EXTERNAL_OK__` markers, and its complete curated Test262 subset passed
all **5,160** cases when selected explicitly through `QJS_CLI_BIN`.

The frozen fast gate required both targets to be at most `0.90x`
candidate/base. Three-block alternating external measurement against the
runtime-identical base binary SHA-256
`95e83d949426a239d9af53b9da84fb8d9bff73a12be5712369f85de0d6e03450` gave
N-body **0.583367x**, but A* **1.154586x**: a 15.5% regression. This fails
the unit even though every predeclared external control was within the 1.03x
ceiling: CDJS 0.954982x, hash-map 0.978916x, public-field raytrace 0.965685x,
3d-raytrace 0.971948x, and controlflow-recursive 0.932408x. The external
report SHA-256 is
`740400b383cbc2db32e95ed07f0dbd47a9181ce2b992b1412ec932c37c602620`.

The five declared broad controls were also sampled before the rejection was
recorded: property read 1.000869x, property write 0.999434x, dynamic array
read 1.001667x, object allocation 0.998540x, and plain function call
0.998475x. Their raw JSONL SHA-256 is
`2de160cd7ae7f9c8fafb65129afa16b7e64d2010dc21b5a2568246ae6c52cc1d`.
The report tool correctly refuses to represent that five-case sample as a
complete 25-case portfolio, and the target failure means the full promotion
bundle was intentionally not run.

The runtime implementation and its dedicated tests were reverted immediately;
only the frozen plan and this negative-evidence record remain. Do not retry
this scalarization by changing its cache ordering, boxed-operation threshold,
or property guard: one independent target gets a large win, but the other
queue-ranked target regresses materially. A future typed-loop proposal needs a
new shared-cost profile and a different mechanism that improves both object
field workloads before it is attempted.

### 2026-07-29 rejected compact generic bytecode core

The fresh exact-`930eaeb8` queue ranked JetStream `controlflow-recursive`,
Kraken `ai-astar`, and SunSpider `hash-map` first through third, with
candidate/QuickJS-NG ratios of 6.607650x, 5.566090x, and 5.455860x. Their
current profiles put the execution dispatcher, virtual-object selection, and
operand-stack handling on every measured path. The frozen one-attempt unit
`tasks/performance-units/compact-generic-bytecode-core.json` (plan SHA-256
`e678b7f5f0eb13a12d1f28425d3f9ae9b3614d49c0d544e45fbb6a8096eae4d3`)
therefore lowered the complete ordinary synchronous bytecode subset into a
generic dense `CompactProgram`, retaining the existing VM helpers and falling
back before execution for generators, dynamic scope, modules, and unsupported
opcodes. It also cached an equivalent compact stream per virtual-object
variant; it contained no workload names or benchmark-specific branches.

Focused compact-core coverage passed four tests, including control flow,
property and accessor effects, and captured `with` scope fallback. The virtual
number lowering regression test and the direct `with`-scope fallback test
passed. `cargo fmt --all --check` passed; the full runtime library passed
1,915 tests during the implementation; and the final exact implementation
passed all **5,160** curated Test262 subset cases.

The clean three-block external run used candidate release binary SHA-256
`79556b2c9074ceb8b5ec188a3f941f5afc58bd75dbe68e1ac36922c6d11da` against
the runtime-identical base SHA-256
`95e83d949426a239d9af53b9da84fb8d9bff73a12be5712369f85de0d6e03450`.
Every frozen `<= 0.90x` target gate failed: controlflow-recursive was
**0.994665x** (77.482 ms / 77.898 ms), A* **0.997122x**
(9,643.316 ms / 9,671.152 ms), and hash-map **1.124792x**
(1,951.761 ms / 1,735.220 ms). Hash-map regressed 12.5%. The independent
external controls were within the 1.03x ceiling (public-field raytrace
0.997750x, CDJS 1.002822x, and string-tagcloud 0.995898x), but that does not
rescue a mechanism which makes none of its shared queue targets materially
faster. The complete external report SHA-256 is
`5c7afcc5db8a16ff68d0ee8ab5203b82a453261c1084cc64935daf2165fb3711`; its
raw measurements SHA-256 is
`b525d16ee55b926a87a331d9dad93fb782e528e41d1cf868942284bae2eee7ce`.

The declared broad controls gave the decisive generality failure: median
candidate/base was plain function call 1.000308x, property read 0.939213x,
object allocation 0.938541x, but dynamic method call **1.621115x**
(507.586 ms / 313.109 ms). This is far above the frozen 1.03x ceiling. The
partial four-case sample is intentionally not a portfolio result; the report
tool correctly rejected it as incomplete. Its raw JSONL SHA-256 is
`cf9b638cd4feb731575ba769e68992466b9fd5c4bc3de17f20d4cea97e1e56f9`.

The runtime implementation was reverted immediately; only the frozen plan and
this negative-evidence record remain. Do not retry this compact core by
changing opcode layout, compact-cache placement, virtual variant caching,
instruction width, or eligibility ordering: all three independent targets
missed and a broad dynamic-call control regressed catastrophically. A future
proposal must start with a fresh profile of a different shared cost, likely
property/value representation or allocation/GC work outside the generic
dispatcher.

### 2026-07-29 rejected constructor transition-shape object storage

The exact `50a20d60` queue ranked Kraken `ai-astar`, JetStream `hash-map`,
and public-field raytrace second through fourth. Fresh profiles connected all
three to repeated ordinary receiver fields: A* constructs five `GraphNode`
fields and later adds four more, hash-map constructs fixed-field entries at
volume, and raytrace repeats public-field and constructor writes. The frozen
one-attempt unit
`tasks/performance-units/transition-shape-object-storage.json` (plan SHA-256
`87b3de88e4aa15c3d079e8bca9e6997a0440f44cc6132f12aafbdd6cc52bc3a4`)
therefore learned a per-constructor field sequence, reused shared compact
slots, and added bounded shape transitions for matching later own fields.
All divergence, deletion, descriptor, accessor, exotic, Proxy, and
non-matching-write paths retained their existing storage route; no workload
names or benchmark-specific conditions were used.

The prototype passed its focused shape, mismatch, cache-read/write, accessor
fallback, and end-to-end constructor tests; the complete runtime library also
passed all 1,917 tests. The clean release candidate binary SHA-256
`088a02cdafcf808912b75ede0474b3a791ebdfe27ef71e0bed8bd4a4793b4ee2` was
measured against a binary rebuilt from exact `50a20d60`, SHA-256
`7c1c93f6f18dc037a4ab0603c57199f31c3ebacb2f55bdae6cd879bdf1c202a8`.

The three-block full external-manifest run produced a strong A* result,
**0.849010x** (8,306.457 ms / 9,783.698 ms), but missed the frozen
`<= 0.95x` gate on both independent co-targets: hash-map was **0.963739x**
(1,592.674 ms / 1,652.599 ms) and public-field raytrace was **0.961343x**
(1,715.395 ms / 1,784.373 ms). Every declared external control stayed below
the `1.03x` regression ceiling: CDJS 0.981849x, controlflow-recursive
0.998482x, and string-tagcloud 0.975669x. The report SHA-256 is
`798c580cc15fc50c06231d0320b2f36fba3985fdf7a39602f1eb0128202ca659`; raw
measurements SHA-256 is
`38ba746a4d18f7338bbb6d7d3ceb78df8b1a557b67aa9efc4aee6c4784e6ead7`.

This fails the frozen one-attempt target gate despite a real A* win. The
runtime implementation and its dedicated tests were reverted immediately;
the full broad portfolio and Test262 promotion bundle were intentionally not
run after rejection. Do not retry this mechanism by tuning transition depth,
shape-cache ordering, or spare slot capacity: it has not shown the required
cross-workload benefit. A successor should start from a fresh profile of a
different shared allocation or Value-lifecycle cost.

### 2026-07-29 rejected owned Number binary operands

Fresh exact-source profiles of A* and hash-map exposed ordinary Number binary
bytecode beside `Value::drop`: A* had 436 `eval_binary` and 221 `Value::drop`
top frames out of 3,636 samples, while hash-map had 66 `eval_binary` plus 45
each of `Value::clone` and `Value::drop` out of 1,102. The frozen one-attempt
unit `tasks/performance-units/owned-number-binary-operands.json` (plan
SHA-256 `57f3342cd35a019c7871612e51afdae17675081be4f62bb885e515796e234376`)
therefore moved two already-popped Number operands out of their `Value` enums
before calling the existing Number helper, reconstructing both values for an
unsupported operator before the unchanged generic path. Selection depended
only on live operand tags and the current `BinaryOp`; all coercive, object,
string, BigInt, Proxy, and unsupported-operator paths retained their existing
route.

The prototype passed its focused bytecode test, including a two-Number `in`
exception fallback, and the complete runtime library passed all **1,912**
tests. `cargo fmt --check` passed. The release candidate SHA-256
`6c9837794e60def86d5a1f5df1d585aa6966bf7d877cefe949093cdb5e6d7254` was
measured against the runtime-identical exact-`50a20d60` base SHA-256
`7c1c93f6f18dc037a4ab0603c57199f31c3ebacb2f55bdae6cd879bdf1c202a8`.

The frozen fast gate used a three-block, seeded role-rotation manifest with
the two target cases and three independent controls; its exact source-file
hash subset manifest SHA-256 is
`222e0f5ab5da114f2eeb57d4babd37caca1e9d5d64e244bc6e7cef999d1d7020`.
Both required `<= 0.95x` targets missed: A* was **0.974437x**
(9,822.494 ms / 10,080.174 ms) and hash-map was **0.988694x**
(1,622.495 ms / 1,641.048 ms). The controls stayed below the 1.03x ceiling
but did not change the decision: public-field raytrace was 0.997401x,
controlflow-recursive 0.985907x, and string-tagcloud 1.006982x. The raw
measurements SHA-256 is
`a286f3a78e35b8e7ce0167196a6eed1203f08b2534f1981d38edfdca94ff1d35`; the
report SHA-256 is
`16635c0afe0342e08d97b106c257d240843e6402c7710b7442e896a57dd16c02`.

The runtime implementation and its dedicated fallback test were reverted
immediately. The complete external manifest, broad portfolio, and Test262
promotion bundle were intentionally not run after this fast-gate rejection.
Do not retry by rearranging the borrowed match, inlining the existing helper,
or changing fallback reconstruction: the shared improvement is materially
below the declared target in both independent workloads. A successor must
profile a different shared allocation or value-lifecycle cost first.

### 2026-07-29 rejected cache-learned dynamic property layout

The exact `fe8d287e` queue ranked Kraken A* second and its fresh profile
showed `NamedPropertyCache::get` and `NamedPropertyCache::update` in 469 and
398 of 8,613 samples. A one-attempt storage proposal therefore learned the
first matching dynamic ordinary-property table and converted later matching
tables to the existing shaped representation. It selected only by live
ordinary-property keys and descriptors; it did not inspect constructors,
source text, bytecode identity, or benchmark names. All mismatches,
deletions, descriptor changes, accessors, exotic objects, and Proxy paths
kept their existing route.

Focused property-storage and cache tests passed, but direct release A* pairs
missed the frozen `<= 0.95x` target gate: the candidate took 9.86 s and
9.92 s against 9.93 s and 9.76 s for the exact base (0.993x and 1.016x).
The implementation and its plan were fully withdrawn before broad or
conformance promotion. Do not retry dynamic-table promotion, cache-learned
layouts, or first-observation conversion variants: their conversion cost
erased the proposed lookup benefit. A future property-storage unit must use a
different mechanism that avoids per-object representation conversion at cache
observation.

### 2026-07-29 retained exact-one simple-RegExp atom continuation

The exact `32856150` queue ranked SunSpider `regexp-dna` seventh, but a
fresh source-matched profile found a distinct shared matcher cost: 3,160 of
3,614 main-thread samples entered `PreparedRegexp::match_input`, including
407 `RawVec::grow_one` samples below `simple_atom_boundaries`. The frozen
one-attempt unit `tasks/performance-units/exact-simple-regexp-atom.json`
(plan SHA-256 `d1bd54e6dc3b4afc81d6f4df8304798059f57dd9bc2fff00ef9befb3723c402b`)
therefore advances an already-classified, capture-free `SimpleAtom` directly
when its parsed quantifier is exactly `{1}`. This avoids materializing the
two-entry repetition-boundary vector. It depends only on existing matcher
classification and quantifier semantics; captures, groups, backreferences,
lookarounds, quantified atoms, and every unclassified atom retain the old
path.

The focused matcher suite passed 43 tests, including capture preservation and
Unicode surrogate-pair advancement; the full runtime suite passed 1,911
tests, and the curated Test262 subset passed all 5,160 cases. `check.sh`, the
staged touched gate, and the pre-push full gate all passed. The candidate is
commit `7d0f6bdbb2e5065155f2562bd7c790617b02abfb` (binary SHA-256
`0894bd6195019d8392af854399d3a16c86b65079e1487939740cbefe91a9f3f1`),
measured against its exact parent `3285615027c056f00d04d3cd51e75e7708dc1b8b`
(binary SHA-256
`188a2da162bac5be55bc20d15a57259bab6f071138085d5e83db86a0faa22d1e`).

The first complete preview used older `fe8d287e` as a diagnostic base and is
not used for this decision. The exact-parent three-block preview has summary,
broad-report, and external-report SHA-256 values
`42400073fcc2ca8bcb6f5591d1ac6807196c5e38b3aa4869a75f9723b8783287`,
`107722273fe22710f6ea9727ae4b3a57074bce41986901a9feed226823d58ca0`, and
`c7592adf4cda455bb1313f7f49a8fdd1980bef19ff9236e10123adb8db942f65`.
It retained the target at **0.474420x** candidate/base (206.218 ms versus
434.674 ms) and kept all declared controls below 1.03x: string-tagcloud
0.933701x, string-validate-input 0.999955x, controlflow-recursive 1.002294x,
A* 1.000419x, and hash-map 1.001336x. The full broad portfolio contained all
25 cases and had a 0.974785x candidate/base geometric mean; it is context,
not a substitute for the frozen target/control gate.

The hash-bound fast decision is **retained** (SHA-256
`46e33921493819f730660812532c9a08451fca15b3d6df3d6a314a3e35c7c105`).
The stricter promotion decision is intentionally **inconclusive** (SHA-256
`c42ab2dca35f405824860e6efda918feae2fbb962a1fdb241567f82e8bc55ffc`):
Kraken `imaging-gaussian-blur` lacks complete candidate/base and QuickJS-NG
comparison, and no exact all-suite Test262 burndown is attached. This is a
retained local performance unit, not a fixed-hardware or full-conformance
promotion claim. Subsequent work must start from a new queue and fresh shared
cost profile rather than retuning this exact-one route.

### 2026-07-29 rejected first-continuation generic-RegExp repetition

The exact `7d0f6bdb` opportunity queue ranked SunSpider `string-tagcloud`
fifth and `string-validate-input` eleventh. Fresh source-matched samples tied
them to a distinct generic matcher cost: Tagcloud put 1,014 of 3,576
main-thread samples under `PreparedRegexp::match_input`, including
`match_pattern_first` (279), `repeat_atom` (240), and `Vec::from_iter` (221).
The independent validate-input profile put 792 of 2,556 samples under
`match_pattern_first`, including `repeat_atom` (176), `Vec::from_iter` (157),
and `RawVecInner::finish_grow` (126). The frozen one-attempt unit
`tasks/performance-units/regexp-first-continuation-repetition.json` (plan
SHA-256 `a9917e1509d5a71df335c8e4b46b7ced3dcfff301e7d72983875443c9ee838b0`)
therefore kept the existing generic repetition DFS but offered each accept
state directly to `match_pattern_first`'s continuation, stopping after its
first success rather than collecting all accepted states first. It did not
alter pattern parsing, the all-state/reverse matchers, simple-atom handling,
or any source-specific condition.

The candidate passed the new greedy/lazy/capture-clear focused matcher test,
the complete 44-test matcher suite, and `qjs-runtime` clippy with warnings
denied. Its isolated release binary SHA-256 was
`133e8cbd52b60def2bce85d5bc4a0f919e76fbfb20eb43aadb3649cd1ad2b87f`; the
runtime-identical `7d0f6bdb` baseline was
`1e02e6bf0b9dfec25b6a9acea1de5335ac42a6748119bc354a9e3981baeac204`.

The frozen seven-block alternating direct-process fast screen used unmodified
official sources and byte-equal candidate/base stdout and stderr. Its raw
receipt SHA-256 is
`15f7456ebd3dd3c5f529a4af623b11dcc9f76179622dbfce418acf4028f276f5`.
The paired median candidate/base ratios were only **0.992507x** for Tagcloud
(196.275 ms versus 197.328 ms) and **0.992173x** for validate-input
(74.187 ms versus 74.776 ms), both far above the frozen `<= 0.90x` target
gate. The first Tagcloud candidate process was a 5.13x cold-start outlier,
but excluding neither it nor any other declared block changes the decisive
insufficiency of the median result.

The runtime implementation and dedicated test were reverted immediately;
the full external manifest, broad portfolio, and Test262 promotion bundle
were intentionally not run after the fast-gate failure. Do not retry this
mechanism by changing callback placement, visitor shape, minimum-repeat
handling, or the collection boundary: the direct early-exit form itself has
shown less than one percent on both independently profiled targets. A future
RegExp unit must begin with a fresh profile of a different cost.

### 2026-07-29 rejected direct-slot ordinary captured closures

The exact `7d0f6bdb` queue ranked JetStream CDJS sixth. Its fresh profile put
1,151 of 1,169 main-thread samples under the recursive generic
`call_callee_with_marker -> call_function -> eval_function_bytecode` chain.
`reduce_collision_set.js` creates ordinary nested functions
`putIntoMap`, `isInVoxel`, and `recurse` within `drawMotionOnVoxelMap`; those
functions capture the parent frame's slot-backed values. The frozen
one-attempt unit `tasks/performance-units/direct-slot-capture-closures.json`
(plan SHA-256
`5c91faf10b8d942cbe3d2d70ba3f415fad76eecbb93fb0e9c8cdb848fc8cb7`)
therefore allowed non-lexical-`this` function literals through the direct-slot
eligibility check and lazily grew the local-upvalue table on first capture.
Classes, lexical-`this`, arguments, direct eval, `with`, `super`, named
function bindings, outer generator/async calls, and dynamic bindings kept
their prior compatibility route.

The candidate did activate the intended route: its CDJS sample (SHA-256
`295d009d5bba58619190859abca1fd88756b0b9affe5d7b592c2bea46e8c6765`)
shows the formerly generic hot call chain as direct-leaf calls. Focused shared
capture, escaping closure, and per-iteration lexical tests passed; the full
`qjs-runtime` suite passed all **1,911** tests and the curated Test262 subset
passed all **5,160** cases. The isolated release candidate SHA-256 was
`b61ad0effe127af28ad23069af89ecf1f2606cd4fe5611deb02679a72c1c3c34`,
against the runtime-identical exact-`7d0f6bdb` base SHA-256
`1e02e6bf0b9dfec25b6a9acea1de5335ac42a6748119bc354a9e3981baeac204`.

The source-matched generated CDJS bundle SHA-256
`8fdde63549dd457d375730cca98efc1be687c3fa32b84e5d63a9903b768fed47`
was then run in seven direct-process base/candidate pairs. Reported base/candidate
wall-clock pairs in seconds were `(2.04, 2.31)`, `(2.08, 1.56)`,
`(1.57, 1.53)`, `(1.55, 1.53)`, `(1.55, 1.53)`, `(1.56, 1.51)`, and
`(1.55, 1.53)`. Their paired median candidate/base ratio is approximately
**0.987x**, far above the frozen `<= 0.95x` target gate; even the steady
pairs show only one to three percent. The full external and broad promotion
portfolio was not run because this fast gate already failed.

The runtime implementation was reverted immediately. Do not retry by changing
lazy-table growth, broadening closure categories, or rearranging the direct
call boundary: this general direct-route conversion removed the profiled chain
but did not produce the required end-to-end CDJS improvement. A successor must
start from a fresh profile of a different shared cost.

### 2026-07-29 retained typed-loop register-file compaction

The exact `7d0f6bdb` queue ranked SunSpider `bitops-bits-in-byte` and
`math-cordic` as independently profiled typed-loop opportunities. Both profiles
showed time under the typed-loop register-file lifecycle, notably boxed
`Value`-vector initialization and clearing. The frozen one-attempt unit
`tasks/performance-units/typed-loop-register-file-compaction.json` (plan
SHA-256 `ed7e97efc6a212e8330cdd067cf1228c85d5f40adf9d7bdaec0eaeae5490ba23`)
therefore remaps each lowered typed-loop program's persistent scalar and boxed
registers immediately after the actually used stack-register range. It visits
every typed operation and site-entry register pair; local/global/constant
metadata and written boxed locals receive the same remap. This is a program
representation compaction based only on the lowered program's register use,
not on source text, workload identity, or loop values.

The candidate is commit `46ceb4f6dcb65083cecd63b5227a81c6b417a916`, measured
against the exact `7d0f6bdbb2e5065155f2562bd7c790617b02abfb` base. Its new
typed-loop test verifies that a pure scalar loop has no boxed register file,
that a dense-array loop retains only its real boxed depth, and that execution
semantics are unchanged. Before the formal run, formatting, Clippy with
warnings denied, the 1,912-test runtime library, and the 5,160-case curated
Test262 subset passed.

The source- and binary-receipted three-block preview retained both frozen
targets: `bitops-bits-in-byte` was **0.839532x** candidate/base and
`math-cordic` was **0.927189x**. All five external controls stayed below the
`1.03x` ceiling: `bitops-3bit-bits-in-byte` 0.985684x,
`bitops-nsieve-bits` 1.001397x, `access-nsieve` 0.991247x,
`math-spectral-norm` 0.998721x, and `crypto-sha1` 1.001257x. The broad
controls likewise held: `array_write` 0.998338x,
`plain_function_call` 1.001180x, and `object_allocation` 1.006151x.
The fast decision is **retained**; its SHA-256 is
`48002d278801ae282bff429e3f8b751ab7d8023b7da14a8ea52f7dbb16b1d696`.
Its bound summary, broad-report, external-report, and external-raw SHA-256
values are `35c1b54a5f874800ce73904423f8e800f25de588ea43c26b7c6d1775726c0ffd`,
`4afee6efea5e166b4bf74b2f4f59b8def07392d8ee35b67be005b9eb1f922ea2`,
`6c0bb7792546d7f6afbc81ee3c2cb2c36487990354e86acf886b835da5071cdf`, and
`7f1f8bf179cdba2b63d91721b10c3a0e3ac335b61a62056d3fb67184183d6c1a`.

This is a retained local performance unit, not a fixed-hardware or
full-conformance promotion claim: Kraken `imaging-gaussian-blur` still lacks a
complete candidate/base comparison, and no exact all-suite Test262 burndown is
attached. Future work must use a fresh queue and profile a different shared
cost rather than tune this compaction mechanism.

### 2026-07-30 rejected native-callback direct-leaf bridge

The current `32a00b0e` queue ranked SunSpider `string-tagcloud` fourth. Its
fresh runtime-identical `bb53c9d6` profile put 263 of 2,196 main-thread
samples in the generic `call_function` bridge beneath native Array sorting,
before 158 samples entered `eval_direct_call_bytecode`. The one-attempt plan
`tasks/performance-units/native-callback-direct-leaf-bridge.json` (SHA-256
`94acdc5284d5398dbd848ffd56673dabb5016529069e36c24644a100841639b2`)
therefore routed only a non-constructor callee that already satisfied the
existing direct-leaf predicate from `call_function` to
`call_direct_leaf_function`. It did not inspect a builtin, source, function
identity, values, or benchmark inputs; every ineligible, constructor, class,
async, generator, eval, closure-capturing, or dynamic call retained the
ordinary route.

The prototype's focused callback tests preserved `this`, received upvalues,
and `arguments` fallback semantics. The full runtime suite passed all
**1,916** tests and the curated Test262 subset passed all **5,160** cases.
The isolated candidate release SHA-256 was
`f6b8be512c7cd9031588ac6a1d9aa54e70c0eaa6fbea78c1ca295a82b6b0d158`,
against the runtime-identical pre-prototype binary SHA-256
`568549b735c590a0787d93a84f67d2f3c65312f917e863714a1bb28e6f757ad4`.

The hash-verified upstream fast screen used three seeded role-rotated blocks
over Tagcloud plus HashMap and A* controls. Its temporary three-case manifest,
raw receipt, and report SHA-256 values are
`ae0fbce8aeae58ba57029504a23cb8ac1801b266dffebc69c4ba4c8294987c3c`,
`ecb93792f3842d2308baf60a34eef721c1e5062161132a6537a576770f0a5820`, and
`513bb644bf00aaeecd3dceddb6d96da2e98000e7491261466694cdc7b1ffef47`.
The frozen Tagcloud `<= 0.95x` gate failed outright at **1.010180x**
candidate/base. The two unrelated controls were similarly neutral: HashMap
was 1.002718x and A* was 0.997648x. This incomplete fast screen is not a
promotion result, but the target regression decisively closes the sole
attempt; the complete external and broad portfolios were not run.

The runtime change and its tests were reverted immediately. Do not retry by
inlining the bridge, reshuffling the leaf probes, or broadening eligibility:
the existing generic path already seeds direct slots, and removing its
compatibility wrapper did not yield a material end-to-end gain. A successor
must profile a different shared cost rather than another native-callback call
boundary variant.

### 2026-07-30 rejected direct-construction realm-context alias

The current `32a00b0e` queue ranks JetStream `hash-map` second and Kraken
`ai-astar` third. Their fresh MallocStackLogging receipts record 23,760 and
10,000 ordinary receiver allocations respectively through
`Vm::construct_callee -> construct_function`, so the frozen unit
`tasks/performance-units/direct-construction-realm-context.json` (SHA-256
`fb4f53b6db7d23361bcd66be5ad2514c93c5c12f35771bcd9bd2fca20e92e9b1`)
initially proposed replacing the caller compatibility environment for an
already direct-leaf ordinary constructor with a realm-only environment.

The proposal was rejected during source audit before any runtime code or
candidate binary existed. `Vm::call_env` takes the user-bytecode path through
`self.attach_host(self.env.new_function_frame())`; `Vm::realm_env` takes
`self.attach_host(self.env.empty_frame())`; and `CallEnv::empty_frame()` is
exactly a `new_function_frame()` alias. The proposed substitution therefore
removes neither an allocation nor metadata work. Adding a direct-leaf branch
would only add a predicate and a duplicate result path around identical frame
construction.

The two profiles still establish that object construction is frequent, but do
not show a distinct parent-`CallEnv` cost. No source patch, focused test,
candidate binary, or fast-gate run was appropriate. Do not retry this as a
realm-context, caller-shell, or `empty_frame` versus `new_function_frame`
variant; a future construction unit must profile a different cost beneath
`construct_function` or receiver/property allocation itself.

### 2026-07-30 rejected ordinary bytecode value-only exit

The current `32a00b0e` profiles for SunSpider `cdjs` and
`date-format-tofte` both attributed the dominant non-direct function stacks to
`call_function -> eval_function_bytecode`. The frozen one-attempt unit
`tasks/performance-units/ordinary-bytecode-value-only-exit.json` (SHA-256
`c50af5c78cd2629c12e4f088688e7f3697ae6296396945d68143528fbcf8acad`)
therefore tested whether a synchronous, non-derived ordinary function could
return its completion `Value` directly, leaving the richer
`FunctionBytecodeResult` solely for derived construction. It did not select by
function identity, source, workload, value, or call depth.

Focused return/throw-identity and derived-constructor tests passed, along with
the affected runtime test groups, `cargo check -p qjs-runtime`, and Clippy
with warnings denied. The isolated candidate binary SHA-256 was
`a7761053866158a52fad0353f276c9ec53d862ca059b40e4e151cbefb192b535`, against
the runtime-identical base SHA-256
`568549b735c590a0787d93a84f67d2f3c65312f917e863714a1bb28e6f757ad4`.

The hash-verified three-block external fast screen's manifest, raw receipt,
and report SHA-256 values were
`12698a36cb9648fd3440d834621f3c466a070ef1fca34be0713580b190e60946`,
`c90d4cfbf8a0997806c49ef8f6430a19f0bd244e8ce0257d8f838d8b754a0f27`, and
`b8b0790de1ae322b2b000b27fc1a4ffb4c87f427a8ccb3893467005097e23175`.
The frozen `cdjs <= 0.95x` target gate failed at **0.997776x**
candidate/base. Controls were neutral: HashMap 0.994082x, raytrace 0.991145x,
A* 1.001217x, nbody 0.989133x, controlflow-recursive 0.994493x, and Tagcloud
1.006328x. Since the first target failed and the unit permits one attempt,
the second target was not run and the complete external, broad, and Test262
promotion work was intentionally skipped.

The runtime patch and focused tests were reverted immediately. Do not retry
this by splitting, borrowing, or inlining `FunctionBytecodeResult`: avoiding
that final aggregate move is not a material end-to-end lever. A successor
must remove a separately profiled shared generic-call cost, such as a proven
safe call-frame or scheduler cost, under a fresh plan and profile.

### 2026-07-30 rejected entry-prepared numeric helper graph

The exact `14d9f2f6` queue ranked Kraken `imaging-darkroom` fourth. A fresh
sample placed 3,768 of 4,011 main-thread stacks in generic
`Vm::run_completion` below `ProcessImageData`, including repeated direct-leaf
frames through `FastGain`, `FastBias`, `FastLog2`, `Clamp`, and pure `Math`
intrinsics. The frozen one-attempt plan
`tasks/performance-units/typed-loop-entry-prepared-numeric-helper-graph.json`
(SHA-256
`be987cbde9321722df000e4922e8a8ea33393366275add86d87572c4ba6754ca`)
therefore tested a mechanism distinct from the rejected per-invocation graph:
the outer typed loop prepared and guarded the complete bounded acyclic helper
graph once at entry, then evaluated only scalar graph instructions during the
pixel iterations.

Focused coverage proved 14 native graph calls across the seven iterations
after backedge discovery, exact results against an ordinary callback loop,
and zero graph calls with ordinary side effects when either a helper binding or
`Math.log` was replaced. All 15 typed-loop tests passed. The candidate release
SHA-256 was
`e50d7fa7e16cb4ec5169c49626fe7d8549fd385719bf49e13451d214e9bad5f4`,
against exact-base binary SHA-256
`919c9cad198c2dcf3a50317e13997743756a084a51394097f3542b9b832fa5cb`.

The complete five-block, seeded role-rotated fast screen used nine
hash-verified upstream cases. Imaging improved from 5,421.789 ms to 1,348.788
ms, or **0.248772x candidate/base**, and reached **0.850201x QuickJS-NG**.
Eight declared controls were otherwise neutral: HashMap 0.992363x, raytrace
0.992486x, A* 1.011120x, audio FFT 1.017029x, nbody 1.002863x,
controlflow-recursive 1.013209x, and Tagcloud 1.007847x. However, the protected
`3d-morph` control regressed from 25.107 ms to 31.004 ms, or
**1.234879x candidate/base**, decisively exceeding the frozen `<= 1.03x`
control ceiling. The manifest, raw receipt, and report SHA-256 values are
`c36e49ab449cfc5292f6a0d5fadc4117614f2918bda434833eb3622f5f446f5e`,
`b6d6b75689740d94436628f725acda25b4f5582500c10a50beacf097a8ef0278`,
and `9c6297c1f67a5b63de2b837ff24fbf4748810389da654d37057b567b3b18ee2d`.

The eager typed-loop integration and focused tests were reverted immediately;
the decisive control failure made broad and Test262 promotion work
unwarranted. Do not retry by changing graph limits, admitted operators, or
benchmark-facing call shapes. A successor may revisit entry preparation only
after a fresh profile explains the independent `3d-morph` regression and with
a design that isolates all helper-graph state and dispatch from typed loops
that do not use it.

The post-revert binary comparison supplied that missing profile. In paired
five-second samples of the same diagnostic-only expanded 3d-morph source, the
candidate put 870 top-of-stack samples in an out-of-line
`typed_loop::execute::typed_binary` call, while the exact base kept the same
work inside `typed_loop::execute::run` and had no corresponding out-of-line
symbol. Merely widening `typed_binary` from private to `pub(super)` for graph
reuse changed the optimizer's inlining decision and imposed a call on every
existing typed numeric operation; 3d-morph never entered a helper graph. The
candidate/base sample SHA-256 values are
`58261c4780bc37e77219911a6a7d4d05e2a1ab35b24f05b6da11319836f0abbb`
and `82d3a3922ff0ad3b06b1f763573274e5437d2a12b30d4d98b3cecdc7395f15a8`.
A distinct successor must preserve that hot inlining boundary and keep graph
evaluation out of line before it may retest entry preparation.

### 2026-07-30 rejected isolated entry-prepared numeric helper graph

The successor unit
`tasks/performance-units/typed-loop-isolated-prepared-numeric-helper-graph.json`
(plan SHA-256
`739c05040dfa2ad6aa4716e46a84142830fdccc08e49b247fdd4c2b55d88efcc`)
preserved the existing private `typed_binary` implementation byte-for-byte,
placed graph arithmetic in a separate out-of-line module, allocated graph
metadata only for programs containing graph calls, and prepared that metadata
only at typed-loop entry. Its diagnostic 3d-morph sample SHA-256
`c67e54e749c111a741416f35884ca08706fffacd49e7061eddde83f823f021f6`
contained 2,776 top-of-stack samples in `typed_loop::execute::run`, no
`typed_binary` symbol, and no graph evaluator frames, confirming that the
previous inlining failure had been removed structurally.

Focused graph semantics covered nested helpers, exact ordinary-callback
results, replaced helper and `Math.log` fallback with preserved side effects,
and Number edge cases including shifts, bitwise operations, NaN, and signed
zero. All 16 typed-loop tests passed. The staged touched gate also passed
formatting, workspace Clippy, the source-size guard, all 1,917 runtime tests,
the performance-unit schema checks, and 65 selected Test262 cases. The
candidate was commit `b32a6e14cce05539cd325416e2251194b70303a5`, with release
binary SHA-256
`4c68f5d8c8531acfca221ffaa9b0dcd13bf1d45eebf55c8946c2f066c15327f2`,
against exact `14d9f2f6ef4a90c1b1b62f38f6fdf9937c8307e1` base binary
SHA-256
`919c9cad198c2dcf3a50317e13997743756a084a51394097f3542b9b832fa5cb`.

The five-block nine-case screen showed that isolation fixed the original
external control failure: imaging-darkroom reached **0.209405x**
candidate/base, while 3d-morph was 1.014971x and the other seven external
controls ranged from 0.983589x to 1.022598x. The external report SHA-256 is
`be85ba015c8e5a086b30494fcc8d938742bd7ad36b71bc3d032755eadc272efc`.
Its candidate/base evidence remains valid, but its QuickJS-NG ratios are not
used because the first screen inherited an older locally built reference
binary whose compiler did not match the frozen recipe.

A subsequent source- and binary-receipted three-block run used the pinned
Homebrew Clang 21 QuickJS-NG recipe and completed all 25 broad cases with all
75 linearity diagnostics passing. Its raw and report SHA-256 values are
`458ae5d2362c136b043f18f65110382e1fcbdb455be686c0b94c260263c7eaa8`
and
`0a0eaabc30ff271b06172df93d2b82912eec0d5a75331931695daec2093f7e1f`.
Five declared broad controls remained neutral: `array_dynamic_read`
0.998088x, `array_write` 1.002701x, `plain_function_call` 1.000393x,
`function_call_two_args` 1.000625x, and `math_abs` 1.002547x. However,
`dynamic_method_call` regressed to **1.049178x** candidate/base, with a
bootstrap interval of 1.048820x to 1.054175x, decisively above the frozen
1.03x ceiling.

The hash-bound fast decision is therefore **rejected** with reason
`control regression gate failed for ['broad/dynamic_method_call']`; its
artifact SHA-256 is
`0613316c3a2468f0bc54f4c0cfa667935d87c5ee6c4b731685f59e6374fbfd20`. The unit allowed one attempt, so
the runtime implementation and focused tests were reverted and the complete
external/Test262 promotion bundle was intentionally skipped. Do not retry
this helper-graph integration by changing graph limits, admission shapes,
module visibility, or dispatch layout: two independent implementations have
now produced large target gains but unacceptable regressions in typed-loop or
generic-call controls. A future imaging unit requires a different execution
mechanism and fresh profile evidence.

### 2026-07-30 rejected copy-on-write RegExp capture snapshots

The exact `14d9f2f6` opportunity queue ranked SunSpider `string-tagcloud`
third at 7.252203x QuickJS-NG. The current `c36139cd` release executable is
byte-identical to that runtime baseline (SHA-256
`919c9cad198c2dcf3a50317e13997743756a084a51394097f3542b9b832fa5cb`).
Its source-matched Tagcloud sample (SHA-256
`74a2ed1d910b8e37450b83a916f3e1286dd767fca9c3b0c718af506788e259df`)
put 949 of 3,411 main-thread samples under `PreparedRegexp::match_input`.
Allocator entry points dominated the complete profile, and the matcher cloned
the owned capture `Vec` at each speculative `MatchState` branch. A fresh
validate-input sample (SHA-256
`a24056f5e48c2a1d835d40d0faf86407fee9b999344f0713ef833dc607687a61`)
independently put 252 of 2,621 samples in the same matcher, while a current
regexp-dna sample (SHA-256
`f1da4270d0192ebb73613b040a2749fd60e9a0dd741037e8692ccc50e72072ef`)
showed that match-result string materialization was below a useful whole-case
ceiling. The latter observation closed a proposed lazy replacement-result
unit before implementation.

The frozen one-attempt plan
`tasks/performance-units/regexp-cow-capture-snapshots.json` (SHA-256
`5c9a275b42b502d5ecee0334a2b0a2053f1eeb1beecc54ca7579d052695f07d9`)
replaced transient owned capture vectors with immutable `Rc<[CaptureRange]>`
snapshots. `PreparedRegexp` shared one all-`None` seed, capture-free patterns
retained an allocation-free representation, branch clones incremented a
reference count, and the first capture write detached through
`Rc::make_mut`. The successful result alone materialized the original ordered
`Vec`. Focused tests proved allocation-free zero-capture state and copy-on-write
branch isolation; all 45 matcher tests passed. The complete 5,160-case Test262
subset also passed on candidate source.

The candidate and exact-base release binary SHA-256 values were
`b92ddf3b54f580117398dd03973ebce552ee064b8633f519163a83c17c26bb6b`
and
`919c9cad198c2dcf3a50317e13997743756a084a51394097f3542b9b832fa5cb`.
The seven-block alternating direct-process screen required byte-equal stdout,
stderr, and successful exit status for every candidate/base pair. Its harness
and raw receipt SHA-256 values are
`017a5becd3430e9223fd6d27ed7e0762447226f4924010463d3eed2b83b69153`
and
`d0b7cd269d948bc208d3c7ad59dd3e2b1dda3a8f93fef602fc06cf1199e1010c`.
The frozen Tagcloud target improved only to **0.954929x** candidate/base
(183.355 ms candidate median versus 191.201 ms base), missing the required
`<= 0.90x` gate. The independent validate-input case improved to 0.968227x.
All other controls stayed below 1.03x: regexp-dna 0.983802x, string-base64
1.003028x, controlflow-recursive 1.011073x, A* 0.986325x, HashMap 0.991561x,
and raytrace public class fields 0.989244x.

The representation is correct and general, but a 4.5% target gain is below
the predeclared campaign threshold. The implementation and its focused tests
were therefore reverted immediately; the rebuilt release executable again
matches the exact base byte-for-byte. Do not retry this mechanism by changing
`Rc`/`Arc`, adding an inline-capture threshold, or moving individual clone
sites. A future RegExp unit needs a fresh profile and a broader state-vector
or capture-delta mechanism that removes materially more of the matcher than
capture payload cloning alone.

### 2026-07-30 rejected shared RegExp literal blueprints

The exact `14d9f2f6` opportunity queue ranked SunSpider
`string-validate-input` seventh at 6.088607x QuickJS-NG after the six higher
entries were closed by current profiles or their one-attempt mechanisms. A
fresh source-matched sample (SHA-256
`5afb51f0c1b6719d5c34cf020471b9708996ebe04bb9bc81ef97cae1f80d3458`)
contained 2,206 main-thread samples. Disjoint branches attributed about 141
samples to repeated RegExp literal construction, 39 to immutable
`PreparedRegexp` creation, and 58 to source/flag recovery and setup between
the native `test` entry and matcher execution: a combined 10.8% measured
ceiling.

The frozen one-attempt plan
`tasks/performance-units/regexp-literal-shared-blueprint.json` (SHA-256
`6b99ec21f08243f19ea48c641e9acf0b0585a9215f48be58c1f7f7e0c33ac0b9`)
introduced a dedicated RegExp-literal opcode. Bytecode owned one validated,
immutable source/flags/prepared-matcher blueprint; every evaluation still
allocated a fresh object and independent `lastIndex` through the active realm
intrinsic. RegExp source, flags, and matcher state moved from guessed
NUL-prefixed ordinary properties into the existing out-of-line object cold
state. Dynamic construction and `RegExp.prototype.compile` remained runtime
validated and did not eagerly prepare an unused matcher.

Focused tests proved that the opcode was used only for syntactic literals,
that a handwritten `new RegExp` retained the generic constructor path, that
literal identity and realm prototype were fresh and correct after rebinding
the global `RegExp`, and that guessed legacy property names could not mutate
the RegExp brand or matching state. All 100 RegExp-selected tests, all 1,917
runtime tests, workspace Clippy, and the complete 5,160-case Test262 subset
passed. The exact official `string-validate-input` source also produced the
same successful output on candidate and base.

The candidate and exact-base release binary SHA-256 values were
`79426706f9fb558510031edd84076e747f1d03c062178794f3d3628c98c52b48`
and
`919c9cad198c2dcf3a50317e13997743756a084a51394097f3542b9b832fa5cb`.
The seven-block alternating direct-process screen required identical exit
status, stdout hash, and stderr hash for every paired run. Its harness and raw
receipt SHA-256 values are
`6aa6b2884e33f794a2d6a2f67bcae96a22ee53f77550926b36a68de829dcca88`
and
`10f8bf5795204ffc9e4f8301ed08645a7e939063bd5ead94a4872934a436caee`.
The target reached **0.910267x** candidate/base (66.231 ms candidate median
versus 72.654 ms base), an 8.97% improvement but short of the frozen
`<= 0.90x` gate. All controls remained below 1.03x: string-tagcloud
0.999010x, regexp-dna 0.996223x, string-base64 0.997230x,
controlflow-recursive 1.008732x, HashMap 1.010793x, raytrace public class
fields 0.996013x, object allocation 0.999859x, dynamic method call 0.999037x,
and string slice 1.000481x.

The hash-bound fast decision is therefore **rejected** with reason `target
improvement gate failed for ['external/sunspider-1.0/string-validate-input']`;
its artifact SHA-256 is
`61e3d8bf85f0ae6eb6b79c6dc1ad0bee87b0faf3c7ab2295d54afe3b75f2b5f2`.
The unit allowed one attempt, so the runtime implementation and focused tests
were reverted immediately. Do not retry this combined blueprint/internal-slot
mechanism by changing flag packing, cold-slot layout, opcode placement, or
matcher ownership: it removed the intended structural work but delivered less
than the campaign's ten-percent minimum. A future literal or RegExp unit needs
fresh evidence for a different shared cost.

### 2026-07-30 accepted frame-independent native construction

The exact `14d9f2f6` opportunity queue ranks SunSpider `date-format-tofte`
ninth at 5.790250x QuickJS-NG. Ranks one through seven are closed by current
profiles and their consumed mechanisms. A fresh rank-eight
`date-format-xparb` sample found only about twelve samples of removable native
constructor environment work among 1,575 useful samples; its remaining String
wrapper and concatenation costs require substantially broader representation
work. The fresh Tofte sample (SHA-256
`c0526ef6ac17aad2cc333cdabad8c3a287e47b3ef931e500927d3d12366a43ac`)
instead found 19 samples in native Date caller-environment materialization and
208 disjoint samples in post-constructor `apply_call_env`, or 227 of 2,178
useful main-thread samples (10.4%).

The frozen one-attempt plan
`tasks/performance-units/frame-independent-native-construction.json`
(SHA-256
`13e68e4dd055d1ebe5afd43439814dc840c8a807256d89d5828e1154df14b139`)
therefore mirrors the existing native-call rule at bytecode `Construct`:
an unbound native Function receives the shared realm environment instead of a
snapshot and later writeback of the caller frame. User-bytecode, bound, Proxy,
class, derived, and forwarding construction retain the generic path. This is
distinct from the rejected user-constructor context alias, where both choices
allocated the same empty child frame. Coercion hooks and callbacks still run
through their own function closures, while `apply_call_env` still refreshes
sloppy realm-backed caller slots.

Focused semantics cover Number coercion through a callback that mutates a
captured local, Date construction beneath direct eval, thrown-value identity,
and a synchronous Promise executor retaining its captured cell. The new test,
all 59 global tests, eight Date-selected tests, two String-constructor tests,
the existing native callback test, workspace Clippy, and 669 targeted Test262
Date/String/Number/Promise/call/new cases pass.

The candidate and exact-base release binary SHA-256 values are
`1598e5352a92c37413ac9e9040a8cc769cf678391db7130fa1dc868f219744f9`
and
`919c9cad198c2dcf3a50317e13997743756a084a51394097f3542b9b832fa5cb`.
The seven-block alternating target receipt (SHA-256
`e0f6e1e3c58869a23671f5b36f9583351ed99c6058e91c0775145dfaf6f62747`)
requires identical exit status, stdout, and stderr for every candidate/base
execution. `date-format-tofte` reached **0.892977x** candidate/base (126.498
ms candidate median versus 141.561 ms base), passing the frozen `<= 0.90x`
target gate.

The independently recorded nine-control receipt (SHA-256
`dbae252cd255296d34791dc1d65bae0cbb87e098ffb2b19f9cc46c0fbf4b8740`)
also passed its `<= 1.03x` ceiling. Ratios were 0.986840x for
`date-format-xparb`, 1.003848x for `controlflow-recursive`, 0.991339x for
`string-tagcloud`, 1.006946x for HashMap, 0.998412x for CDJS, 1.001362x for
`access-nbody`, 1.001382x for `dynamic_method_call`, 1.000707x for
`plain_function_call`, and 0.998588x for `object_allocation`.

The retained implementation is commit
`620bb67b9c7c36714177b7006a7ccaddd88653f5`. Its standard-recipe candidate,
exact-base, and pinned QuickJS-NG release binary SHA-256 values are
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`,
`540fc89a37c71dd1f0ac4a0916a6f8e0ccbc418c27498f960fedcfb2510df3bf`,
and
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
The clean three-block broad summary, report, and raw SHA-256 values are
`76cdfb6fe4fd8833839e16025e4c6b97d762289f685d2c80c25c112f9014fba2`,
`a320011570f8178b8be208abc6bea97d0ccbf20ba98e619714a810fb8c009e7e`,
and
`bf7234c6e3eb65ada0eabd818aa10e9d1cdc8528e3669f8a8875d6cde1db38aa`.
All 25 cases and all 75 linearity diagnostics completed. Candidate/base
geometric mean was 1.000334x, the largest individual broad ratio was
`top_level_function_call` at 1.010124x, and the declared broad controls were
`dynamic_method_call` 0.998416x, `plain_function_call` 1.001574x, and
`object_allocation` 0.994472x.

The default 30-second external preview completed 44 comparable cases and
exposed one honest coverage gap: Kraken `imaging-gaussian-blur` timed out for
candidate and base. The source- and binary-identical 60-second rerun closed
that gap and completed all 45 cases for candidate, base, and QuickJS-NG. Its
manifest, report, and raw SHA-256 values are
`a8ddeded582573bc676bf3f7bbbaf2625f6dfa7742f07bcdd6aaa26366f4e6c4`,
`ed0f904c65ab557b68a85d2ff3a5ba0f0f71616a53faf990b541f972a6cbf773`,
and
`d468a142a618b98bb89c8dea80232cfae29442761fc8f625810903d74c7cd574`.
The target measured **0.899021x** candidate/base and 2.738381x
candidate/QuickJS-NG, so it clears this unit's ten-percent base gate but not
the campaign's overall two-times-QuickJS-NG goal. All declared external
controls remained below 1.03x: xparb 0.992505x, recursive 0.997750x,
tagcloud 0.996059x, HashMap 0.990611x, CDJS 0.996163x, and nbody 0.988177x.
The complete portfolio also records, rather than hides, a non-control
`bitops-bitwise-and` ratio of 1.034481x; it is a very short case and was not a
frozen acceptance criterion for this constructor-specific unit.

The hash-bound fast decision is **retained** (SHA-256
`509fa5cbe97728eef0f93b834e4bf039a0649215c8748abc805fd6f8c1d6a065`).
The strict promotion decision is **inconclusive** (SHA-256
`ecca153b01d18199e6654887ba925c0ade44a4cd19307ca512552345502b22b4`)
solely because no exact project-wide zero-gap Test262 burndown is attached;
this is not a global conformance or campaign-completion claim. The staged
touched gate passed all 1,915 runtime tests and 65 selected Test262 cases;
`./scripts/check.sh` passed all workspace checks and the complete 5,160-case
curated Test262 subset, and `./scripts/compare-qjs.sh` passed. The accepted
general reduction therefore advances the campaign, whose next unit must start
from a refreshed exact-current queue and fresh shared-cost profile.

### 2026-07-30 rejected contiguous direct-call frame stack

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked SunSpider `controlflow-recursive` first at 5.8835x QuickJS-NG and
JetStream HashMap second at 5.3652x. Fresh source-matched samples had SHA-256
values `289f099ccf6dda2a8ef273584296de5e54e4476d7527e67d9f82d4c93e2ebc03`
and `9621e95e6b582b797b713374b4e2df15b8abd56fcfa59feff06a0c5c929f2877`.
The recursive profile attributed 844 of 2,254 useful samples (37.4%) to
direct-leaf call setup, VM/frame construction and teardown, CallEnv teardown,
marked-realm setup, and operand-stack recycling. HashMap attributed 634 of
3,627 samples (17.5%) to the same boundary, with another 326 samples in
`Value` clone/drop work.

The frozen one-attempt plan
`tasks/performance-units/contiguous-direct-call-frame-stack.json` (SHA-256
`4092dcf45b66d684747df470a14db0440065f603beedc9b3bfb7f2c3d1d612f8`)
therefore tested a mechanism distinct from the earlier independent-Vec and
same-function schedulers. It split each frame into a move-cheap dispatch header
and boxed cold state, kept bytecode/immutable plan views move-stable, and moved
one contiguous operand `Vec<Value>` between parent and child handles while
retaining relative bases. Admission remained the existing ordinary
`is_direct_leaf_function` predicate after the existing numeric and
this-property probes; no source, function, workload, path, checksum, iteration,
or expected-result condition was added.

The structural prototype compiled cleanly. Its layout-only stage passed all
1,915 runtime tests. The complete scheduler stage passed five focused tests,
including 10,000 recursive frames, zero-through-three arguments and receiver
binding, parent operands, thrown-value identity through nested `finally`,
cleared storage reuse, and eval/with/closure/generator fallbacks; all 1,920
runtime tests then passed. These correctness results do not override the
predeclared performance gate.

The final exact-base five-block alternating screen compared standard-recipe
candidate SHA-256
`6a7b5b99d1240921d0f6840741cff6460b037eab0e1cec564a0f6be5b810fd1e`
with exact `620bb67b` base binary SHA-256
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
Every paired process had identical exit status, stdout hash, and stderr hash.
The raw receipt SHA-256 is
`8eb2833660a2c058441b309e4a973ffb81e5d5623dfdd98057438086e17d4b7b`.
Both targets regressed decisively instead of reaching the required `<= 0.90x`:

| Target | Candidate median | Base median | Candidate / base |
| --- | ---: | ---: | ---: |
| `controlflow-recursive` | 78.392 ms | 66.363 ms | **1.180476x** |
| JetStream HashMap | 1,868.258 ms | 1,616.959 ms | **1.150426x** |

The unit is **rejected** after its single allowed attempt. The runtime,
function-call, and focused-test changes were reverted; the restored worktree
compiled cleanly. Because both independent targets moved in the wrong
direction by 15-18%, controls, full Test262, broad, and external promotion runs
were not started. Do not retry this hot-header plus boxed-cold-state,
move-stable-pointer, contiguous-operand-buffer transfer shape by changing pool
bounds, inline argument width, stack-base placement, or child-frame storage.
The measurement closes this shared frame route for the current representation;
the next unit must use a refreshed queue and a different profiled mechanism.

### 2026-07-30 rejected fused dense-object Number read

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked Kraken A* third at 4.8729x QuickJS-NG after the two higher-ranked
direct-call and frame routes were closed. A fresh exact-current A* sample
(SHA-256
`a32d90e6139faa8f15f5cff1ee19c9f63143e45a2da18f9dd2750b012d5f35e5`)
placed 2,139 of 4,388 main-thread samples in generic VM dispatch, 548 in
`Value` clone/drop, 228 in the named-property cache, 235 in its fast helpers,
and 72 in the dense-index read. The hot `openList[i].f` comparison performs two
independent dense-element plus Number-field chains per iteration. A separate
N-body control profile (SHA-256
`1f33298aaf4ab1c3a89ffafd44ac77e2a0e81643b7deb7e12ff3a4e27fa656e5`)
confirmed that this array/object/Number family is shared rather than unique to
A*.

The frozen one-attempt plan
`tasks/performance-units/typed-loop-dense-object-number-read.json` (SHA-256
`ebd9824d34272e59a21ec99db31a3740e1c7757b90eb722ec72d140982df3eef`)
tested a narrower mechanism than the earlier rejected broad numeric-object
field scalarization. It recognized only a typed-loop straight-line
`LoadLocal(array)`, `LoadLocal(index)`, computed element read, then named field
read. One guarded operation borrowed the present dense element, required an
ordinary object with a current own Number data slot, and wrote the `f64`
directly to the scalar register file. Neither intermediate `Value` was cloned
or boxed. Holes, descriptors, accessors, inherited or non-Number fields,
changed layouts, unsupported indices, and unsupported control flow all
deoptimized at the original computed read before observable work.

The prototype compiled cleanly. Its focused tests proved that the exact A*
shape emitted the fused operation and returned the expected result, while an
accessor replacement deoptimized and invoked its getter exactly once. All 14
typed-loop tests passed. These semantic checks established a valid candidate;
they did not relax the performance threshold.

The exact-base five-block alternating screen compared standard-recipe
candidate SHA-256
`797ea840011914e2467b796a6d452ff071793a5e7adb9e1fdd9d819f02442d96`
with exact `620bb67b` base binary SHA-256
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
Every warmup and measured process exited successfully with identical stdout
and stderr hashes. The A* source SHA-256 was
`a3653c77773ce2b424301835021957b26119240810f43d5434d98fd88d7a416c`;
the raw receipt SHA-256 is
`5922343e9dd03f11804758aa81be838268225627018539bd7ba344043d4852c1`.
The five measured candidate times had an 8,366.721 ms median, versus an
8,210.598 ms base median, and the paired candidate/base median was
**1.024107x**. The result missed the required `<= 0.90x` reduction and moved in
the wrong direction by 2.4%.

The unit is therefore **rejected** after its single allowed attempt. All
runtime and focused-test changes were reverted, and the restored runtime source
matches `HEAD`. Controls, full Test262, broad, and external promotion runs were
not started because the sole target failed the first gate. Do not retry this
exact typed-loop fusion by moving the guard, changing cache warmup, retaining
one of the two boxed intermediates, or adjusting typed-loop admission. Together
with the earlier boxed numeric-object-field rejection, this closes typed-loop
scalarization of the `denseArray[index].numberField` chain for the current
object and `Value` representations. A future attempt requires a different
profiled representation-level mechanism, not another variant of this fused
operation.

### 2026-07-30 rejected boxed-String virtual index properties

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked SunSpider `string-tagcloud` fourth at 4.1325x QuickJS-NG after the first
three frame, recursion, object-layout, and typed-loop routes were closed. A
fresh exact-current tagcloud sample (SHA-256
`079339f4d5b4d85ff7338b1f7375787645790dde03accfaadffd8366d5a9417a`)
contained 7,206 main-thread samples. Primitive-receiver boxing accounted for
989 samples, including 770 under `boxed_string` while it created the internal
StringData and length plus one ordinary `Property` per UTF-16 code unit. The
descendants included property-table insertion and promotion, per-code-unit
String construction, allocation, and destruction. An independent current
`date-format-tofte` sample (SHA-256
`8bf689bf4ac341177de2b55a1648da3d489811a4f14854670878cdc82d91bed9`)
confirmed the same `new String` route but placed only 23 of 11,310 samples in
`define_string_data`, making it an explicit short-wrapper overhead control.

The frozen one-attempt plan
`tasks/performance-units/boxed-string-virtual-index-properties.json` (SHA-256
`ca9837710a1f458e56c0bf7e284f1f7962d82c5c982ed251fe923d106f74c6d3`)
tested a mechanism distinct from the rejected StringData-buffer sharing unit.
It retained the existing owned StringData copy and ordinary fixed `length`,
but stopped creating indexed ordinary properties eagerly. Central object get,
has, descriptor, define, set, delete, and own-key paths instead synthesized the
fixed enumerable/non-writable/non-configurable UTF-16 index descriptors on
demand. The representation applied to `new String`, `Object(string)`, String
subclasses, and non-strict primitive receiver boxing without inspecting source,
benchmark, function, input, contents, length, iteration count, checksum, or
result.

Four focused test groups covered compatible and incompatible definitions,
strict and Reflect assignment, deletion, UTF-16 surrogate code units, ordinary
numeric and named key ordering, Object values/entries/assign/spread, freeze and
seal state, inherited and receiver writes, Proxy invariants, subclasses, and
sloppy primitive receiver boxing. All six boxed-String tests, all 30 String
tests, and all 34 Object builtin tests passed. These semantic results
established a valid candidate but did not override the frozen broad control
ceiling.

The five-block alternating screen compared standard-recipe candidate SHA-256
`250591e329714df7590c93234d84b1c3bf1679a46c1bd1016f18c04daed5a8b3`
with exact `620bb67b` base binary SHA-256
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
Every warmup and measured process exited successfully with identical stdout
and stderr hashes. The target receipt SHA-256 is
`81d4fdf8975a3a610a78f2ae8ad4011ecbdd979d5d153fe36d85baebb00fdd58`;
tagcloud reached **0.775197x** candidate/base, a 22.5% improvement and well
past the required `<= 0.92x` target.

The independently measured control receipt SHA-256 is
`953c60fd12e1f85eb71497b7cc8bf3e817c7258b0b38c8fd2c00518be3da222e`.
Ten controls stayed below the frozen `<= 1.03x` ceiling: `date-format-tofte`
0.994412x, `date-format-xparb` 0.936216x, `string-base64` 0.980507x,
`string-validate-input` 1.003042x, HashMap 1.000888x, A* 1.002198x,
`property_read` 1.000391x, `property_write` 1.000596x, `string_slice`
0.954262x, and `dynamic_method_call` 0.968973x. The required
`object_allocation` control, however, was **1.077567x**. Four of its five
paired blocks were tightly grouped between 1.075727x and 1.077783x; the
candidate itself stayed between 5,534 and 5,541 ms in all five blocks. This is
a stable 7.8% broad regression, not timing noise, and exceeds the control cap
decisively.

The unit is therefore **rejected** after its single allowed attempt despite the
large target win. All runtime and focused-test changes were reverted, and the
restored runtime source matches `HEAD`. Full Test262 and complete portfolio
promotion runs were not started after the mandatory broad control failed. Do
not retry centralized virtual boxed-String indices by rearranging the ordinary
`ObjectRef` helper checks, changing key guards, or selectively materializing a
length threshold. That on-demand representation has useful negative evidence
but violates the broad allocation boundary; a future String-boxing unit needs
a newly profiled representation or escape mechanism that does not perturb the
ordinary object core.

### 2026-07-30 skipped public-class-field raytrace after profile screen

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked JetStream `raytrace-public-class-fields` fifth at 4.0395x QuickJS-NG.
The commits after `620bb67b` change only task evidence, so the standard-recipe
release binary remains exact; its SHA-256 is
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
A fresh run of the hash-verified upstream bundle (SHA-256
`824daa5582289787f6e25a200892a1d6bdfa682afe9ce76a30e520dc7e03528c`)
produced the expected marker and a 1,324-sample main-thread profile with
SHA-256
`54cf33e2cc93f4865a7f7ba0a5820b7ef5c1a76a2191f80c90a2c56ab7c9917c`.

The exclusive samples show that the formerly dominant class-field path is no
longer a bounded high-ROI opportunity. `initialize_instance_fields`,
`call_field_initializer`, `set_shared_key`, and `PropertyStorage::insert`
together account for only 28 top-of-stack samples, or 2.1%. The remaining
cost is distributed across generic opcode execution (253 samples), direct-call
and frame lifecycle, allocator entry points, and `Value` clone/drop work.
Those families are not new mechanisms: current evidence has already rejected
the compact generic core, contiguous direct-call stack, lazy-weak and
Realm-arena ownership representations, constructor transition shapes, default
data-property storage, and owned-operand transfers. Current base-class direct
slots and isolated shared-key field installation already cover the retained
field-specific reductions.

No performance-unit plan or runtime candidate was created. A field-only
mechanism has an observed upper bound far below the campaign's materiality
threshold, while aggregating the remaining unrelated costs would simply
rebrand multiple exhausted architectural routes. This is a deliberate
low-benefit stop, not a conformance or performance claim. Do not revisit
public-field key sharing, field descriptor installation, constructor shapes,
or base-constructor slot seeding without a future exact profile that exposes a
new independently bounded cost above ten percent. The queue should advance to
rank-six `imaging-gaussian-blur`.

### 2026-07-30 rejected borrowed fast-native predispatch

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked Kraken `imaging-gaussian-blur` sixth at 3.9988x QuickJS-NG. A fresh
full run of the official generated bundle (SHA-256
`54a60944b49ce59f85524db09ea4eba6f2137036e19487a45844d728b843bd38`)
produced the expected marker and a 7,384-sample main-thread profile with
SHA-256
`ee644edb30089322d645f00fbee263ca37624df30565294f96f5b8791c46f3cb`.
The mutually exclusive hot leaves included 583 samples in
`call_callee_with_marker`, 252 constructing empty `CallEnv` frames, 140
dropping those frames, and 47 in `pop_arguments`: 1,022 samples, or 13.84%,
before their allocator descendants. Source and route inspection tied the cost
to the inner convolution's repeated `Math.abs` calls. The ordinary call path
allocated an argument `Vec`, eagerly created a realm environment, and only
then reached the existing environment-independent unary-Math fast native.

The frozen one-attempt plan
`tasks/performance-units/borrowed-fast-native-predispatch.json` (SHA-256
`4bf10fcd0dcc322c6095ae5a22ac2d7d5ca2349caec74327ccc676ab9eb6b026`)
tested one general call mechanism. Fixed calls of at most three arguments
first borrowed the callee, receiver, and argument slice from the operand stack
and invoked the existing semantic fast-native classifier. Accepted calls
truncated the exact operands before processing their return or abrupt
completion; declined calls performed no mutation and fell through unchanged.
The classifier also received a lazy realm-environment factory, so accepted
environment-independent natives avoided both the argument allocation and the
empty frame. Eligibility used only the native function identity and ordinary
`Call` or `CallResolved` stack shape; spread, constructor, bound, Proxy,
replaced, coercive, unsupported, and user-function routes were unchanged.

Focused runtime tests passed for resolved and unbound primitive calls,
realm-dependent String slicing, coercion fallback, thrown-value identity and
catch-stack restoration, replaced Math natives, indirect eval, and caller
locals. The existing primitive-native fallback test passed alongside the new
coverage. These semantic checks established a valid candidate but did not
relax the predeclared broad control ceiling.

The exact-base five-block alternating screen compared standard-recipe
candidate SHA-256
`45df9dbb0cd0a73706fdf535e395e8f6cd7e53623428a4c6e248266a23c0e895`
with exact `620bb67b` base binary SHA-256
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
Every warmup and measured process exited successfully with byte-identical
stdout and stderr hashes. The target receipt SHA-256 is
`a47f194e833ec145edfe93fd796d37a4fac3a2af865ee32d47b76f715f83e6df`.
Its five paired candidate/base ratios were 0.790214x, 0.786895x, 0.787794x,
0.802361x, and 0.779258x. The **0.787794x** median is a 21.2% improvement and
comfortably passed the required `<= 0.90x` target.

The complete frozen control receipt SHA-256 is
`4da0f2e0d1eb32f726bda3ec1dffe2d3177a55d74614c6d8874ebbab1707a087`.
Thirteen controls stayed below the `<= 1.03x` ceiling: `imaging-darkroom`
0.890908x, `3d-morph` 0.984161x, `audio-fft` 0.941089x, A* 0.996347x,
public-field raytrace 0.966698x, `date-format-tofte` 0.989809x, tagcloud
0.996359x, `math_abs` 0.999569x, `plain_function_call` 1.000200x,
`function_call_two_args` 0.999536x, `dynamic_method_call` 0.923722x,
`property_read` 1.000165x, and `array_dynamic_read` 0.990688x. The required
`object_allocation` control, however, was **1.077990x**. All five paired
ratios were between 1.060424x and 1.082633x, so the 7.8% regression is stable
and decisively exceeds the cap.

The fail-closed decision receipt (SHA-256
`10598f2886afc2f0ab4866c32e0d5914453debc44abd10981eced3202d6bc30b`)
therefore classifies the unit as **rejected** after its single allowed
attempt. All runtime and focused-test changes were reverted, and the restored
runtime source matches `HEAD`; full Test262 and complete portfolio promotion
runs were not started after the mandatory broad control failed. Do not retry
this borrowed-stack plus lazy-realm predispatch shape by moving the branch,
changing the fixed arity, splitting native families, or retuning inlining.
The large target and independent Kraken gains show that generic native-call
setup remains valuable, but a future unit must remove it through a different
profiled call representation that does not perturb the ordinary allocation
loop. The queue should advance to rank-seven JetStream `cdjs`.

### 2026-07-30 skipped CDJS after current profile screen

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked JetStream CDJS seventh at 3.7303x QuickJS-NG. The later commits are
evidence-only, so the runtime remains exact at `620bb67b`; its standard
release binary SHA-256 is
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
A fresh high-frequency run of the unmodified generated CDJS bundle (SHA-256
`8fdde63549dd457d375730cca98efc1be687c3fa32b84e5d63a9903b768fed47`)
exited successfully with the expected marker. Its 1,162-sample main-thread
profile has SHA-256
`226d6386ccbeb675b7f2cfb0e42da433666944bd1a40fd8b84c7e81e69edfe43`.

The exclusive sample families reproduce already closed costs rather than a
new bounded mechanism. Generic `Vm::run_completion` dispatch accounts for
318 samples, or 27.4%, but both compact direct-call dispatch and the complete
generic compact bytecode core have already failed their target and broad
gates. The call family contains 93 samples in `call_direct_leaf_function`, 55
in `Vm` construction, 31 dropping `FrameState`, 22 creating and 23 dropping
`CallEnv`, and 19 in `call_callee_with_marker`. Those are not one untested
20.9% lever: the independently removable VM-plus-frame lifecycle is 7.4%,
the environment pair is 3.9%, the generic wrapper is 1.6%, and the direct
wrapper is 8.0%. The full recursive frame scheduler, same-function frame
stack, contiguous direct-call frame stack, tail-frame reset, value-only exit,
and direct-slot captured-closure conversion have each already been measured
and rejected. In particular, the earlier CDJS captured-closure candidate
removed the same inclusive generic call chain but achieved only about
0.987x, while the later value-only exit reached 0.997776x.

The remaining flat costs are likewise distributed: `Value` clone/drop totals
118 samples (10.2%) across calls, properties, and containers rather than one
ownership boundary; the named-property cache, ordinary slot/storage reads,
string comparison, and property helpers are each smaller slices; allocator
entry points total under 6%. Current evidence has already rejected the owned
operand, discarded assignment transfer, lazy weak-refcount, Realm-arena,
immediate property-cache, transition-shape, and property-storage variants
which targeted those families. Aggregating them under one CDJS label would
combine unrelated mechanisms and violate the single-unit evidence rule.

No optimization plan, runtime patch, candidate binary, or timing gate was
created. This is a deliberate low-benefit and duplicate-route stop, not a
performance claim. Do not retry CDJS through another direct-call scheduler,
captured-closure eligibility expansion, compact opcode stream, value-only
result transfer, or a bundle-specific combination of its scattered costs.
A future CDJS attempt requires a fresh representation-level mechanism with an
independently profiled share above ten percent. The queue should advance to
rank-eight Kraken `imaging-darkroom`.

### 2026-07-30 rejected typed-loop entry-expanded numeric helper bodies

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked Kraken `imaging-darkroom` eighth at 3.6449x QuickJS-NG. A fresh full
run of the official generated bundle (SHA-256
`bfc464b658a5b69810769631928842241a46d8d9bfbfb62da53804843eadfaba`)
produced its expected marker. Its 4,406-sample main-thread profile (SHA-256
`179f6fc7a9270a62e556602d1bead193b43064b4a1d93f9c1ffe9d5ee7da6066`)
placed 4,214 samples, or 95.64%, below `ProcessImageData` in the repeated
`FastGain`, `FastBias`, `FastLog2`, `Clamp`, and pure-Math helper graph.
`Vm::run_completion` contributed 1,985 exclusive samples, direct-leaf calls
274, numeric binary work 201, VM construction 163, frame destruction 69, and
call-environment construction/destruction 88. The outer arithmetic, branches,
dense reads, and writes already fit the typed-loop IR; ordinary helper calls
were the bounded exclusion.

The frozen one-attempt plan
`tasks/performance-units/typed-loop-entry-expanded-numeric-helper-bodies.json`
(SHA-256
`27b71e62ef678fe6e677886c1735d224267f8a94b25f18f3a9f055cacdb5332c`)
tested a structural mechanism rather than another call-graph evaluator. At
typed-loop entry it validated a bounded acyclic pure-Number helper graph,
including exact function identity, live T016 upvalue cells, creation-realm
`Math`, and own data properties, then renamed its registers and translated it
to the existing `Move`, `Binary`, `JumpIfFalsy`, `Jump`, and
`CallNumericNative` operations. Unsupported graphs, dynamic realms, accessors,
replacement, coercion, recursion, backward branches, writes through shared
cells, named-property stores, and excess size declined before loop effects.
There was no new opcode, graph executor, parser or AST change, dependency,
source-name heuristic, or benchmark-specific admission rule.

Seventeen focused typed-loop tests passed, including nested helpers, cached
entry reuse, function and `Math` replacement, numeric-capture invalidation,
NaN/signed-zero and bitwise behavior, exact generic results, and T016
shared-cell decline; 1,902 unrelated runtime tests were filtered in that
focused run. An early uncached form made darkroom 0.162269x exact base and
0.593989x QuickJS-NG, but a nine-block audit reproduced
`math-spectral-norm` at 1.056984x because short inner loops rebuilt the same
graph on every function entry. A program-local cache that revalidated the
complete live graph before reuse removed that issue: the follow-up nine-block
medians were spectral-norm 0.999436x and HashMap 1.004681x.

The final standard-recipe candidate binary SHA-256 was
`7d3337ee26a08fd9f809c7a10e590725d6a33bfee3f008b4065bf31f09457d78`;
the exact base binary remained
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
Every warmup and measured process exited successfully with identical stdout
and stderr hashes. The complete frozen five-block receipt SHA-256 is
`a7555f38585186bf03e4cc4d20a09ec18289a800cafc84cf104229b3a44866ad`.
Darkroom reached **0.163388x**, an 83.7% reduction and well beyond the required
`<= 0.35x` target. Thirteen controls passed the `<= 1.03x` ceiling:
3d-morph 1.015369x, gaussian blur 1.002667x, audio FFT 0.982324x, A*
1.008417x, HashMap 1.003677x, public-field raytrace 1.003539x, recursive
0.991882x, nbody 0.991757x, dynamic method call 1.000205x, plain function
call 1.005971x, two-argument call 0.996617x, dynamic array read 0.992882x,
and array write 1.001155x.

Three mandatory controls failed: `string-tagcloud` was **1.107604x**,
`math_abs` **1.033666x**, and `object_allocation` **1.081317x**. The
`math_abs` failure was independently stable across nine seeded alternating
blocks at about 1.04x. Three corrective layouts preserved the ordinary
`run`/seed/execute call graph, restored its inlining, and restored all old
program-field offsets, but did not remove that regression. Equal-duration
profiles of an intermediate candidate and exact base (SHA-256
`53bfe3b95b4550bae961acb16def320e4780c11bd834bd26f1311d8938df74cc`
and `478d47ab42fddaf8d3c9447b2190b77251701ea314cf6edabd1b5e012e10ce01`)
placed every sample in the pre-existing `vm_numeric_loop` and `math_unary`
path, not the new typed-loop helper path. Together with the unchanged source
for those modules, this is diagnostic evidence of reproducible 16-CGU codegen
perturbation, not grounds to waive a measured control failure.

The standard fail-closed decision receipt (SHA-256
`2331837849c2e3204890f96d2a3097fca61826f1cbf17cbb5af413777816b6d3`)
therefore classifies the unit as **rejected**. All runtime and focused-test
changes were reverted, and `crates/qjs-runtime` again matches `HEAD` exactly.
Full Test262 and complete promotion portfolios were not started after the
mandatory fast gate failed. Do not retry this entry-expanded helper module by
retuning cache, inlining, field order, function order, or code padding; a
future helper-body mechanism needs a different representation with stable
ordinary-code generation and fresh profile evidence. The queue should advance
to rank-nine SunSpider `access-nbody`.

### 2026-07-30 skipped access-nbody after current profile screen

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked SunSpider `access-nbody` ninth at 3.5831x QuickJS-NG. The later commits
remain evidence-only, so the runtime-exact standard release binary is still
SHA-256
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
A fresh five-second run used the unmodified upstream source (SHA-256
`84f08150e27c075e4a6b1b900b743cc2845abd3a47175cd444f6a1178493487c`)
and the existing sampling wrapper (SHA-256
`1f65354dce2dcf44b2f38f2394ead3c1f167872e5270ae5e5684c8d97da94250`).
It exited successfully with `Undefined`, no stderr, and a 3,702-sample
main-thread profile whose SHA-256 is
`df00da80d4adf4758ac778aba422fc3479f3c2ee9a3f1dbc9b1a76c7af161b8f`.
The stdout and empty-stderr SHA-256 values are
`50fbe849aa61688a0dde78393afa32aba45d9f4a52109662bea06fa4c45715d5`
and
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

The exclusive distribution is stable against the earlier exact 4,448-sample
receipt. Generic `Vm::run_completion` dispatch accounts for 1,856 samples, or
50.1%; `Value` clone/drop totals 488, or 13.2%;
`NamedPropertyCache::get` contributes 261, or 7.1%; ordinary
`eval_binary` contributes 190, or 5.1%; and the paired named-cache write is
only 73 samples, or 2.0%. The prior receipt recorded the same families at
2,260, 587, 293, 228, and 82 samples respectively, so sampling variance does
not expose a new boundary.

Each material family is already closed by current evidence. The compact
generic bytecode core failed its independent target and broad gates. The
typed-loop numeric-object-field unit directly removed generic dispatch,
boxing, and object-field work and made N-body 0.583367x, but regressed the
independent A* object-field workload to 1.154586x; the narrower fused dense
object Number read then made A* 1.024107x and was also rejected. Shared-slot
cache promotion reached only 0.988474x on N-body while making A* 1.102369x.
The remaining named writes are already covered by the retained paired
compound-store cache, which previously made N-body 0.945x. Binary evaluation
and every other exclusive family are individually below the ten-percent
materiality boundary.

No performance-unit plan, runtime patch, candidate binary, or timing gate was
created. Combining generic dispatch, `Value` lifetime, property-cache reads,
and numeric operations under one N-body label would reassemble the rejected
numeric-object-field scalarization rather than define a distinct general
mechanism. Do not retry cache-entry ordering, typed-loop object-field
scalarization or fusion, boxed-operation admission, property guards, or
compound-store pairing without a future exact profile that exposes a new
independently bounded representation cost above ten percent. The queue should
advance to rank-ten SunSpider `string-base64`.

### 2026-07-30 rejected chunked compound-string accumulator

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked SunSpider `string-base64` tenth at 3.4700x QuickJS-NG. The exact base
profile used the standard release binary SHA-256
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`,
the unmodified upstream source SHA-256
`48e6106fc6df6cb725b2c56934aff1591c97527862d27accfc11e419467639fc`,
and a 40-run diagnostic wrapper SHA-256
`58685e645bfd519793ea3be46954be8d5b3a16147199e6bdc7ff0f3670713293`.
Its 2,168-sample main-thread receipt (SHA-256
`2a1afc57aa3a7d34b678ff70afaa42becd70670784c8793fab0cc7a1d4fd3850`)
placed 128 samples under flat `String` growth, including 127 in `RawVec`
reserve and 85 in `memmove`, plus 117 samples under shared-right/self-append
conversion, including 105 in `JsString::into_string` and 92 in `memmove`.
These distinct representation costs totaled 245 samples, or 11.30%, after
the retained compound-string binding guard had already admitted buffer reuse.

The frozen one-attempt plan
`tasks/performance-units/chunked-compound-string-accumulator.json` (SHA-256
`eb51277672684b1b0e9e91d10e250e3d8594c1176395672a9274cf5ebf426f6d`)
tested a private, bounded string representation. Only a uniquely owned result
accepted by the existing successor-and-binding guard could accumulate in
4 KiB chunks; completed chunks were immutable, exact self-concatenation could
duplicate their handles, and every contiguous-string consumer lazily
flattened the value. Aliased, coercive, failed-guard, ordinary append, and
all parser, AST, property, and binding routes retained the flat path. Focused
tests passed for large Unicode append and UTF-16 metadata, alias immutability,
exact self-append ordering, later visible aliases, and returning a previously
unflattened result. The focused runtime tests and `cargo clippy -p qjs-runtime
--all-targets -- -D warnings` also passed before measurement.

The standard-recipe candidate binary SHA-256 was
`b6a39c03a6d8798c1cdcc058d3a720152e6cb24db27f0af1c935fa4c557d3d5d`;
the exact base binary remained
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
Every warmup and measured process exited successfully with byte-identical
stdout and stderr hashes. The seven-block alternating target receipt SHA-256
is `cc25ed7fd65da23a90569fee2ed1c22f11e517fe4fe70cc79e1deabf41a5f0a4`.
Its paired candidate/base ratios were 0.620728x, 0.948602x, 0.971491x,
1.330664x, 1.005836x, 0.948158x, and 0.988536x. The **0.971491x** median
missed the predeclared `<= 0.90x` target by 7.1 percentage points; the
separate candidate/base medians were 82.486 ms and 84.705 ms, confirming that
the mechanism removed only a small part of total process time despite noisy
individual pairs.

The unit is therefore **rejected** after its single allowed attempt. The
mandatory target failed before control or promotion measurement, so no broad,
external-portfolio, or Test262 performance claim was made. All runtime and
focused-test changes were reverted, leaving only the immutable plan and this
negative result. Do not retry this 4 KiB flat-tail/chunk-handle shape by
retuning chunk size, flatten threshold, or self-append eligibility. A future
string representation needs fresh exact evidence for a different bounded
cost and must preserve the existing compound-string reuse mechanism. The
queue should advance to rank-eleven SunSpider `3d-raytrace`.

### 2026-07-30 skipped 3d-raytrace after current profile screen

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked SunSpider `3d-raytrace` eleventh at 3.4552x QuickJS-NG. The later
commits remain evidence-only, so the runtime-exact standard release binary is
still SHA-256
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
A fresh five-second sample used the unmodified upstream source (SHA-256
`87a1cb968113dcaf427dc2634e95f6ee6460f38e132c26cdf640639521620591`)
and a diagnostic 100-extra-run wrapper (SHA-256
`727015925e8e7f724c8df53a4e0c8cb7962a75f4da16417af9e823a4fded829d`).
It exited successfully with `Undefined`, no stderr, and a 3,552-sample
main-thread runtime profile whose SHA-256 is
`26e38d92610ba468e5754a52c0348f2ed104abbcfdafe37f773ca9576616bd4a`.
The stdout and empty-stderr SHA-256 values are
`50fbe849aa61688a0dde78393afa32aba45d9f4a52109662bea06fa4c45715d5`
and
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

Generic `Vm::run_completion` contributes 1,379 exclusive samples, or 38.8%.
Instruction-level inspection of the sampled PCs distributes that work across
ordinary direct-call staging, binary evaluation, global loads, named-property
cache reads, dense-index reads, returns, and `Value` clone/drop boundaries;
it does not expose one new hot opcode. `Value` clone/drop itself totals 445
samples, or 12.5%, but its call-tree parents span those same independent
binary, global, property, array, and frame-lifecycle routes. The next flat
families are individually smaller: named-cache lookup is 164 samples (4.6%),
ordinary plus fast Number binary helpers total 271 (7.6%), direct-leaf call
wrapping is 140 (3.9%), dense-index reads are 98 (2.8%), VM construction is
86 (2.4%), and ordinary own-data reads are 78 (2.2%). Allocator entry points
also remain below the ten-percent materiality boundary.

Those concrete mechanisms are already closed. The complete compact generic
bytecode core failed all three of its external targets and catastrophically
regressed the dynamic-call control. The direct-leaf frame-stack variants and
contiguous operand stack failed their frozen gates. Moving owned Number
binary operands achieved only 0.974437x on A* and 0.988694x on HashMap. The
fixed dense-index numeric leaf targeted this exact workload and reached only
0.986477x while regressing A* to 1.095380x. Immediate property-value caching,
shared-slot promotion, lazy ownership representations, the Realm object
arena, and typed-loop object-field scalarization have likewise failed their
independent target or control gates.

No performance-unit plan, runtime patch, candidate binary, or timing gate was
created. Treating the aggregate `Value` lifetime samples as one mechanism
would combine unrelated ownership boundaries, while a global tagged-value or
tracing-GC rewrite is not a bounded consequence of this single workload
profile. Do not retry fixed dense-index leaves, direct-call frame layouts,
compact opcode streams, owned binary operands, or property-cache placement
for this case without a future exact profile that isolates a new general
representation boundary above ten percent. The queue should advance to
rank-twelve SunSpider `string-validate-input`.

### 2026-07-30 skipped string-validate-input after current profile screen

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked SunSpider `string-validate-input` twelfth at 3.4370x QuickJS-NG. The
evidence-only commits after that queue do not change the runtime, so the exact
standard-release executable remains SHA-256
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
A fresh five-second sample used the unmodified upstream source (SHA-256
`518ed0c67fde0c0d65af6238b91f5e498d8ba5d188c428f4a76dd498704737d5`)
and the existing diagnostic 80-extra-run wrapper (SHA-256
`1afe3fa8de3fc05a27b9dcbf3f454170b4fddddd655c89e2f578e2ced3c91ef5`).
It exited successfully with `Undefined`, no stderr, and 3,672 useful
main-thread runtime samples. The profile SHA-256 is
`837da0bb48f65fefe6f8ebdd00e303d861ff6e2113d73aafa245874ceadaf65a`;
the stdout and empty-stderr SHA-256 values are
`50fbe849aa61688a0dde78393afa32aba45d9f4a52109662bea06fa4c45715d5`
and
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

Generic `Vm::run_completion` contributes 609 exclusive samples (16.59%), but
its call tree distributes that dispatch work across ordinary RegExp calls,
ZIP-code `charAt` calls, binary operations, globals, properties, and frame
lifecycle rather than one new opcode. `Value` clone/drop totals 256 samples
(6.97%): 162 belong to call/frame routes, 74 to property/binding routes, and
the remainder to binary, string, and RegExp setup. Allocator and free entry
points total 837 samples (22.79%), but nearest-runtime-ancestor attribution
again splits them below the materiality boundary: RegExp matching 210
(5.72%), call/frame work 193 (5.26%), strings 169 (4.60%), binary VM work 121
(3.30%), properties/bindings 84 (2.29%), and RegExp construction or validation
60 (1.63%). No individual allocation call path exceeds 20 samples.

The matcher is the only inclusive subsystem near ten percent:
`PreparedRegexp::match_input` contains 380 samples (10.35%). Its concrete
costs are not new, however. Streaming simple-atom boundaries regressed this
case to 1.020318x; first-continuation generic repetition reached only
0.992173x; exact-one simple atoms were neutral at 0.999955x; copy-on-write
capture snapshots reached 0.968227x; and the shared literal blueprint removed
the separate construction/setup ceiling but stopped at 0.910267x, just above
its frozen target gate. The previously rejected capture-free compiled program
and captured-result materialization likewise close their respective matcher
representations. The call-frame, string-append, and generic VM alternatives
are independently closed by the borrowed-native predispatch, chunked compound
accumulator, compact bytecode, frame-stack, and operand-ownership experiments.

No performance-unit plan, runtime patch, candidate binary, or timing gate was
created. Aggregating all allocator frames or all VM dispatch frames would join
unrelated semantic mechanisms, while retrying the boundary vector,
continuation visitor, capture storage, literal blueprint, borrowed call stack,
or chunk sizing would violate their one-attempt decisions. A future attempt
requires a fresh exact profile that isolates a different general boundary
above ten percent. The queue should advance to rank-thirteen SunSpider
`access-binary-trees`.

### 2026-07-30 skipped access-binary-trees after current profile screen

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked SunSpider `access-binary-trees` thirteenth at 3.0141x QuickJS-NG. The
subsequent commits remain evidence-only, so the runtime-exact standard release
binary is still SHA-256
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
A fresh five-second sample used the unmodified upstream source (SHA-256
`af16d6f52b448094138cfd8e5f6e24c8d60772654463a4a8627cd54f910f93b5`)
and an existing diagnostic 80-copy wrapper (SHA-256
`358b2d14d51f3b053c28c667759692f525ed0fc186b885f8a675369e137c0203`).
It exited successfully with `Undefined`, no stderr, and 3,591 useful runtime
samples. The profile SHA-256 is
`26e6b954b845dbb4029ba3ec121615e371fdf17aed679c03ccae18c79c9d93a7`;
the stdout and empty-stderr SHA-256 values are
`50fbe849aa61688a0dde78393afa32aba45d9f4a52109662bea06fa4c45715d5`
and
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

Mutually exclusive call-tree attribution places 1,590 samples (44.28%) under
`TreeNode` construction and 1,980 (55.14%) under ordinary recursive or method
direct calls. The direct-call side contains 525 samples in generic dispatch,
280 in call wrappers, 216 in VM/frame lifecycle, 203 in `CallEnv` lifecycle,
184 in `Value` clone/drop, 157 in allocator entry points, and 144 in object or
property operations. This is the exact boundary already tested by the current
queue's rank-one contiguous frame stack: that candidate covered 37.4% of its
recursive profile and regressed both declared targets by 15-18%. The earlier
same-function scheduler, independent frame pools, tail-frame reset,
value-only exit, compact dispatch/core, frame cold-state, environment
shrinking, and numeric recursive cluster independently close the remaining
layout, scheduling, dispatch, and scalar-call shapes.

The constructor side is also a collection of consumed mechanisms rather than
one new leaf. Object and property work totals 408 samples (11.36%), but its
largest disjoint leaves are default-property creation 74 (2.06%), small
property lookup 68 (1.89%), ordinary receiver allocation 49 (1.36%), small
property mutation 45 (1.25%), direct own-data reads 31 (0.86%), shared-key
writes 29 (0.81%), existing-data writes 26 (0.72%), and ordinary descriptor
lookup 25 (0.70%). The other constructor families are generic dispatch 229
(6.38%), allocator entry points 209 (5.82%), `Value` lifecycle 188 (5.24%),
environment setup 159 (4.43%), VM/frame lifecycle 131 (3.65%), and call
wrappers 118 (3.29%). Even the dispatch leaf is spread over 126 sampled PC
groups; no group exceeds 75 samples, or 2.09% of the runtime profile.

Current code already retains slot-backed ordinary construction, fast missing
ordinary data-property creation, and compact small-object storage. The
straight-line constructor receiver-write leaf, constructor transition shapes,
default data-only property representation, boxed storage layout, lazy weak
reference counts, and Realm-local object arena each exhausted a frozen
attempt and were rejected. In particular, the constructor leaf reached one
target at 0.852996x but missed its independent co-target at 0.986041x, while
the ownership and arena representations regressed their object-heavy targets.
Combining those rejected paths under this tree workload would not make a new
general mechanism.

No performance-unit plan, runtime patch, candidate binary, or timing gate was
created. Do not retry recursive frame layouts, constructor receiver-write
plans, transition shapes, property payload/storage thresholds, intrusive
object counts, or Realm block arenas for this case without a fresh profile
that isolates a different representation boundary. The queue should advance
to rank-fourteen SunSpider `date-format-tofte`.

### 2026-07-30 retained idempotent dynamic-cell overlay

The exact `620bb67b` opportunity queue (SHA-256
`0b381ac46e93fd834186a2b16fe5a2c1cfbbaebf6c9c8b65c9ff3195473b37b1`)
ranked SunSpider `date-format-tofte` fourteenth at 2.7384x QuickJS-NG. A fresh
run used the unmodified upstream source (SHA-256
`cbefaffbecb6769a85f5877765b21f967a1cff5ab2625d4a9066c050fcdc7b5e`)
and diagnostic wrapper SHA-256
`e16199f1e0c97fe27b996180b3cfbb5a6f2fb6bd38328a497cd28ded8927b56c`.
It exited successfully with `Number(3971006963208.0)`, no stderr, and 3,654
useful main-thread samples. The profile SHA-256 is
`5b9e87f5169932278d2502146f1b362721069e73f77e0716b59a97ae65ed61a4`;
the stdout and empty-stderr SHA-256 values are
`ddd86a18e60d7abff8c5c55a868396b1bdd71c489b591d3098ca572f235060ad`
and
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

`Op::NewFunction` accounts for 942 samples (25.78%) and its
`frame_deopt_bindings` calls for 904 (24.74%). The formatter declares 28
nested local functions per invocation. Each closure asks the same frame to
overlay its live locals into one shared `DynamicBindings` map, even when the
same name already points at the exact same `Upvalue`. Inside the opcode,
exclusive samples include 125 in hash-map insertion, 42 in `String` cloning,
and 512 across allocation/free leaves. This is distinct from cached direct
eval's retained selected-binding environment, the rejected sparse eval frame,
and the rejected capture-repair scan: the cost is repeated identity-equal map
replacement, not binding selection, snapshotting, or writeback.

The frozen one-attempt plan
`tasks/performance-units/idempotent-dynamic-cell-overlay.json` (SHA-256
`1239e5ca61a56c65cc5d252a8f0a4c7335ec53b9fcbd074548a954903401a8e6`)
adds one identity-preserving operation to `DynamicBindings`.
`frame_deopt_bindings` now holds one short mutable map borrow and skips an
overlay only when the existing same-name cell is pointer-identical to the
incoming live cell. An absent name or a different cell still clones the name
and replaces the entry, preserving first capture, lexical shadow changes,
direct-eval visibility, and shared-cell write-through. There is no source,
workload, binding-name, checksum, or result-value condition.

The focused cell test covers first insertion, an identity-equal no-op,
write-through, and replacement by an equal-valued but distinct shadow cell.
The existing direct-eval/closure shared-cell test also passed. The staged
touched gate passed formatting, clippy, all 1,916 runtime tests, plan
validation, and 116 direct-eval/function/call Test262 cases.

The standard-recipe candidate and exact-base binary SHA-256 values are
`fb90b58b3164eda22f04954eb55698b04ef15cc0feba6d2c1053cae7b636a69e`
and
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
The one allowed fast attempt retained the target at **0.832420x** base and
2.303145x QuickJS-NG. Every frozen external control stayed below 1.03x:
HashMap 0.999663x, public-field raytrace 1.004676x, A* 1.007822x,
recursive 1.015583x, xparb 0.999025x, and tagcloud 1.009285x. The frozen
broad controls also passed: plain function call 1.002258x, dynamic method
call 1.002460x, object allocation 0.999891x, and closure allocation/call
0.997496x. The focused external manifest, raw, and report SHA-256 values are
`2b3076bc65e7908cf50fd2e664bd228dd43b49da636bd4c19fe4c18df09fb763`,
`627b215bb4db9920c8c5596a44e99736d05b7d3bc6577763075a7973fa21005a`,
and
`e907f81c0b8d02d41bf6db731639f1b55f34505c301a18b63db0fdf2948203e9`;
the focused broad raw SHA-256 is
`7da92adf11cb89d5fb0754c4addd228d6c33d4e8aeff489b11e4bfdf6f0d6b45`.

A complete 25-case pre-commit broad diagnostic (raw SHA-256
`10b6e19a5eafcd2fcd9fb1b5daa6982ad0cc2142284b5889eccc1bbdea4a15a4`)
completed all 225 formal samples. Candidate/base geometric mean was
1.009999x and every broad candidate/QuickJS-NG ratio remained below 0.5x.
The run exposed a reproducible non-control `array_dynamic_read` code-layout
regression: 1.180618x in the full matrix and 1.177266x in an isolated recheck
(SHA-256
`78a49095e835e616cc4cfa65aef46565675603fda97987eba83a07171e870a46`).
That case still measured 0.399485x QuickJS-NG and was not a frozen control,
so it does not override the one-attempt decision; it remains explicit debt
for a separately profiled unit. The same recheck reduced the full-run
`captured_write` 1.036574x observation to 1.004232x.

The default 15-second external run completed 44 of 45 cases; candidate and
base timed out only on the known Kraken `imaging-gaussian-blur` capability
case. A manifest-bound 60-second rerun closed that gap and completed all 45
cases for all three roles. Its manifest, raw, report, and summary SHA-256
values are
`2df77d269c535af13879b3392da56c13db78f165582521c2dd4721725d88d354`,
`d1207acb8fd45ee8cf862df386da352469595a19159c95ec540920e46345e0a2`,
`349a5141c8e4901127fa76c131bcf0e20b05de4cb7112164d6fee261c8e9adea`,
and
`1f950f9145980cfc692f28ce3dd25bf25f2e34031aa640186146512622b07718`.
The target was **0.829725x** base and 2.286054x QuickJS-NG. JetStream,
Kraken, and SunSpider candidate/base geometric means were 0.999159x,
0.998724x, and 0.993245x. All frozen external controls again stayed below
1.012x, and the largest individual external base ratio was the non-control
`crypto-md5` at 1.019531x.

`./scripts/check.sh` passed formatting, clippy, agent-feature tests, all
workspace tests, 211 benchmark-tool tests, every performance-plan validator,
and all 5,160 curated Test262 cases; `./scripts/compare-qjs.sh` passed every
fixture. The implementation commit is
`10ba7595c07748318e9708a9ede15c3a6778dcbd`, pushed on `main`. A subsequent
owned-harness preview bound that clean commit to candidate binary SHA-256
`fb90b58b3164eda22f04954eb55698b04ef15cc0feba6d2c1053cae7b636a69e`,
exact base `620bb67b9c7c36714177b7006a7ccaddd88653f5` / binary SHA-256
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`,
and QuickJS-NG `f7830186043e4488f2998759d60a514faf07cbc9` / binary SHA-256
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
The candidate, base, and QuickJS-NG receipt SHA-256 values are
`efb969ac779cc3d06e3aa9eb20efb271f2ec23be9cf700c071d1337d8447868f`,
`9282b1711ca5434d6dbb8ad0ba28488c6ce19c077160799fe8897758ffc17564`,
and
`b96f63f100f0ddfb6e7dedfdbcb195f579144cf01af814706d3d7a1b24b90b0f`.

The exact-commit 25-case broad run completed all three blocks. Its raw,
report, and summary SHA-256 values are
`803324e9731f4d38ab1a458d9591cbe1ccad9b06de4d2a28bcf2ffdfaedf7394`,
`b69723ba382787527e6afd67bce09b2034ed8000361bc54c7eb0f0d56695f58f`,
and
`7688b26106961774151b631df5d8ca2215eac7ede4c42d1d7f9024ceb8de8f52`.
Candidate/base geometric mean was 1.005690x, and all candidate/QuickJS-NG
ratios remained below 0.5x. The reproducible non-control
`array_dynamic_read` observation remained explicit at 1.177053x base and
0.400747x QuickJS-NG; it is outside the frozen control set and still requires
a separately profiled unit rather than post-hoc retuning of this one-attempt
plan.

The exact-commit 60-second external rerun completed all 45 cases for all
three roles. The separately archived timeout-override manifest is still
SHA-256
`2df77d269c535af13879b3392da56c13db78f165582521c2dd4721725d88d354`;
the runner also copied the canonical 15-second registry manifest, SHA-256
`a8ddeded582573bc676bf3f7bbbaf2625f6dfa7742f07bcdd6aaa26366f4e6c4`,
into its output. The exact raw, report, and rendered-summary SHA-256 values are
`6a142c59d4d0524adf2bbc6d573449f3ff141923be6dac620b557ba9b1f1b5a1`,
`d9d1433866bd153288c13e0de20548d8e95c60f6d82f6365a0bbab6394dc04a9`,
and
`a7b31a175da31b112a763b1f8e2ff88bc50f4d96802e83d22c9ea14a4af6f08c`.
The target reproduced at **0.832444x** base and 2.281x QuickJS-NG.
JetStream, Kraken, and SunSpider candidate/base geometric means were 1.003x,
0.999x, and 0.996x. The previously missing Kraken Gaussian case completed at
1.003x base and 3.991x QuickJS-NG.

The exact-commit Test262 Coverage artifact from run `30605695518` is SHA-256
`f35a66049f4a55e233884209dd1da58d88402615177f46cd7e20ae30ad55f8ef`.
It covers all 53,572 pinned cases: 10,900 are outside the QuickJS-NG
configuration, while qjs-rust passes all 42,672 configured cases with zero
failures, zero timeouts, and zero actionable gaps against QuickJS-NG. The
hash-bound fast and promotion decisions both retain the unit; their SHA-256
values are
`945336794035ac26f54c7e6e69b899375117c1f09988da0708f3e7539a8ee1f7`
and
`22e50a688c067f8ecd25ff9825857b4adb71b35b5d7d705eda68a08ed8995624`.
The promotion receipt binds the queue, immutable plan, exact broad report,
complete external report, and zero-gap Test262 artifact. This closes the unit
as a semantics-preserving retained optimization; it is not campaign
completion because most external cases remain above 0.5x QuickJS-NG. Refresh
the queue from `10ba7595` and continue from its highest unclosed shared cost.

### 2026-07-30 re-screened refreshed rank-one recursive cost

The exact `10ba7595` opportunity queue (SHA-256
`b5568ff2525a585d6b9cbcfbf3f2a1342239e1e2444336316169e8c26f47ac60`)
again ranks SunSpider `controlflow-recursive` first, now at 5.8534x
QuickJS-NG. The documentation-only `9d66a8ba` commit does not change the
runtime, so a fresh five-second sample used the exact promoted candidate
binary SHA-256
`fb90b58b3164eda22f04954eb55698b04ef15cc0feba6d2c1053cae7b636a69e`,
the pinned upstream source SHA-256
`7689048105ae415ad60df2a882384063df640371d806d7a7d91446f2881b7d83`,
and the existing source-faithful 100-repeat wrapper SHA-256
`52ecb05f622d41dd35db1d476f1f5c46d16e252efa72946516b5396c33f56261`.
The process exited successfully with `Undefined`, no stderr, and 2,391 useful
main-thread runtime samples after excluding the initial dyld segment. The
profile SHA-256 is
`4e7f4a8f94b3762ac452cfb4b16a3db4e24f6bd4528cc0149adc9574c914b0f3`;
the stdout and empty-stderr SHA-256 values are
`50fbe849aa61688a0dde78393afa32aba45d9f4a52109662bea06fa4c45715d5`
and
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

The current profile reproduces the already-closed recursive-call boundary.
Generic `Vm::run_completion` dispatch contributes 763 exclusive samples
(31.91%), `call_direct_leaf_function` contributes 381 (15.93%), and
`Vm::new_with_globals_upvalues_with_stack_and_direct_call_slots` contributes
233 (9.74%). `Value`, `FrameState`, and `CallEnv` destruction contribute 124,
88, and 47 samples respectively; binary evaluation and `Value` cloning are
only 62 and 52. The remaining direct-call helpers, stack recycling, local
loads, numeric probes, and allocator leaves are individually below two
percent or split across independent mechanisms.

There is no new general mechanism in those samples. The compact dispatcher
reached only 0.9573x, the scalar recursive cluster failed its A* control after
reaching 0.2238x on this target, the direct-leaf cold-state and lazy-loop-state
representations missed their frozen gates, the exact tail-frame reset stopped
at 0.9418x, and the contiguous frame-stack transfer regressed both recursive
and HashMap targets by 15-18%. Earlier same-function scheduling, frame
packing, environment grouping, result materialization, and operand-stack
reuse experiments close the remaining construction and teardown shapes.

No performance-unit plan, runtime patch, candidate binary, or timing gate was
created. Retrying one of those mechanisms with a different threshold or
layout would violate its frozen one-attempt decision, while treating generic
dispatch plus all call lifecycle work as one cost would combine independent
representations. Advance the current queue to rank-two JetStream HashMap and
require its fresh profile to expose a different shared boundary before
implementation.

### 2026-07-30 re-screened refreshed rank-two HashMap cost

The exact `10ba7595` queue ranks JetStream `hash-map` second at 5.3476x
QuickJS-NG. The runtime remains byte-for-byte identical through the later
evidence-only commits, so a fresh five-second sample again used exact promoted
candidate binary SHA-256
`fb90b58b3164eda22f04954eb55698b04ef15cc0feba6d2c1053cae7b636a69e`.
The pinned upstream source SHA-256 is
`9789c4d06f12ee4e4836c669b5515a40c6caaf5e9321c08da548740225fb46fb`;
the source-faithful diagnostic wrapper repeats the official iteration four
times only to sustain sampling and has SHA-256
`c67d6168472c68ec2a902fbd8d0b1c9d49b725233d06ac7db25263e18e7625fe`.
The process exited successfully after printing `__QJS_EXTERNAL_OK__` and
`Undefined`, with no stderr and 3,630 useful main-thread runtime samples. The
profile SHA-256 is
`b238cfde83f142f18c6e1ae4f31be5d6bc7feb6211ebab4cea4a184b531cf582`;
the stdout and empty-stderr SHA-256 values are
`89eb657835671c4963858500b421a87c683cf5e381c80ba84af40da3055372ba`
and
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

The current profile reproduces the same already-partitioned costs as the
earlier exact HashMap evidence. Generic `Vm::run_completion` dispatch
contributes 1,186 exclusive samples (32.67%). Direct-leaf call setup and
lifecycle remain distributed across `call_direct_leaf_function` (321,
8.84%), VM construction (187, 5.15%), `FrameState` destruction (78, 2.15%),
and `CallEnv` destruction (54, 1.49%). `Value` destruction and cloning
contribute 206 (5.67%) and 135 (3.72%). The largest named-property leaves are
`own_data_property_read` at 137 (3.77%), cache lookup at 52 (1.43%), and the
cached and direct string probes at 41 (1.13%) and 32 (0.88%). No remaining
property, allocation, binary-operation, or call helper is independently ten
percent of the profile.

Those clusters do not establish a new mechanism. The complete compact
bytecode core, contiguous frame stack, direct-leaf cold state, lazy loop-plan
state, direct read-only upvalue sharing, argument moves, and Number-operand
ownership units already tested the dispatch, frame, and value-lifetime
routes. The retained shared small-object slot cache already validates one
interned field key across distinct receivers; transition shapes, default-data
storage, larger compact storage, immediate object-slot installation, and
object-header layout experiments cover the property-storage alternatives.
Combining those individually smaller costs into a nominal HashMap fast path
would be workload specialization rather than a profile-backed shared engine
mechanism.

No performance-unit plan, runtime patch, candidate binary, or timing gate was
created. Advance the refreshed queue to rank-three Kraken A* and require a
fresh exact-current profile to expose a distinct general cost before opening
another one-attempt unit.

### 2026-07-30 re-screened refreshed rank-three A* cost

The exact `10ba7595` queue ranks Kraken `ai-astar` third at 4.8649x
QuickJS-NG. A fresh five-second sample used the same exact promoted candidate
binary SHA-256
`fb90b58b3164eda22f04954eb55698b04ef15cc0feba6d2c1053cae7b636a69e`
and the source-faithful generated adapter SHA-256
`a3653c77773ce2b424301835021957b26119240810f43d5434d98fd88d7a416c`.
The pinned Kraken source and data files have SHA-256 values
`ab1778d3625e51a9e54a24a7692a7729ae88922162ea3bfbe6acfb5562247467`
and
`22366873662558d87ff8ed05e5c65085d7f3a619c9ab62dee7078daba5ae33d0`.
The process exited successfully after printing `__QJS_EXTERNAL_OK__` and the
matching String completion, with no stderr and 3,649 main-thread samples. The
profile SHA-256 is
`cb2a5529ab5edc595344145223bd244afcc5fa57971900edcc5f10985025da21`;
the stdout and empty-stderr SHA-256 values are
`f5bc4f369844bf414bcaa550808d7e5406037ad4da77bde3ae30fcaa7701bdfc`
and
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

The current profile again isolates the previously tested A* mechanisms.
Generic `Vm::run_completion` dispatch contributes 1,749 exclusive samples
(47.93%), and `eval_binary` contributes 553 (15.15%). `Value` destruction and
cloning contribute 275 (7.54%) and 162 (4.44%). Named-cache lookup and its
cached/direct probes contribute 177 (4.85%), 134 (3.67%), and 55 (1.51%);
global loads contribute 98 (2.69%), and dense-index reads contribute 59
(1.62%). Every other binary, update, loop-tier, allocation, and property
helper is individually near or below one percent.

The only independent cluster above ten percent is the ordinary binary route,
and its frozen owned-Number experiment already moved the two popped Number
operands through the existing Number helper. It reached only 0.974437x on A*
and 0.988694x on HashMap and was rejected. The complete compact bytecode core
closed the generic-dispatch alternative. The remaining object chain is also
not new: typed-loop numeric-object scalarization regressed A* to 1.154586x,
the narrower dense-object Number fusion regressed it to 1.024107x, and the
small/dynamic storage, transition-shape, shared-slot, immediate-cache, and
store-cache experiments already cover the cache and property representations.
The current profile supplies no evidence that another threshold or fusion of
those paths would behave differently.

No performance-unit plan, runtime patch, candidate binary, or timing gate was
created. Advance the refreshed queue to rank-four SunSpider `string-tagcloud`
and require a different profile-backed RegExp or String representation rather
than retrying the closed binary, typed-loop, or named-cache mechanisms.

### 2026-07-30 rejected inline RegExp match-state lists

The exact `10ba7595` queue ranks SunSpider `string-tagcloud` fourth at
4.1335x QuickJS-NG. A fresh exact-current sample used promoted candidate
binary SHA-256
`fb90b58b3164eda22f04954eb55698b04ef15cc0feba6d2c1053cae7b636a69e`,
official source SHA-256
`9634886bcb846c76141f97b681b33a11df444901e0b4c899d8f72a3b6544f9d5`,
and diagnostic wrapper SHA-256
`5b4e3d1d0561062006c8bc12c700b7fc7769ca12f11a04674259184b13c41264`.
The successful process produced 2,925 main-thread samples, 811 under
`PreparedRegexp::match_input`. The hot captured-group route put 212 samples in
`repeat_atom`; 197 immediately entered `Vec::from_iter`, and 192 continued
through its iterator while nested `match_pattern` and `match_atom` calls
mostly returned empty or singleton state lists. The profile, stdout, and
empty-stderr SHA-256 values are
`48d7b28c8b6caaa21026bf3d8b28e288e1a5ce88249cd5ecb0ee1c03e33a042f`,
`89eb657835671c4963858500b421a87c683cf5e381c80ba84af40da3055372ba`,
and
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

The frozen one-attempt plan
`tasks/performance-units/inline-regexp-match-state-list.json` (SHA-256
`265d9dec87ce730f98da0f1cd68a08f59052c9540d18e7ed1b5dd46ff7bb3dcc`)
gave the general matcher one inline `MatchState` in each transient state or
indexed-state list and retained ordered heap spill for arbitrary alternation
and backtracking fan-out. This changed neither captures nor continuation
order, so it was distinct from the rejected capture COW and first-continuation
mechanisms. The implementation used `smallvec` 1.15, already present in the
workspace lockfile through tooling, rather than introducing a custom unsafe
container. A focused structural test proved inline singleton storage plus
ordered spill with independent capture vectors, and all 44 matcher tests
passed.

The standard-recipe candidate binary SHA-256 was
`b5b1a4a3cf78408d49f6af8562dcb45d27c915fd4c74885b3f5ebfe70204352c`;
the exact base binary SHA-256 remained
`c7b9b627e6b03e1e08c80967be6ee43e81b34401d130bd94ab754f9ef2b2f81b`.
A seven-block seeded role-rotation screen used a three-suite diagnostic
manifest SHA-256
`37dac1cb9578e3bf37f7057d8ef018a8345a4e8ef3f8d377ab9656ac376f5b86`.
Every process completed with matching capability. Tagcloud measured
194.795 ms candidate versus 194.955 ms base, or **0.999179x**, far above the
frozen `<= 0.90x` target. The independent JetStream Gaussian and Kraken JSON
parse controls were neutral at 0.998059x and 1.006104x. Raw and report
SHA-256 values are
`cc61b480f452b1d664b6d79fd9908b6ae08a68d9ae1e298e1c0ded69df78b67a`
and
`7f8145d25ec9816826a2c13f6d6739dd702ea07d218faf28ef2223d7baa4d514`.

The target failed before the remaining frozen controls or any broad,
full-external, or Test262 promotion run was warranted. The runtime, focused
test, direct dependency, and lockfile changes were reverted; the restored
runtime compiles cleanly. Do not retry singleton capacity, inline width,
container choice, or spill threshold: removing the result-list heap boundary
is not a material whole-workload mechanism here. Advance the refreshed queue
to rank-five public-class-field raytrace.

### 2026-07-30 re-screened refreshed rank-five public-class-field raytrace cost

The exact `10ba7595` queue ranks JetStream
`raytrace-public-class-fields` fifth at 4.0154x QuickJS-NG. A fresh run used
exact promoted candidate binary SHA-256
`fb90b58b3164eda22f04954eb55698b04ef15cc0feba6d2c1053cae7b636a69e`,
pinned upstream source SHA-256
`5bb6cbf1f8c771604921eee1c1d8dcc7e5de3b8005fe5489bad9b29c741c6697`,
and source-faithful wrapper SHA-256
`824daa5582289787f6e25a200892a1d6bdfa682afe9ce76a30e520dc7e03528c`.
The process exited successfully after printing `__QJS_EXTERNAL_OK__` and the
matching String completion, with no stderr and 1,324 main-thread samples. The
profile SHA-256 is
`ab13f9382f7d8c12a8003bd31461653ba48a82d7d63986daf51883aa80eb4ae9`;
the stdout and empty-stderr SHA-256 values are
`f5bc4f369844bf414bcaa550808d7e5406037ad4da77bde3ae30fcaa7701bdfc`
and
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

The refreshed exclusive profile confirms that no class-field-specific cost is
independently material. `initialize_instance_fields` contributes 12 samples,
`call_field_initializer` 9, `ObjectRef::set_shared_key` 14, and
`PropertyStorage::insert` 7: 42 samples in total, or 3.17%. The remaining
cost is divided among generic `Vm::run_completion` dispatch (242, 18.28%), VM
construction (53, 4.00%), direct-leaf invocation (47, 3.55%), generic
`function_env` work (41, 3.10%), `Value` clone/drop (104, 7.85%), allocator
entry points, frame teardown, and named-property operations. Apart from the
already-tested generic dispatcher, none is independently ten percent of the
profile.

These are the same closed mechanism families seen in the earlier raytrace
screen. Base-class constructor direct slots, isolated shared-key field
installation, transition shapes, default-data storage, immediate slot reads,
direct read-only upvalue sharing, the contiguous frame stack, compact generic
dispatch, and ownership-layout experiments already tested the corresponding
constructor, field, property, call, and lifetime boundaries. Combining those
distributed leaves into a raytrace-specific constructor or field path would
not establish a new general engine mechanism.

No performance-unit plan, runtime patch, candidate binary, or timing gate was
created. Do not retry public-field key sharing, field installation, constructor
shape seeding, or generic frame packing without a future exact profile that
exposes a new independently bounded shared cost above ten percent. Advance the
refreshed queue to rank-six Kraken `imaging-gaussian-blur`.

### 2026-07-30 retained guarded unary Math call opcode

The exact `10ba7595` opportunity queue (SHA-256
`b5568ff2525a585d6b9cbcfbf3f2a1342239e1e2444336316169e8c26f47ac60`)
ranked Kraken `imaging-gaussian-blur` sixth at 3.9908x QuickJS-NG. A fresh
five-second exact-current sample (SHA-256
`b10415352db0d939a25d9cf431726df19f13b5376be5b8ca4976635129d1d04c`)
contained 7,410 main-thread samples. The ordinary `Math.abs(number)` call
boundary contributed a 16.63% cluster across resolved-call setup, native
dispatch, argument movement, and result materialization. An earlier borrowed
native predispatch proved the target causal at 0.787794x but regressed the
independent `object_allocation` control to 1.077990x, so the new unit used a
dedicated bytecode representation rather than changing generic call dispatch.

The frozen one-attempt plan
`tasks/performance-units/guarded-math-unary-call-opcode.json` (SHA-256
`aa05f4db88fc4443ff06a3b7ee75c13cb099428717dd216a23601b5d5f3d21f2`)
introduced `CallResolvedGuardedMathUnary` only for direct one-argument static
calls to the existing supported unary `Math` family. Receiver lookup, property
lookup, and argument evaluation retain ordinary source order. At execution the
operation requires the live callee's exact native identity and a primitive
Number argument; otherwise it replays unchanged `CallResolved(1)` without
first mutating the stack. Getter, rebinding, coercive-object, computed-property,
missing/extra/spread-argument, and unsupported-native cases therefore remain on
their existing semantic paths. The VM, compiler, stack-flow analyzer, numeric
loop recognizer, and typed-loop compiler share that private operation; no
parser, AST, public API, dependency, or environment-model boundary changed.

Focused tests cover IR admission and exclusion, the intrinsic hit, getter/call/
argument/receiver order, live replacement, and coercive fallback. All 1,918
runtime tests and the 5,160-case curated Test262 subset passed. The staged
touched gate, `./scripts/check.sh`, `./scripts/compare-qjs.sh`, the pre-push
gate, and exact-commit CI run `30612421572` also passed. The retained runtime
commit is `0745c0187a005de3c5cbe0d4c317fd9e5a9b3536`; candidate, exact-base,
and QuickJS-NG executable SHA-256 values are
`c0eded40cbf77482bb8e6e106334c65e3a1fc45b5f8006d1ad31a1899cd14b56`,
`fb90b58b3164eda22f04954eb55698b04ef15cc0feba6d2c1053cae7b636a69e`,
and
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
Their build-receipt SHA-256 values are
`cc381f7003e9d5f77dab848d6e6ca0e9c0935beae71edc58161b89c633375d49`,
`efb969ac779cc3d06e3aa9eb20efb271f2ec23be9cf700c071d1337d8447868f`,
and
`b96f63f100f0ddfb6e7dedfdbcb195f579144cf01af814706d3d7a1b24b90b0f`.

The five-block exact-binary target screen measured Gaussian blur at
**0.783654x candidate/base**, below the frozen `<= 0.90x` gate. All thirteen
declared controls remained below 1.03x: darkroom 0.973639x, 3d-morph
1.000201x, audio FFT 0.988352x, A* 0.989706x, public-field raytrace
0.991797x, Tagcloud 0.984018x, `math_abs` 0.999845x, plain call 1.001475x,
two-argument call 1.000086x, dynamic method call 0.999748x, property read
1.000999x, dynamic array read 0.855495x, and object allocation 1.002583x.
The target and control receipt SHA-256 values are
`1317ef440cbcb7941260a1835973582d8715939edefaee80d119bc183a4c9475`
and
`729d09a2a22aa0f32759ab5790ba10b289fc58985c0a3c3a3ca64c976378009d`;
the hash-bound fast decision retains the unit at SHA-256
`5218e480ea209caf2a625f35a6bbef7ae1539c24eefa4f2e7f505de368c86af3`.

The exact 25-case broad matrix completed all three role-rotation blocks.
Candidate/base geometric mean was 0.9910x with interval 0.9811x-0.9922x;
the worst broad case was `math_abs` at 1.002785x, and every broad case remained
below 0.5x QuickJS-NG. Manifest, raw, report, and summary SHA-256 values are
`5c3c586de23444190437652cf99a57da46730a4473d22e281c9e388f8c4215c0`,
`4859cc045b5bad0d97a02dd11e079e383ad12ad8b337a06c8f3052e92550e63e`,
`da9b219a21737fcc30202573ee221b30163ee6542fed32e73ec545697cb2f841`,
and
`b65dfdd95ff990cb256179be5d1370def4f0316450dbdae7e5d2cb29b7560acb`.

The exact 45-case external portfolio completed all three 60-second blocks.
Gaussian blur reproduced at **0.779639x candidate/base** and 3.1130x
QuickJS-NG; JetStream, Kraken, and SunSpider candidate/base geometric means
were 0.996x, 0.973x, and 0.999x. Two very short full-run outliers were audited
with 31 paired blocks: `bitops-bitwise-and` resolved to 0.996166x and
`math-cordic` to 1.022725x. The outlier receipt SHA-256 is
`7f714131176518481d554c7c29e98d8606f7ccc9ae5242298873dd5029a1addc`.
External manifest, raw, report, and rendered-summary SHA-256 values are
`2df77d269c535af13879b3392da56c13db78f165582521c2dd4721725d88d354`,
`a2551254bc48a23538e90bb9068902ff6eb613a857510b2fa6167ab577239218`,
`41d45bd5e921e4602c402c24f1afbd5eee3788676ae2cd872d6c4de25d777e00`,
and
`8d8705833e06173e41a80743f4427070467e041f58fb8ac5a81cada88af2299e`.

Exact-commit Test262 Coverage run `30612682674` passed and produced burndown
SHA-256
`aa760a75346aa2ef6e85c2e9b854fbff3871137ab9adc9658d6c8ebdcd61c546`.
It covers all 53,572 pinned cases: 10,900 are outside the QuickJS-NG
configuration, while qjs-rust passes all 42,672 configured cases with zero
failures, zero timeouts, and zero actionable gaps. The promotion decision
retains the unit and has SHA-256
`f7a3c5e77d339a43b52d8d996220e02028136dcbc8fa73033037957484e4e067`.

The refreshed exact `0745c018` queue has SHA-256
`664737b4614cbdd6d6628114250a0e13332e6e35234b5a9aefb7c7a093d16e02`.
Gaussian moves from rank six to rank twelve at 3.1130x QuickJS-NG; no broad
case is above the campaign threshold, but 38 external cases remain above
0.5x. This closes the opcode unit, not the campaign. Continue from the new
queue's highest unclosed current-profile mechanism.

### 2026-07-30 rejected cached direct-leaf numeric call graph

The exact `0745c018` queue ranks Kraken `imaging-darkroom` seventh at 3.5275x
QuickJS-NG. A fresh exact-current sample used promoted candidate binary
SHA-256
`c0eded40cbf77482bb8e6e106334c65e3a1fc45b5f8006d1ad31a1899cd14b56`
and the source-faithful bundle SHA-256
`bfc464b658a5b69810769631928842241a46d8d9bfbfb62da53804843eadfaba`.
The successful profile contains 3,694 main-thread samples; 3,500 are below the
`ProcessImageData` direct frame and 1,305 (35.33%) enter its immediate repeated
direct-leaf child route, followed by deeper `FastGain`, `FastBias`, `FastLog2`,
`Clamp`, and Math frames. The profile SHA-256 is
`64d87294cf2a00e7ee7a4bdf75b896abf1d0f3b1d65f96ea8b1cdfd4bebd6383`.

The frozen one-attempt plan
`tasks/performance-units/cached-direct-leaf-numeric-call-graph.json` (SHA-256
`55e78cad4927272d215f44bc93d4fa79d84a1ec808b1d7baff657b92fcf721bd`)
tested a bounded acyclic Number-only helper graph cached in the existing lazy
numeric-leaf plan. Admission was bytecode-structural and limited to pure
Number operations, forward branches, read-only Number or ordinary-function
captures, and guarded Math data properties. The cache held only immutable
graph data, weak bytecode/object references, and raw identity addresses; each
call re-read live captures and guarded the root, nested functions, realm Math
object, property revision, and native identity before scalar evaluation. It
did not change parser, AST, environment, `FunctionData`, generic VM dispatch,
or typed-loop paths. Focused tests covered both branches, live Number capture,
Math and nested-function replacement fallback, coercive argument fallback
before effects, and exact `NumericLeafPlan` layout parity. `cargo check`,
clippy with warnings denied, and all five focused tests passed.

The standard-recipe candidate binary SHA-256 was
`6e2fa211af85b2162f1b4f158ff1740837f5c7fa120e5ffb04a2463b3249cca9`;
the exact base remained
`c0eded40cbf77482bb8e6e106334c65e3a1fc45b5f8006d1ad31a1899cd14b56`.
The blocked alternating-process fast gate preserved byte-identical successful
output and measured the target at **0.652603x candidate/base**, well below the
frozen `<= 0.80x` target. Seventeen of eighteen controls remained below the
1.03x ceiling: recursive 1.007518x, HashMap 1.012698x, A* 0.999380x,
tagcloud 1.000760x, raytrace 1.004328x, CDJS 0.996820x, base64 1.000545x,
3d-morph 1.004261x, nbody 0.999435x, audio FFT 1.003926x, Gaussian blur
0.989994x, dynamic array read 1.002753x, property read 1.001226x, plain call
0.999535x, two-argument call 1.001733x, dynamic method call 0.999755x, and
object allocation 1.002590x. The receipt SHA-256 is
`29512e6412ac635f2ef5f529f3c556c42ac89f43fdf9268eb12d97fa6aac5b7d`.

The remaining broad `math_abs` control measured 1.039981x across nine paired
blocks. Because that exceeded the frozen 1.03x ceiling, an isolated 31-block
alternating-process recheck was permitted only to distinguish noise, with no
code or gate change. It confirmed **1.040674x**, again with byte-identical
successful output; the recheck receipt SHA-256 is
`6445b4d0adc1d9c0a10e71c668930edf60f423c732b747d2e9203d5e362e2948`.
The target gain therefore does not compensate for the reproducible unrelated
broad regression. The one allowed attempt is rejected before complete broad,
external, or Test262 promotion. The runtime and focused-test implementation
was reverted; retain only the frozen plan and this evidence. Do not tune graph
bounds, cache shape, enum boxing, or instruction layout under this unit.

### 2026-07-30 retained realm String prototype direct read

The exact `0745c018` opportunity queue (SHA-256
`664737b4614cbdd6d6628114250a0e13332e6e35234b5a9aefb7c7a093d16e02`)
ranked SunSpider `string-base64` eighth at 3.4738x QuickJS-NG. A fresh
source-faithful exact-current sample contained 3,654 main-thread samples; 518
(14.18%) entered primitive String prototype resolution, including environment,
constructor, prototype, function, property, and hash work. The eventual
`charCodeAt` native leaf contributed only 12 exclusive samples, so the unit
targeted the shared lookup boundary rather than the builtin. The profile
SHA-256 is
`edbf7d0eed078b2ac18111d567d7341a0e22b4dd8a2fa379692b7fc283afa02a`.

The frozen one-attempt plan
`tasks/performance-units/realm-string-prototype-direct-read.json` (SHA-256
`ae0f3a61eb6b2b3977bc892ec3a589474a0ee42fe1af781225c9ef04e13ae289`)
adds the canonical live `%String.prototype%` object to the existing realm
intrinsic state. After preserving primitive `length` and indexed-character
precedence, ordinary named String reads use the existing ordinary-chain data
reader directly on that live prototype. Data replacement remains observable;
accessors, proxies or exotic chain entries, borrow conflicts, and marked
dynamic cross-realm globals replay the existing generic path. Rebinding the
global `String` name no longer changes primitive intrinsic lookup. The cache
stores neither a property value nor a site guard, and no bytecode, frame,
parser, AST, public API, dependency, or environment-model boundary changed.

Focused tests cover global constructor rebinding, live prototype data
replacement, accessor fallback with the strict primitive receiver, and marked
dynamic cross-realm fallback. All 1,919 runtime tests and the 5,160-case
curated Test262 subset passed. The staged touched gate, `./scripts/check.sh`,
`./scripts/compare-qjs.sh`, the pre-push gate, and exact-commit CI run
`30617551260` also passed. The retained runtime commit is
`4023b3dd4d677ad349fd0e677289dead840d7dab`; candidate, exact-base, and
QuickJS-NG executable SHA-256 values are
`498e5bd03ce89f2f8100f77f2e9026da3f24903b2d617897688b235b37af6d81`,
`c0eded40cbf77482bb8e6e106334c65e3a1fc45b5f8006d1ad31a1899cd14b56`,
and
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
Their build-receipt SHA-256 values are
`0264066a3f864429d14853f893c98ef3b21e83d89a9cc6a6fde20d49c116fe06`,
`cc381f7003e9d5f77dab848d6e6ca0e9c0935beae71edc58161b89c633375d49`,
and
`b96f63f100f0ddfb6e7dedfdbcb195f579144cf01af814706d3d7a1b24b90b0f`.

The 31-block exact-binary fast target measured `string-base64` at
**0.832831x candidate/base**, below the frozen `<= 0.92x` gate. All sixteen
declared controls remained below 1.03x; the worst was A* at 1.011280x, while
`string-validate-input` was 0.947844x and the remaining broad/string/external
controls were between 0.969885x and 1.008188x. Successful output was
byte-identical. The target and control receipt SHA-256 values are
`ef1694fbb67cd6a94cdee84e2845eb4b21a6911ffaf007f004d749987420629c`
and
`cffc75763c68cb1c57c4ccd66a79a2501fff3dceffcaf65de06bd35d74ca018c`.

The exact 25-case broad matrix completed all three role-rotation blocks.
Candidate/base geometric mean was 1.000229x with interval
0.999335x-1.001389x; the worst broad case was `branch_arithmetic` at
1.005956x. Every broad case remained below 0.5x QuickJS-NG, with the worst
`closure_allocation_call` at 0.433197x. Manifest, raw, report, and summary
SHA-256 values are
`5c3c586de23444190437652cf99a57da46730a4473d22e281c9e388f8c4215c0`,
`0f983db89e47d9156e1249c4bd6ea6c8d90d9c00f680051807ef114caba493ac`,
`7006249fdcd24ddd2d1434e1e862705faa66175d9ace6715dc7c279cf36c1a8f`,
and
`48a4d2a9223de1084c0da7fe63cdccee843c690e6883cd184b912e98a4cffc59`.

The exact 45-case external portfolio completed all three 60-second blocks.
Base64 reproduced at **0.839687x candidate/base** and 2.8750x QuickJS-NG.
JetStream, Kraken, and SunSpider candidate/base geometric means were
0.994487x, 0.993841x, and 0.985502x. The worst external candidate/base row was
`math-partial-sums` at 1.018490x, below the frozen 1.03x ceiling, so no
post-hoc outlier remeasurement was needed. External manifest, raw, report, and
rendered-summary SHA-256 values are
`a8ddeded582573bc676bf3f7bbbaf2625f6dfa7742f07bcdd6aaa26366f4e6c4`,
`1841998fd79ea5d57dce649e902378cb6c35f8cd099ce0eb00a9c1a165d11971`,
`fc6dddbcbdc8a1404646e0ab5cc810515c15ce3bfa5c9354a1283c67453ab366`,
and
`388da8cf92d390de453894268dd7d7e364b0d7c78b8372923ee020fcf053c86e`.

Exact-commit Test262 Coverage run `30617806617` passed and produced burndown
SHA-256
`36b7545c38345a1b81bb87d1246b69d8a2bed076cec6d584452a1df4c7dfa1b1`.
It covers all 53,572 pinned cases: 10,900 are outside the QuickJS-NG
configuration, while qjs-rust passes all 42,672 configured cases with zero
failures, zero timeouts, zero not-run cases, and zero actionable gaps. The
promotion decision retains the unit with no exception and has SHA-256
`26f8261e7df45e6112d9aa57c1dce4a0608f298535acd4333ffb49586dd9cf97`.

The refreshed exact `4023b3dd` queue has SHA-256
`1a68df2ccd12ed444316096150c6f4e75968bf2c1f6687fa082832a73f312f50`.
Base64 moves to rank thirteen at 2.8750x QuickJS-NG; no broad case is above
the campaign threshold, but 38 external cases remain above 0.5x. The new
rank-one opportunity is SunSpider `controlflow-recursive` at 6.0132x. This
closes the realm String prototype unit, not the campaign; continue from that
fresh queue and a new exact-current profile.

### 2026-07-30 retained branchy nested dense typed loops

The exact `4023b3dd` opportunity queue ranked Kraken
`imaging-gaussian-blur` eleventh at 3.1320x QuickJS-NG after the guarded unary
Math opcode. A fresh source-faithful sample contained 3,696 main-thread
samples: bytecode dispatch accounted for 1,383 (37.4%), `Value` clone/drop for
694 (18.8%), generic binary operations for 402 (10.9%), and the remaining Math
path for only 23 (0.6%). The profile SHA-256 is
`892f7c436301021ce1d8d81d8447ed472dadf183169cae5d2c130453eea30052`.
Bytecode inspection identified the shared missing mechanism: four nested
numeric loops with truthy short-circuit bounds and a two-level dense read,
`kernel[Math.abs(j)][Math.abs(i)]`, were not admitted by the typed-loop tier.

The frozen one-attempt plan
`tasks/performance-units/typed-loop-branchy-nested-dense-read.json` (SHA-256
`f5eb4a9b20e219006803d10791c00698c451444922493d42e81ed767f9f1dc81`)
extends the existing typed-loop compiler rather than adding a benchmark leaf.
The compiler now admits truthy conditional branches through its existing
numeric-not operation, skips unreachable instructions until a known reachable
target, and learns which intermediate dense reads must remain boxed for a
subsequent indexed read. The existing guarded element-read operation and
deoptimization stack preserve live array semantics; getters, non-array
intermediates, and unsupported shapes replay the generic VM path. Parser, AST,
environment, public API, dependency, and generic bytecode-executor boundaries
did not change.

Focused tests cover truthy short-circuit values including negative zero and
NaN, nested ordinary-array reads, getter observation and non-array fallback,
and a reduced four-channel Gaussian kernel. All 1,922 runtime tests, the
5,160-case curated Test262 subset, the staged touched gate,
`./scripts/check.sh`, `./scripts/compare-qjs.sh`, the pre-push gate, and
exact-commit CI run `30622615132` passed. The retained runtime commit is
`60e28ecf84aa8a9328377f7f6343465da59849c4`; candidate, exact-base, and
QuickJS-NG executable SHA-256 values are
`04d1ea96981f83afe8a34ae83b9d98da17a1d1d641f9bcd9a950d74a4645c6d1`,
`498e5bd03ce89f2f8100f77f2e9026da3f24903b2d617897688b235b37af6d81`,
and
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
Their build-receipt SHA-256 values are
`8ac8f5f9126cbaaba66b86cf3f94d615696ce42bdf489fea745feea862543c5c`,
`0264066a3f864429d14853f893c98ef3b21e83d89a9cc6a6fde20d49c116fe06`,
and
`b96f63f100f0ddfb6e7dedfdbcb195f579144cf01af814706d3d7a1b24b90b0f`.

The five-pair exact-binary fast target measured Gaussian blur at
**0.301961x candidate/base**, below the frozen `<= 0.65x` gate, with one
byte-identical successful output set. All seventeen declared controls stayed
below 1.03x; the worst was CDJS at 1.006378x. The target and control receipt
SHA-256 values are
`7b1a9492fc638ed158f2d49e3d543577008af1a05253a95d8f29871cae45566a`
and
`b8af17b3108f2c96965d2ed59b04651d97a504325edbe133e27a52db14ec597a`.

The exact 25-case broad matrix completed all three role-rotation blocks and
all 225 formal samples. Candidate/base geometric mean was 1.000793x; the worst
broad row was `array_dynamic_read` at 1.019958x. Every broad case remained
below 0.5x QuickJS-NG, with the worst `closure_allocation_call` at 0.429781x.
Manifest, raw, report, and summary SHA-256 values are
`5c3c586de23444190437652cf99a57da46730a4473d22e281c9e388f8c4215c0`,
`642cab2fc95ea55960dc544768886c170f73eab40bf50e0340b459c92fab94d7`,
`3f2880e9713b5ebc32a3266cc5e1792c1759ae115146d7265af29ac7beb681a6`,
and
`dc39e0b484db74fb2a750767030bcc80fd62749425b23fc23d969a08354da370`.

The exact 45-case external portfolio completed all three 60-second blocks for
all roles. Kraken Gaussian reproduced at **0.302290x candidate/base** and
0.943971x QuickJS-NG. JetStream, Kraken, and SunSpider candidate/base geometric
means were 0.997783x, 0.918029x, and 0.995593x. The worst external
candidate/base row was the non-control Kraken `audio-dft` at 1.016804x, below
the frozen 1.03x ceiling. External manifest, raw, report, and rendered-summary
SHA-256 values are
`a8ddeded582573bc676bf3f7bbbaf2625f6dfa7742f07bcdd6aaa26366f4e6c4`,
`c0a01b17466206ebaa8a8d123bc7694bc43e02f6ea5243cdbee08505365f0f0d`,
`eb9a9d56c68e3c39d9490e41bb8e5955c85b42f4885364747b3b5ee1b2aea16e`,
and
`3d61da152703724f045ec94a94ad3b798bd9cf784bb4a63c4a354e0ff93eecb6`.

Exact-commit Test262 Coverage run `30622894172` passed and produced burndown
SHA-256
`d5e8ceed4b1b3004b21c2d61e90e284700bec7a5dae8e5091a461a309bb0d43b`.
It covers all 53,572 pinned cases: 10,900 are outside the QuickJS-NG
configuration, while qjs-rust passes all 42,672 configured cases with zero
failures, zero timeouts, zero not-run cases, and zero actionable gaps. The
promotion decision retains the unit with no exception and has SHA-256
`b3b0f9d5ab7c20f412205811a14ca4c05613eb645e916a41816e2a6af93c91db`.

The refreshed exact `60e28ecf` queue has SHA-256
`608f966a5ca40eb0f40cfba31d9ce515370e042ca4bc3f324e72599c896495d1`.
Gaussian moves from rank eleven to rank thirty-five at 0.943971x QuickJS-NG;
no broad case is above the campaign threshold, but 38 external cases remain
above 0.5x. The new rank-one opportunity is SunSpider
`controlflow-recursive` at 5.8391x. This closes the branchy nested dense-loop
unit, not the campaign; continue from the new queue's highest unclosed
current-profile mechanism.

### 2026-07-30 rejected typed-loop fixed-index nested dense reads

The exact `60e28ecf` queue ranked JetStream and Kraken Stanford AES fourteenth
and fifteenth. Fresh source-faithful samples reproduced the same constructor
shape: 45.9% of 3,643 JetStream main-thread samples and 45.1% of 3,382 Kraken
samples landed exclusively in generic `Vm::run_completion`, while only ten
and eight samples reached typed-loop entry. Their profile SHA-256 values are
`d315217e4b49eef04538722475fd2293d8371b469acee3663f9a1997c99d2d93`
and
`29d1aefd19bc900a18ec0f1edff948e679684a404df764967a4cb81e87c4e6e6`.
A reverted compiler trace with SHA-256
`d1c33d14537cd0fde116d8d19adb9ded425a88b6802a17ab891f5df481dda0bc`
showed that the first key-expansion loop already compiled, while the inverse
key-schedule loop stopped at four fused fixed-index table-row reads.

The frozen one-attempt plan
`tasks/performance-units/typed-loop-fixed-index-nested-dense-read.json`
(SHA-256
`71e3d35d1d306d7d73b860e2e408099d6e762855e5f59d864bfed4ff9d276416`)
therefore admitted `GetPropIndex` through the existing guarded scalar or boxed
dense-read operations. Five focused tests passed with 1,919 filtered tests,
including fixed-row nested reads, indexed-accessor deoptimization, ordinary
object fallback, and scalar fixed-index reads. Candidate and exact-base
executable SHA-256 values are
`21186dddc97b0ee9d2f60cb6afe085dcd29eb3ef18d2da77d87fc86690acf1ae`
and
`04d1ea96981f83afe8a34ae83b9d98da17a1d1d641f9bcd9a950d74a4645c6d1`.

The five-block alternating exact-binary target gate rejected the unit.
JetStream measured **0.954071x candidate/base** and Kraken measured
**0.965981x**, both well above the frozen `<= 0.85x` ceiling. Every measured
process exited successfully with byte-identical stdout and empty stderr. The
runner and result SHA-256 values are
`18d8ea8c226b4749b850d2ff9fa23a23b52336bf90ac05fa48ca1741519acb45`
and
`317a283776d58f2abcd38e6ed9e58b169521072badbde6fa68fdd155985d5a9c`.
No control, broad, external-portfolio, or Test262 promotion work was run after
the frozen target gate failed.

Inspection after the failed gate found the next independent boundary. The
inverse key schedule creates an empty result array and assigns its next
sequential element inside the loop. The typed-loop `DenseWrite` guard accepts
only an already-present element, so the candidate deoptimizes on the first
`index == length` write even after fixed-index reads admit the loop. The
runtime and focused-test changes were reverted. Do not retry fixed-index
admission alone; any later unit must independently justify and freeze a safe,
general dense tail-append mechanism, preserve array length and indexed
property semantics, and then remeasure both AES targets.

### 2026-07-30 correction: AES stops before `DenseWrite`

The causal sentence at the end of the preceding section was too strong. A
source-equivalent preflight against the exact inverse-key-schedule bytecode
showed that fixed-index admission does **not** make that loop reach
`DenseWrite`. Its `e[b] = condition ? value : table-expression` assignment has
conditional jumps between the compiler-temporary key store and the final
`SetProp`. `compile_element_assignment` rejects any such control flow before
lowering the assignment, so the fixed-index-only candidate still leaves this
whole loop in the interpreter. The existing trace with SHA-256
`d1c33d14537cd0fde116d8d19adb9ded425a88b6802a17ab891f5df481dda0bc`
records the exact loop at bytecode 214..303, including the conditional RHS,
four fused `GetPropIndex` reads, and the final `SetProp`.

The fixed-index unit's measured **0.954071x** JetStream and **0.965981x**
Kraken results remain valid negative evidence: that isolated compiler change
missed its frozen `<= 0.85x` target. They do not prove that tail growth was the
next executed guard. Runtime and test changes had already been reverted, so
the correction changes no promoted engine behavior.

The subsequently frozen
`tasks/performance-units/typed-loop-dense-tail-append.json` plan (SHA-256
`58f5d29b3ef2dec7a2191f6f22363f5b1e422240827e86456bcea6c1873b644f`)
is withdrawn at preflight for relying on that incorrect causal inference. No
candidate benchmark attempt was made and no runtime change is retained under
that plan. A later unit may target the actual combined bytecode mechanism only
after freezing it explicitly: normal branch-aware computed element assignment
must carry its boxed receiver and scalar key/value through joins to an exact
`SetProp` deoptimization site; only then can a separately guarded exact dense
tail store publish the new element and Array length. This correction supersedes
the preceding section's final diagnosis without changing its rejection
decision.

### 2026-07-30 rejected branchy dense element transforms

The frozen one-attempt plan
`tasks/performance-units/typed-loop-branchy-dense-element-transform.json`
(SHA-256
`ae9554a9d6e77358c578559284d5900e32afbe34d1f0c60cbcf843faa8561f21`)
combined the actual boundaries seen in the exact AES trace. The candidate let
the main typed-loop abstract interpreter carry a computed assignment through
ordinary RHS branch joins, lowered fixed-index row reads through existing
guarded element reads, and emitted an exact boxed-receiver `ElementWrite` at
the original `SetProp`. Existing dense elements reused `DenseWrite`; one exact
tail append additionally required an ordinary realm Array prototype without
indexed hazards and repeated Array storage, length, descriptor, extensibility,
and borrow checks immediately before publication. A general compiler fix also
kept the common scalar `Move` at branch-join targets instead of folding only
one predecessor's producer into the destination.

Focused coverage passed before measurement: all 21 typed-loop tests and all 13
Array value tests passed. They included the exact AES-shaped branchy transform,
boxed and scalar fixed-row reads, successful exact-tail growth, gap and
inherited-setter fallback, and deoptimization after an already completed write
without replay. Candidate and exact-base executable SHA-256 values are
`58976d533d3e9380d131fe11d3dda229bbcaf9c9f428f94c3c7c45c2f4deb3cf`
and
`04d1ea96981f83afe8a34ae83b9d98da17a1d1d641f9bcd9a950d74a4645c6d1`.

Both five-block alternating exact-binary targets passed their frozen
`<= 0.80x` ceiling: JetStream Stanford AES measured **0.793291x
candidate/base** and Kraken Stanford AES measured **0.774597x**. All measured
processes completed successfully with byte-identical output. The target runner
and result SHA-256 values are
`7d23d055eab2c62de9914276e8561153ae1b872ec7360b043147e81e1a75c1ba`
and
`5f05ea03e1452e22dac6fba10c23f839875c8f8e17d71a3e056ef40544f98f59`.

The frozen control gate nevertheless rejected the unit. Ten of eleven controls
remained below 1.03x, including `array_dynamic_read` at 0.994710x,
`array_read` at 0.999441x, `array_write` at 0.998510x, and
`plain_function_call` at 1.000937x. `object_allocation` measured **1.082193x
candidate/base**, exceeding the ceiling by more than five percentage points.
The control runner and result SHA-256 values are
`6052b8824ae290f1852bbf4787539a8279391333cf69cc208fe085a1213434af`
and
`50a0f4f1070596ba3e10bdf93a52a028c1cd17e62b27548b152223e5c24df4f4`.
The one-attempt contract forbids threshold changes or a selective rerun, so the
runtime and focused-test changes were reverted. No full broad/external
portfolio or Test262 promotion scan was run for the rejected candidate. Do not
retry this combined compiler/runtime unit without new exact-current evidence
that independently explains and removes the broad allocation regression.

### 2026-07-31 rejected Number-only local-assignment admission

After the AES rejection, exact-current profiles attributed the next queue
slice rather than reopening an older mechanism. Rank-sixteen
`bitops-bits-in-byte` placed 1,496 of 4,042 top-of-stack samples in the body of
the already-retained typed-loop executor; its entry, scratch, and register-file
lifecycle costs were small. Rank-seventeen `crypto-md5` placed 440 of 3,129
useful samples in the already-retained `NumberOnlyProgram::eval`, without a new
admission boundary. Rank-eighteen `bitops-3bit-bits-in-byte` exposed a more
bounded gap: 1,211 of 3,940 useful samples landed exclusively in the general
`try_eval_numeric_leaf` path and another 632 in its out-of-line
`direct_number_binary`, or 46.8% together. Its pure local `+=` body emitted
statically numeric `Dup`, `Pop`, and `ToNumeric` shapes that the existing
Number-only compiler rejected. The rank-eighteen and independent MD5 profile
SHA-256 values are
`53f6c48cf5221a911a30fe5406a33e48ca392a30e051eeb48a6d1ca7e0afc49c`
and
`ad7611cdfb920e5975f4eae9bb5ad9a3cd83fb9d241f5620d19fffd6bf7671d1`.

The frozen one-attempt plan
`tasks/performance-units/number-only-local-assignment-program.json` (SHA-256
`9c2d0f28a58c5a72ec0d4c40ea4d269b613b0b0a978de4402fb26735100e0d76`)
therefore admitted only stack manipulation and local mutation whose abstract
values were already proven ECMAScript Numbers. It added scalar duplicate,
discard, and update operations, treated `ToNumeric` as an identity only after
that proof, and allowed older pure numeric statement completions to die with a
returning frame. Received upvalues, non-Number or missing arguments, coercive
values, calls, properties, control flow, and unsupported bytecode still
declined before observable work. Seven focused Number-only tests passed,
covering the source-faithful three-bit lookup, local compound assignment,
prefix and postfix update, comma discard, Number edge operations, and coercive
fallback.

The 31-block alternating exact-binary target gate rejected the unit.
Candidate executable SHA-256
`fb2ecbfa63dee5772bc0744ec5b961fba7cdb9588ed854618bec4ac0e6b50d8e`
was compared with exact-base executable SHA-256
`04d1ea96981f83afe8a34ae83b9d98da17a1d1d641f9bcd9a950d74a4645c6d1`
on the unmodified pinned source (SHA-256
`908076ee39ddf74f3d6be54b7f7c78fae8ddc9a308849f1ab1e40f1676779b41`).
Every process completed with byte-identical stdout and empty stderr, but the
median was only **0.837388x candidate/base**, above the frozen `<= 0.80x`
ceiling. The target runner and result SHA-256 values are
`d0f0d2099c981bc8a8f1e243f59fa8e02c86f4ffde4d0f7adbbf7998d0834397`
and
`dd62db87113cd8f1f80974351a6263125862ecfdac869331525726f33360f6b0`.

The failed target gate stopped the attempt before controls, complete broad or
external portfolios, or Test262 promotion work. Runtime and focused-test
changes were reverted. Do not retune this unit through scalar stack-op fusion,
completion-value elision, a registerized Number-only instruction layout, enum
packing, or a relaxed target threshold; any successor must start from a
distinct new exact-current shared-cost profile.

### 2026-07-31 rejected dense primitive predicate scan

The next exact-current queue slice was profiled before implementation with the
same standard-recipe runtime-`60e28ecf` executable SHA-256
`04d1ea96981f83afe8a34ae83b9d98da17a1d1d641f9bcd9a950d74a4645c6d1`.
Rank-twenty SunSpider `crypto-aes` placed 1,406 of 3,236 main-thread samples
in generic VM dispatch, but reproduced the already-consumed dense-read and
rejected branchy-transform boundaries. Rank-twenty-one `access-fannkuch`
placed 1,669 of 3,277 samples in generic dispatch while another 433 samples
were already inside the retained compact dynamic dense executors. Their sample
SHA-256 values are
`bfa210e9b495365d2fd5e3289ee57e4ff0bf7d540133b91f2e920216fb81dece`
and
`8f31f9b51da5f477411a1867ae5b52ec88c70913747426405bb3d953bffce511`.

Rank-twenty-two `access-nsieve` exposed a narrower apparent gap. Its sample
placed 1,840 of 3,259 stacks in generic dispatch, 259 in `Value` drop, 134 in
`Value` clone, and 226 in virtual-object operations. The counted outer
`isPrime[i]` predicate has the pure false-prefix shape of the retained dense
numeric predicate scanner, but the live array contains Booleans plus two
unrelated prefix holes. The source-faithful wrapper and sample SHA-256 values
are
`5428f7e48905cd0a633952ce0dc6e21fc68a5494ad28b590596f12281ca4c618`
and
`45b18697adef32413f9c91d0a62c1206b3f28d75cb533996ecfc4e894b5bc304`.

The frozen one-attempt plan
`tasks/performance-units/dense-primitive-predicate-scan.json` (SHA-256
`20bc3b399b8d37581423e7c80ea96adfac4701c87b0cb03d887d53b9d015d0cc`)
therefore extended only the existing scanner. Its strict dense Number lease
remained preferred; a failed whole-array lease retried each index through the
present-own dense-element guard, accepted Number or Boolean predicate values,
and deoptimized before the first hole, descriptor, inherited getter, or
unsupported value. The ordinary VM still ran every true body and all inner
Boolean stores. Eighteen focused predicate-scan tests passed, including sparse
Boolean prefixes and inherited-getter replay.

The 31-block alternating exact-binary target gate rejected the unit. Candidate
executable SHA-256
`7586c05f3e6bfc854fbafe6ac90f2aace83d7e1e27ec4698f43f463a794a0247`
was compared with the exact base above on the unmodified pinned source SHA-256
`ef62b42b6f926d61d9741a8e57b2758a9aea28ad2d1ee1e7d4747957d00fdc20`.
Every process completed with byte-identical stdout and empty stderr, but the
paired median was **0.999001x candidate/base**, far above the frozen `<= 0.80x`
ceiling. The target runner and result SHA-256 values are
`2ee3dcc8500bca987b51389becf4841dd1a1e82999da51c76b65f504920059c2`
and
`6123cbc6cc32f7664fe8fb36cd2b95d1a07c06263f853dbd3412a21ef13123a9`.

The failed target gate stopped the attempt before controls, complete broad or
external portfolios, or Test262 promotion work. Runtime and focused-test
changes were reverted. Do not retry Boolean admission, sparse per-index
fallback, a wider primitive truthiness set, or a relaxed target threshold
under this unit: the exact target shows that false-prefix scanning is not a
material share of `access-nsieve` end-to-end time. A successor must identify a
distinct shared cost, most likely in the retained inner Boolean store path,
from new exact-current evidence.

### 2026-07-31 rejected incremental frame deopt cell overlay

Exact-current profiling then covered queue ranks twenty-three through
twenty-six with the standard-recipe runtime-`60e28ecf` executable SHA-256
`04d1ea96981f83afe8a34ae83b9d98da17a1d1d641f9bcd9a950d74a4645c6d1`.
SunSpider `date-format-tofte` placed 363 of 2,846 inclusive samples (12.75%)
under `NewFunction`'s frame-deopt overlay, including 164 exclusive samples in
`DynamicBindings::overlay_cell`, 139 in string comparison, and 68 in the frame
scan itself. Kraken `stanford-crypto-ccm` instead placed 275 of 2,376 samples
in its already-retained `LegacyDynamicDensePlan`; Kraken iterative SHA-256
placed 677 of 2,328 in its already-retained typed-loop executor; and spectral
norm placed 456 of 2,908 in its already-retained `NumberOnlyProgram`. Their
source-faithful profile-wrapper SHA-256 values are, respectively,
`49f32f3a6aa0b633ab7424b197380fde1934b7afbd215044c6ce6a3995781dc5`,
`37bea1f06bc65282ca24f6ea5a0710493a22dab20da947fdccaf8a21354f355a`,
`282d81e7773818a48d85b0bfc0bef826b74a6aa0781ffa21fd53819f9e4badd6`,
and
`a4f2f2f854b2c854e91d9f35b20dc636b5a97d06ea4ccdeb5f8bd9feeb04a3d9`;
the corresponding sample SHA-256 values are
`cab1103cb319e02fc32efd550f98fa6a962a1a73bfe33aa4a5898b5277f20083`,
`fbfbe9f35be0ff6b59804bd5e8706ba4f667685f9752214ee4fdf8fb51efb28a`,
`e3058cf0a3fc555fee80ac534ef7dda463275709622eb8169d1fad78203fe25f`,
and
`91594cf5c6d4342979315d567274bb56de58bdb6eeb72ca50581f6b2ded317b5`.

The risk-adjusted order was incremental frame overlay, numeric direct-leaf
call fusion, CCM legacy-plan specialization, then SHA typed-loop core packing.
The first had the only new exact boundary and the lowest semantic risk; the
other three overlapped rejected call-graph work or retained executors and had
materially higher implementation risk. The frozen one-attempt plan
`tasks/performance-units/incremental-frame-deopt-cell-overlay.json` (SHA-256
`e1146b13c01e53d5df5e40859db07c1ce8ace4358fc6cf40fab66dd359ea18b4`)
therefore added an exact structural revision to `DynamicBindings` and cached,
per frame, the map identity plus active `Upvalue` identities established by
the previous `NewFunction`. Only monotonic new live cells used the incremental
ordered overlay; map mutation, cell replacement or deactivation, and shadow
ambiguity retained the complete ordered fallback. Focused revision and
block-shadow activation/exit tests passed before measurement.

The 31-block alternating exact-binary target gate rejected the unit. Candidate
executable SHA-256
`0fc38ae636713bafc55f5e9ded0c86ce2505be1b1eb432ec5f715228aef57a96`
was compared with the exact base above on the unmodified pinned Tofte source
SHA-256
`cbefaffbecb6769a85f5877765b21f967a1cff5ab2625d4a9066c050fcdc7b5e`.
Every process completed with byte-identical stdout and empty stderr, but the
paired median was only **0.980412x candidate/base**, far above the frozen
`<= 0.90x` ceiling. The target runner and result SHA-256 values are
`de2b6343a3cfb281724c31be7eda52f2a51c99fb013dbb208e5e08da63d85638`
and
`9b07800fa402ea6054b80345bc7beea1446a8381ee55c94223be9053d2a4ba6d`.

The failed target gate stopped the attempt before controls, complete broad or
external portfolios, or Test262 promotion work. Runtime and focused-test
changes were reverted. The exact end-to-end gain shows that prefix hashing is
not the dominant remainder implied by the inclusive profile; snapshot
validation and revision bookkeeping consume most of the removable work. Do
not retry per-frame active-cell snapshots, structural-revision invalidation,
or a relaxed target threshold under this unit. A successor must isolate a
different shared cost rather than elaborating this cache.

### 2026-07-31 retained shared functional-replace input

Exact-current profiling then covered queue ranks twenty-seven through thirty
with the standard-recipe runtime-`60e28ecf` executable SHA-256
`04d1ea96981f83afe8a34ae83b9d98da17a1d1d641f9bcd9a950d74a4645c6d1`.
Rank-twenty-seven SunSpider `string-unpack-code` placed 7,877 of 9,431
main-thread samples in `String.prototype.replace`. The builtin RegExp replace
path copied the complete immutable subject into a Rust `String` for every
functional-replacement callback: `String::clone` accounted for 3,102
inclusive samples and platform `memmove` for 3,100, or 32.87% of the whole
profile before allocation and release costs. Its sample SHA-256 is
`1c3e8ae0f2813c6d76203aaa4d6a866dbfc1f0c78d492a8f21e4f82b4d3194d1`.

The adjacent profiles did not expose a lower-risk new mechanism. Rank
twenty-eight `string-fasta` placed 3,601 of 9,726 samples in flat generic VM
execution and 962 in distributed `Value` ownership. Rank twenty-nine
`regexp-dna` placed 8,198 of 9,412 samples in the global RegExp match path but
reproduced matcher mechanisms already consumed by earlier units. Rank thirty
Kraken PBKDF2 placed 3,187 of 8,519 samples in the already-retained typed-loop
executor. Their sample SHA-256 values are, respectively,
`0452dbe0d1ba03002ff366e6b4b71ddb8683cfdad3289d8df5081c51904f2fb7`,
`58afa1ce297ce588f45430c3ffcb47a16eae82d46c4f675c52a51d957dbf7c35`,
and
`404ca1d36a020f75fa7ae01912d4fb43460125fd2c0c792f8ff15bb8113d6997`.
The distinct directly removable copy at rank twenty-seven therefore had the
best risk-adjusted return.

The frozen one-attempt plan
`tasks/performance-units/shared-functional-replace-input.json` (SHA-256
`18fe623146017de29091d816d77018b11b13fd62c93cc3bb2546c5bb2186f82a`)
keeps the subject in its existing shared immutable `JsString` representation
through result construction. Each ordinary builtin functional-replacement
callback receives a cheap clone of that handle in the same argument position,
instead of a newly allocated full-buffer copy. Match order, captures, UTF-16
positions and slicing, callback result coercion, custom protocol boundaries,
`lastIndex`, and result allocation are unchanged. A focused global-replace
regression verifies that every callback observes the complete original
primitive string, its length, and the expected per-match position and result.

Candidate executable SHA-256
`b3f992cfb182137b4d4d28fdabbaa5a863da1919f54892f2df04ecc22a399062`
was compared with the exact base above on unmodified pinned source SHA-256
`6ff9856ad51b877ef29262b942015ec04dd2ffe916b5adc85324aee2a144d382`.
The 31-pair alternating direct-process target gate retained the unit at
**0.613206x candidate/base**, below the frozen `<= 0.75x` ceiling; candidate
and base medians were 89,050,917 ns and 147,059,542 ns. Every process exited
successfully with byte-identical stdout and empty stderr. The target runner
and result SHA-256 values are
`45b8fe96ddac2c293e62024576b6f64de2d805ce98b53dd9436237f48181e024`
and
`884f44fca41da83838073527b16f11eff05a63c2d1adea4839658f8b20c4847a`.

All frozen controls stayed below 1.03x. Seven-block broad medians were
0.925661x for dynamic method calls, 1.002511x for plain function calls,
1.003171x for property reads, and 0.952484x for string slicing. The broad raw
and summary SHA-256 values are
`dbe7e38f7b0c90b8bdf703d6eb47c01b7b787986b1919fb7094be2e6d91dd25a`
and
`3107e7a2358296de3568a81d78d41f1b597a3abe6ffa6be2f23453bec7702850`.
Seven-block external medians were 0.955080x for HashMap, 1.010184x for PBKDF2,
0.900400x for recursive control flow, 1.007704x for RegExp DNA, 0.968516x for
FASTA, and 0.997958x for tagcloud. External manifest, raw, report, and rendered
summary SHA-256 values are
`be52e9069d3c4a1db4249e71b67885d026b8ac74f42d12047be2a140bf1c17b4`,
`aa97ad5d4d08935b62eb4aae58ecf4cd027d6970a1494095f0c3d52bead253d1`,
`ecc19305ec26376973960b673e87de4c5c2f3d6cb0705164ab20f1056d22e882`,
and
`2a6eba145925c334efff810fbf01f7fa22bcd28fca222a385a06a262b69eee2b`.

A post-change source-faithful sample contains 4,736 main-thread samples.
Platform `memmove` falls from 3,249 of 9,431 base samples (34.45%) to 147
(3.10%), and the per-callback full-subject clone stack is absent. The sample
SHA-256 is
`665aef59df13c3f9d0f1da81a789982a08d258f080b44707914103c641eeffd7`.
The focused test, all 1,922 runtime tests, all 211 benchmark-tool tests, every
performance-plan validator, all 5,160 curated Test262 cases,
`./scripts/check.sh`, and every `./scripts/compare-qjs.sh` fixture passed.
This retains the single allowed fast attempt without changing a threshold.

The implementation and its initial evidence were committed as
`c823043e2ae348a20587dee25c956b81e59210c1` and pushed on `main`. Exact-commit
standard-pair candidate, base, and QuickJS-NG executable SHA-256 values are
`82335e8280120666716e86e09755decc560e93923811894a188ca6d1a1c760bb`,
`04d1ea96981f83afe8a34ae83b9d98da17a1d1d641f9bcd9a950d74a4645c6d1`,
and
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
Their receipt SHA-256 values are
`ff8a089db3be6eb1b79097652d29d28feeba55d96fe53390c3e8189842311967`,
`8ac8f5f9126cbaaba66b86cf3f94d615696ce42bdf489fea745feea862543c5c`,
and
`b96f63f100f0ddfb6e7dedfdbcb195f579144cf01af814706d3d7a1b24b90b0f`.
An initial exact orchestration built both Rust roles beneath explicit target
triple directories and was excluded before use because all three broad blocks
made the base `plain_function_call` timer-limited at its lower iteration cap.
A focused standard-pair capacity run instead made all three roles eligible;
no health threshold or manifest capacity was changed.

The complete exact-commit 25-case broad matrix then finished all 225 formal
samples. Candidate/base geometric mean was 1.001359x. The four frozen broad
controls were 1.010383x for plain function calls, 1.011336x for dynamic method
calls, 1.000945x for string slicing, and 1.010726x for property reads. Every
broad candidate/QuickJS-NG row remained below 0.5x; the worst was closure
allocation/call at 0.429972x. The non-control `many_locals_call` row measured
1.036329x base and remains an explicit observation for a separately
profiled unit. Broad manifest, raw, report, and summary SHA-256 values are
`5c3c586de23444190437652cf99a57da46730a4473d22e281c9e388f8c4215c0`,
`5a767d832b4e9028eff6fc76f24e858e949e80e51401f8efc6d948e5cd3dbb5e`,
`b3edd735fd314207599ce5cc1a592bcb18b27d0f049b7d2482b8f470d3cb9fa6`,
and
`338f2c11b78a76f6655994485b555faccd613705b955ef8a935a2feb0b140dec`.

The exact-commit 60-second external run completed all 45 cases for all three
roles. `string-unpack-code` reproduced at **0.650552x candidate/base** and
1.188030x QuickJS-NG. JetStream, Kraken, and SunSpider candidate/base geometric
means were 1.003407x, 1.000353x, and 0.981842x. All frozen external controls
were at or below 1.011052x. The non-control SunSpider `access-nbody` row was the
largest observed base ratio at 1.091118x; it remains explicit debt requiring a
new profile rather than post-hoc retuning of this unit. External manifest,
raw, report, and rendered-summary SHA-256 values are
`a8ddeded582573bc676bf3f7bbbaf2625f6dfa7742f07bcdd6aaa26366f4e6c4`,
`e29506807792b738124b2ceedf10fbad392ed337c0f9abf677bd096785492e7d`,
`12ac4419b2734695722f10f4e4569e6aeb0f70be388f295ac5284862a8c5d5f6`,
and
`396fec7e2d64b206d97c9157c2db8838b6dfa732e0bd469e6f54a5e3157056e8`.

Exact-commit Test262 Coverage run `30635826875` passed and produced burndown
SHA-256
`de37243f26917fbd9d8e1a29e4f7adfafc191feef41abb15f66c0a325098ec8f`.
It covers all 53,572 pinned cases: 10,900 are outside the QuickJS-NG
configuration, while qjs-rust passes all 42,672 configured cases with zero
failures, timeouts, not-run cases, or actionable gaps. Exact-commit CI run
`30635509140` also passed. The hash-bound promotion decision retains the unit
without an exception and has SHA-256
`9bad5dbcb55a06665a530b1624b34d89bdbe84358a69d06b30d2d1ed61140a0d`.
This closes the functional-replace input unit as a semantics-preserving
optimization, not the campaign: most external rows remain above 0.5x
QuickJS-NG, so refresh the queue from `c823043e` and continue from its highest
unclosed exact-current shared cost.

### 2026-07-31 aborted stage: routing through a moved frame

The call-frame migration's plumbing landed as `8974a69a` (call-environment
provenance), `cbbb0fcc` (`PreparedBytecodeCall`), `e0cbc752`/`9d4da8d6`/
`c7daae80` (a frame owns its bytecode), `b0f93999` (a frame-stack driver),
`0da1dffe` (a frame built from a handle it owns), and `e8641244` (a handler
requests frame entry rather than performing it). All are green and pushed.

Routing itself was then attempted and **aborted** under the T022 migration
stage gate. It is recorded here because the negative result is precise and
closes one implementation shape rather than the mechanism.

The shape: `prepare_direct_leaf_call` split the slot-seeded cohort's
preparation from its execution, `call_direct_leaf_callee` built the callee
frame with `Vm::with_frame_bytecode(FrameBytecode::Shared(..))` and recorded it
in `Vm::pending_frame_entry`, the six call opcode arms ended the activation
with `FrameExit::EnterFrame`, and the driver installed it with `push_frame`.

It was semantically correct. All 1,937 runtime tests passed, `compare-qjs.sh`
was clean, and both feature builds linted. Two capability facts confirmed the
routing really happened:

- `nested_vm_constructions` collapsed from **12,700,005 to 0** on
  `recursive_call_tree` while `same_vm_frame_entries` rose to 12,700,004.
- Recursion depth went from a **process abort at 2,000** to a correct result
  at **200,000**, which is the catchable-error behavior QuickJS-NG has and
  this engine does not.

It was also decisively slower. Paired, strictly alternating, seven samples
against the `cbbb0fcc` migration base: `recursive_call_tree` **1.3848x**
[1.3561, 1.4004] and `prototype_method_call` **1.2769x** [1.2565, 1.2922].
The two cases dominated by closed-form leaf evaluation improved
(`polymorphic_call_site` 0.9650x, `capturing_closure_call` 0.9499x), which is
consistent: they do not build frames. Cumulative 1.1283x, outside the 1.10
stage budget.

Attribution is unambiguous. `_platform_memmove` became the second hottest
symbol at 818 of roughly 3,700 samples, having been absent from the top twenty
before. `FrameState` is **704 bytes**, and the routed path moves one about six
times per call -- constructor return, `into_frame`, into the pending `Option`,
out of it, `mem::replace` into `current`, and into the `callers` vector -- where
a nested `Vm` constructed its frame in place and never moved it. At 12.7
million calls that is roughly 50 GB of `memmove`.

Boxing the frame to make those moves pointer-sized was measured and is
**worse**: `recursive_call_tree` 1.6170x and `prototype_method_call` 1.4480x.
It trades six memcpys for a heap allocation and free per call.

The runtime changes were reverted; the plumbing commits above were kept
because they are correct, independently tested, and are what any next shape
builds on.

What this closes: recording a frame in an `Option` and moving it into place,
boxed or not. What it does not close: running ordinary calls on one VM. The
next shape must **construct the callee frame in place in its final location and
never move it** -- for example `Vm { frames: Vec<FrameState>, current: usize }`
with the frame written directly into the vector, which also requires the frame
constructor to initialize in place rather than return by value. Note that
`Vec::push` of a returned value would itself reintroduce one move, so the
constructor's signature is part of the mechanism, not an implementation detail.

A second, independent lever is now measured and available: `FrameState` at 704
bytes is itself the cost driver. Shrinking it -- for example by boxing the cold
state a frame rarely touches -- would reduce every frame operation, including
the ones the current nested path already performs.

### 2026-07-31 frame size is on the critical path of the existing route

The aborted routing stage above attributed its regression to moving a
704-byte `FrameState`. That raises a question the abort does not answer: is
the frame's *size* expensive even where it is not moved -- that is, on the
nested route the engine uses today?

An inverse experiment answers it directly. Padding `FrameState` by 320 bytes
and measuring against the unmodified `cbbb0fcc` base, paired and alternating
over seven samples, gives `recursive_call_tree` **1.1258x** [0.9978, 1.3866]
and `prototype_method_call` **1.0695x** [0.9691, 1.1101].

The direction is consistent with the `memmove` attribution and with the
mechanism -- a larger frame costs more to construct, initialize, and drop --
but the ranges are wide enough that the magnitude is not established. Treat
this as "frame size is on the critical path", not as a 7-13% estimate. A unit
that shrinks the frame should measure its own effect rather than inherit this
number.

The composition, for whichever unit takes it: 36 fields, 704 bytes, of which
`CallEnv` alone is 208. Cold candidates a slot-seeded leaf call never touches
include `try_stack`, `pending_throw`/`pending_return`/`pending_jump`,
`resume_mode`, `sloppy_global_names`, `with_stack`, `disposable_scopes`, the
three prototype caches, `numeric_mutation_loop_plans`, `virtual_values`, and
the two `declined_*` bitsets -- roughly 290 bytes that a lazily allocated
`Option<Box<FrameColdState>>` would replace with eight, the same shape
`ObjectData` already uses for its own cold state. The lazy allocation matters:
allocating the cold box per frame would trade this cost for a worse one, which
is what boxing the whole frame did in the aborted stage.

Access-site counts, which decide the sequencing rather than the byte counts
alone. The cheapest first slice is the state only unwinding, generators,
`using` declarations, and sloppy-global fallback ever touch: `pending_throw`
(8 sites), `pending_return` (7), `pending_jump` (6), `resume_mode` (6),
`disposable_scopes` (6), and `sloppy_global_names` (6) -- 39 sites for roughly
112 bytes. Adding `try_stack` (11 sites, 24 bytes) is the natural second step,
though note `try_stack.is_empty()` is consulted on every runtime error, so it
is the coldest field with a warm read.

Deliberately not in a first slice: `with_stack` (28 sites) and `virtual_values`
(13) are large and widely touched, and the three prototype caches are eight
bytes each and read on ordinary paths. Those need their own justification
rather than riding along.

The correctness risk is concentrated, not diffuse: the cheap slice is entirely
unwinding and suspension state, which is where subtle bugs hide and where this
repository has historically spent the most debugging effort. A unit taking it
should expect its cost to be in focused tests for throw/finally/generator
resume ordering, not in the mechanical field moves.

### 2026-08-01 root cause: the generic dispatcher's register pressure

`Vm::run_current_activation` is one function of 24,792 machine instructions
with a stack frame of about 4.3 KB, all ten callee-saved GPRs and all eight
callee-saved FP registers saved, 335 distinct `[sp,#imm]` slots, and 3,644
spill/reload instructions (14.7% of the function). Forty-eight opcode arms
branch back to the same address, and that address is a stack reload. The
preamble every one of them pays is:

```asm
ldr  x10, [sp, #0x370]      ; reload `self` FROM THE STACK
ldr  x20, [x10, #0x288]     ; self.ip from memory
ldr  x14, [sp, #0x360]      ; execution_code.len from the stack
ldr  x13, [sp, #0x368]      ; execution_code.ptr from the stack
cmp x20, x14 / b.hs         ; bounds check
madd x26, x20, #0x60, x13   ; ip * 96 + base
add  x8, x20, #1
str  x8, [x10, #0x288]      ; ip back to memory
ldr  x8, [x26]              ; discriminant
ldrh w10, [x11, x8, lsl #1] ; jump table
br   x9
```

Eight memory operations and a roughly five-deep dependent load chain run before
any opcode does any work -- about 15-20 cycles, or ~5 ns, which is most of the
measured 9.5-15.6 ns per dispatched instruction against QuickJS-NG's 2.0-2.4.
`self`, `execution_code.ptr`, and `execution_code.len` are loop-invariant and
still reloaded every dispatch: the allocator has no registers left for them.

This is one mechanism for four results that previously looked unrelated: the
per-instruction gap at parity instruction counts, the 2026-07-24 `unreachable!()`
arm costing about 17%, a 2026-08-01 single added fast-path arm costing 6.8%, and
a per-backedge probe skip costing 7.9% with provably identical dynamic counters.
Any source change that raises pressure taxes all ninety-odd opcodes at once.

Machine layout is **not** the cause, and that line of inquiry is closed.
Identical source, only alignment changed, nine alternating repetitions each:
`-align-all-functions=5` 1.0025, `=6` 1.0114, control (base against itself)
1.0036. Pure address shift is at the noise floor, so I-cache set aliasing and
BTB aliasing are ruled out. Basic-block alignment does cost -- `-align-all-blocks=4`
1.0539 and `=5` 1.2495 -- but it inserts NOPs and inflates the code, so it
measures code size, not placement.

By contrast `typed_loop::execute` (inlined into `run`) is 2,972 instructions
with a 688-byte frame, 42 spill slots, `pc` live in `x8`, and two memory
operations before its dispatch. The constraint that new execution machinery
belongs in a separate small executor is therefore confirmed by code generation,
not only by timing.

### 2026-08-01 retained typed-loop admission without the boxed-operation ratio

A fresh per-sentinel counter profile at 200,000 iterations showed five of the
six generic-path sentinels declining the typed tier on essentially every
backedge (`prototype_method_call` 1 entry against 200,063 declines,
`heterogeneous_property_read` 0 against 200,064), and three of them --
`prototype_method_call`, `polymorphic_call_site`, `capturing_closure_call` --
already answering all 200,000 calls with closed-form leaf evaluations and
building no frames. Their remaining cost is the generic dispatcher running
21-23 instructions per iteration *around* an already-free call.

Bisection identified the cause as this file's own admission rule rather than a
missing operation: `checksum += pool[i & 63].step` was admitted, while
`var row = pool[i & 63]; checksum += row.step` and two element reads in one
iteration were both rejected, because more than a third of their operations
were boxed. The rule compared this tier against an interpreter assumed to be a
peer; the disassembly above shows the fallback it chose is not one.

Removing the rule, measured with a runtime switch inside one binary and then
confirmed between separate binaries, nine alternating repetitions:
`heterogeneous_property_read` **0.5016** [0.4718, 0.5102], the other five
between 0.997 and 1.017, six-sentinel geomean **0.8946**. That case's executed
instruction count falls from 5,202,553 to 2,579 and its declines from 200,064
to 64, moving it from about 2.98x to about 1.49x QuickJS-NG.

The 40-case cached SunSpider/Kraken corpus, seven alternating repetitions, is
geomean **0.9981** with worst cases 1.0164, 1.0166, and 1.0174 -- all inside
the 1.03 control ceiling. The mandatory canaries are clean: Kraken `ai-astar`
**0.9999** [0.9902, 1.0018] and SunSpider `access-nbody` **0.9925**, plus the
three cases the rule was introduced to protect (`math-cordic` 0.9947,
`3d-cube` 0.9676, `audio-fft` 1.0031).

This does **not** reopen the 2026-07-29 rejected numeric-object-field
scalarization, whose record above stands unchanged. That unit changed property
*representation* -- scalar numeric field reads, unboxed field writes, and dense
local receivers promoted to guarded boxed ones -- and regressed `ai-astar` by
15.5% while winning `access-nbody` by 41.7%. Its warning against retrying by
"changing its cache ordering, boxed-operation threshold, or property guard"
was about that representation. The present change alters no representation, no
handler, and no guard: the identical compiled program is now permitted to run,
and its canary is flat rather than 1.155x.

One honest cost: `prototype_method_call` regresses **1.0174** [1.0116, 1.0238].
Its loop still declines, but a program now exists for it, so every backedge
pays `try_run_typed_loop`'s linear scan over `plans.typed` before the frame's
decline bit short-circuits, where the empty-program check used to return
immediately. This is the same per-backedge probe cost that three separate
memoization attempts failed to remove on 2026-08-01, so it was accepted rather
than retried. It should convert into a win once a boxed-register call operation
admits that loop instead of declining it.

### 2026-08-01 retained closed-form leaf calls inside typed-loop regions

Three of the six generic-path sentinels answer every one of their 200,000 calls
with a closed-form leaf evaluation and build no frame, so their remaining cost
was never the call: it was the generic dispatcher running 21-23 instructions per
iteration *around* an already-free call, because `Op::CallResolved` compiled to
`CallNumericNative`, whose run-time check rejects any callee that is not a
`Math` intrinsic and deoptimized the whole region on its first iteration.

`TypedOp::CallClosedFormLeaf` keeps a resolved call's receiver, callee,
arguments, and result in registers. It is admitted only in the form that
preserves this tier's rule that an operation either succeeds or stops the
program before becoming observable: the two closed-form evaluators answer with
`Option<Value>`, computing only bodies they have already proven total, so there
is no state in which a callee has half-executed and the loop must unwind it.
No `Outcome` variant for a thrown error was needed, and none was added. The
receiver is kept rather than dropped, because a receiver-property body is what
the second evaluator answers. `is_direct_leaf_function` is asked first because
it is the interpreter's own precondition for reaching those evaluators.

A receiver read straight from the `Math` global still compiles to the unboxed
intrinsic operation, so hoisted-intrinsic loops keep a scalar result. The
discrimination is a register hint and is sound in both directions: a false
positive compiles the intrinsic operation, whose run-time check deoptimizes
exactly as before; a false negative takes the general operation, which tries the
same intrinsic first.

Executed instructions per 200,000 iterations, and the closed-form evaluation
count that proves the semantic work still happens:

| case | executed ops before | after | closed-form evaluations |
|---|---:|---:|---:|
| `prototype_method_call` | 4,401,801 | 1,833 | 200,000 |
| `polymorphic_call_site` | 4,200,262 | 292 | 200,000 |
| `capturing_closure_call` | 4,600,287 | 320 | 200,000 |

Nine alternating repetitions: `prototype_method_call` **0.5033**
[0.4984, 0.5135], `polymorphic_call_site` **0.5090** [0.5057, 0.5096],
`capturing_closure_call` **0.4696** [0.4672, 0.4729], the other three
0.985-0.999, geomean **0.7001**. All six sentinel checksums are byte-identical
to the base binary's.

The 40-case SunSpider/Kraken corpus is geomean **1.0000** at seven repetitions.
Five cases read above 1.03 there; at twenty-one repetitions two were noise
(`crypto-sha1` 1.0707 -> **0.9875**, `string-tagcloud` 1.0353 -> **0.9951**) and
three are real but inside the 1.03 control ceiling: `audio-fft` **1.0262**,
`json-stringify-tinderbox` **1.0223**, `audio-beat-detection` **1.0082**. The
218 QuickJS-NG comparison fixtures pass.

Two of this unit's tests were fail-open when first written and were rewritten
after checking. Asserting `compile_all` is non-empty does not test admission:
the old path also produced a program, which then deoptimized at the call. The
test now asserts the program *contains* the operation. Separately, a class
constructor whose body writes `this` is declined by the evaluators regardless,
so it does not demonstrate that `is_direct_leaf_function` is load-bearing --
and probing bound functions, `arguments` users, generators, async functions and
mutable captures found no callee the evaluators alone admit wrongly. The guard
is retained as alignment with the interpreter's entry condition, and the code
comment says so rather than claiming a defence it does not demonstrably provide.

### 2026-08-01 reverted increment fusion for incomplete analyses -- it hung ordinary loops

`92f49f8b` extended `fuse_local_increments` to bodies whose virtual-object
candidate analysis did not complete. That is wrong, it was mine, and it shipped
this morning. **The claim in an earlier draft of this file that the resulting
hang predated 2026-08-01 was incorrect**; bisection across `92f49f8b`,
`5b80dabc`, `d6fb08d5`, `771c4eef` and `f0e94f5f` puts it exactly at
`92f49f8b`, whose parent is clean.

Minimal reproduction -- no class, no throw, no call:

```js
eval('1');
function run(n) {
  var total = 0;
  for (var i = 0; i < n; i++) { try { total += 1; } catch (error) {} }
  return total;
}
run(6);   // QuickJS-NG: 6.  This engine: never terminates.
```

Both conditions are ordinary and independent. A `try` anywhere in the function
leaves the candidate analysis incomplete, which is what routes the body to the
new path; a direct `eval` anywhere in the *script* is what makes the fused
operation wrong. `while` is unaffected, indirect `eval` and the `Function`
constructor are unaffected, and a body without `try` is unaffected.

The observable failure is that the loop's induction variable never advances
while the body keeps running: instrumenting the loop shows `i` fixed at 0 while
the body's own accumulator grows without bound. The static
`is_assignment_authoritative` evidence the fusion relies on does not survive a
script whose locals a direct `eval` can redirect, so the fused operation writes
a slot that the surrounding reads no longer consult.

Reverted rather than re-admitted under a narrower condition. The revert is
neutral on five of the six sentinels and costs `string_key_map_churn` **1.0463**
-- and that case is the next unit's target, which rewrites its loop entirely.
Re-landing a hang-capable path to hold 4.6% on a case about to be replaced is
not a trade worth making; a future attempt needs a run-time-checked condition,
not a static one.

Two regression tests cover it. The structural one asserts that a body with an
incomplete analysis keeps an unfused stream, and the semantic one runs the
loop with a guard counter so a regression returns `-1` instead of hanging the
test process. Both were checked against the reverted code: they fail, and the
semantic one fails rather than hangs.

### 2026-08-01 the eval/class/loop hang, as first (mis)diagnosed

Retained as a record of how the entry above was reached, and of a wrong
attribution corrected in the same session. The reproduction below is real, but
the class, the `eval`-created callee, and the `try`/`catch` were all incidental:
the cause is the increment fusion recorded above, and the claim that it predated
this day's work was wrong.

```js
var C; eval('C = class { constructor(v) { this.v = v; } }');
function run(n) { var t = 0;
  for (var i = 0; i < n; i++) { try { t += C(i); } catch (e) { t += 7; } }
  return t; }
run(6);   // QuickJS-NG: 42.  This engine: hangs.
```

All three conditions are required. A class *declared* rather than created
through `eval` returns 42; the same `eval`-created class called outside a loop
throws correctly; an ordinary throwing function in the same guarded loop returns
42; and the loop without the call terminates. The suspect is
`class_constructor_call_error` -- which was wrong. Narrowing the reproduction
showed the callee is irrelevant: an `eval`-created function that merely returns
a constant hangs the same loop, and so does a bare `throw 1` with no call at
all. What "reproduces on binaries built from `92f49f8b`" actually meant was
that `92f49f8b` is the commit that introduced it.

### 2026-08-01 whole-function compact register executor: admitted, and it proves the gap is not dispatch

R3's premise -- and the 2026-08-01 register-pressure root cause -- said the
recursive sentinel is slow because its body runs through
`Vm::run_current_activation`, whose dispatch preamble costs ~5 ns before any
opcode works. The stated arithmetic was ~20.6 instructions per activation x
~5 ns ~= 103 ns per call against QuickJS-NG's ~42 ns, concluding that "frame
elimination alone can never win". This unit tested that premise by removing
the dispatcher, and the premise is **wrong in the direction that matters**.

`bytecode/compact_fn/` is a whole-function `Value`-register executor. It lowers
a stack-machine body to fixed register indices (operand depth `d` becomes
register `d`), admits all-or-nothing, and keeps the nested-`Vm` calling
convention: a call inside an admitted body re-enters `Vm::call` and still
builds a nested VM. Selection happens in `eval_direct_call_bytecode` before any
observable work, so a declined body is bit-for-bit the previous path.

Admission and mechanism are confirmed by counters, not inferred. At 2,000
iterations of `recursive_call_tree` (254,004 calls):

| counter | value |
| --- | ---: |
| `ordinary_call_attempts` | 254,004 |
| `compact_function_entries` | 254,001 |
| `executed_ops` (generic loop) | 34,245 |
| `compact_function_ops` | 4,680,009 |
| `nested_vm_constructions` | 254,005 |

Generic dispatch inside `callTree` collapsed to ~0, the compact tier ran
~18.4 operations per activation (against the 20.6 estimate), and the calling
convention is untouched. Both of the unit's mechanism gates passed.

The timing gate did not. Five paired alternating reps against QuickJS-NG:
`recursive_call_tree` 6.3253 -> **6.0381**, six-sentinel geomean 1.8355 ->
**1.8061**. Per call, over 7,620,000 calls:

| build | seconds | ns/call |
| --- | ---: | ---: |
| QuickJS-NG | 0.301 | 39.5 |
| base `7273d08a` | 1.904 | 249.9 |
| compact tier | 1.814 | 238.1 |

**Removing 100% of generic dispatch from this body saved 11.8 ns per call,
against a predicted 92 ns.** The executor is not the reason: disassembly puts
`compact_fn::execute` at 836 instructions and 17 stack slots, against
`typed_loop::run`'s 3,204/56 and `run_current_activation`'s 24,600/332. It is
the smallest executor in the engine, and it still bought 4.7%.

So for a recursive body, ~200 of the ~238 ns per call is **call construction,
not dispatch**. This inverts the unit ordering the task has assumed since R2:
it is dispatch elimination that cannot win alone, and the 704-byte
`FrameState` is the load-bearing cost. It also retroactively explains R3b
(1.5%) and R3c: those measured the same 5% ceiling from the other side.

One unambiguous secondary result, measured by bisection rather than assumed.
Maximum recursion depth on a test thread's stack:

| build | max depth |
| --- | ---: |
| base `7273d08a` | 40 |
| compact tier | 250 |

A compact activation's native frame is a fraction of `run_current_activation`'s
~4.3 KB, so admitted recursion goes roughly 6x deeper before the Rust stack
overflows. That is a correctness-relevant improvement on its own: QuickJS-NG
raises a catchable error where this engine aborts the process.

The unit is retained -- it is a prerequisite that removes dispatch from the
measurement, so the next unit's frame work is no longer masked by it -- but the
next unit must be frame construction, and it should be measured against this
tier rather than against the generic interpreter.

### 2026-08-01 compact tier follow-ups, and a measurement-hygiene correction

Two follow-up units landed on the compact executor.

`a69ecae6` dispatches a direct-leaf callee straight from the registers instead
of staging it on the operand stack for `Vm::call` to pop back off. The entry it
reaches, `call_direct_leaf_function`, already takes an argument slice, so the
staging was a pure round trip. Non-direct-leaf callees still go through the
operand stack, so general call semantics keep one implementation. Paired A/B,
nine and seven reps: **0.9193** [0.9160, 0.9248].

`67555208` reads an upvalue-backed local from `local_upvalue_cell` instead of
re-resolving it through `Vm::load_local` on every read. **0.9914**
[0.9581, 0.9973] -- small, but every repetition was below 1.0.

One experiment was rejected: replacing the `Value::clone` of call operands and
store sources with `mem::replace`, on the theory that the abstract stack
consumes them. Measured **1.0235** and **1.0240** over nine and seven reps --
consistently *slower*. `Value::clone` on an `Rc`-backed value is a
non-atomic increment; `mem::replace` forces an extra discriminant write and
appears to block keeping the value in a register. Reverted.

**Measurement hygiene, and a correction to the entry above.** A hung test
process from an earlier session (`closed_form_leaf_calls_match`, started
04:53) was holding 100% of a core for four hours, with system load average at
20.6, throughout this session's first measurements. It was killed at 08:57.

Interleaved candidate/base *ratios* are unaffected, because both sides paid the
contention -- but the **absolute per-call figures in the entry above are
inflated**. The 238.1 ns/call and the 11.8 ns dispatch saving were measured
under contention; on a quiet machine the same build measures ~212 ns/call. The
*conclusion* stands unchanged -- dispatch elimination bought a few percent, not
the predicted ~40% -- but treat the ns figures as an upper bound, and re-measure
before using them as a budget. Kill stray `target/debug/deps/*` processes and
check `uptime` before any timing run.

Session totals against clean base `7273d08a`, quiet machine, seven paired reps:

| case | cand/base |
| --- | ---: |
| `recursive_call_tree` | **0.8770** |
| `prototype_method_call` | 0.9963 |
| `polymorphic_call_site` | 1.0024 |
| `capturing_closure_call` | 1.0064 |
| `heterogeneous_property_read` | 1.0048 |
| `string_key_map_churn` | 0.9971 |
| **geomean** | **0.9795** |

Against QuickJS-NG on the same quiet machine: `recursive_call_tree`
6.30 -> **5.5279**, six-sentinel geomean 1.839 -> **1.8008**.

### 2026-08-01 the next unit, and what the arithmetic says about winning

Reviewed with Codex against this profile. Agreed next unit: a **VM-free
activation path** for bodies the compact tier already admits, selected in
`eval_direct_call_bytecode` *before* `Vm::new...`. On decline, the existing
`Vm` is built unchanged.

Expected movement, from Codex's read of the profile:

| unit | expected |
| --- | ---: |
| VM-free compact activation (keeping `CallEnv`, locals, register pools) | ~165-190 ns |
| lazy 112-byte `FrameColdState` extraction | ~228-235 ns |

So cold-state extraction is *not* the next recursion unit, even though it is
the cheaper change: `Vm` construction plus `FrameState` drop is only ~13.8% of
samples, and shrinking 704 bytes by ~104 net bytes touches part of that.

What the standalone path must preserve, in order of subtlety:

1. The callee-specific `CallEnv` from `direct_leaf_function_env` -- **not** the
   caller's. It performs `this` normalization, creation-realm selection,
   dynamic-realm marking, module-host routing, and private-environment install.
2. A value-level call helper. `CompactOp::Call` is *not* proven direct-leaf at
   compile time (only arity is checked), so the guard must stay: direct leaf ->
   `call_direct_leaf_function`, otherwise ordinary `call_function` semantics.
3. A value-level binary helper extracted from `vm_ops.rs`, not a duplicate of
   just the arithmetic cases -- `valueOf`, `toString`, `Symbol.toPrimitive` and
   `@@hasInstance` all reach user code.
4. Local loading beyond `locals[slot]`: TDZ diagnostics, received cells,
   realm-backed cells. V1 may decline module-import frames.

Predeclared gates against `815d8b79`: `nested_vm_constructions` 254,005 -> ~4 on
the 2,000-iteration sentinel; no `Vm::new`/`FrameState` drop/operand-stack
construction beneath compact activations in the profile; seven paired reps;
retain only if `recursive_call_tree <= 0.85x` and every other sentinel
`<= 1.03x`.

**Can recursion beat QuickJS-NG? Not with the current executor.** With `E` the
execution cost after construction is removed and `C` construction cost, parity
needs `E + C < 39.5 ns`, and the portfolio condition
(`recursive x string_key_map_churn < 0.49772`) needs `E + C < 19.66/s` ns.
`compact_fn::execute` self-time alone is ~56 ns and the whole executor category
~74 ns, so **the present construction budget is negative**. A plausible eventual
split is 20-25 ns of compact execution plus 10-15 ns of construction, which
requires a second structural stage after V1: compact-to-compact activation
switching in one loop, locals and operands in one register file, no per-call
`CallEnv`, no per-call vector resize or recycler borrow, and fewer `Value`
clone/drops.

Note the product condition means recursion need not win individually if string
churn wins strongly -- at `s = 0.40`, recursion may be as slow as 1.244x NG.

### 2026-08-01 VM-free activation for compact bodies: mechanism met, timing gate missed by 0.045

Built to the specification agreed with Codex above. `compact_fn/activation.rs`
runs an admitted body with no `Vm` at all. Selection moved from inside
`eval_direct_call_bytecode` to *before* `Vm::new...`; `env` and the direct-call
slots are passed as `&mut Option<_>` and taken only once admission is certain,
so a declined body builds exactly the frame it always did.

A `CompactActivation` is four fields -- borrowed `Bytecode`, the callee's own
`CallEnv`, `locals`, and the retained upvalue owner plus its slot mask --
against `FrameState`'s 36 fields and 704 bytes. The admitted body can use none
of the rest: it has no handler, cannot suspend, runs no loop plans, and holds
its operands in registers.

Entry guards, all checked before anything is consumed: the environment supplies
no named binding, has no module imports, no deopt bindings, and no dynamic
realm global; received cells are reachable from one retained function; and the
bytecode's own authoritative mask covers every slot the program reads. With no
per-slot cells the mask is `authoritative_mask_clean()`, so the check is two
loads rather than a per-local environment query.

Three semantics kept exactly one implementation rather than being reimplemented:
a direct-leaf callee reaches `call_direct_leaf_function`, any other callee
reaches `call_function`, and a non-numeric binary reaches
`operations::eval_binary` on an empty realm frame -- which is what the
interpreter's own path uses, since binary coercion never runs in the caller's
lexical environment.

**Mechanism gates: met exactly.** On the 2,000-iteration sentinel
(254,004 calls):

| counter | before | after |
| --- | ---: | ---: |
| `nested_vm_constructions` | 254,005 | **4** |
| `compact_standalone_activations` | -- | 254,001 |
| `compact_function_ops` | 4,680,009 | 4,680,009 |
| `executed_ops` | 34,245 | 34,245 |
| `ordinary_call_attempts` | 254,004 | 254,004 |

**Timing gate: missed.** Paired alternating A/B against a base rebuilt from
`63c775bd`, nine and eleven reps: `recursive_call_tree` **0.8909**
[0.8475, 0.9137] and **0.8956** [0.8783, 0.9148]. The predeclared bar was
`<= 0.85`. Per call that is 219 -> 195 ns, against Codex's predicted 165-190 ns
band -- the mechanism landed where predicted, the bar was set below the band's
midpoint.

Every other sentinel passed its `<= 1.03` bar: `prototype_method_call` 0.9979,
`polymorphic_call_site` 1.0116, `capturing_closure_call` 0.9994,
`heterogeneous_property_read` 1.0066, `string_key_map_churn` 1.0136. Six-case
geomean 0.9865.

Retained despite the missed bar, for reasons that should be argued rather than
assumed: the mechanism gate was met exactly, the change *removes* a code path
rather than adding one, no sentinel regressed, and it is the prerequisite for
the next unit -- compact-to-compact activation switching cannot be built on a
path that constructs a `Vm` per call. The honest reading is that 0.85 was a
slightly optimistic bar, not that the representation failed.

Semantic coverage added for the call path that changed: native, bound, Proxy,
and class-constructor callees, plus a live self-binding replacement observed
across calls. 1,966 runtime tests, 218/218 `compare-qjs` fixtures.

Remaining, and unchanged by this unit: `compact_fn::execute` self-time is still
~25% of samples and the per-call construction budget is still negative against
QuickJS-NG's ~35 ns. The next unit is the one Codex named: locals and operands
in one register file, compact-to-compact switching in one loop, no per-call
`CallEnv`.

### 2026-08-01 locals into the register file: the frame is now one flat buffer

Follow-on to the VM-free activation. Locals moved out of indexed frame storage
and into the low registers of the same file that already held the former
operand stack: register `n` is local `n`, and stack depth `d` is register
`local_count + d`. `LoadLocal`/`StoreLocal` therefore lower to `Move`, and the
whole per-call frame setup becomes one `resize` of a pooled buffer plus a
parameter copy.

This is only safe because of the admission narrowing landed in `63c775bd`:
every local a body reads is a parameter, a hoisted `var`, or a received
upvalue. Parameters are copied in, hoisted `var`s want `undefined` -- which is
what the resize already writes -- and an upvalue is read from its cell rather
than from a register. So there is nothing left for `Option<Value>` to
represent, and `Vm::initial_direct_call_slots`, `seed_direct_call_slots`, and
the `LocalSlotRecycler` round trip all drop out of the path.

Counters unchanged from the previous unit: `nested_vm_constructions` 4,
`compact_standalone_activations` 254,001, `compact_function_ops` 4,680,009,
`executed_ops` 34,245.

Paired alternating A/B against a base rebuilt from `be43c1f5`, nine and seven
reps: `recursive_call_tree` **0.9092** and **0.9074** [0.9025, 0.9110]. Every
other sentinel between 0.978 and 1.007; six-case geomean 0.9828.

Position against QuickJS-NG after this unit is recorded with the session
summary below.

### 2026-08-01 session close: where the compact tier leaves T021

Seven commits, all pushed, `main` green (1,966 runtime tests, 218/218
`compare-qjs`, Test262 slices clean at every commit):

| commit | unit | candidate/base |
| --- | --- | ---: |
| `815d8b79` | whole-function compact register executor | -- |
| `a69ecae6` | direct-leaf dispatch from registers | 0.9193 |
| `67555208` | upvalue-backed local read from its cell | 0.9914 |
| `63c775bd` | admission narrowed to initialized slots | (safety) |
| `be43c1f5` | VM-free activation | 0.8956 |
| `227f7d20` | locals into the register file | 0.9074 |

Sentinels against QuickJS-NG, quiet machine, seven paired reps:

| case | session start | now |
| --- | ---: | ---: |
| `recursive_call_tree` | 6.3253 | **4.4246** |
| `prototype_method_call` | 1.3413 | 1.3479 |
| `polymorphic_call_site` | 1.0277 | 1.0429 |
| `capturing_closure_call` | 0.9851 | 0.9474 |
| `heterogeneous_property_read` | 1.4796 | 1.4860 |
| `string_key_map_churn` | 3.0097 | 3.0092 |
| **geomean** | **1.8355** | **1.7250** |

Recursion is down 30%; the geomean is down 6%. T018's `<= 0.50x` contract is
not met and is not close.

**What this session established, which is worth more than the 6%:** the unit
ordering this task assumed since R2 was backwards. Dispatch elimination was
measured directly -- 100% of generic dispatch removed from the recursive body,
proven by counters -- and bought ~5%. Call construction was the cost. Acting on
that inverted ordering is what produced the remaining 25%.

**Why the sentinel still cannot win, stated as a budget.** Parity needs
`E + C < 39.5 ns` per call. After removing the `Vm`, the frame, the locals
vector and its recycler, `compact_fn::execute` self-time alone is still ~30% of
samples. The remaining path is ~15% `call_direct_leaf_function` (which inlines
`direct_leaf_function_env`) and ~18% `Value` clone/drop. Closing that needs the
things Codex named and this session did not build: compact-to-compact
activation switching in one loop, no per-call `CallEnv`, and fewer `Value`
clone/drops.

Codex's explicit constraint on the `CallEnv` unit, recorded because it is the
easy thing to get wrong: **do not reuse the caller's `CallEnv`**.
`direct_leaf_function_env` performs `this` normalization, creation-realm
selection, dynamic-realm marking, module-host routing, and private-environment
installation. For an admitted body most of those are no-ops -- so the opening
is *proving* that per body and skipping the work, not sharing the caller's
environment.

### 2026-08-01 compact-to-compact calls share the environment they would have rebuilt

The largest single unit of the session. A compact body calling another compact
body no longer constructs a `CallEnv`, and no longer passes through
`call_direct_leaf_function` or `eval_direct_call_bytecode` at all.

The argument is an equality proof, not a shortcut.
`new_direct_leaf_function_frame` derives realm, the global-lexical handles, the
immutable bindings, the module host and the agent context from its parent and
sets every other field to a fresh empty value. So two direct-leaf frames in one
realm can differ only through what the four remaining steps of
`direct_leaf_function_env` add: a `this` binding, a marked call realm, a module
host, and a private environment. `shares_caller_environment` rejects a callee
that would add any of them -- and, for the module host, accepts the ordinary
case where callee and caller carry the *same* handle, since installing it again
is idempotent. When it accepts, the environment the callee would have built is
equal field for field to the one the caller holds, so borrowing it is not
"reusing the caller's environment"; it is skipping the reconstruction of an
identical one.

`CompactActivation::env` is therefore `&CallEnv` rather than an owned one.
`is_direct_leaf_function` remains the outer gate: it is what proves seeding
parameters into slots is safe for a callee at all, and the compact program's
own admission does not subsume it.

**This condition was wrong on the first attempt, and the counters caught it.**
Requiring `function.module_host.is_none()` looked obviously right and admitted
*nothing*: an ordinary script's functions all carry the host of the environment
that created them, so `compact_direct_calls` was 0 while the timing moved
1.3% in the wrong direction. A probe on the predicate found it in one run. The
lesson is the one this file keeps relearning -- assert the mechanism fired
before believing a timing result, in either direction.

Mechanism, 2,000-iteration sentinel (254,004 calls):

| counter | before | after |
| --- | ---: | ---: |
| `compact_direct_calls` | -- | **252,000** |
| `direct_leaf_frames` | 254,004 | **2,004** |
| `ordinary_call_attempts` | 254,004 | 254,004 |
| `nested_vm_constructions` | 4 | 4 |

99.2% of calls now build neither a frame nor an environment. The counter
invariant that every attempt is attributed to exactly one tier still holds,
which is why `compact_direct_calls` is a separate counter from the mechanism
counter `compact_standalone_activations`.

Paired alternating A/B against a base rebuilt from `50be13ec`, nine reps twice:
`recursive_call_tree` **0.7007** and **0.7036** [0.6997, 0.7106]. Every other
sentinel between 0.9937 and 1.0052; six-case geomean **0.9443**.

### 2026-08-01 the TDZ regression the local gate could not see

`be43c1f5` (VM-free activation) took `actionable_gap` from 0 to **12** in the
Test262 Coverage workflow, and three commits landed on top before the failure
was noticed. The 12 are one family: `closure-get-before-initialization` for
`let`, `const`, `using` and `await-using`.

Cause: rewriting the tier to run without a `Vm` removed the general-path
fallback, and the uninitialized-lexical check on `LoadUpvalueLocal` went with
it. A closure called before the `let` it captures is reached returned
`undefined` instead of raising `ReferenceError`.

The admission narrowing in `63c775bd` looked like it covered this and does not.
It proves every local a body *reads* is a parameter, a hoisted `var`, or a
received upvalue -- but whether a received cell is *initialized* is a fact about
the caller's execution, not a property of the callee's body. No static
admission can rule it out. Fixed in `001b216b` by checking at the read.

**Why the local gate missed it, which matters more than the bug.** The pre-push
gate runs `tests/test262/allowlist.txt` -- 5,160 cases at the time. The CI
workflow runs the full configured inventory, 42,672. None of the 12 were in the
allowlist, so every local run was green. Nine are in it now; the three
`await-using` cases are excluded because the subset runner rejects their `async`
flag.

The general lesson for this tier: an admission predicate can only constrain the
*body*. Anything about the caller's state -- cell initialization, receiver
identity, environment shape -- has to be checked where it is read or at entry,
never assumed from what the body looks like.

### 2026-08-01 rejected: virtual stack / copy propagation in the compact compiler

`LoadLocal` lowering to a `Move` is redundant when the value is already in a
register -- the local's own. Recording "abstract depth d is local s" and letting
the consumer read `s` directly removes about a third of the emitted operations
for the recursive sentinel, materializing only where the layout must be real:
branches, merge points, and a call's contiguous argument window.

Implemented and **rejected**: it produced wrong results twice, and the second
diagnosis was still incomplete.

1. Aliasing. A stack level that *names* a local rather than holding a copy
   observes a later `StoreLocal` to that local, when the language says it
   captured the old value. Fixed by materializing aliases before the write.
2. Merge edges. `cond ? a : b` whose arms share a `Return`: the branch edge
   materializes before jumping, but the fall-through edge does not, so `Return`
   read an empty canonical register. `pick('y')` returned `undefined`.
   Materializing on the fall-through edge into a branch target did **not** fix
   it, which is where this was abandoned -- the remaining defect is not
   understood, and a data-flow transform whose failure mode is silently wrong
   values is not worth carrying on an incomplete model.

Whoever picks this up: the payoff is real (roughly a third of dispatched
operations) but it needs a proper liveness/aliasing analysis over the compact
IR, not the incremental holder-array approach tried here. Reproduce with
`var s = Symbol('s'); function pick(k){ return k === s ? 'S' : k; } pick('y')`,
which must be `'y'`.

### 2026-08-01 computed string keys: borrow before allocating

With recursion down to ~3.0x, `string_key_map_churn` became the largest single
gap. Its profile is not the dispatcher: allocation and free are **10.7%** of
samples (`_xzm_free` 3.5%, `xzone_malloc` 2.3%, `malloc_zone_malloc` 2.3%,
`_free` 1.4%, `finish_grow` 1.2%).

The cause is one line. `PropertyKey::String` owns a `String`, and
`try_to_property_key_without_coercion` builds it with `value.to_string()`, so
`table[key]` copies the key on **every** access -- while the lookup that
consumes it, `try_direct_get_string`, already takes `&str`. A 600k-iteration
dictionary loop therefore performed 1.2M allocations to answer 1.2M borrows.

Both sides now try the borrow first and build the owned key only on a miss:

- read: `try_direct_get_string(&object, name.as_str())` before
  `coerce_property_key`;
- write: `write_existing_own_data_property(name.as_str(), &value)` before it,
  under exactly `set_property_value`'s own guards. `symbol_primitive_set_fails`
  needs no restating -- it returns `false` for every string key by inspection.

`ToPropertyKey` on a string is side-effect free, so trying the borrow first is
observably identical rather than merely usually right.

Paired alternating A/B against a base rebuilt from `cf77711d`, nine reps:
`string_key_map_churn` **0.8061** [0.8021, 0.8119]; read-only intermediate
measured 0.8986. Every other sentinel 0.9901-1.0017; six-case geomean 0.9612.

Eight focused tests pin what the borrow must not change: non-writable
properties in both modes, own and inherited setters, property creation, frozen
objects, Proxy traps, getter/prototype walks, and global-object writes.

### 2026-08-01 a polymorphic shape cache for typed-loop property reads

`heterogeneous_property_read` already runs natively -- `typed_loop::run` is 60%
of its profile -- but `slot_reads` was **27%** and `memcmp` 3.1%, which is a
cache that never hits. Two reasons, both in `get_named`:

1. Its cache was one way. The sentinel rotates **three** distinct
   object-literal shapes through a single read, so a one-way cache misses every
   iteration by construction.
2. `shared_data_slot`, the only lookup it could record, answers **only**
   `PropertyStorage::Small`. These receivers have four or five properties and
   are `Shaped`, so the cache was never populated at all -- every access
   resolved the name.

`literal_data_slot` / `literal_data_slot_value` already exist and are the right
primitives: they key on `Rc<ObjectLiteralShape>` identity and re-check the
property revision, so validation is a pointer comparison and a stale entry
misses rather than reading a wrong slot. `get_named` now remembers up to four
shapes per site and scans them after the one-way cache misses.

**Where the cache lives took three attempts, and the counters were not the
guide -- a control run was.** `prototype_method_call` regressed ~3.6%, and a
base-against-base control put this machine's noise floor at **0.5%**, so it was
real. That sentinel declines this tier on essentially every backedge, and a
declined backedge still moves the scratch twice:

| shape cache location | `prototype_method_call` |
| --- | ---: |
| widened `caches` entry | 1.0372 |
| own array in the scratch | 1.0296 |
| own array, slow path `#[inline(never)]` | 1.0414 (and halved the win) |
| **on the program, boxed ways** | **1.0270** |

Keeping it on `TypedLoopProgram` rather than in `TypedLoopScratch` is also the
better model: a site's shapes are a property of the site, not of one
activation, so a short loop entered repeatedly now reuses what it learned.
Boxing the ways vector is deliberate against `clippy::box_collection` and
measured: unboxed is 3.9% against 2.6% boxed.

Paired alternating A/B against a base rebuilt from `6db1fe43`, thirteen reps:
`heterogeneous_property_read` **0.8784** [0.8631, 0.8940],
`prototype_method_call` 1.0270 [1.0138, 1.0424], others 0.9925-1.0065.
Six-case geomean **0.9837**.

Six tests pin the invalidation: polymorphic reads, a shape change mid-loop, an
overwrite, a delete, six shapes against four ways, and an accessor shadowing a
cached shape.

### 2026-08-01 the biggest remaining cost in the compact tier was `drop_in_place`

With the environment and the frame gone, the recursive sentinel's profile was
`compact_fn::execute` ~50%, **`drop_in_place<Value>` 22%**, `Value::clone` 10%.

That 22% is for registers that, in this body, only ever hold numbers. `Value`
is 16 bytes with `needs_drop = true`, so `registers[dst] = value` drops the old
value -- and `drop_in_place::<Value>` stays an out-of-line call. Every register
write in an 18-operation activation paid one, as did the per-recycle reset.

Register stores now go through an `#[inline(always)]` helper that tests the
discriminant first and `mem::forget`s a value that owns nothing. The list is
**positive**: `Number`, `Boolean`, `Null`, `Undefined` are forgotten, and
everything else -- including any variant added later -- takes the ordinary
drop. A mistake in that direction costs a branch, never a leak. The recycler's
reset uses the same test instead of `Vec::fill`.

Paired alternating A/B against a base rebuilt from `27702b16`, eleven reps:
`recursive_call_tree` **0.7550** [0.5485, 0.7746]. Every other sentinel between
0.9979 and 1.0078; six-case geomean **0.9555**.

Three tests cover what the shortcut must not break: non-primitive values
flowing through an admitted body, a register alternating heap and primitive
values across 200 activations, and string accumulation across a recycled
buffer.

### 2026-08-01 rejected: making the register reset conditional

With the inline drop test landed, `recycle_registers` became the second entry
in the recursive profile at **10.9%** -- a full sweep of the register file per
activation, for a body whose registers only ever hold numbers.

The obvious repair: track whether the activation ever stored a non-primitive,
and skip the reset when it did not. A fresh activation blanks its hoisted
locals (parameters are seeded, and stack registers are provably written before
they are read), so skipping is safe.

Measured **1.1348** [1.0016, 1.1447] against `377f614a` -- **13.5% slower**.
Reverted.

The reason is the shape of the trade, and it is worth stating because it keeps
recurring in this file: the flag costs one test per `store` -- about eighteen
per activation -- to avoid one sweep of about seven registers per activation.
Per-operation cost on the hot path beats per-activation cost on the cold path
almost every time, even when the operation counts look favourable. The same
mechanism explains why `mem::replace` lost to `Value::clone` earlier, and why
an extra match arm in `run_current_activation` costs 6.8%.

That also rules out the analogous change in the generic interpreter's
`StoreLocal`, which was the next candidate for the inline drop test:
`run_current_activation` is 24,600 instructions with a saturated register
allocator, so adding a test there is the same trade at worse odds.

### 2026-08-01 the typed-loop churn blocker, found by tracing instead of guessing

Six earlier hypotheses about why `string_key_map_churn`'s loop is never
admitted each failed with byte-identical counters. The note left for this
session said to stop inferring and trace every `?` in the walk loop. Doing that
answered it in one run -- and the answer is not where any of the six looked.

Instrumenting each exit of `Builder::compile`'s walk gives, per fixpoint pass:

```
pass 1   ip=21  compile_op GetProp            (normal: records a discovery)
pass 2   ip=23  compile_element_assignment
pass 3   ip=23  compile_element_assignment
```

The walk never reaches op 29. It exits inside `compile_element_assignment` at
ip 23 -- and that call is not a failure to lower an instruction, it is
`table[key] = (table[key] || 0) + 1` matching the *array element assignment*
idiom by shape and then discovering it is not one. Probing inside it:

```
pass 2   inner=29  compile_op GetProp          (computed string key)
pass 3   inner=30  control_flow JumpIfTrue(33)  (the `||`)
```

So there are **two** blockers in series, which is why the earlier prototype
that added `GetComputedString`/`SetComputedString` still saw no admission: the
`||` would have stopped it anyway.

The structural defect underneath is the interesting part.
`compile_element_assignment` lowers to exactly one thing, `TypedOp::DenseWrite`,
so a dictionary receiver was never expressible there. But it discovers that
*after* emitting the key expression, and at that point "cannot lower this" has
to be `None` -- which aborts the **whole region** -- rather than `Some(None)`,
which would hand the instruction back to the ordinary per-instruction path.
A pattern that does not apply was taking down everything around it.

`8880c0c3`'s successor moves the branch-free requirement to a pre-scan, before
anything is emitted, so that case now declines cheaply and leaves the region
alive. Sentinels are unchanged (0.9915-1.0078, geomean 0.9982): churn still
declines, because the remaining blocker is the computed string key itself.

**What this leaves for the churn unit**, now that its shape is known rather
than guessed: `TypedOp::GetComputedString`/`SetComputedString` over boxed
registers, with the runtime helpers that already exist and take `&str`. The
region will then compile as ordinary instructions rather than through the
element-assignment idiom, so no join bookkeeping for elided temporaries is
needed. Note the receiver's array-ness is not decidable at compile time --
`receiver_index` only allocates an index, and `seed_registers` is what
validates -- so the ordinary path is the correct home for this shape.

### 2026-08-01 churn admission: the fixpoint now advances, and where it still stops

Building on the region-survival fix above, a prototype of the computed-access
unit was written and **not landed**. It is recorded because it converts "six
failed hypotheses" into a measured sequence, and because the remaining blocker
is now one specific thing rather than an unknown.

The prototype: `TypedOp::ComputedRead`/`ComputedWrite` over boxed registers,
whose runtime helpers discriminate `(Object, String)` and `(Array, Number)`
instead of asserting array semantics -- Codex's round-7 correction, that
`boxed_element_reads` conflates "this read's result must be boxed" with "this
read is an array access". Plus a `GetProp` arm that emits a computed access for
a boxed key and demands a boxed key when the existing one came from an element
read, and the `SetProp` arm that `compile_op` never had at all.

With it, the fixpoint **advances every pass instead of standing still**:

```
pass 1  ip=21  GetProp        (normal discovery)
pass 2  ip=29  GetProp        computed string read
pass 3  ip=24  StoreLocal(12) receiver temp must be boxed
pass 4  ip=39  SetProp        no arm existed
pass 5  ip=39  SetProp        operands not yet boxed
pass 6  ip=26  StoreLocal(13) index temp
pass 7  ip=22  StoreLocal(7)  key slot
pass 8  --     slots 7/12/13 all store successfully
```

Every one of those was a real missing capability, and each was answered. The
walk stops producing `compile_op` failures entirely by pass 8.

**It still declines, and the remaining stop is now located but not identified.**
A probe on the post-fixpoint checks never fires, so `compile()` never reaches
them: the fixpoint gives up because pass 8 fails *without* recording a new
discovery, and that failure is no longer in `compile_op` -- it is one of
`normalize`, `merge_states`, `open_site`, `compile_element_assignment`, or the
forward-jump patch. Those are exactly the five places to instrument next.

Not landed because the two new operations never executed: unverifiable code is
not a performance change. The diagnosis is the deliverable; reproduce it by
instrumenting each `?` in `Builder::compile`'s walk and running
`string_key_map_churn` with two iterations.

**Process note, now three sessions old.** Every inference about this blocker
has been wrong and every trace has been right. Instrument the exits; do not
reason about where a trace happens to print.

### 2026-08-01 the churn loop is admitted: computed string keys land

Seven prior attempts failed to admit `string_key_map_churn`'s loop. Tracing
each `?` -- rather than inferring -- found the chain, and each link turned out
to be a real missing capability rather than a bug:

| what was missing | where it stopped |
| --- | --- |
| region survives an inapplicable element assignment | `ip=23` (landed `58f2dcf0`) |
| a computed read that does not assert array semantics | `ip=29` |
| a `SetProp` arm -- `compile_op` had none at all | `ip=39` |
| boxed compiler temporaries for receiver/key/value | `ip=24/26/22/40/42` |
| a join that widens a scalar to meet a boxed value | `ip=33` `merge_states` |
| a seed that lets a boxed register hold a string | `seed_registers` |
| creating a property, not only overwriting one | first iteration |

The last two are the ones that had gone unexamined for three sessions.

**`value_is_ordinary_object` in the seed** rejected the region at entry even
after it compiled, because the key register holds a *string* -- which is the
entire point of a computed access. That check was redundant: every operation
consuming a boxed register already checks what it holds and deoptimizes
(`get_named` needs an object, `element_read` an array, `Unbox` and
`call_numeric_native` bail, and the new computed access discriminates). Audited
all of them before widening it.

**Property creation.** `write_existing_own_data_property` only overwrites, so
the first iteration -- which creates the key -- deoptimized, and a deoptimized
program is never retried for that frame. The loop was lost to its own first
write. `Vm::try_create_ordinary_own_data_property` handles the rest and
re-checks extensibility, exotics and the prototype chain itself.

**One narrowing, driven by the corpus rather than the sentinel.** The first
version also demanded a boxed key from every read whose key came from an
element read. That admitted churn, and it also forced an ordinary `a[b[i]]`
through the computed access: 40-case corpus geomean 1.0102 with `regexp-dna` at
**1.191**. Removing that discovery leaves churn admitted -- unchanged counters
-- because the *write* drives the chain and the read follows a pass later.
Corpus is then **1.0009**, worst case 1.0353.

Sentinels against a base rebuilt from `377f614a`, nine reps:
`string_key_map_churn` **0.4195** [0.4148, 0.4236]; every other sentinel
1.0019-1.0192; geomean 0.8715. Checksums byte-identical at 1k, 100k and 2M
iterations. `declined_loop_plan_edges` 4046 -> 2047 and `executed_ops`
125,502 -> 51,515 for the 2,000-iteration counter run.

Seven tests cover what the tier must not change: the churn loop's own totals,
create-then-overwrite, prototype-chain reads, an accessor declining rather than
being read as a slot, a frozen receiver, a numeric index still reaching the
array after its producer is boxed, and a join keeping both branch values.

### 2026-08-01 remembering where an inherited property resolved

With the churn loop admitted, `prototype_method_call` (1.38) became the largest
non-recursive gap. Its profile is not the dispatcher either: `typed_loop::run`
53%, then `slot_reads` 12.7%, `ordinary_data_property` 3.7% and `memcmp` 2.2%
-- roughly 19% resolving one name.

The name is `advance`, and it lives on `Stepper.prototype`. The shape cache
added earlier only records *own* properties, so every iteration walked the
chain: an own miss on the receiver, then a hash lookup and `memcmp` on the
prototype.

`get_named` now remembers the holder, its property revision, and the slot.
Revisiting is a prototype pointer comparison, a revision comparison, and a slot
read. Soundness rests on the own lookups that already run first: a property
that later appears on the receiver shadows the remembered one rather than being
masked by it. Only the immediate prototype is remembered -- a deeper chain
still walks, so no multi-level invalidation problem is created.

Paired alternating A/B against a base rebuilt from `e1ebc45d`, nine reps:
`prototype_method_call` **0.8991** [0.8873, 0.9094]; every other sentinel
0.9875-1.0241; geomean 0.9815.

Six tests cover the invalidation paths: an own property shadowing the
remembered one, mutating the prototype mid-loop, adding an own property
mid-loop, replacing the prototype with `setPrototypeOf`, and an inherited
accessor declining rather than being read from a slot.

**A correctness defect surfaced while writing those tests**, unrelated to this
session's work and present in every earlier build: `(true ? a : b).x` evaluated
to the object rather than to `x`, because the named-property peephole removes a
`LoadLocal` that a conditional's already-patched `Jump` lands on. Fixed
separately in `e6aa57e1` with a `compare-qjs` fixture.

### 2026-08-01 rejected again: moving register blanking from recycle to entry

`recycle_registers` is 11.1% of the recursive profile, so it keeps looking like
a target. The first attempt (a flag to skip the sweep entirely) measured 13.5%
slower. This second attempt changed shape: the recycler rewrites only registers
that actually own something -- a numeric register is read and compared but not
written -- and a `blanked_locals` list re-initializes the hoisted locals at
entry instead. No per-store cost this time.

Measured **1.0283** on `recursive_call_tree`. Reverted.

Walking a `Vec<u16>` of slots at entry, with its bounds check per element,
costs more than the straight-line writes it removes. Two different shapes of
the same idea have now lost, which is enough to call it: **the register sweep
is not where this sentinel's remaining time is.** 61.8% is inside `execute`
itself and 14.5% is `run` plus `call_from_activation` -- the per-activation
setup and the Rust-stack recursion around it. Those need compact-to-compact
activation switching in one loop, not another way to arrange the same writes.
