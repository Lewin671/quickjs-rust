# T056: Interpreter core v2

- Status: active; M1 stage 1 integrated at `ab9c29b6`; M1.2 implemented on
  `agent/t056-m1-stage2/codex`; incremental-dictionary follow-up verified, screen pending.
- Implementation: stage 1 present; stage 2 adds uniform layouts for all four payloads and function/array named maps.
- Verified at: follow-up tree from `c5dfa1c6`, 2026-10-18 UTC: full gate, 2,320 runtime tests, subset 5,170/5,170, NG 249/249.
- Evidence: [part 1](archive/T056-unit0-part1-evidence.md), [part 2](archive/T056-unit0-part2-evidence.md),
  [M1 stage 1](archive/T056-m1-stage1-evidence.md); frozen plan
  [`interpreter-core-v2-atoms-layouts`](performance-units/interpreter-core-v2-atoms-layouts.json).
- Unresolved: stage-2 exact scan and iMac screen; stage 1 has no formal stage decision.
  Carried forward: M2 evidence, unit-0 bitops precision, architecture binding compatibility-view wording.
- Next action: verify/commit M1.2, run its sole screen, record the result and stop before M1.3.

## Goal and scope

Reduce the cost of the ordinary interpreter when accelerators decline, toward
T018's general performance target. Atoms/layouts, stable frame ownership and
an opaque 8-byte value word form one architecture; preserve slot/cell binding
identity throughout. The target lives in the linked design; stage order and acceptance are below.
This design does not approve an implementation, reopen an exhausted leaf
budget, establish a fresh performance priority, or claim a measured speedup.

Original design-only owner: `agent/interpreter-core-design/codex`, base
`6af6230036f917f6b8df4dc51aa6402a7ef367ce`, worktree
`worktrees/quickjs-rust/interpreter-core-design`. Allowed changes are the new
design, this task and its tasks/README row. No runtime, dependency, benchmark,
third_party, other worktree or performance-plan changes; no iMac jobs, push
or merge. Future structural runtime work is serialized because APIs and
ownership span execution, properties and values.

M2 re-plan owner: `agent/t056-m2-replan/codex`, base `290e3f51`, worktree
`worktrees/quickjs-rust/t056-m2-replan`. Only this task and the design changed;
no runtime edits, iMac jobs, push or merge. The design's [frames section](../docs/design/interpreter-core-v2.md#frames-storage-activation-context-and-lazy-names)
owns the source-grounded consumer inventory and target contracts; the M2 rows below own staging.

## Source checks and input limits

Read the task entry, architecture, protected env/cell invariants, suspension
invariants, performance knowledge/workflow and T022 before designing.
Paths below are relative to `crates/qjs-runtime/src/`, inspected at the base.

| Finding | Source / implication |
| --- | --- |
| Frame calls still construct a nested Vm. | `bytecode/vm.rs` eval entry points and `function/call.rs::run_prepared_bytecode_call`; `bytecode/frame_stack.rs` is dormant. |
| Activating the driver alone repeats the moved-frame design. | `push_frame` uses `mem::replace(&mut self.current, frame)` and stores complete FrameStates; REPORT W2's claim that this path avoids movement is false. |
| Cold splitting already exists. | `bytecode/vm.rs::ColdFrame`; old 704-byte measurements are historical, not today's measured frame size. |
| Switching requires a view boundary. | `bytecode/frame_program.rs` owns code through a retained handle and forbids frame replacement under a live view. |
| All four property stores still compare string names. | `value/object.rs::PropertyStorage` and `ObjectLiteralShape`; `value/object/dynamic_storage.rs` maintains its own name-to-slot index. |
| Existing caches are not one uniform shape authority. | `bytecode/named_property_cache.rs` has Exact/LiteralShape/OwnSlot/SharedSlot/prototype entries, weak identities and revisions. |
| Value is a 16-byte enum of immediate values and Rc-like handles. | `value/mod.rs`, including its size test; clone/drop is ownership work, not necessarily removable work. |
| Scalar replacement already handles virtual reads directly. | `bytecode/vm_virtual_object.rs::LoadVirtualValue` reads the frame bank; it need not perform a named-property cache probe. |
| Accelerator decline restores ordinary execution. | `bytecode/vm_loop_dispatch.rs`; `virtual_object.rs` proves identity/escape and treats suspension as escape. Exact current workload reach still needs counters. |

