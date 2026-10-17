# T056 unit 0 part 2: mechanism counters, development profiles and first-unit order

Part 2 counted and sampled the ten development programs and the suite cases on
the shared iMac, ranked the candidate first runtime units and froze the first
plan. No runtime optimization was written. The diagnostic overlay that produced
the counters was evidence-only and has been reverted (below). Held-out programs
were not profiled, counted or used to select anything; the queue's development
lane, whose programs may select work, supplies the targets.

## Identities

| Item | Value |
| --- | --- |
| Measured runtime | `b481894d351d933e94717a9c295c15fe4a466804`; runtime/build diff to queue candidate `351004f386f4c2c1d870467ae6e1e481952a330c` is empty |
| Canonical image | `c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e` (iMac fat16/abort agents recipe; both revisions) |
| Diagnostic source | `84fee838c7e184e239f81edcce945ae7e8ddf682` (reverted by `04c67bf1`) |
| Feature-off image at 84fee838 | byte-identical to c7f0f66a (SHA-256 compared on the iMac build recipe) |
| Diagnostic image (`agents,perf-counters`) | `6f60d7ee2f99fc37ee847046c5ceec2ca2f745cdd5025081111dae541f6f1828`; never timed |
| Counter job | `20261017T100318Z-t056-part2-complete-counts-gates`, exit 0, 117 records, every checksum and probe/hit/reject partition passes; resumes 69 records of `20261017T093912Z-t056-part2-overlay-identity-counts-gates` after an identical-image check |
| Gates at 84fee838 (same job) | `check.sh` pass including 5170 Test262 subset cases; `compare-qjs.sh` 248 fixtures ok |
| Profile job | `20261017T095048Z-t056-part2-development-sampling-windows`, exit 0: 3 sample windows (10 s, 1 ms) per development program on the canonical image, every stdout checksum-validated |
| Queue / policy | v3 queue `424a1e79700353a842d5d6bb53cc03c285388a77b43695c9c95300541fddea56` (job `20261017T095818Z-t057-v3-qualify`); policy v3 `f655f25a86f4933cfb11a097a7e109b493628bc1f9a9d320cd27bd7bf300ba33` |

Raw outputs stay in the workspace `outputs/imac-bench/<job>/`. Moment and
papaparse outlive the 10 s window; their profiles are windows, not whole runs.
Suite shares reuse part 1's exact-image samples (job
`20261017T061409Z-t056-unit0-attribution`).

## What the overlay measured, and why it is gone

The overlay was a proc-macro crate, `crates/qjs-runtime-diagnostics`, applying
text patches to runtime sources under the existing `perf-counters` feature. It
counted: property-name hashes/comparisons/bytes in all ordinary stores (Small,
Dynamic, Shaped, ShapedPair, function and array named maps), scoped to the
innermost VM ("ordinary") or compact/typed ("accelerated") named handler; VM
versus accelerated named read/write attempts; probes/hits/rejects for nine
cache kinds; Value clones (and the heap-owning subset); handle drops per heap
kind; payload allocations/frees per heap kind; frame constructions, nested Vm
constructions and source-level frame transfers; virtual-field dispatches.
These are source events, not emitted copies or latency. Its comparison counter
counts equality calls; the accelerated/ordinary split omits unscoped setup work.

Text patches against files M1/M2 will rewrite are a maintenance hazard, so it is
not proposed for main and was reverted from this branch; the source remains at
84fee838. Durable counters, when a later unit needs them, become plain
`cfg(feature = "perf-counters")` code admitted with a layout-rule neutrality screen.

Two external programs read the clock into their control flow, so their executed
work varies in every engine: Kraken audio-beat-detection (lead check: same binary,
three runs, loop_plan_entries 107708/109748/110003; 29 counters differ from part 1)
and SunSpider crypto-aes (`new Date().getTime()` nonce; four compact/call counters
differ by under 20%). That is a benchmark property, not an instrumentation defect.
All internal N=10,000 counts equal part 1's exactly.

## Counters (whole process, diagnostic image)

Named = VM-path plus accelerated attempts (reads + writes). Cmp = property-name
equality calls; Ord cmp = those inside VM named handlers. Rej = cache probe rejects.

