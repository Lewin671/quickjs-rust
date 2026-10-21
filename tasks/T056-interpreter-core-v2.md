# T056: Interpreter core v2

- Status: active; M1 stage1 on main (`ab9c29b6`); M1.2 closed. Replacement plan
  [`interpreter-core-v2-layout-edges`](performance-units/interpreter-core-v2-layout-edges.json) is implemented
  as stage A on `agent/t056-m1-stage-a/claude`. Valid0a9af81b K=6 spent attempt1/2; one remains across A+B.
- Implementation: stage 1 on main. Stage A on its branch, not on main; the rejected M1.2 candidate stays on
  `agent/t056-m1-stage2/codex`; the write-site add replay removed from stage A stays on
  `agent/t056-add-transition-replay/claude`.
- Verified at: pragmatic8f898dd9 lead211111Z full exact pass,213443Z86/86;213757Z triage unclear;
  214401Z mini A/A failed (inconclusive), global excess absent5/6 but unpack slow6/6.
  Sparse-index hot/cold implementation now present;32 object tests pass locally, full gate pending.
- Evidence: [stage A record](archive/T056-m1-stageA-record.md),
  [allocation follow-up](archive/T056-m1-stageA-allocation-diagnosis.md),
  [architecture review](archive/T056-architecture-review.md), [stack/startup diagnosis](archive/T056-stack-phase-diagnosis.md), [PMU/FP diagnosis](archive/T056-pmu-attribution.md), [source bisection](archive/T056-source-bisect.md), [header factors/full-loop accounting](archive/T056-header-subparts.md), [dispatch addresses](archive/T056-dispatch-address.md), [JCC policy diagnosis](archive/T056-jcc-policy.md), [PMU helper](archive/T056-kpc-tool.md), [PMU setup failure/v2](archive/T056-kpc-setup-failure.md), [PMU counting v3](archive/T056-kpc-counting.md), [collector recovery](archive/T056-kpc-collection-retry.md), [PMU closure](archive/T056-pmu-closure.md), [pragmatic implementation](archive/T056-pragmatic-stageA.md), [pragmatic qualification](archive/T056-pragmatic-qualification.md), [M2 inputs](archive/T056-m2-design-inputs.md);
  plans [`interpreter-core-v2-atoms-layouts`](performance-units/interpreter-core-v2-atoms-layouts.json) (closed),
  [`interpreter-core-v2-layout-edges`](performance-units/interpreter-core-v2-layout-edges.json).
- Unresolved: unpack wall1.033–1.057/instructions1.0105, class-fields1.0434/1.0095, empty instructions1.0721.
  Header crossing alone is insufficient: historical unpack has only110 requests of136B.
- Next action: follow the [hot/cold preparation](diagnostics/t056-hot-cold/implementation-plan.json):
  verify112B sparse header, native classes and instruction attribution, fast-family PCs, then fresh lead bundle.
  No owner timing; PMU stays closed with installed helper untouched; last frozen attempt unspent.

## Goal and scope

Reduce the cost of the ordinary interpreter when accelerators decline, toward
T018's general performance target. Atoms/layouts, stable frame ownership and
an opaque 8-byte value word form one architecture; preserve slot/cell binding
identity throughout. The target lives in the linked design; stage order and acceptance are below.
This design does not approve an implementation, reopen an exhausted leaf
budget, establish a fresh performance priority, or claim a measured speedup.

Earlier owners and boundaries: [task history](archive/T056-task-history.md). Structural runtime work is
serialized because APIs and ownership span execution, properties and values.

Design-time source checks and report limits: [design inputs](archive/T056-design-inputs.md).

## Prior attempts

These close particular experiments at their named revisions, not the target
architecture. Read their full records before freezing a new unit.

