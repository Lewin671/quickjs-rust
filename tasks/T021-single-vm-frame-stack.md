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

### 2026-07-27 rejected bounded numeric control/call-graph leaf

A fresh `imaging-darkroom` profile showed a bounded pure-numeric call graph
(`ProcessImageData -> FastGain -> FastBias -> FastLog2`) whose repeated
branches and direct calls still entered the ordinary VM. The candidate compiled
only a small statically proven numeric control-flow/call graph, with strict
limits on locals, operands, call depth, and operations. Any dynamic binding,
redefined intrinsic, accessor, Proxy, unsupported opcode, or uncertain
receiver falls back to the ordinary VM. Focused coverage exercised signed zero,
captured numeric calls, `Math` rebinding fallback, and observability of an
unbound discarded global. The full local gate passed, including 5,160 curated
Test262 cases and the comparison fixtures.

The one-attempt fast screen used strict receipt-verified three-engine evidence
from clean candidate commit `64eb8cd46d1a29a1ea21146a8f5e65baabfb8b4b`
against base `7fffa88d9c70dd33ef51c786f8d0930447792d16`. The broad report
SHA-256 is
`57f7a48f1071366a1efda997292cbd0de4b9256f17ea9e3cb8edf3789e03e087`; the
complete external report SHA-256 is
`a3fd8c1eba6d9569ee4832f2225737da9fb7ecfcd9e925b28001054162d254cf`.
`imaging-darkroom` met its target at **0.814908x** candidate/base, while
`audio-oscillator` was 1.001979x, `math-spectral-norm` was 0.991263x, and
broad `local_read` was 1.001456x. However, the independently sourced
JetStream `hash-map` control regressed to **1.074431x**, exceeding its frozen
1.03x ceiling. T022 therefore recorded the unit as rejected; the candidate
was reverted by `ee8dbdc7` without rewriting published history.

Do not retry this eager numeric-control planner by widening its bytecode
classifier, changing its thresholds, or adding another speculative probe. The
unrelated `hash-map` regression shows that plan construction/dispatch is not a
net shared win at this layer. A successor must start with a new profile of a
different shared cost.

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

### 2026-07-27 rejected own-slot cache for non-Exact values

The current, receipt-bound queue at `e9841e32` still has closed recursion at
rank 1, followed by JetStream `raytrace-public-class-fields` (5.742x
QuickJS-NG), Kraken `ai-astar` (5.568x), and JetStream `hash-map` (5.498x).
Fresh profiles found the same named-own-property cache family in all three:
`NamedPropertyCache::get` accounts for 26/707 top-stack samples in raytrace,
35/727 in ai-astar, and 11/725 in hash-map. The corresponding current profile
SHA-256 values are `9fc21a797ae0b52d351388be238bf686340db011d9d4cc53d360c652bf1ef8ef`,
`cd32fe54624e394c5dd3ed9e7bdb0d4125d929d9bae28aceac514656253b8f4f`, and
`31789088096705391d76ace722575079e2747440bcca0d0ffa88710ef95c8e81`.

The frozen unit plan is
`tasks/performance-units/own-slot-heap-value-cache.json` (SHA-256
`358a64f123d0e7f958eb19425fb0b86abd73c84c96cbc2909298486426a4b8bb`). It
tested a distinct mechanism from the previously rejected prototype cache:
after an existing ordinary own-data read had proved a slot but the value could
not be kept in an `Exact` entry without retaining heap state, retain only the
weak receiver, layout revision, and slot; each hit re-reads the current value.
The focused cache test covered value replacement, structural invalidation, and
weak receiver lifetime.

