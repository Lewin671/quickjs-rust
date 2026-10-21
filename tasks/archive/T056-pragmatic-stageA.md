# T056 pragmatic stage-A candidate

- Status: implementation present; local gates pass, new-image iMac qualification pending.
- Implementation: based on runtime dfff3bb6; retain packed flags alone (120B
  ObjectData), amortize repeated large-shape misses and batch native method
  descriptors. No other header, alignment, parser, tier or binding change.
- Verified at: parent0a7bb6e5;13 layout tests,2350 runtime tests and5 batch
  perf-counter tests passed on working tree. Full gate/NG255/subset5170 pass; iMac86 pending.
- Evidence: [frozen implementation scope](../diagnostics/t056-pragmatic/implementation-plan.json),
  [initial event attribution](../diagnostics/t056-pragmatic/initial-attribution.json),
  [PMU closure](T056-pmu-closure.md), [header factors](T056-header-subparts.md),
  [architecture review](T056-architecture-review.md), [startup attribution](T056-stack-phase-diagnosis.md).
- Unresolved: new canonical image/instruction ratios and future diagnostic timing;
  equal-instruction relative-PC sensitivity remains an unexplained risk.
- Next action: finish local gates, qualify exact new canonical image and86
  non-timing instruction snapshot, then prepare lead correctness→86→triage→four-case
  K=6 diagnostic. No owner timing or use of the last acceptance attempt.

## Chosen change and evidence boundary

Keep the packed cold-flag byte and restore index_property_count to usize,
including its constructor/class-field arithmetic. Existing declaration order,
hot exotic booleans, scalar alignment and other layout changes are untouched.
Both single factors retain120B ObjectData; packing flags preserves its general
initialization representation while removing checked narrowing. Historical
lead123327Z flags_order global/local ratios0.996488/0.968720 versus count_order
1.002008/0.970059 are diagnostic support, not proof of this candidate's timing.
No future layout or inline/padding selection is authorized.

Count+flags together was the minimal observed trigger, with56 relative-PC
changes in the typed executor's numeric loop and essentially equal instructions.
The full finite-number loop is152(global)/197(local) instructions, not150;
there are no per-iteration ObjectData flag/count loads. Ops/bank data placement,
stack phases, allocator zone, uniform code shifts and JCC boundary counts did
not account for the event. These exclusions are bounded to the recorded cases,
not proof that all data/frontend/predictor effects are impossible. Evidence:
[header/full-loop](T056-header-subparts.md), [dispatch PC audit](T056-dispatch-address.md),
[JCC](T056-jcc-policy.md), [data](T056-m1-stageA-data-diagnosis.md),
[stack](T056-stack-phase-diagnosis.md), [ops confirmation](T056-pmu-attribution.md).
Hardware cause stays unknown; PMU is closed. The architecture review's small
non-generic kernel/cold-service boundary remains M2 work under its own plan,
not an unimplemented claim or a reason to silently spend the last attempt now.

## General lookup and initialization work

The previous failed-write admission never warmed an index on missing keys,
even when the same large shared shape was probed repeatedly. Reads and writes
now share the existing4-probe linear budget, including misses; the next probe
builds the index. One-pass transient growth still does not index each prefix.
No key spelling, source, library or bytecode-template admission.

The startup cost predates8f/dfff: stage-A commit3506f4db introduced the
per-add layout authority/prefix construction used by intrinsic initialization.
Prior end-of-builtins snapshot accounted for663 shapes/11,662 copied fields;
empty canonical13.153M→14.574M instructions at8f, versus CLI-only+0.16%.
Current non-timing event attribution175023Z confirms String1499, TypedArray1346,
Date1218, Math1135, Array1068 copied fields (6266 total), with other installers
accounted separately. These instrumented events identify work, not instruction
or time shares; no exact instruction attribution is invented.

Explicit complete descriptor batches publish one final immutable shape through
the existing weak content cache; normal incremental edits retain bounded owned
edges. Batch roots add no strong retention cycle; dictionary boundary remains
128 fields. Keys normalize/deduplicate once, first slot/last descriptor; preserve
old properties, symbols/prototypes, index classification and cached-read safety.
Numeric/globalThis/ordered-state special cases keep sequential semantics. Native
method tables finish before aliases or readers. No new ObjectData field/global
construction mode/dependency/unsafe/FFI, nor an environment name snapshot.