Inputs were workspace-local `tmp/perf-gap-analysis/REPORT.md` and REPORT-2.md,
read for this design. They are not retained evidence; the durable facts needed
to motivate the design are recorded here, with their limitations:

- Lead-verified input: hash-map counted 7,020,006 OwnSlot hot rejects versus
  3,599,982 hits. Counts establish reach, not removable latency. The input
  identifies job `20261016T123302Z-general-fresh-route-counts` as diagnostic,
  never a timing binary.
- Lead-verified input: when typed-loop declines, ordinary named reads were
  estimated around 50 ns versus NG 11.6 ns. These are supplied path-cost
  estimates, not a new measurement here or proof that ICs explain the gap.
- The reports bind the 76-case baseline to binary
  `926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85`,
  job `20261016T122024Z-general-current-baseline`; source main `6af62300`,
  executable source `9a0eeeca`. [T053](archive/T053-ordinary-own-slot-hot-read-fat-lto.md)
  records that exact baseline and its byte-identical restoration.
- Lead-verified NG input: 8-byte JSValue and atom/shape lookup with no per-site
  IC at pinned `f7830186043e4488f2998759d60a514faf07cbc9`. This worktree's
  submodules are uninitialized; NG source/ABI was not independently inspected.
  No assertion about NG's call-stack implementation is needed for our design.
- REPORT's 845/866 inclusive AES call-chain samples include the work of all
  callees. They are not a removable-call-overhead fraction or a 10–30% forecast.
  Reports' expected percentage wins remain unverified hypotheses.
- REPORT-2's escape/warm/cache experiments suggest virtual receivers block
  tier seeding. Source supports the distinct virtual bank, not the claim that
  every fallback read executes GetPropNamed or that all excess cost is IC work.
  Unit 0 must count actual lowered opcodes and admission/exit reasons.
- Reusing compiled eval source needs fresh instantiation/identity and scope
  validation; no general eval cache is smuggled into frame or atom migration.
  Held-out npm versions, licenses and plain-shell compatibility in REPORT-2
  were not verified here. Pin runnable inputs and validate outputs before use.

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
| M2.1 — reusable frame store + split borrows (migration) | Existing nested executors lease slots from one shared store; routing stays unchanged. Fieldwise initialization, zero whole-frame transfers; warm slot/buffer allocations plateau. Development immutable/persistent and graphlib/paths diagnose allocation/setup. | Emitted aggregate moves, per-call frame boxing, per-op window arithmetic, or retained memory beyond frozen ceiling; unchanged nested-Vm count is expected here. |
| M2.2 — indexed entry + typed activation context (migration) | BindingPlan/CallSeed seeds general parameters, this, mapped arguments and captures by index; special metadata uses typed fields. Remove general `function_env` name inserts and `initial_slots` name import, including closure creators. Legacy accessors read typed fields; unconverted dynamic/helper consumers retain existing adapters until M2.3. Development lodash/transform and diff/compare diagnose env assembly. | Entry still allocates names/FrameBindings, changed prologue/TDZ/alias ordering, borrowed inputs survive a switch, or a new snapshot path is introduced. |
| M2.3 — lexical scope leases + lazy dynamic consumers (migration) | Migrate every inventory row's name service, including helper/native callbacks, coercion, eval, with, results and realm fallback authority. Ordinary call/return has zero caller-slot export, name overlay, `apply_env` write-back or realm refresh scans. Development lodash/transform and diff/compare diagnose removed env work. | Any consumer needs caller value copies, escaping eval sees invocation scope, a realm mutation needs blanket refresh, scope leases retain caller frames, or ordinary calls still materialize names. Zero dynamic-name work does not mean zero required capture cells. |
| M2.4 — ordinary call/throw/return driver (migration) | General JS-to-JS calls, including captures/dynamic cells, push FrameIds; no nested Vm for this cohort. Development immutable/persistent and graphlib/paths diagnose frame setup/call work; suite cdjs is a call-dispatch cross-check. | Unwind/cleanup mismatch, live view across switching, unexpected tier admission change, or setup cost merely moves into dispatcher self time. Other boundaries may still use the legacy executor. |
| M2.5 — constructor/native/eval/suspension routing, retire adapters (final payoff) | Same-Vm synchronous callback and detachable resume leases; zero nested Vms within an active context, no hot CallEnv assembly or reconciliation. End-to-end payoff targets: development lodash/transform, diff/compare and immutable/persistent. | Re-entry borrow, detached lifetime, derived completion/job-order error, leftover hot compatibility path, or final payoff/portfolio controls fail. |
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