The one permitted dirty-candidate fast screen used candidate release SHA-256
`a5c38162c2a444a6b3d95ca68bee13c06f0886efa37e43899342418e40b52bc6`, exact
`e9841e32` base SHA-256
`a490883b34e1f7304089d120f7046a133f53b9728929670f9de0a2dee18ae81d`, and
pinned QuickJS-NG SHA-256
`cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0` across
all 45 external cases and three role-rotated blocks. Raytrace met its frozen
target at **0.969856x** candidate/base and hash-map improved to **0.953756x**,
but independent target ai-astar regressed to **1.014760x**, missing the
required `<= 0.980000x` shared-target gate. Controls remained within their
1.03 ceiling: controlflow-recursive **0.939998x** and date-format-tofte
**1.001592x**. The external report and raw SHA-256 values are
`9d1dd5bbcd80562e97b9a0ddffef8837ec9f2a223640f0f887cc9a3a3ca45f29` and
`d00b555eebec88928a2b84c3b318ce134686b96e4fa628bd208d5318ec2e852f`.

The runtime change and its focused test were reverted immediately. Do not
retune this non-Exact own-slot fallback by expanding its value types or cache
capacity: it is not a shared improvement across the declared targets. A
successor must target a different current-profiled cost.

### 2026-07-28 rejected static dense-array numeric leaf reads

A fresh current-candidate profile of SunSpider `3d-raytrace` used the
unchanged upstream source (SHA-256
`87a1cb968113dcaf427dc2634e95f6ee6460f38e132c26cdf640639521620591`) and
showed repeated direct-leaf VM entry alongside dense element reads, numeric
binary operations, and `Value` lifetime work. The sample receipt is
`/private/tmp/qjs-profile-3d-raytrace-6cac-v2.sample` (SHA-256
`20a39e7669457da1004833d99cc98fe8394195f2cf593e35a8dd909b9c0238ff`).

One narrow prototype extended the existing preclassified numeric own-property
leaf plan only for a compiler-fused parameter `GetPropIndex` with a static
index. It accepted a direct function such as a vector dot product only when
each receiver was a real dense Array and every accessed own element was a
Number. Holes, indexed descriptors, non-Number values, non-Array receivers,
and every unfused or dynamic index returned to the original VM before any
observable access. Focused tests covered current element values plus getter
and coercion fallbacks; the exact upstream result remained identical.

The prototype failed its first end-to-end fast gate and was removed. Seven
alternating process pairs used the unchanged source followed by twenty extra
`raytraceScene()` calls in the same Realm solely to make timing measurable.
Candidate release SHA-256 was
`de97ff6418e00677e5357682a9ad4ca0bf537c8ebb9e9ac6c38424ea71411e73`; the
exact `6cac` base release SHA-256 was
`b861e96bf99af4c8fd5e50c044afcb05d9bf9b7eb79616ba8a095388e5c8bbd7`.
Candidate times were 1.65, 1.64, 1.63, 1.63, 1.63, 1.63, and 1.63 seconds;
base times were 1.60, 1.60, 1.61, 1.61, 1.60, 1.61, and 1.61 seconds. The
medians are **1.63s candidate / 1.61s base = 1.012422x**, a regression rather
than the predeclared at-most-0.90x retention gate. This is diagnostic evidence
only, not an external-suite performance claim.

Do not retry this static dense-array direct-leaf read shape by adding more
fixed vector indices or arithmetic forms. A successor must remove a different
profiled shared cost and demonstrate an end-to-end win before broadening its
semantic surface.

### 2026-07-28 rejected prepared native global RegExp match

The refreshed evidence queue ranks SunSpider `regexp-dna` ninth at 3.771672x
QuickJS-NG. A fresh diagnostic profile of eighteen calls of the exact
408,689-byte upstream source (wrapped only to make sampling long enough)
showed 3,938 of 4,247 user-space samples through
`native_regexp_prototype_match` / `global_match` / `regexp_exec` /
`native_regexp_prototype_exec`. The receipt is
`/private/tmp/qjs-profile-regexp-dna-6cac-v1.sample` (SHA-256
`ec0f1d3bcd01aabc623c07573f286357614ad7476b7b575369665549b6fb2662`).
Although `simple_atom_boundaries` is visible there, its removal is already
closed by paired negative evidence; this unit instead tested the distinct
native `/g` protocol cost recorded in
`tasks/performance-units/regexp-dna-native-global-match.json`.

