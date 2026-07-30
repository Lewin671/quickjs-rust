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