| Program | VM named | Accel named | Cmp | Cmp/attempt | Ord cmp | Hashes | Probes (rej) | Nested Vms | Calls | Clones (heap) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| moment/dates | 31.08M | 12.22M | 172.1M | 4.0 | 77.3M | 20.8M | 49.3M (60%) | 938,664 | 7.68M | 138.5M (67%) |
| diff/compare | 13.20M | 10.22M | 133.0M | 5.7 | 3.7M | 7.1M | 42.9M (73%) | 77,297 | 2.46M | 109.9M (65%) |
| meriyah/parse | 2.31M | 7.44M | 18.8M | 1.9 | 1.0M | 10.6M | 32.6M (93%) | 279,539 | 2.32M | 42.1M (64%) |
| js-yaml/document | 4.55M | 12.09M | 12.5M | 0.8 | 2.5M | 6.6M | 32.7M (75%) | 220,013 | 2.20M | 49.2M (66%) |
| immutable/persistent | 4.03M | 7.33M | 21.6M | 1.9 | 1.2M | 3.2M | 13.7M (46%) | 487,770 | 5.44M | 66.6M (69%) |
| graphlib/paths | 1.13M | 4.25M | 11.9M | 2.2 | 0.5M | 2.5M | 8.8M (51%) | 89,264 | 1.00M | 41.0M (91%) |
| lodash/transform | 0.08M | 2.86M | 1.8M | 0.6 | 0.05M | 0.8M | 4.3M (65%) | 41,661 | 4.79M | 29.0M (89%) |
| matter-js/simulate | 0.25M | 69.07M | 7.3M | 0.1 | 0.03M | 7.0M | 96.8M (59%) | 3,156 | 0.70M | 103.5M (42%) |
| big.js/arith | 0.02M | 0.66M | 0.5M | 0.8 | 0.03M | 0.2M | 0.4M (42%) | 56 | 0.03M | 128.9M (15%) |
| papaparse/csv | 1.33M | 3.71M | 36.5M | 7.2 | 0.7M | 3.5M | 4.1M (47%) | 97 | 1.08M | 34.2M (78%) |
| hash-map | 0 | 10.29M | 3.5M | 0.3 | 0 | 0.09M | 18.6M (57%) | 9 | 2.34M | 17.3M (72%) |
| raytrace-public-class-fields | 0 | 11.39M | 13.4M | 1.2 | 0 | 1.5M | 12.3M (13%) | 249 | 2.01M | 23.3M (54%) |
| cdjs | 0.50M | 11.63M | 4.0M | 0.3 | 0.07M | 0.3M | 15.0M (27%) | 18,046 | 2.29M | 21.8M (79%) |

Development virtual-field dispatches are at most 0.48M (immutable); broad
property_read/array_read/property_write add 6/7/11 per declared operation.
Every frame construction is a nested Vm, source-level frame transfers are zero
in every case, and same-Vm frame entries are zero.

## Sampled shares (canonical image, main thread)

Owner attribution charges each allocator, mem*, hash and Value-glue self sample
to the nearest engine caller, so the groups partition samples. Named = property
storage, caches and named handlers. Vm = nested Vm construction/teardown and
prepared-call wrappers. Env = `CallEnv` creation/drop, name-keyed
`FrameBindings`/`DynamicBindings`, `apply_env` write-back,
`refresh_realm_backed_locals_from_realm`, slow local paths. Call = call dispatch.
Loop = interpreter/compact loop self including inlined handlers (not attributable).
Glue = Value clone/drop self samples (overlaps the owner groups). Shares, not
removable fractions; inlining moves cost into Loop.

| Program | Samples | Named | Vm | Env | Call | Loop | Strings | Virtual | Glue |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| moment/dates | 24945 | 22.3% | 2.6% | 8.3% | 7.6% | 39.6% | 7.0% | 0.0% | 9.4% |
| diff/compare | 18104 | 19.3% | 3.2% | 19.3% | 5.1% | 33.8% | 10.2% | 0.1% | 8.0% |
| meriyah/parse | 7259 | 26.0% | 2.9% | 8.0% | 4.7% | 51.9% | 2.7% | 0.7% | 8.4% |
| js-yaml/document | 12268 | 14.2% | 4.3% | 9.5% | 4.2% | 42.2% | 20.4% | 0.2% | 7.8% |
| immutable/persistent | 10490 | 11.5% | 10.3% | 13.5% | 4.3% | 50.1% | 1.0% | 0.8% | 9.7% |
| graphlib/paths | 3559 | 16.6% | 6.1% | 15.4% | 3.7% | 50.7% | 2.8% | 0.1% | 10.7% |
| lodash/transform | 4692 | 6.1% | 2.0% | 25.5% | 2.1% | 55.1% | 1.3% | 1.1% | 8.4% |
| matter-js/simulate | 7851 | 20.5% | 0.1% | 0.8% | 0.4% | 72.1% | 0.9% | 0.2% | 10.5% |
| big.js/arith | 5488 | 1.5% | 0.0% | 1.0% | 0.1% | 85.1% | 1.1% | 0.0% | 15.5% |
| papaparse/csv | 25046 | 2.5% | 0.1% | 0.8% | 0.5% | 3.5% | 91.7% | 0.0% | 0.8% |
| hash-map | 482 | 22.6% | 0.0% | 1.2% | 0.6% | 72.8% | 0.0% | 0.0% | 5.6% |
| raytrace-public-class-fields | 733 | 15.7% | 0.0% | 0.1% | 8.6% | 73.1% | 0.0% | 0.0% | 9.0% |
| cdjs | 880 | 10.6% | 3.0% | 4.3% | 2.0% | 68.0% | 10.0% | 0.0% | 11.2% |
| broad/property_read | 416 | 2.6% | 0.0% | 0.0% | 0.0% | 34.9% | 0.0% | 62.5% | 11.1% |