The one prototype retained the required observable `flags` read and initial
`lastIndex` write, then admitted only an unchanged realm-native RegExp with no
own `exec`, the current realm `RegExp.prototype`, and its original native
`exec`. It reused one `PreparedRegexp` and one prepared input through the
global scan and materialized only the full-match strings that global
`@@match` returns. Non-global patterns, custom exec methods, altered
prototypes, RegExp-like receivers, and mismatched global/Unicode bits retained
the ordinary protocol. A first diagnostic profile exposed a non-performance
guard bug: literal flag source order such as `ig` differs from canonical
`.flags` order `gi`. The repaired guard compared the semantic global and
Unicode bits; the second profile directly enters `PreparedRegexp::match_input`
from `native_regexp_prototype_match` with no global-match/exec loop in the
hot chain. Its receipt is
`/private/tmp/qjs-profile-regexp-dna-native-global-match-v2.sample` (SHA-256
`e5d7f5bfb9fc5a82463cfb793ec1a49c37c2a857ce633a5ebf6fdd7e9c73f3cb`).

The exact upstream output hash was identical for candidate and base:
`770c2d18aa4ec3159f46da461ef452972949942ff4f6a9771044485faa146398`.
Focused `regexp_symbol_match` tests passed, including captured global output,
canonicalized flag ordering, empty Unicode matching, and own/prototype `exec`
fallbacks. Candidate release SHA-256 was
`66bc7ebeb67c7be1e93bbed8ddaa3579c898161d02ca8189d4dfd821fd75ed39`; exact
`6cac` base release SHA-256 was
`b861e96bf99af4c8fd5e50c044afcb05d9bf9b7eb79616ba8a095388e5c8bbd7`.

Despite confirmed admission, the predeclared fast gate failed. Seven
alternating exact-source process pairs had candidate times 405,833,960,
404,337,883, 404,986,143, 404,706,955, 406,368,017, 405,165,911, and
405,169,010 ns, versus base times 422,340,870, 421,157,122, 422,080,994,
422,096,968, 422,369,003, 422,354,937, and 422,410,965 ns. The medians are
**405,165,911 / 422,340,870 = 0.959334x candidate/base**, far short of the
frozen `<= 0.75x` retention gate. The prototype and its focused tests were
reverted immediately; no Test262 or broad-suite promotion work is warranted.

Do not retry prepared native global `@@match` by reshaping its guard, reusing
the input/matcher again, or omitting its intermediate exec-result array. The
confirmed gain is only about four percent. A successor must attack a different
current-profiled matcher cost and must remain distinct from the already
rejected simple-quantifier boundary-vector route.

### 2026-07-28 rejected conditional bit-test / shift control loop

The current evidence-bound queue placed SunSpider `bitops-bits-in-byte` at
rank 14 (3.209996x QuickJS-NG). A fresh exact-bytecode inspection showed a
pure local loop with the sequence `b & m`, conditional local increment, and
`m <<= 1`; it was already admitted by the generic typed-loop executor but not
by either existing scalar-bitwise or counter-and-constant control plans. The
profile receipt is `/private/tmp/qjs-profile-bitops-bits-in-byte-6cac-v1.sample`
(SHA-256 `7e3ec1c5120cca529609c678a3137aa222eae89c1df04ee173fb9fad3e5c541a`).

The one frozen unit at candidate `e339b055a39703ba734ada6b416997d3327e1150`
used a strict local-only matcher and a bounded `ToInt32` recurrence proof; any
captured binding, dynamic scope, non-number, unsupported completion shape, or
unproven exit used the existing execution path. Focused execution and
fallback tests passed before the exact candidate was measured against base
`6cac3f50d2c17195d52fa94eaafbcf97ab6f2447` from two clean isolated source
trees.