## Unit 0 evidence and first-unit order

Owner branch: `agent/t056-unit0/codex`; no runtime
optimization, recipe, order, manifest or benchmark changes. The linked records own
every measurement and hash. Held-out programs never selected or profiled anything.

Part 1: leading broad literal-field deficits execute virtual banks, not name lookup;
HashMap and public-field Raytrace reach property helpers through accelerators only.
`ddca5bdf` fixed the schema-3 wrapper so intermediate stages use the cumulative
budget and final/leaf stages keep payoff.

Part 2 counted the development lane with a feature-only overlay (byte-identical
canonical image with the feature off) and sampled it on the canonical image:

- Named-property work owns a mean 15.4% of samples (at least 10% in 7/9 non-papaparse
  programs) and is reached on the VM path, not only through accelerators: moment and
  diff make 31.1M/13.2M VM-path attempts with 4.0/5.7 name comparisons per attempt.
- Call/activation work is as large (mean 18.3%), but nested-Vm construction is only
  3.5%; `CallEnv` construction, its name-keyed frame-cell layer and write-back take
  11.3% (lodash 25.5%, diff 19.3%). These owner groups include non-removable
  semantic work; they bound reach, not an 11.3% promised win. The design assigns that state to M2.
- Value clone/drop glue is 7.8–15.5% everywhere; virtual-field work is at most 1.1%.

Order stays M1, M2, M3, with M2's scope widened; virtual-field admission stays an L4
leaf. M1's frozen plan targets development moment/dates, diff/compare and
meriyah/parse (stage cap 1.025, final 0.97 or 1/B_i, K=6, one attempt). Durable
counters, when needed, become plain `cfg(feature = "perf-counters")` code
admitted with a layout-rule neutrality screen, never text patches.

## M1 stage 1 (value facade + atom linking)

Owner `agent/t056-m1-atoms/claude` from `a0b6bb23`: realm-owned `AtomDomain`, named operands
linked once per body and domain, Value facade; stores still key by text. Its single screen is a
v3 `reject` (development lane, layouts 0 and 2) and a v4 `pass`; v4 governs as main's bound
v2/v3 successor (`HELD_OUT_SUCCESSOR`). See the [stage 1 record](archive/T056-m1-stage1-evidence.md).

## Design acceptance and verification

- [x] One target specifies flat frames, every ordinary named-property storage
  form, the value word/refcounts, unsafe boundaries, env/cells and tiers.
- [x] Ordered migrations have mechanism evidence, expected effects and stop
  conditions; intermediate regression gates differ from final payoff/leaf gates.
- [x] Prior experiments and report inferences are checked against source/history.
- [x] Documentation budgets, links and task index pass `check-context.py`.
- [x] Staged documentation fast gate passes for the three allowed files.

The original design changed only Markdown; unit 0's verification is in its archive
records. Every runtime stage runs full configured Test262, `check.sh` and NG comparisons.

M2 re-plan verification on the documentation change over `290e3f51`:
`./scripts/check-context.py` and
`PYTHONDONTWRITEBYTECODE=1 ./scripts/check-touched.sh --staged --explain` passed.
Runtime gates were not rerun for this design-only change. Next M2 action after
M1: refresh the proposed targets' profiles and freeze the complete five-stage plan.

M1.2 implementation and verification: [M1 stage 2](archive/T056-m1-stage2-evidence.md).
