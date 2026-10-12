# Mixed scalar/boxed execution across ordinary calls (proposal)

**Design only; no runtime implementation or frozen plan.** Queue/base is
`e54e33eb`; [T036](../../tasks/T036-mixed-call-execution-design.md) owns
measurements and readiness. The [schema-2 draft](mixed-call-execution-plan-draft.json)
is outside the frozen-plan directory deliberately. The lead must approve the
proposed four payoff targets (validate remains a mandatory control) and
bounded experiment sequence before a new owner freezes it.

## Thesis and decision boundary

Keep numeric operations, arguments and results scalar across ordinary calls,
while object/string operands remain owned boxed handles and captured bindings
remain canonical cells. Use an activation-local mixed register bank and an
explicit continuation ABI; materialize only at an exact semantic boundary.
This is a replacement for the wide tier's value transport, not an extra arm
in the numeric or typed hot loops. Preserve their existing good paths.
Parity on all five targets is **not** the expected outcome: pointer traversal,
allocation and string/native work survive. The cost model and prerequisites
for parity are in [the measurement record](../../tasks/archive/T036-mixed-call-execution-measurements.md).

## History read before design

| Record | Learned | Why this proposal differs |
| --- | --- | --- |
| [T021](../../tasks/archive/T021-single-vm-frame-stack.md) and its log | The scheduler moved a 704-byte FrameState about six times/call (~50 GB at 12.7M calls); recursive 1.3848, method 1.2769; boxing worse (1.6170/1.4480). A shared register window also lost (1.2678) to per-access base arithmetic. Compact-fn VM-free calls landed. | Initialize small descriptors and reusable banks in their final slots; acquire a zero-based bank once per run segment, never add a window base per operand. No FrameState queue or per-call Box. |
| [T024](../../tasks/archive/T024-general-register-core.md) | Preamble shrank 25,020 to 1,680 instructions; corpus only 0.9906. Scalar-only general core was ~9% slower and displaced good typed loops; caller shape hid admission. | Mixed inputs on the first execution slice; do not claim already-fast scalar bodies. Dynamic coverage and representation transitions, not dispatch count, are gates. |
| [T027](../../tasks/archive/T027-frame-verified-direct-local-opcodes.md) | 99.9999% authority-hit coverage bought only 0.9841, missing 0.97. | No direct-local opcode or authority-proof relocation. Change transport and number lifetimes across calls. |
| [T030](../../tasks/archive/T030-packed-typed-loop-scalars.md) | 16-to-8-byte private scalars regressed 1.0170 despite unchanged path. | Keep the existing safe scalar format; no NaN payload, pointer packing, or public Value rewrite. |
| [T033](../../tasks/T033-wide-tier-interpreter-exits.md) | Exact exits/hand-back are present despite formal rejection. Old stack runs are not current decisions. | Reuse semantic exit boundaries, replace bank/call transport under independent cumulative gates. Do not recount existing admission wins. |