The receipt-bound three-block external report is
`/private/tmp/qjs-construct-parent.y39c53/repo/target/perf-bitwise-conditional-shift-e339-vs-6cac-retry1/external-report.json`
(SHA-256 `c8bc240f32cdcd4c988f71cc659d49a73a7d3d5b390e42d9070b9f023470eeaf`),
with raw samples SHA-256
`7b47b13de2fc2d2da3065783a637d4829ee26c60648cabf228e8ef58d06c44cd`.
The target improved from 87.152 ms to 48.624 ms, or
**0.557922x candidate/base** and **1.810x candidate/QuickJS-NG**, satisfying
its target and explicit under-2x reference condition. But the independent
`bitops-bitwise-and` control regressed to **1.065019x candidate/base**,
exceeding the frozen 1.03x ceiling. The other controls did not explain that
away: `bitops-3bit-bits-in-byte` was 0.990724x, `math-partial-sums` 0.984193x,
`access-nbody` 1.012730x, `local_read` 0.999770x, and
`plain_function_call` 1.002229x. The broad report SHA-256 is
`043daa189115e1ae28fb69838dab3904f7b9e97c0fd7180d2af78441fa37b9c5`; its
hosted-preview health is informational/non-claim, and the external report is
also incomplete for qjs-rust `imaging-gaussian-blur`, so it cannot substitute
for a promotion claim.

The bound fast decision is **rejected** solely for the control regression;
its JSON receipt is
`/private/tmp/qjs-construct-parent.y39c53/repo/target/performance-decision-bitwise-e339-fast.json`
(SHA-256 `5b381dccaddfc1429b3a070341e367d9c8e0f7a55bb29c0fd48f3cec9d10717f`).
The runtime and frozen plan were reverted by `2a66c9cd` rather than retaining
a benchmark-shaped local win. Do not retry this conditional-shift matcher by
widening its bytecode grammar, relaxing guards, or adding more shift variants:
the shared control-plan admission/dispatch cost is not a net win under its
declared independent control. A successor must begin from a new profile of a
different shared cost.

### 2026-07-28 rejected scalar numeric-leaf bitwise / constant-right lowering

Queue rank 17, SunSpider `bitops-3bit-bits-in-byte`, entered the existing
numeric-leaf executor on every `fast3bitlookup` call. The current 400-times
profile receipt is
`/private/tmp/qjs-profile-bitops-3bit-641-v1.sample` (SHA-256
`38fcb6bfa72a2bdee41f1dd4adb8550fd59593a8a034271a9104ef7d7c25c2dc`): of
7,411 samples, `try_eval_numeric_leaf` had 2,147, `fast_number_binary` 1,070,
and `direct_number_binary` 794. The frozen one-attempt plan is
`tasks/performance-units/numeric-leaf-scalar-bitwise.json` (SHA-256
`c5581bbd41151b55fbe62ffc22f880c6beed4e9cf261e71c1cbe12989c51c9f7`). It
tested a general lowering only after the existing primitive-number leaf
admission: direct scalar evaluation of bitwise/shift operations and immutable
right-literal `ToInt32`/`ToUint32` preparation. It introduced no source,
function-name, workload, input-size, call-graph, or new-admission condition.

The candidate profile
`/private/tmp/qjs-profile-bitops-3bit-const-right-v1.sample` (SHA-256
`adf8d362f815797e265fa311499ac1f28dcbb3246d711bd1d845d7ad5fc2ec55`) did
remove the inner leaf `fast_number_binary` cost: `try_eval_numeric_leaf` was
1,802 samples, `direct_number_binary` 612, and
`direct_bitwise_const_right` 242. One preliminary 400-times target wrapper
diagnostic measured 12.48 seconds for candidate versus 16.61 seconds for base
(about **0.751x**); repeated `crypto-md5` and `crypto-sha1` diagnostics were
about **0.865x** and **0.868x**. These timings were diagnostic only, not a
promotion or fast-retention claim.

