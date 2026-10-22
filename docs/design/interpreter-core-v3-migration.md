# Interpreter core v3 migration (proposed)

Sequencing for the [target architecture](interpreter-core-v3.md), pending lead
approval. [T065](../../tasks/T065-interpreter-core-v3.md) owns status and evidence;
[T022 proposal](../../tasks/proposals/T065-core-migration-acceptance.md) owns the
proposed payoff/budget amendment. This document grants no runtime edit authority.

## Migration choice

| Route | Benefit | Cost / failure mode | Decision |
| --- | --- | --- | --- |
| New core crate + old-builtin adapter | Hard ownership boundary and small core | Today's builtins pattern-match Value, directly borrow object payloads, create handles without a context and call back through CallEnv. A universal adapter must duplicate or wrap mutable graphs, preserve identity/cells and trace cross-heap cycles. Per-operation conversion can erase the gain; porting is still required. | Reject the object-graph adapter. A later crate extraction is possible after APIs settle, not a prerequisite. |
| Strangler inside qjs-runtime | Reuses parser, binding analysis, tests, semantic helpers and builtin family boundaries; no new dependency/crate | Temporary duplicate implementations and feature combinations need explicit containment; layer-by-layer changing public Value would touch almost every builtin before a real program runs. | Recommend a private alternate backend, ported in vertical families, then switch the entire runtime. |
| In-place flag day for Value/objects/VM | No long-lived duplicate engine | Large unexecutable interval; root bugs and semantic regressions are discovered after most of the port. | Reject for the roughly 189k-line runtime. |
| Keep Rc and replace dispatch first | Smallest initial diff | Retains clone/drop and call/name work; T024's preamble experiment and T056's narrow prototype do not support a dispatch-only payoff. | Reject as the destination; use only a reference representation for tests. |

Create private `core_v3/{word,heap,object,code,vm,builtins}` modules under an opt-in
feature. The old backend remains the default and correctness oracle until cutover.
Select a whole backend before evaluating source; no mid-program fallback to the
legacy VM, no mutable values crossing heaps and no old builtin secretly servicing
a new-heap object. Unsupported v3 operations report an explicit experimental
coverage failure, distinct from a JS exception. Never turn that into silent fallback.

Port builtin families to the new Context/ArgRange ABI. Share pure algorithms
(number math, Unicode tables, parser/binding analysis) directly; abstract only a
small demonstrated semantic operation used by both backends. Do not genericize
all 189k lines or add a trait method for every old helper. Temporary family copies
carry a deletion milestone; the final tree has one engine and one builtin family.
A root/Value facade bridges *source APIs*, not live mutable object graphs.

Public construction/access will require an explicit runtime or rooted handle;
old context-free ObjectRef/ArrayRef creation cannot safely survive tracing GC.
Keep the old public API while the feature is experimental. At cutover update CLI,
tests and embedders atomically, document the source break and remove old exports.
A host root may keep the heap owner alive; heap payloads never own host roots, so
that ownership does not create another uncollectable JS cycle. Cross-realm values
in one runtime group share identity; cross-runtime transfer is explicit/rejected.

## M0: retire the combined-core risk before the full builtin port

Run **the complete pinned development/big.js/arith program**, including unmodified
big.js 6.2.2, the existing 300-iteration driver and stdout checksum `d4940ea1`.
It exercises real constructor/prototype/array/string/numeric behavior with a
smaller builtin closure than graphlib. This is a whole program, not extracted
arithmetic, a translated library, special opcode or benchmark-specific admission.
Choose it for port size and value traffic, not evidence of whole-lane sufficiency.

M0 includes packed words, arena collection, ordinary shapes, register bytecode,
single-driver JS calls, shared capture cells, the new native ABI, and *all*
builtin/exception behavior needed by library initialization and the driver.
Use the existing AST/binding compiler as input to a verified v3 lowerer; lowerer
rejections fail coverage. No old Vm, CallEnv, Rc JS payload, compact/wide executor
or boxed-value conversion may execute in the v3 process. Rust compiler metadata
Rc is allowed. Instrument backend entry and compatibility counters to prove this.

Collect three alternating whole-process instruction pairs v3/main and v3/NG on
the iMac, pinned program/source/build/binary hashes, valid output and raw rusage.
Also repeat with the safe reference representation and allocation-stress GC for
correctness, not timing. Profile only development cases. Count clone/drop owners,
frame entries, name assembly, allocations/collections, live bytes and rooted
native callback values. Main means the frozen T065 baseline, never a moving HEAD.

**Proposed stop checkpoint, frozen before coding:** median I(v3)/I(main) ≤0.85
and I(v3)/I(NG) ≤2.0, every pair improves versus main, per-program max-RSS ≤1.25
of main, successful full checks on the default engine and zero failures on v3's
supported Test262 slices. Require forced collections with escaping captures,
callback mutation/throw, cyclic garbage and repeated load/run/drop returning live
bytes to the root baseline. An allocation-only arena that never collects fails.
Instruction gains do not establish latency gains; a quiet-host coarse screen is
required before broadening, followed by the final formal gate much later.