Every Rejected entry in [T033's screen log](../../tasks/archive/T033-screen-log.md)
was read. They rule out: Math/field-thunk arm growth (tree 1.064), an out-of-line
per-action loop (tree 1.026, md5 1.034), CompareSkip (fewer instructions,
cycles worse), eval-closure admission (xparb 1.007/tofte 0.993), own-miss and
proven-prototype shortcuts (micro instruction wins, corpus regression),
empty-window skip (-19/1005 instructions only), replacing typed loops by wide
(2.3x/3.4x slower), fixed boxed-register arrays (+20% cycles), order-file-only
layout repair, incremental overlays (tofte 1.019; repeated rejection),
expanded/handwritten Value clones (md5 1.056/churn +15%), global helper
flattening, borrowed prototype proof, inherited-read value caching, and
compiler postfix-shape changes. None is part of this design. Keep existing
layout slots; measure callees as well as the executor, never assume pinning
eliminates the codegen band.

## Activation and continuation ABI

Internal qjs-runtime types only; no crate/API/dependency change, FFI or unsafe.
`ActivationId` identifies a slot in an execution-context-owned depth pool.
Each slot owns separately allocated **reusable** scalar/boxed bank storage;
reserve on first depth/size use, initialize in place and retain capacity.
A pool vector may move small handles; no live reference crosses its growth.
No raw pointers or fresh bank allocation per warmed call. Contexts nest for
native reentry: a callback gets its own context while the suspended parent
remains rooted; avoid borrowing a thread-local RefCell across user code.

A hot descriptor has code/plan handles, bank id, region/pc, cell-owner id and
continuation id; target <=96 bytes, cold interpreter/handler state allocated
only when needed. A continuation stores caller id, next bytecode ip,
result lane/index, stack depth, completion kind and constructor-return rule.
It retains required Function/receiver handles once, not on every operand move.
The executor receives zero-based bank slices and its program directly.
It returns `Enter`, `Return`, `ExitBefore`, or `Throw`; the scheduler drops
borrows, grows/selects storage and binds fresh slices only at these boundaries.
Separate numeric, typed and mixed run functions retain their own tiny hot loops.

Calls evaluate callable, receiver and arguments in JS order before `Enter`.
Check callee identity/kind, environment equivalence and parameter mapping;
Proxy, bound, native, default/rest/destructuring and unsupported functions use
the canonical call path. A successful return commits the continuation once;
throws unwind once through the existing handler/disposal semantics. `new`
retains its receiver and implements the object-return override separately.
Initially those complex callees are boundary fallbacks, not partly entered.

## Representation and binding authority

Compile each basic region with static operand lanes: existing Copy scalar
(Number/Boolean/undefined), owned `Value`, and shared-cell references.
Number results stay scalar; heap values stay owned in the boxed bank. No
runtime lane tag/slot map lookup per numeric operand. Region entry guards
prove input lane/type; mismatches exit **before** the first failing operation.
Joins and type-changing results may select another region or box once; they
cannot optimistically reinterpret stale scalar bits. Preserve NaN, infinities,
-0 and JS ToInt32/ToUint32, rather than adding another packed scalar.

The ABI argument/result descriptor states lane and index. Copy scalars directly;
move dead boxed temporaries, clone live aliases once. Never borrow an object
handle across reentry. Unknown callees receive canonical Values; their result
is checked once on return. Bytecode parameters retain undefined/missing and
excess-argument behavior. A call is not a proof that numeric inputs stay numeric.

[The protected binding model](env-model-rewrite.md) remains mandatory:
private locals have one slot authority; captured/received/module/realm/mapped
arguments bindings have one shared cell. Known captured slots are cell-backed
at entry and excluded from persistent scalar storage. Before lazy capture,
transfer the authoritative private value to one canonical cell and invalidate
its scalar authority, then build the closure. Never keep a scalar mirror of a
cell across a call, getter, Proxy or eval. Dynamic eval/with and suspension
initially decline at entry; no name-keyed value snapshot or refresh pass.
Scalar banks may hold transient cell-read results like ordinary expressions.

## Exact-exit protocol

Each region carries a verified map to original bytecode ip, live local slots,
operand-stack order, TDZ/const state and ownership. `ExitBefore(ip,map)` means
no part of that instruction has executed: materialize live scalar values once,
move owned boxed stack values, attach the same cells and resume at ip.
Completed stores/calls are already committed and never replayed. Property fast
paths must prove ordinary own-data/layout/writability before touching state;
a failed proof exits before lookup can invoke user code. Scalar locals are
private; object and shared-cell writes publish immediately, with no deferred
observable field/global writes in this migration.

If the fallback instruction calls user code, release every bank/cache borrow
first. On successful completion resume at the **next** ip with the returned
value installed; revalidate all receiver/callee/type facts invalidated by
reentry. On throw use the original instruction's exception edge. Distinguish
pre-op exit from post-call continuation in types, not a sentinel arity. Never
restart a body or a compound assignment after its getter/read has run.
A deoptimizing loop records its exact declined-program bit and resumes through
existing LoopFrame semantics; retain already-good typed/numeric accelerators.

## Landable stages and pre-build kill gates

Cheapest first: run the nonempty 4/8 component/coverage experiment and cost
ceiling before **any** runtime work. Stop if useful cross-call scalar/boxed
transport cannot exceed 5% whole-case savings in at least two targets.
The task record states its measured limits; full workload coverage remains a
required pre-freeze check. These are gates, not permission to build speculatively.

| Stage | Independently usable result | Cheap falsification BEFORE building | Cumulative regression budget vs frozen base |
| --- | --- | --- | --- |
| 1. Activation ABI | Existing wide execution uses reusable zero-based banks and typed continuations; old executor remains rollback path. | Standalone storage/scheduler harness, replaying measured bank sizes/call depths, must allocate zero warmed banks, move no FrameState, and cost <=80 cycles/call including boundary rebinding. Otherwise abort this storage shape. | All five, controls and six sentinels <=1.02; one implementation screen attempt. |
| 2. Mixed operations/calls | Existing admitted mixed bodies carry scalar parameters/results through a call; property/cell helpers still canonical. | Standalone mixed-op replay of the measured operation mixes: scalar step <=20 cycles, boxed numeric read <=30, mutation <=60, boundary <=80; no boxing of admitted numeric arguments/results. Compare with the task's measured slopes; kill if net savings including lane guards <20% on vector and write kernels. | <=1.03; one attempt; no worse typed/numeric path selection. |
| 3. Exact boundary bridge | Unclaimed ordinary regions can run and return after one fallback instruction without rebuilding/replaying the call. | Trace original-bytecode region maps offline and count fallback frequency; priced materialization must consume <25% of stage-2 predicted savings on >=2 targets. Table-driven state replay must distinguish before-op/post-call and preserve cells under reentry; any duplicate effect or >budget kills this bridge shape. | <=1.03; one attempt; complete exact-exit regression coverage. |
| 4. Payoff/default | Proven regions replace wide transport; redundant copies/materializations removed, existing fast tiers retained. | On stage 3, trace actual eligible coverage and price remaining work. If upper bound cannot meet 0.95 on each of the four payoff targets, stop before rollout; do not widen arms for hope. | <=1.03 controls; four payoff targets <=0.95 cycles/base; validate <=1.03, one attempt. |

Schema 2 encodes one common stage ceiling (draft 1.03); stage 1's stricter
1.02 is an additional recorded screen gate, never a post-measurement plan edit.
All gates include real-output validation; unit tests cover lane mismatches,
TDZ/const, capture siblings, missing/excess args, constructor returns, deep
recursion, native reentry, getter/Proxy mutation, side effects before throw,
mid-expression exits, loop deopt, and exclusions for eval/with/suspension.
Each stage updates its resume block and runs focused tests/Test262, full gate
and NG comparison; the lead schedules complete cumulative stage decisions.
An inconclusive result pauses advancement; a failed screen consumes the stage's
single attempt. No further implementation variant without a new approved plan.