The earliest independent broad control already falsified the unit. The
three-block, candidate/base-only raw screen is
`target/benchmarks/fast-numeric-leaf-scalar-bitwise-controls.jsonl` (SHA-256
`de83fc8d8e3c2d74a45f20923799ce36b50c7ee6a213599952b23ab522f1683a`). It is
explicitly dirty, provenance-unverified, partial (3 of 25 broad cases), and
`claim_eligible: false`; its normalized median ratios are therefore negative
screen evidence only. `plain_function_call` regressed to
**1.140703x candidate/base**, beyond the frozen 1.03x control ceiling, while
`branch_arithmetic` was 1.000139x and `dynamic_method_call` 0.999877x. The
candidate binary SHA-256 was
`25f3e198168e3cfce90671d47f553d40b0239cb72a88330713677914d6a8d47f`; the
exact-base binary SHA-256 was
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`.

The unit is **rejected and reverted** before a complete external/broad run or
Test262 scan: a failed frozen control cannot be rescued by filling in the
remaining cases. Do not retune this same direct-leaf scalar lowering through
helper inlining, enum reshaping, or code-layout changes; that would be a
second attempt after a decisive shared-call regression. A successor must start
from a new profile of a distinct shared cost and keep ordinary
`plain_function_call` off its perturbation path.

### 2026-07-28 rejected detached numeric-native ordinary-call typed-loop lowering

Queue rank 18, SunSpider `3d-morph` (2.901753x QuickJS-NG in the receipt-bound
queue), has a local `sin = Math.sin` whose inner `sin(...)` call compiles as an
ordinary `Op::Call`, rather than the receiver-preserving `Op::CallResolved`
form the generic typed-loop tier already lowers. The current base profile is
`/private/tmp/qjs-profile-3d-morph-05f-v3.sample` (SHA-256
`05a481511cc8fe27bcde904fa2e1f76d5141a5823394cd1228f3fbcbd177773c`), taken
from the pinned source SHA-256
`9a782188384af592c308338225f014cc7df246614f47f312cf71f29ed06e9f2d`. It
showed the generic native-call chain beneath `morph`, including
`call_callee_with_marker`, `try_fast_global_native_call`, and temporary
`CallEnv` work.

The frozen one-attempt plan is
`tasks/performance-units/typed-loop-detached-numeric-native-call.json`
(SHA-256 `14ae2cd11321bb8fd4b24d7212c5c04b23216e88fcc44c66fb75d877f7639b0f`).
The prototype lowered one- and two-argument ordinary calls into the existing
`CallNumericNative` typed operation. It retained the existing run-time guard:
only an unbound pure numeric native with scalar operands executes there; user,
bound, non-Math, and non-scalar calls deopt at the original call site. Focused
typed-loop tests, including the existing receiver-preserving Math coverage,
passed, and the exact upstream source still completed normally.

A three-second candidate diagnostic sample at
`/private/tmp/qjs-profile-3d-morph-detached-call-candidate.sample` (SHA-256
`5d393f9a4fc869cbae9ad300bbdd68ca906d7c209d24a09606bcb10c66ad116e`) recorded
the new tier attempting execution, but was intentionally stopped after the
sample and is not a completion or timing receipt. The decisive preliminary
black-box screen used the exact upstream source followed by 500 extra
`morph` calls in the same Realm (wrapper SHA-256
`9e23925045055773ed9e3facc9fad9d1005c7a85dff9048f30927102d8669ec5`). Five
candidate process times were 2.33, 2.34, 2.32, 2.33, and 2.33 seconds; five
base times were 2.32, 2.30, 2.30, 2.31, and 2.30 seconds. Thus the medians are
**2.33s candidate / 2.30s base = 1.013043x**, nowhere near the frozen
`<= 0.68x` target gate. Candidate and exact-base release binary SHA-256 values
were `f671d6fe12a52c3320ec6208194331a4637b250798885b71a547fdca2fe62fd1` and
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`.

This was an ordered, diagnostic screen rather than a complete external or
broad report, but the target miss is far larger than its timing precision. The
prototype and its focused test were reverted immediately; no controls, full
report, or Test262 promotion scan are warranted. Do not retry this same
ordinary-`Call` lowering by widening its admission, changing receiver handling,
or reshaping the helper: its declared target has no end-to-end win. A successor
must begin from a new profile of a different shared cost.

### 2026-07-28 rejected full-dense Boolean predicate scan