| Record | Constraint carried into v2 / difference |
| --- | --- |
| [T021](archive/T021-single-vm-frame-stack.md) and [log](archive/T021-single-vm-frame-stack-log.md) | July 31 moved-frame route: recursive 1.3848x, prototype calls 1.2769x; boxing worse. A contiguous-buffer/hot-header prototype failed recursive 1.180476x and hash-map 1.150426x; compact windowed registers lost at 1.2678x. V2 initializes persistent slots fieldwise, binds zero-based per-frame buffers, verifies emitted copies/borrows before routing, and removes general activation/name assembly beyond the direct-leaf cohort. |
| [T024](archive/T024-general-register-core.md) | Dispatch-preamble change delivered external 0.9906x, sentinels 0.9989x; scalar whole-body replacement lost to existing typed loops. V2 retains bytecode dispatch/scalar tiers and targets ownership, names/layouts and boxed transport together. |
| [T027](archive/T027-frame-verified-direct-local-opcodes.md) | Hash-map 0.9841x missed 0.97 target despite direct-local mechanism success. V2 preserves authoritative slot/cell access, adds no direct-local opcode recognizer. |
| [T019](archive/T019-object-layout-rewrite.md) | Cold Dynamic payload/accessors landed; pair boxing and lazy weak sidecars lost. V2 retains useful payload layouts, does not add pair allocations or lazy weak state, and separates metadata authority from payload packing. |
| [T023](archive/T023-realm-object-arena.md) | Intrusive counts plus constructor arena lost (hash-map 1.016x, A* 1.088x). V2's common header is a dependency of one-word ownership across all heap kinds, not an allocator win or arena rebranding; count overhead has its own migration gate. No block allocator/collector is proposed here. |
| [T030](archive/T030-packed-typed-loop-scalars.md) | Private scalar 8-byte packing lost 1.01702x. V2 changes public boxed transport, retains scalar tiers, and requires identical tier coverage and end-to-end wins rather than size alone. |
| [T041](archive/T041-own-data-cache-hot-answers.md) | OwnSlot hot answers failed their sole screen. V2 does not revive weak-object hot answers; layout lookup becomes the ordinary path for every store. |
| [T049](archive/T049-property-access-path-study.md) | Duplicate-probe removal missed target/controls; same-receiver proofs had low reach. V2 changes name/layout authority rather than deleting one probe. |
| [T053](archive/T053-ordinary-own-slot-hot-read-fat-lto.md) | Hash-map 0.917431x did not excuse six failing controls; layout shifts were not isolated causality. V2 requires complete controls, exact-binary attribution and layout audit at each stage. |

## Ordered migration

The numbered units run in order. M1/M2/M3 are separate migrations with their own
fixed base; numbered substeps are independently correct stage boundaries.

