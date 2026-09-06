# T032: Slot reads for object-literal prototypes

## Goal and evidence

Close the cache coverage gap for ordinary prototypes stored as `Shaped` or
`ShapedPair`. The latest complete 30-block comparison places HashMap first at
5.2186x NG. The exact runtime base is `5702c789`; current `24126432` changes
only task documentation. The replayed queue and a new sample of the exact
candidate executable are in `target/performance-current-5702c789/`.

The profile and frozen plan are bound by
`performance-units/shaped-prototype-slot-reads.json`. HashMap's prototype is
an object literal, but `own_data_slot` accepts only Small storage, so the
prototype cache cannot record its otherwise stable method slots. This unit
adds addressability for already-existing literal slots. It does not change
the object or binding model, intern names, or expand a workload recognizer.

## Scope

- Slot resolution/reads in `value/object/slot_reads.rs`, their existing
  prototype-cache VM/compact call sites, focused cache/storage tests, this
  task and its frozen plan; main-agent ownership on one branch.
- Keep the prototype cache representation and its receiver-absence, prototype
  identity and holder-layout guards. Dynamic storage remains a fallback.
- No parser/AST, dependencies, unsafe code, benchmark changes or third-party
  edits.

## Acceptance and verification

1. Validate the frozen plan against the replayed exact queue and hashed raw
   profile before implementation.
2. Prove actual cached slot coverage for both literal storage representations;
   test live assignment, descriptor changes, pair materialization, deletion,
   shadowing, prototype replacement, polymorphism and observable accessors.
3. Run runtime tests, touched/full checks, focused Test262 and NG comparisons.
4. Diagnostic screen: eleven alternating candidate/base pairs over the original
   HashMap bundle plus controls. This can falsify but cannot retain the change.
5. Formal evaluation: one complete 30-block candidate/base/NG cohort, with
   HashMap <=0.97x base and controls <=1.03x including all six sentinels.
   Promotion checks every broad/external case and exact full Test262 parity.
6. At most two implementations; record negative evidence without retuning
   thresholds or substituting amplified sampling workloads for timing.

Passing this unit is incremental progress. The user's campaign target is a
per-case win over NG across the benchmark inventory; neither a geometric mean
nor improvement against the Rust base establishes that result.

## Status

Implementation complete. `prototype_data_slot` extends read-only prototype
resolution without changing the existing own-property write-cache policy;
`own_data_slot_value` reads both literal representations. Cache layout and
guards are unchanged. Mechanism tests assert that 2-, 4- and 20-property
literal holders actually produce a `PrototypeCandidate`, observe value writes
through its slot, and miss after storage materialization. Same-site JavaScript
tests cover invalidation through both compact and eval-forced general bodies.

The 12 focused cache tests, all 2,111 runtime tests, touched/full checks
(including all 5,169 curated Test262 cases), and `compare-qjs.sh` passed.
The initial eleven-pair diagnostic reduces HashMap to 0.9118x base (interval
0.9079-0.9141); this is not a formal retention decision. Complete thirty-block
measurement and exact-commit Test262 parity are pending.

### First implementation: rejected by a broad control

The complete 30-block broad lane of `candidate-r1-30-clean` measured
`array_index_of` at **1.064246x** base (95% interval 1.063621-1.064610), above
the frozen 1.03 cap. All other broad points were <=1.0031. Stop before completing
the sentinel/external lanes; preserve the broad report and `stop-decision.json`.
This is decisive negative control evidence, not a complete promotion bundle.
The candidate's exact CI and 42,672-case configured Test262 coverage passed.

Fresh base/candidate samples of the regressed workload contain only the dense
array search and its numeric-loop caller as material hot frames. Both routines
retain the same instruction counts (80 and 2,864); disassembly differences are
address/constant relocations rather than a changed array-search algorithm.
Code placement is a plausible contributor, not an established microarchitectural
cause. Do not tune padding, alignment flags, or benchmark-specific code.

Before timing implementation two, separate the new literal prototype reader
from the pre-existing small-object reader. Only the VM and compact prototype
fallback call it; own-property caches and prepared loop reads retain their
original reader. Keep the literal fallback out of line to bound its impact on
those existing paths. The mechanism and thresholds stay frozen, and this is
the last implementation allowed by the two-attempt plan. Add the discovered
array-search regression to the diagnostic controls without removing any case.

### Second implementation: diagnostic verification

The second implementation passes the complete local checks, NG comparisons,
and all 12 cache tests. Its eleven-pair diagnostic covers the original target
and controls plus every remaining broad case (38 cases total, no repeated
case between the two diagnostic files). HashMap is 0.922487x base; the
regressed array search is 0.999690x. A* is 1.020618x and heterogeneous property
reads 1.020399x, both within the 1.03 cap but explicitly watched in the formal
cohort. No diagnostic control exceeds the cap. Artifacts: `screen-r2/` and
`screen-broad-r2/` under the unit evidence directory. These small-cohort
results are not retention evidence. Run a new complete 30-block cohort for
this final implementation; keep all thresholds and the original base fixed.