Queue rank 23, SunSpider `access-nsieve` (2.554890x QuickJS-NG in the
receipt-bound queue), initializes `Array(m + 1)` with Boolean `true` and marks
composites Boolean `false`. Its outer `if (isPrime[i])` has the existing
false-prefix scan bytecode shape, but the scanner's dense load previously
accepted only `Number`. The current diagnostic profile is
`/private/tmp/qjs-profile-access-nsieve-7c-v1.sample` (SHA-256
`f61648fc98248715a8104ec3715e2a57238b6bcc8abb948ecff816387b2ab884`) from the
exact upstream source SHA-256
`ef62b42b6f926d61d9741a8e57b2758a9aea28ad2d1ee1e7d4747957d00fdc20`. It
showed 626 exclusive samples in `run_virtual_object_op` as well as the
ordinary loop/property path.

The frozen one-attempt plan is
`tasks/performance-units/boolean-dense-predicate-scan.json` (SHA-256
`5987a26faa6ed9292daa65de622194059bdc7d401c362e8956f8e43ee79f996a`). Its
prototype admitted Boolean elements only under the pre-existing pure-read,
authoritative-local, and fully-dense own-data guards; every other Value,
hole, descriptor, proxy, or borrow conflict stayed on the VM path. Focused
predicate-scan coverage passed 18/18 tests, including Boolean false prefixes,
true-body handoff, non-Boolean deoptimization, holes, and indexed accessors.

The actual array is never fully dense: indices zero and one remain holes after
the initialization loop. Consequently the existing full-array lease rejects
before the Boolean load is reached, so the unit did not remove the profiled
cost. Five alternating candidate/base process pairs used the exact upstream
source inside a profile-only same-Realm 20-call wrapper (wrapper SHA-256
`9b6f8545c35e24348e433725b0ce8daf88dc7cbee5669e1e6e2eb4faa151d001`). Candidate
user times were 1.52, 1.52, 1.52, 1.52, and 1.52 seconds; base times were
1.53, 1.52, 1.53, 1.53, and 1.53 seconds. The medians are **1.52s / 1.53s =
0.993464x candidate/base**, far short of the frozen `<= 0.78x` gate. The dirty
candidate and exact-base binary SHA-256 values were
`04e826e7cd9b75e4a19a7840071ce72a2c8f716adb3b1d38bc84c4919975adee` and
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`.

The Boolean change and its focused tests were reverted immediately; no
controls, full report, or Test262 promotion scan are warranted. Do not retry
Boolean support under the same full-dense lease or try to tune this miss by
changing its type checks. A successor may separately profile and justify a
range-aware present-own read lease that can prove every index it actually
reads; that is a different semantic mechanism with new prototype, descriptor,
and hole obligations.

### 2026-07-28 rejected present-own primitive predicate scan

The distinct successor plan
`tasks/performance-units/present-own-primitive-predicate-scan.json` (SHA-256
`9f8ae265f1a2aa47255790394edb5c5468b05ffc74417d464d804db8efdf3c1d`) retained
the existing false-prefix CFG and completion proof, but replaced the
full-array dense lease with a read-only lease that exposed dense backing plus
the hole set. Each scanned index had to be present own; unrelated holes and
prototype overrides were harmless because a present own element shadows the
prototype. Any hole reached by the scan, own special property, borrow conflict,
non-primitive value, or unsupported numeric coercion deoptimized at the
current iteration. The Boolean extension was therefore coupled only to the
same per-index primitive-read proof, not to a new opcode or workload matcher.

Focused candidate coverage passed 18/18 predicate-scan tests and 12/12 array
tests. It included the exact `Array(n)`-with-unrelated-leading-holes shape,
Boolean false-prefix/true-body behavior, and a scanned hole whose inherited
getter must run. The exact upstream `access-nsieve` source still completed
normally. These are semantic checks only; they do not override the frozen
performance threshold.

The first and only timing attempt reused the exact source in the same
20-call profile wrapper (SHA-256
`9b6f8545c35e24348e433725b0ce8daf88dc7cbee5669e1e6e2eb4faa151d001`). Five
alternating candidate times were 1.52, 1.52, 1.52, 1.52, and 1.52 CPU seconds;
the corresponding base times were 1.52, 1.52, 1.52, 1.53, and 1.52 seconds.
Both medians are **1.52s**, or **1.000000x candidate/base**, not the frozen
`<= 0.78x` target. Candidate release SHA-256 was
`d2029a0d76954001df36c28fadc997de2a1c5ab5037c117b750dc935beafc6bb`; the
exact-base release SHA-256 was
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`.

