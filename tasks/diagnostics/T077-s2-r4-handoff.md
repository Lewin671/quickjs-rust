# S2 frozen-source handoff: correctness qualified

The lead's post-round4 decision treats external 1.001593 as within five-pair
triage noise and is running formal 30-block, held-out and RSS acceptance for
fc3deef7 against dcee3c56. This owner completed the cached/off
[freeze correctness qualification](T077-s2-freeze-qualification.md) for fc3deef7
with the specified shared Test262 corpus: 42,672 configured passes per mode,
zero failures/timeouts/not-run cases and zero actionable gaps. Runtime code is
frozen; this owner submitted no iMac jobs. Typed static size remains a reported gap.

Runtime source **dc710e4b**, x86 order candidate **4098ee67**. Later commits are
records only. Canonical x86 SHA
`b756fe7409b65b6c273d71dfbbc06238fbb73b32341797c4df0c19a4725841a5`;
base SHA `6fc7251582f597f2e85cc8ce9e3a37cb7aaf9a32c442ab03a2abdaccb1204d3b`.
[Image](T077-s2-r4-final-canonical.json), [strict layout audit](T077-s2-r4-final-layout-audit.json),
[order provenance](T077-s2-r4-final-order.json). Default wide offset 0x400,
zero stale order entries; arm64 order/build.rs unchanged. The default generator
uses external/sentinel/broad only, with no offset/address search.

## Final public86

Job `20261031T030416Z-t077-s2-r4-final-public86` completes with 430 valid pairs,
zero invalid/heavy pairs; natural layout, five pairs per case. Automatic verdict
**likely loss**, diagnostic only. [Raw report](T077-s2-r4-final-public86.json).

| Cohort | Wall candidate/base | Retired instructions candidate/base |
| --- | ---: | ---: |
| development | 0.928813 | 0.924027 |
| external | 1.001593 | 0.989302 |
| broad | 0.982888 | 0.970633 |
| sentinel | 0.941874 | 0.922069 |
| all86 | 0.983194 | 0.971279 |

## Original five-case classification after correction

Ratios below come from the final report. Self-profile evidence belongs to the
[exact-image root pass](T077-s2-r3-root-pass.md), not this later binary.
[Before counters](T077-s2-r4-counters-before.json), [final counters](T077-s2-r4-final-counters.json)
use identical original public argv and separate canonical instruction runs at
their named source revisions, before final order regeneration. The final ratio
table uses the audited 4098ee67 image.

| Case | Final wall / instructions | Correction and remaining classification |
| --- | ---: | --- |
| external/sunspider-1.0/3d-raytrace | 1.066042 / 1.048179 | 39,139 array-length IC/miss-handler visits removed; identical tier operations. Residual +4.8% instructions, with a smaller time-per-instruction gap. Extra per-operation work is not isolated to a named function. Dense-write failures match base (18 numeric, 870 boxed), so the dense-write proposal remains deferred. |
| broad/method_call | 1.057076 / 0.942035 | Own addOne data property in the public fixture, so prototype constants intentionally do not apply. Fewer instructions but slower wall: residual time-per-instruction/layout or dependent-load pattern; typed body remains oversized. No qualified hardware CPI/cache-miss claim. |
| external/jetstream3-js-subset/hash-map | 0.930359 / 0.881420 | Overall repaired. Fewer instructions and faster wall; a remaining time-per-instruction gap is not a proven hardware cache-miss cause. |
| external/jetstream3-js-subset/gaussian-blur | 0.993558 / 0.981915 | Repaired redundant ordinary-IC/accessor routing. Final hot IC attempts 20 versus 720,022 before; indexed-name tests 0 versus 360,001; ID probes 1,253 versus 1,081,256. Native length getter is cell-proven, recomputed live; custom getters execute once. |
| broad/array_index_of | 0.913386 / 0.921093 | Inherited builtin method now uses a cell-protected weak constant. Both wall and instructions improve; no remaining overall loss in this case. |

The own-property distinction is visible in [runMethodCall](../../benchmarks/workloads/broad-micro.js).
Gaussian canonical instructions fall from 7,500,871,478 at 1352178a to
6,629,090,969 at dc710e4b; raytrace falls from 502,619,366 to 468,405,602.
Same-input arm64 allocations 403,293 → 43,293 prove 360,000 hit-time accessor
pair allocations removed. The final x86 pair uses 43,294 allocations versus
base 41,615. Weakly upgrading the existing pair preserves pointer identity and
never pins the getter. Counter builds are never timed.