| Unit / stages | Expected measurable effect | Additional stop condition |
| --- | --- | --- |
| 0 — research prerequisite | Establish ordinary-path cost, frame copies/allocations, key comparisons, boxed traffic, target ABI and memory baseline; replay held-out programs. | No exact-binary attribution or supported pointer/lifetime protocol: no runtime plan. |
| M1.1 — value access facade + atom linking | Named operands link once, with no per-dispatch atom interning; facade leaves size/ownership unchanged. | Per-op re-interning, stale foreign atoms or facade conversion temporaries. |
| M1.2 — layout authority for all stores and mutations | Ordinary own hits take one atom-to-slot lookup across all four stores; no descriptor cloning for data reads. | A storage family bypasses invalidation, or churn memory grows without bound. |
| M1.3 — route ordinary Get/Set, retire old probes | Fewer probes/comparisons on interpreter paths and independent external property workloads; final M1 payoff. | Coverage shifts only on a tier/literal path, or final payoff fails. |
| M2.1 — names off the callee-ordinary entry: indexed entry + typed context (migration) | For [callee-ordinary entries](../docs/design/frame-activation.md#cohorts-two-predicates-and-the-replacement-rule) replaces `function_env`/`FrameBindings` inserts, `initial_slots` name import, `apply_env` write-back, `current_env` export, realm refresh scans, the direct-leaf CallEnv, argument vectors, per-call name lookups of static facts and runtime received-cell arithmetic; natives get the [typed caller-varying fields](../docs/design/frame-activation.md#native-environment-m21-until-natives-take-a-capability-in-m25) instead of a per-call `empty_frame`; `Vm` holds ActivationContext plus a borrowed services env (wide constructor API change). Other entries keep the whole legacy route. Reach bound ~3.0 of 28.7 sampled s; development lodash/transform and diff/compare. | A typed field mirrored by a legacy name entry, an ordinary-cohort counter above 0, a cloned env per nested Vm, a wrong referrer/private/with/new.target in a native callback, changed prologue/TDZ/alias ordering, or a borrowed argument range surviving a switch. |
| M2.2 — lexical scope leases + lazy dynamic consumers (migration) | Migrates the remaining inventory rows (eval, with/deopt, eval-capable closures, coercion filtering, results, realm fallback including `CallEnv::get`/`DynamicBindings` name reads), then deletes legacy `CallEnv` name storage. Mostly retirement; small development-lane reach. | Any consumer needs caller value copies, escaping eval sees invocation scope, a realm mutation needs a blanket refresh, a lease retains a caller frame, or a legacy name layer survives. |
| M2.3 — one driver for generic-to-generic calls (migration) | Frame store and same-Vm driver land together for this population (a store with unchanged routing has no reach and moves frames, T021); replaces nested Vms for generic JS-to-JS calls and in-memory transitions on admitted call/return. Generator/async frames do not lease yet. Reach bound: nested-Vm plumbing 1.85 s; development immutable/persistent and graphlib/paths, suite cdjs as cross-check. | Emitted aggregate store/reload or per-op re-resolution on the admitted path (release-binary audit), a pointer-based view kept across a re-entrant call, per-call frame boxing, unwind/cleanup mismatch, or retained memory above the frozen ceiling. |
| M2.4 — compact windows join the store (migration) | Replaces the compact thread-local register pool and frame stack, `call_from_activation` CallEnv/nested Vm for compact-to-non-compact calls, whole-window clears and exit window copies; declined backedges stop exiting; exits continue in place only where the register layout maps onto generic slots, else one counted conversion. Reach bound: compact driver 1.58 s plus part of the 15.4 s under `call_from_activation`; development lodash/transform and immutable/persistent. | An unexpected tier admission change, an unmapped exit without its counted conversion, or any M2.3 stop. |
| M2.5 — constructor/native-capability/eval/suspension routing, retire adapters (final payoff) | Same-Vm synchronous callbacks; generator/async frames lease store slots through detachable resume leases; natives take a capability instead of `&mut CallEnv`; zero nested Vms in an active context. End-to-end payoff targets: development lodash/transform, diff/compare and immutable/persistent. | Re-entry borrow, detached lifetime, derived completion/job-order error, leftover hot compatibility path, or final payoff/portfolio controls fail. |
| M3.1 — common heap headers under enum backend | One header allocation per heap payload; preserved weak/COW/escape behavior, counts quantified. | Extra wrapper allocation, unsafe proof failure or ownership/count overhead beyond budget. |
| M3.2 — alternate 8-byte backend through same facade | Exactly 8-byte values/boxed banks/slots, fewer moved bytes, same tier coverage; test both backends. | ABI qualification, NaN/zero, TDZ, provenance or weak-reuse failure. |
| M3.3 — select qualified backend, remove hot conversions | Lower boxed transport/clone/drop cost across property and call workloads; final M3 payoff. | Benefits are lost to tag/count cost, controls fail or required portability remains unresolved. |
| L4 — optional independent tier/IC leaf units | General virtual-field admission or a measured layout-cache win after baseline improvements. | No independent benefit within its own attempt budget; do not widen to rescue it. |

Every implementation stage requires focused tests, `check.sh`, NG fixtures and the
**full configured Test262 set**, with no exclusions added to make a stage pass.
Run the exact complete comparison and record its burndown per the
[conformance runbook](../docs/conformance/README.md); curated subsets alone are insufficient.
Freeze queue/profile-bound plans before runtime edits under the
[performance workflow](../docs/performance-workflow.md). Intermediate migrations use
`decide --mode stage` against one cumulative regression budget, not leaf speed gates;
the final stage must pass ordinary screen/promotion payoff and all portfolio controls.
Unit 0 sets numeric budgets, target improvements, memory ceilings and attempt limits
in those plans; this design supplies no invented speedup percentage or budget waiver.
Any failed correctness/mechanism obligation stops advancement; exhausted budget or
`abort` closes that implementation shape. Keep the last independently verified stage.
Measure exact uninstrumented binaries, A/A noise and linked layout; counters diagnose
only. Confirm ordinary-path reach separately from accelerated reach, use complete
portfolios and held-out applications selected by operations, and serialize timing.

M2.1–M2.4 use the **stage budget**, cumulatively against M2's fixed base, even
when an individual stage removes real work; only M2.5 claims the final payoff.
Order rationale, cohorts, native env, driver contract and M1 interface: [frame activation](../docs/design/frame-activation.md)
(inputs: [M2 design inputs](archive/T056-m2-design-inputs.md)). M2's plan waits until M1 finishes (B accepted or M1 closed).
The named cases are proposed development/suite targets and diagnostics, not a
frozen plan or speed forecast. Refresh their exact-binary reach after M1; bind
numeric stage cap, final per-case payoff, memory ceilings, K=6 and attempt limits
in one validated migration plan before M2.1. Follow the current policy, not M1's
v3 numbers. No held-out case selects targets, scope or profiles; held-out stays
acceptance-only. Compare per-stage and cumulative allocations/env work/frame
entries plus uninstrumented CPU/wall time; counters must confirm ordinary-path
reach. All rows inherit the full correctness gates above, with no stage waiver.

Focused coverage before each full gate:

| Migration | Required discriminators |
| --- | --- |
| M1 | All four stores; constructor/literal/native receivers; computed/lone-surrogate keys; delete/reinsert, descriptor and prototype edits; getters/Proxy re-entry; arrays/strings/functions/exotics; cross-realm/foreign code linkage; atom/shape churn memory. |
| M2 | Deep/mutual recursion; sibling/loop cells, TDZ, duplicate mapped parameters, defaults/rest/destructuring; nested finally/using; callback eval through native/accessor/Proxy/coercion/iterator after creator return and with caller-name collisions; nested eval/with/unscopables, new/deleted eval vars and parameter/body conflicts; immutable self, module live cells, global delete/accessor/recreate and cross-realm calls; derived-this/super/field errors; generator prologue/yield*/re-entry, await/FIFO/jobs and leases outliving creator. |
| M3 | Bit-pattern round trips for finite/NaN/infinity/signed zero; raw buffer aliasing; every heap kind; count overflow, weak upgrade and address reuse; COW uniqueness; long drops/cycles/escaping values; both backends and agents/perf-counters builds. |

At least two independent real programs must exercise each claimed cost class;
pin inputs/checksums, verify both engines' outputs, and include constructor-heavy
and computed-key code alongside literal fields. Held-out inputs stay distinct
from the targets used to select the mechanism. Counter evidence must show that
ordinary-path wins survive when accelerators decline, rather than a tier switch.

## Unit 0 and M1 stage 1

Unit 0's summary (order M1, M2, M3; durable counters as plain `cfg(feature = "perf-counters")` code) and
M1 stage 1 (on main; v4 `pass`) are in the [task history](archive/T056-task-history.md); records stay linked above.

## M1 stage 2 (layout authority): first shape closed

Sole screen `20261018T055933Z-t056-m1s2-screen1` of `873c9bc5` under the frozen plan (`current_stage` 2)
rejected; the [rejection record](archive/T056-m1-stage2-evidence.md) owns the gates and verdict. It rejected
stage 2 (v4: 42/86 cases; external, development, sentinel and portfolio lanes above ceiling on
all six layouts; not DISTURBED); the plan's single stage-2 attempt is spent and its JSON stays
frozen. Under T022 the `abort` closes this implementation shape (value republication, weak
intermediate shapes, private domains, double-hashed probes), not the layout-authority family.
The [diagnosis](archive/T056-m1-stage2-diagnosis.md) reproduces the screen's instruction ratios
and its ablations remove most of the excess. The replacement is
[property layouts](../docs/design/property-layouts.md), which assumes T063's invariant that
stored strings keep valid surrogate pairs as scalars, so typed keys need no canonicalization scan.
It needs a new queue/profile-bound plan (the closed shape's data is not reused) and the
work-count obligations listed in that design before any screen.

Plan `interpreter-core-v2-layout-edges` (base and queue candidate `f58c696e`, post-T063; queue `c50ee7df`)
freezes these parameters: migration, stages A (layout replacement)
and B (routing, final); cumulative 1.025; payoff 0.97 or 1/B_i; rank ceiling 5; `max_attempts` 2; a frozen,
block-only non-timing instruction pre-gate over all 86 policy cases before each screen, recorded here.
Pre-gate cutoff: `max(1.03, I_i)`, the existing hard cap. Plan-before-implementation statements: the mechanism
summary discloses the diagnostic prototype and its measured ratios; budget, payoff and rank ceiling reuse the
existing values, not values derived from the diagnosis; the candidate is reimplemented on a post-T063 base whose
plan commit is an ancestor of every runtime commit, with no diagnostic-branch cherry-picks; diagnostic samples are
never profile receipts.

Targets and receipts: profile job `20261020T125346Z-t056-m1-profile` (exact queue image `b0dfc65d`, `sample` 60 s
at 1 ms, not disturbed) on all rank-1-5 development and external cases. Named-property/layout envelope, inclusive /
owner-attributed: moment/dates (rank 3) 27.3% / 23.9%, diff/compare (4) 32.3% / 22.4%, meriyah/parse (5)
27.9% / 23.0% — the three targets. Not targets: js-yaml/document (1) 16.9% / 9.5%, its excess is string/RegExp
work owned by T057; immutable/persistent (2) 17.5% / 15.1%, its largest excess is call/frame/env work (M2);
external rank 1-5 cases 0.1-12.8%. T057 (main `fa96320f`) is not in this base and touches mainly js-yaml, so the
queue is not refreshed. Receipts and scripts: workspace `tmp/t056-m1-profile/`.

Plan rules beyond the parameters: (a) candidate isolation — every stage A/B candidate is the plan commit (on
`ae45226e`) plus M1's own runtime commits; until the unit closes no main runtime change after `f58c696e` is merged
(including T057 `fa96320f`, whose search/units/substitution/sequence paths `diff/compare` uses, and
string-search-linear); tooling/docs merges are allowed; before each screen record `git diff f58c696e <candidate> -- crates`
here, showing only M1 files. (b) `max_attempts` 2 counts failed screens over A and B together; a stage abort counts,
inconclusive does not; after an A abort the remaining attempt may reimplement A and is the last. (c) The screen uses
the exact image hash that passed the pre-gate; every pre-gate run is recorded whatever its result; a pre-gate never
replaces or cancels a timed attempt already started.

### New-shape work-count obligations (perf-counters, before any timed screen)

- Repeated construction (constructor, class fields, JSON shape, literal, host map) creates O(1) shapes, field
  copies and atoms after the first object, also when the previous object is dead; an add probes once.
- `JSON.parse`, wrapper, function and array creation create no atom domain and do no re-link; an append edge hit
  interns nothing and compares no field vector; a small-shape lookup hashes nothing, a large one hashes once.
- Repeated `Small`→`Dynamic` growth follows a re-encoding edge; bulk freeze/seal interns no keys; payload-side
  name indexes are gone (A replaces, not overlays); `ObjectData` size test asserts at most 120 bytes.
- Retention: `dictionary_order_tombstones_stay_bounded_under_fixed_live_churn` asserts
  `live_count() <= 1 + 16`, `table_len()` within the sweep bound of that count, the dictionary's own
  order/positions/index exactly as today, and `live_count() == 0` after the layout and roots drop.
- Budget: a test keeps 5000 objects alive, each with one distinct key (each leaf is held by its object even after
  its root edge is evicted), asserts the live counter reaches 4096 and that misses at or above it record no edge,
  then drops the objects and asserts the counter falls below 4096 and new misses record edges again.
- Semantics (differential and unit): deleting/observing initializers, base constructors returning another
  object, equal keys with different attributes, delete/re-add order, more than 32 and 128 properties, freezing
  one literal instance, literals across realms and `eval`, null-prototype and host-map objects, lone-surrogate
  and halves-built keys.

## M2 leaf placement and work-count obligations

Leaf reach of 1–2% has not survived layout noise and complete controls (T041, T049, T027 0.984; T053's 0.917 did not
excuse six failing controls), so diagnosed candidates ([inputs](archive/T056-m2-design-inputs.md)) ride structural stages:

| Candidate | Reach | Home |
| --- | ---: | --- |
| #2 value moves without stack temporaries, inline clone | 2.06 s | M3 |
| 3a function own data properties, primitive-prototype methods | part of ~0.5 s | M1 stage B routed Get; the compact inline arm uses the same lookup |
| 3a array/string `length` (exotic or virtual fields; M1 keeps their full algorithms) | part of 7.3M array/string slow reads | compact-tier typed arm as an L4 leaf, after M2.4 |
| 3b polymorphic sites probe every cache entry | ~1.3 s | M1 (stage B lookup, then the L4 layout IC) |
| 4 upvalue cell index / borrow-free cache state / borrow-free cell reads | 0.11 / part of 0.42 / 0.16 s | M2.1 BindingPlan / M1 stage B / M3 |
| 5 exit fast path | ≤1.18 s inclusive | M2.4 (declined backedges, in-place continuation); new exit opcodes L4 |
| 6 `is_html_dda` on truthiness | 0.08 s | no unit; any later truthiness change (M3.3 tag tests) may absorb it |

Counters are plain `cfg(feature = "perf-counters")` code. Fixtures count exact per-call events for fixed and variable
arity, methods, closure creation and invocation, a native with a JS callback (sort, map), a getter, a constructor,
compact-to-non-compact calls and both cohort predicates' fixtures.

| Stage | Obligation |
| --- | --- |
| M2.1 | `call_env_constructions`, `frame_binding_inserts`, `env_write_backs`, `env_exports`, `realm_refresh_scans`, `argument_vectors`, runtime `name_slot_lookups` and native-call `with_stack_clones` are 0 per callee-ordinary call, nested Vms included; non-ordinary fixtures still count legacy work; the native-env debug assertion holds on every fixture. |
| M2.2 | Callback eval, with, coercion and result fixtures reach names through leases; ordinary calls stay at 0; legacy `CallEnv` name storage is deleted. |
| M2.3 | `nested_vm_constructions` is 0 and `same_vm_frame_entries` equals calls for generic JS-to-JS calls; warm frame-slot allocations plateau. |
| M2.4 | Compact-to-non-compact calls build no Vm or CallEnv; `window_slots_cleared` equals written registers; `compact_driver_breaks` is 0 on admitted call/return; declined backedges run no exit; `exit_conversions` counts only unmapped layouts. |
| M2.5 | 0 nested Vms in an active context, no per-native-call frame, no adapters. |

## Verification of design-only changes

The checklist and earlier records are in the [task history](archive/T056-task-history.md).
M2 re-plan verification on the documentation change over `290e3f51`:
`./scripts/check-context.py` and
`PYTHONDONTWRITEBYTECODE=1 ./scripts/check-touched.sh --staged --explain` passed.
Runtime gates were not rerun for this design-only change; the 2026-10-20 revision
(`agent/t056-m2-design/claude`, base `9ec13250`) passed the same two checks. Next M2 action after
M1: refresh the proposed targets' profiles and freeze the complete five-stage plan.