The runtime change and focused tests were reverted immediately; no controls,
full report, or Test262 promotion scan are warranted after this decisive target
miss. Do not retry this same present-own primitive scan by widening the lease,
changing plan selection, or adding more primitive forms. A successor must
profile a different shared cost rather than turning the rejected target into a
series of admission tweaks.

### 2026-07-28 rejected two-dimensional dense ordinary-array mutation

Queue rank 24, SunSpider `crypto-aes` (2.354737x QuickJS-NG in the
receipt-bound queue), repeatedly uses dynamic two-dimensional ordinary Array
accesses such as `state[row][column]` in Cipher, SubBytes, ShiftRows,
MixColumns, AddRoundKey, and KeyExpansion. The current profile is
`/private/tmp/qjs-profile-crypto-aes-7c-v1.sample` (SHA-256
`03042fe9252b3a5efd7569c84d53f9eb1bd2cace4e25075668e9051d2d1e57e4`) from the
exact upstream source SHA-256
`7151c362dd1d10ec6bf8bd332a57bf96aa3d6bf64a87695b7b670af09e430365`. It
contains 162 exclusive `run_virtual_object_op` samples and 59 direct
dense-index samples under repeated direct-leaf calls. The existing dense
translator can form one dense receiver but treats an indexed result as a
Number, so it cannot represent that result as the next dense receiver.

The frozen one-attempt plan is
`tasks/performance-units/two-dimensional-dense-mutation.json` (SHA-256
`0e2782f83ac3a1bb298faacfeb3d7dda46bee5205dcb70adce4044849902d1e0`). The
prototype added a bytecode-derived, transactional 2D path beside the existing
numeric loop executor. It admitted only local ordinary outer Arrays and live,
ordinary, own, fully dense Number rows, and declined holes, descriptors,
prototype-index hazards, Proxies, row aliases or cycles, immutable rows,
non-Numbers, and borrow conflicts. A failed guard discarded the current
iteration before ordinary VM replay. It had no source, function-name, input,
result, checksum, or benchmark-identity admission condition. Focused 2D
coverage and the full `qjs-runtime` test suite passed before timing; all
prototype source and test changes were then reverted after the gate result.

The direct fast screen ran the exact upstream source with one warmup per
binary, followed by 25 interleaved candidate/base pairs in alternating order;
stdout and exit status were checked before timed runs. The candidate release
binary SHA-256 was
`0e521fe08a4ba1d4fb1c5d9ed1e9bdf3e12d14c5e4d548c6740d150ef71df859`; the
exact-base binary SHA-256 was
`70208d9c129430c98e186956b01f0384eb6525aaa8f30e8fe02a9551a7f9b45c`.
Candidate/base median wall times were 81.177750 ms / 83.115708 ms, with a
median paired ratio of **0.976855x** (mean **0.977101x**, observed range
0.964674x--0.989036x). This decisively misses the frozen target gate of
`<= 0.84x`; a roughly 2.3% movement cannot close the 15.1% reduction still
needed to cross the `< 2x` QuickJS-NG boundary.

The unit is **rejected and reverted**. The target miss is sufficient, so the
frozen controls, complete broad/external reports, and Test262 promotion scan
were intentionally not run. Do not retry this same two-dimensional receiver,
row-lease, or transactional-staging mechanism by relaxing its guards or
reshaping its implementation: that would be a second attempt after a decisive
end-to-end miss. A successor must begin from a new profile of a distinct
shared cost.