Remaining worst external cases, classified only by wall/whole-case instruction
ratios (no new profiling or tuning):

| Case | Wall / instructions | Remaining signature |
| --- | ---: | --- |
| external/sunspider-1.0/string-unpack-code | 1.107776 / 1.085275 | Instruction-heavy; precise function attribution unresolved |
| external/sunspider-1.0/bitops-3bit-bits-in-byte | 1.103978 / 1.006516 | Time-per-instruction/layout or dependent-load pattern; hardware cause unqualified |
| external/kraken-1.1/json-stringify-tinderbox | 1.092274 / 1.067737 | Instruction-heavy; precise function attribution unresolved |
| external/sunspider-1.0/string-validate-input | 1.077657 / 1.054620 | Instruction-heavy; precise function attribution unresolved |
| external/sunspider-1.0/access-nbody | 1.076287 / 1.115678 | Instruction-heavy; precise function attribution unresolved |

## Static size and functional verification

[Exact ordered-image counts](T077-s2-r4-final-ordered-static.json), address-based
within unique nm extents; these are static instructions, not per-function
dynamic retired counts. Main body excludes separately listed wrapper stubs.

| Executor role | Base | Final | Target |
| --- | ---: | ---: | --- |
| Typed VM body (base run, candidate execute) | 3,024 | 3,649 | missed |
| Typed Wide-frame body | 3,006 | 3,651 | missed |
| Wide dispatcher | 5,372 | 5,316 | met |

[Complete check record/pinned CLI hashes](T077-s2-r4-final-checks.json):
full check.sh passes, including 5,170 curated Test262 cases; 2,542 debug
IC-verified agents+perf-counters tests pass on each of arm64 and x86; all
268 NG fixtures pass in default/release, IC-off/release and IC-verify/debug;
300 original + 50 warmed fuzz seeds pass on/off/verify (1,050 processes).
Fuzz adds stable prototype constants, write routes, accessor replacements and
instability transitions. Release/debug CLIs use the CLI agents feature and
both pass the actual --agent smoke. No new dependencies, unsafe or exclusions.

The post-round4 lead decision requested complete configured exact qualification
for fc3deef7. Both cached and IC-off scans now pass all 42,672 configured cases,
with zero failures, timeouts, not-run cases and actionable gaps; both commands
exit 0. Full inventory 53,572, unchanged configuration skips 10,900. NG has
42,602 passes/70 failures; Rust passes all 70 reference-only failures. Complete
burndown rows name fc3deef7 (cached) and doc-only equivalent 4a6cbab8 (off).
[Receipt, identity proof, summaries and case hashes](T077-s2-freeze-qualification.md).
The two full case-result files have identical SHA
`2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a`.
Earlier interrupted scans remain historical partial runs with no burndown row.

## RSS and retained source

Job `20261031T030538Z-t077-s2-r4-final-rss` repeats the original four scripts,
three pairs each, using the final canonical images. [Raw RSS](T077-s2-r4-final-rss.json).
Diagnostic medians only; lead owns RSS acceptance.

| Case | Base bytes | Candidate bytes | Ratio |
| --- | ---: | ---: | ---: |
| arr_mem | 42,008,576 | 42,356,736 | 1.008288 |
| arr_mem_nocall | 42,532,864 | 42,315,776 | 0.994896 |
| fn_mem | 120,180,736 | 110,833,664 | 0.922225 |
| fn_mem_nocall | 110,481,408 | 110,792,704 | 1.002818 |

Historical fn_mem_nocall still executes f(); it omits the method call only.

Main changes: value/layout prototype state and slot write barrier; weak
constant payloads/accessor pairs and cold cache admission; typed/compact/wide/
interpreter hit/miss boundaries and named hooks; regenerated x86 order; focused
semantic/lifetime tests and fuzz. [Architecture contract](../../docs/design/core-rebuild-objects.md)
owns the invariants; [checkpoint history](T077-s2-r4-checkpoints.md) owns attempts.
Commits 7ef4e68f (constants), 1352178a (cold paths), 2c47383a (hooks/pair reuse),
dc710e4b (final outlining), 4098ee67 (order) remain on this owner branch.

No push to origin, merge into main, formal or held-out run. The tracked corpus
pointer is unchanged; pre-existing untracked harness files were left alone.
Frozen-source exact qualification and both burndown records are complete. The
lead owns formal, held-out and RSS acceptance; no acceptance claim is made here.
