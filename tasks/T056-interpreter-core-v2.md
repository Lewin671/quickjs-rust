# T056: Interpreter core v2

- Status: blocked; unit 0 complete; M1 stage 1 implemented on
  `agent/t056-m1-atoms/claude`, its stage screen blocked by a tool error.
- Implementation: present on that branch (`11c9fc08`): atom domain, cached
  per-domain linking of named operands, Value facade. Not on main.
- Verified at: `11c9fc08`, 2026-10-17: fast gate, curated Test262 5170/5170,
  NG fixtures 248/248 (local), exact Test262 scan with 0 Rust failures, iMac
  `check.sh` pass; see [stage 1 record](archive/T056-m1-stage1-evidence.md).
- Evidence: [part 1](archive/T056-unit0-part1-evidence.md),
  [part 2](archive/T056-unit0-part2-evidence.md), [M1 stage 1](archive/T056-m1-stage1-evidence.md);
  plan [`interpreter-core-v2-atoms-layouts`](performance-units/interpreter-core-v2-atoms-layouts.json).
- Unresolved: stage-1 screen blocked: both the screen and its whole
  replacement ended in the load-monitor settle guard (host wall clock ran
  short of `sleep(8)`), no verdict; M2 stages/budgets must absorb the activation-environment
  scope; part 1's imprecise bitops-3bit-bits-in-byte interval.
- Next action: lead decides the settle-guard fix or a third whole run; then
  screen `11c9fc08` under the stage budget and, if it passes, start M1.2.

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
| [T021](archive/T021-single-vm-frame-stack.md) and [log](archive/T021-single-vm-frame-stack-log.md) | July 31 moved-frame route: recursive 1.3848x, prototype calls 1.2769x; boxing worse. Also a distinct contiguous-buffer/hot-header prototype failed recursive 1.180476x and hash-map 1.150426x. V2 requires fieldwise initialization in persistent slots, zero-based per-frame buffers and no contiguous buffer transfer; verify emitted copies and borrow boundaries before routing. |
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
| M2.1 — reusable frame store + split helper borrows | No whole-frame transfer; slots own and reuse activation-environment state, with no eager per-call name-keyed frame-cell install; bounded slot/buffer allocations after warm-up, existing routing preserved. | Disassembly/counters show aggregate moves, per-call boxing or per-op window arithmetic. |
| M2.2 — ordinary call/throw/return routing | JS-to-JS frame calls stop constructing nested Vms, including capturing/dynamic-cell calls. | Wrong unwind/cleanup or reduced construction fails to reduce measured call work. |
| M2.3 — constructors, eval/with, native and suspension boundaries | Same-Vm callback/resume routes, detachable leases; zero nested Vms inside an active context; final M2 payoff. | Borrow survives re-entry, lifetime/job-order mismatch, or final payoff fails. |
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

Focused coverage before each full gate:

| Migration | Required discriminators |
| --- | --- |
| M1 | All four stores; constructor/literal/native receivers; computed/lone-surrogate keys; delete/reinsert, descriptor and prototype edits; getters/Proxy re-entry; arrays/strings/functions/exotics; cross-realm/foreign code linkage; atom/shape churn memory. |
| M2 | Deep and mutual recursion; live sibling captures, mapped arguments, per-iteration cells and TDZ; nested finally/using; native callback throws; eval/with and derived-this/super; generator prologue/yield*/re-entry; await/FIFO/job order; suspended frames outliving their creator. |
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
  11.3%. The design's frame section now assigns that state to M2.
- Value clone/drop glue is 7.8–15.5% everywhere; virtual-field work is at most 1.1%.

Order stays M1, M2, M3, with M2's scope widened; virtual-field admission stays an L4
leaf. M1's frozen plan targets development moment/dates, diff/compare and
meriyah/parse (stage cap 1.025, final 0.97 or 1/B_i, K=6, one attempt). Durable
counters, when needed, become plain `cfg(feature = "perf-counters")` code
admitted with a layout-rule neutrality screen, never text patches.

## M1 stage 1 (value facade + atom linking)

Owner `agent/t056-m1-atoms/claude` from main `a0b6bb23`. Realms own an
`AtomDomain` (dynamic `Function` scopes share their creator's); bytecode named
operands index a symbolic pool and link once per body and domain; foreign or
dead domains re-intern by content. Stores still key by text, so stage 1 is
judged only against the cumulative 1.025 stage cap. Mechanism, tests, gates and
the screen are in the [stage 1 record](archive/T056-m1-stage1-evidence.md).

## Design acceptance and verification

- [x] One target specifies flat frames, every ordinary named-property storage
  form, the value word/refcounts, unsafe boundaries, env/cells and tiers.
- [x] Ordered migrations have mechanism evidence, expected effects and stop
  conditions; intermediate regression gates differ from final payoff/leaf gates.
- [x] Prior experiments and report inferences are checked against source/history.
- [x] Documentation budgets, links and task index pass `check-context.py`.
- [x] Staged documentation fast gate passes for the three allowed files.

The original design changed only Markdown and did not claim runtime verification.
Unit 0's tooling and measurement verification is recorded in its archive records.
Every future runtime migration stage must run full configured Test262,
`check.sh` and NG comparisons as the design requires.