Timebox the first complete slice to six engineer-weeks (20 days to first working
program, ten more for validation/repair); permit at most two predefined three-day
mechanism repairs within that cap. Missed coverage, unsound rooting, persistent
instruction/RSS failure or lack of a working program stops the route for lead
review. Keep the default engine; do not rescue the result by shrinking the input,
turning off collection, adding a recognizer or changing thresholds after seeing it.
An approved alternative handle encoding needs a new explicit M0 plan, not a reset.

Big.js alone does not retire call-heavy/eval/exotic risks: require **graphlib/paths**
next before authorizing the bulk builtin port. It supplies closures, computed
keys, object churn and callbacks; all reachable builtins must run natively on v3.

## Milestones and effort

Estimates are engineering effort for one owner familiar with this runtime, not
measured delivery dates. Serial ownership covers shared types and builtin APIs;
independent testing may be delegated only with an explicit boundary/authorization.
Do not start all families before the two early program checkpoints pass.

| Milestone | Result / deletion boundary | Measurable exit checkpoint | Engineer-weeks |
| --- | --- | --- | ---: |
| M0 combined slice | Above vertical slice; experimental backend only | Exact checksum, root/collector stress, zero legacy execution, ≤.85 main / ≤2 NG instruction ratios, default full checks | 4–6 |
| M1 object/call generality | Complete graphlib; descriptors, prototypes, dictionaries, symbols, bound/native/callback calls; typed BindingPlan and cell ownership | graphlib median I(v3)/I(main) ≤.85 and /NG ≤2; neither early program exceeds main instructions; shape/atom retention bounded under churn; focused object/function/closure/arguments Test262 + all NG fixtures applicable to supported semantics | 5–8 |
| M2 builtin and dynamic coverage | Port all remaining standard builtin families, eval/with, private/class/derived constructors, modules; primary code-unit String/RegExp storage | All ten development programs validate outputs; report per-case v3/main/NG instructions; development instruction geomean ≤.85 main; default full gate; supported v3 Test262 slices pass, complete v3 exact scan/burndown records remaining gaps; all applicable NG fixtures; no feature delegated to old heap | 6–10 |
| M3 suspension and host lifetime | Generators, async/generators, promises/jobs, module await, disposal, WeakRef/ephemerons/finalization; feature-gated agents backing | Resume/callback/root stress and all suspension/dynamic-binding discriminators; subset + full exact configured Test262 zero gaps and recorded burndown; default/agents/perf-counters configurations pass | 4–7 |
| M4 accelerators / portfolio | Port typed numeric/array kernels and scalar replacement to common windows/root maps; retire separate compact/wide drivers | All selection lanes execute; case and lane instruction tables vs main/NG, coarse lane wall ≤1.05 main before formal; identical observable exits; full checks, NG comparison and exact zero-gap scan | 3–5 |
| M5 cutover / retirement | Final acceptance, explicit public-root API switch, delete old Value/object/CallEnv/VM/builtin copies and feature split | Approved whole-portfolio/held-out/RSS formal gates, complete exact Test262 and NG fixtures on final uninstrumented source, public API migration notes and no legacy compatibility path | 2–4 |

Total **24–40 engineer-weeks**, before contingency; reserve roughly 25% for
GC/builtin/dynamic-scope integration (planning envelope **30–50 weeks**). Re-estimate
after M0/M1 from actual port throughput and dependency closure. More agents do not
linearly shorten shared-value or GC work. M2's complete exact scan may expose suspension
failures reserved for M3: record them explicitly, never declare parity early or
add skips. Any gap outside the declared remaining M3 scope blocks M2 exit.

## Evidence and stage discipline

Before M0 implementation freeze the source baseline, queue/binary/corpus hashes,
root and ABI contracts, milestone bounds and complete failure/formal budgets in
an approved core-migration plan. Current schema tools do not accept that kind;
implement/review policy support first. The design branch freezes no JSON plan.
Existing T056/T064 failures and reserves remain historical and cannot be spent here.

At each exit record candidate revision, recipe, executed backend/tier counters,
all development instruction ratios to both the fixed main and NG, explicit
unsupported coverage, tests, RSS and retained memory. Early milestones may report
unrunnable programs as incomplete, never omit them from the inventory. Run focused
slices per newly supported family plus full default-engine checks at every stage.
An isolated backend may be incomplete; the shipped default may never regress.
Only M5 claims retained performance, and held-out is never sampled for selection.

For each builtin port prove observation order around getters/Proxy/coercion/species,
callbacks and thrown values. For each allocation path prove trace edges and root
publication before enabling collection. Validate all public bit-patterns, OOM
paths, weak reuse, cross-realm errors and dynamic binding discriminators. Use the
existing fixtures as independent semantics checks, not just old/new agreement.

If a stage fails, retain its evidence and last verified default; repair within the
frozen budget or return to lead. Do not silently activate partial v3 or erase typed
wins to make the generic core easier. Timing remains serialized through the iMac
wrapper; check status first, use no timing flag for instruction-only runs, check
free disk before builds and stop below 8 GiB. No direct SSH or local timing.
