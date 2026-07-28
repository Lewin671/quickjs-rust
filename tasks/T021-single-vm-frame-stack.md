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
