# T032: Slot reads for object-literal prototypes

## Goal and evidence

Close the cache coverage gap for ordinary prototypes stored as `Shaped` or
`ShapedPair`. Selection used the complete 30-block comparison placing HashMap
first at 5.2186x NG. The exact runtime base is `5702c789`; starting checkout
`24126432` differed only in task documentation. The replayed queue and a sample
of the exact candidate executable are in `target/performance-current-5702c789/`.

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

**Retained** after the single predeclared independent 60-block confirmation.
Final runtime commit: `9d344a0f0b5b9eb721425046a84e700b72c9240b`.
HashMap takes 0.920749x base time, a 7.9% reduction; all 76 watched comparisons
pass their unchanged gates. This is one retained optimization, not completion
of the user's all-benchmark NG target.

`prototype_data_slot` extends read-only prototype resolution without changing
the existing own-property write-cache policy. `prototype_data_slot_value`
uses the original small-object reader and an out-of-line literal fallback;
only the general VM and compact prototype-cache call sites use it. Cache
layout and guards are unchanged. Mechanism tests assert that 2-, 4- and
20-property literal holders actually produce a `PrototypeCandidate`, observe value writes
through its slot, and miss after storage materialization. Same-site JavaScript
tests cover invalidation through both compact and eval-forced general bodies.

The 12 focused cache tests, all 2,111 runtime tests, touched/full checks
(including all 5,169 curated Test262 cases), and `compare-qjs.sh` passed.
Exact candidate CI and all 42,672 configured Test262 cases also passed with
zero NG gaps (53,572 pinned files, 10,900 configuration exclusions).

### First implementation: rejected by a broad control

The initial eleven-pair diagnostic reduced HashMap to 0.9118x base (interval
0.9079-0.9141); this was not retention evidence.

The complete 30-block broad lane of `candidate-r1-30-clean` measured
`array_index_of` at **1.064246x** base (95% interval 1.063621-1.064610), above
the frozen 1.03 cap. All other broad points were <=1.0031. The sentinel/external
lanes were stopped; the broad report and `stop-decision.json` were preserved.
This is decisive negative control evidence, not a complete promotion bundle.
The candidate's exact CI and 42,672-case configured Test262 coverage passed.

Fresh base/candidate samples of the regressed workload contain only the dense
array search and its numeric-loop caller as material hot frames. Both routines
retain the same instruction counts (80 and 2,864); disassembly differences are
address/constant relocations rather than a changed array-search algorithm.
Code placement is a plausible contributor, not an established microarchitectural
cause. Do not tune padding, alignment flags, or benchmark-specific code.

Implementation two separated the new literal prototype reader from the
pre-existing small-object reader. Only the VM and compact prototype fallback
call it; own-property caches and prepared loop reads retain their original
reader. The literal fallback stays out of line to bound its impact on those
existing paths. The mechanism and thresholds stayed frozen, and this was the
last implementation allowed by the two-attempt plan. The discovered
array-search regression was added to the diagnostic controls without removing
any case.

### Second implementation: diagnostic verification

The second implementation passes the complete local checks, NG comparisons,
and all 12 cache tests. Its eleven-pair diagnostic covers the original target
and controls plus every remaining broad case (38 cases total, no repeated
case between the two diagnostic files). HashMap is 0.922487x base; the
regressed array search is 0.999690x. A* is 1.020618x and heterogeneous property
reads 1.020399x, both within the 1.03 cap but explicitly watched in the formal
cohort. No diagnostic control exceeds the cap. Artifacts: `screen-r2/` and
`screen-broad-r2/` under the unit evidence directory. These small-cohort results
were followed by a complete 30-block cohort with the original base and
thresholds, rather than being treated as retention evidence.

### Complete thirty-block result: inconclusive

The sealed `candidate-r2-30/` bundle completed all 76 comparisons without
measurement issues. HashMap is **0.918x** base (95% interval 0.916-0.920),
roughly an 8.2% reduction. Every broad and external control passes. The former
array-search regression is 0.999464x (0.997365-1.000929). Exact candidate
`9d344a0f` passes CI and all 42,672 configured Test262 cases with zero gaps.

One sentinel prevented promotion in that cohort: heterogeneous property reads are
**1.026603x** with interval **1.023612-1.035302**. This crosses 1.03, so the
verified decision is **inconclusive**, not retained. Preserve the full bundle
and `promotion-r2-30.json` (SHA-256
`d22b7af8f46f099fc1f8180a896179307869147db0aff8e911bfc7d03e2c7e84`).

Before further measurement, exactly one independent **60-block** confirmation
was declared over all three complete lanes, using the same executable and
unchanged plan/base/thresholds. Its frozen rules prohibited pooling or
replacing the first cohort, discarding observations, changing code, or repeating
again conditional on the result. This was the final experiment: rejection or
continued uncertainty would require reverting the runtime unit and preserving
both results. Declaration:
`target/performance-current-5702c789/confirmation-60-plan.json`, SHA-256
`22e647ef39b1f582d651a274ea7d81547a7bb3bd23094f5e856efc4aeb2b9eb0`.

### Final independent sixty-block confirmation: retained

The `candidate-r2-60/` cohort used the identical candidate, base and NG
executables, without a third implementation or pooled samples. Both complete
cohorts remain separate. Raw evidence was replayed before the promotion
decision; all 25 broad, six sentinel and 45 external cases were complete.

| Case | Candidate/base | 95% interval |
| --- | ---: | ---: |
| HashMap | 0.920749 | 0.917598-0.923550 |
| array_index_of | 0.999649 | 0.998010-1.002423 |
| heterogeneous_property_read | 1.022257 | 1.019304-1.026347 |
| A* | 1.018785 | 1.016053-1.020954 |

The largest point regression is the heterogeneous-property sentinel at 2.23%,
whose upper bound is now 2.63%, within the original 3% cap. The other five
sentinels improve in this cohort. HashMap's median process wall time is
1,349.660 ms against 1,466.158 ms for the base; paired-block analysis gives the
7.9% reduction rather than treating the ratio of medians as the estimator.

The formal promotion decision is **retained**, no reasons, 76 comparisons:
`target/performance-current-5702c789/promotion-r2-60.json`, SHA-256
`e980b6900cafdcb8f00a79875e70271c6851d3c5d94e82acdc7dd1f8af9f1682`.
Exact candidate [CI](https://github.com/Lewin671/quickjs-rust/actions/runs/34042686236)
and [Test262 coverage](https://github.com/Lewin671/quickjs-rust/actions/runs/34042949278)
passed; their runtime revision is `9d344a0f`, not a later documentation SHA.

### Remaining NG target

Keep lanes separate: all 25 broad specializer cases beat NG, all six generic
sentinels still trail it (1.366-1.701x), and the external portfolio has ten
resolved wins and 35 losses. These are pinned shell ports, not official suite
scores or a fixed-hardware production claim. External times include startup,
parsing, execution and shutdown.

The largest remaining external ratios are HashMap 5.022x, bits-in-byte 3.131x,
3d-raytrace 3.083x and string-validate-input 2.935x. HashMap therefore remains
the first workload to profile for the next **different shared cost**; this
unit does not justify another unprofiled cache variation. A refreshed queue
at the user's per-case 1.0x target is saved as
`target/performance-current-5702c789/remaining-ng-gaps.json`.