Allocator self samples are 10–25% in seven development programs; the largest
callers are nested Vm initialization, call/env construction and `CallEnv` drop,
object-literal creation and property reads.

## Ranking for the first runtime unit

Means are equal-weight over the nine non-papaparse development programs
(papaparse is T057's string work and is excluded from ordering).

| Rank | Candidate | Development reach | Suite reach | Generality / risk |
| --- | --- | --- | --- | --- |
| 1 | M1 atoms/layout authority | Named mean 15.4%, at least 10% in 7/9; 4.0/5.7 name comparisons per attempt in moment/diff (1.11/1.39 GB compared); 46–93% probe rejects; VM-path share of attempts 72/56/35/27/24% in moment/diff/immutable/js-yaml/meriyah | hash-map 22.6%, raytrace 15.7%, cdjs 10.6%, property sentinels 12–30% | Every program does named access; staged, safe Rust, design and draft exist; facade is also M3's dependency |
| 2 | M2 frames and activation | Vm+Env+Call mean 18.3%, at least 10% in 7/9, but nested-Vm work only 3.5% (max 10.3%); Env 11.3% is not in M2's current scope | cdjs 9.3%, raytrace 8.7% (call dispatch); recursion sentinel invisible | General; touches the protected env boundary, suspension and eval; T021 history; needs the design scope change below first |
| 3 | Value transport (M3) | Glue 7.8–15.5% in every program (mean 9.8%); 29–139M clones, 42–91% heap-owning | 5.6–11.2% | An 8-byte word keeps refcount traffic; highest risk (unsafe, ABI, headers); needs the M1.1 facade |
| 4 | Virtual-field / boxed dispatch | Virtual at most 1.1% (mean 0.4%); at most 0.48M dispatches | broad field cases only (48–66%) | Broad microbenchmark shape; remains an L4 leaf |

Decision: M1 is first; its plan is frozen below. The evidence does not reorder
M1 before M2 before M3, but it changes M2's scope: per-activation environment
construction and its name-keyed frame-cell layer cost about three times nested-Vm
construction, so M2 must own that state (see the design's frame section).

## Frozen plan binding

`tasks/performance-units/interpreter-core-v2-atoms-layouts.json`, schema 3,
migration of 3 stages at stage 1, base/queue candidate 351004f3, unit SHA-256
`e0f3272ca61da4e1f373cf88f27408579d3931d8b9d0a93876656fce9f4e54a2`. Targets:
development moment/dates (queue rank 4), diff/compare (5), meriyah/parse (6);
rank ceiling 6; every other policy case is a control. Stage budget 1.025
cumulative; final payoff 0.97, tightened by the rule to 1/B_i (0.9620 moment,
0.9615 diff); K=6 panel, cpu-work, 30-pair held-out, 30-block formal, one attempt.
js-yaml (rank 2) is excluded as a target because T057 profiles it for string work.
`validate-unit` against the v3 queue and the receipt root returned `valid`.

Receipts re-import repeat 0 of each target from the profile job with base
351004f3. The sampled executable was the rev-b481894d cache path; its bytes are
the queue candidate's image (c7f0f66a), which each receipt records and the
validator checks. Inclusive fractions are the named-frame stack envelopes
(25.0/24.5/31.7%). They include mandatory lookup and ownership work; no speedup is
forecast. The validation artifact, queue copy and receipts are in workspace
`outputs/t056-part2-plan/`.

## Open issues

- M2's design now names the activation-environment cost; its migration stages
  and budgets are not yet re-planned.
- The formal bitops-3bit-bits-in-byte interval from part 1 remains imprecise.
- No emitted-copy count exists; source-level transfer counts are zero.
