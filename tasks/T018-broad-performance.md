# T018: Broad Performance Campaign

- Status: active. This is the performance campaign contract.
- Implementation: partial. Milestones B1-B2 are complete; B3-B6 are open.
  Both screened leaves are integrated on `integration/batch-1`; formal
  decisions are pending in [T037](T037-integration-batch-1.md).
- Verified at: `e54e33eb`, 2026-10-11, fresh 30-block three-engine
  comparison against `d1313d52` and NG `f7830186`; all lanes complete.
  Integration correctness gates at `7095a4aa` and placement at `20c3cd22`
  are recorded in [T037](T037-integration-batch-1.md); formal decisions pending.
- Evidence: standing below; brief 01 bundle/queue/profile index are local
  artifacts, not retained. SHA-256 values bind the recorded conclusions.
- Unresolved: decision readiness is `inconclusive` (audio-dft precision);
  SunSpider geomean and ten external cases miss the near-term <=1.0 goal.
  No campaign completion threshold below is recorded as met.
- Next action: complete T037's placement review, then run one quiet-host
  formal batch against the original queue candidate executable. Use the live
  `global-numeric-binding-sinks-repin` and `class-field-batch-install` plans;
  their leaf screens passed. T036 remains design only.

This file holds the contract and the last formal standing. It does not
record units or select work.

## Standing at e54e33eb

Candidate `e54e33eb9371c5cb4627bd8e590a648085a7fa26`, base `d1313d52`,
NG `f7830186`. Same-host diagnostic, not fixed-hardware qualification.
Command: `./scripts/perf-compare.sh --base d1313d52 --candidate e54e33eb
--blocks 30 --output-dir target/comparison/brief01-e54e33eb-20261011`.
Bundle `summary.json` SHA-256:
`febc1b2f4904f113eca4ca4850630012e4e08ded0f85829ace3c91c9f48b6955`.
Queue `target/comparison/brief01-e54e33eb-20261011-queue.json` SHA-256:
`d064c7e9a06d2e7d2d998a26bc1d0febb0dc7e8177ccb7508d0d0f7108442e8c`.
Profile index: `target/profiles/brief01-e54e33eb/evidence-index.json`.

| Lane | Wall candidate / NG | Cycles candidate / NG |
| --- | --- | --- |
| Broad (25) | 0.9701 | 0.9690 |
| Sentinel (6) | 0.8583 | 0.8588 |
| JetStream JS subset (5) | 0.9864 | 0.9852 |
| Kraken (14) | 0.6047 | 0.6013 |
| SunSpider (26) | 1.0224 | 1.0206 |

External >1.10 wall/NG, in queue order: `bitops-bitwise-and` 4.2509,
`date-format-tofte` 1.3974, `hash-map` 1.3581, `cdjs` 1.3488,
`3d-raytrace` 1.3340, `access-binary-trees` 1.3058,
`string-validate-input` 1.2447, `crypto-md5` 1.2326,
`raytrace-public-class-fields` 1.1877, `crypto-aes` 1.1091.
Sentinel wall/NG: recursive 0.7455, prototype 0.9930, polymorphic 0.8188,
capturing 0.7179, heterogeneous 0.999928, churn 0.9190. None >1.0;
heterogeneous CI [0.99697, 1.00241] does not establish stable <=1.0.

Decision readiness: **inconclusive**, not promotion evidence. The only
imprecise case is audio-dft: candidate/base cycles CI [0.96136, 1.02474],
relative half-width 4.25% >3%. The complete queue is usable for selection.

Linearity diagnosis: old d1313d52 normalized wall probes 1.168/1.245 had
instructions 0.9985/0.9995 and cycles 1.0168/1.0038. Fresh probes of that
same base executable give 0.9894/0.9950, HEAD 0.9972/0.9988; all 75 probe
groups pass. Host timing interference is supported, its specific source
unproven. Neither whole-work elimination nor N/2N tier switching explains
it: many_locals_call has 1003/2003 call attempts at N=1000/2000;
property_read has 1000/2000 backedges and 12290/24290 interpreter ops.
The latter uses virtual object slots and declines typed entry; it is not
an ordinary property-cache control. A complete quiet-host remeasurement
resolved the failure; no case or threshold change is warranted.

## Goal

Beat the pinned QuickJS-NG reference by at least 2x on every admitted benchmark
case. For broad v2, each of the 25 case ratios, every family ratio, and the
overall geometric-mean wall ns/op ratio must be at most 0.50x. For the external
JetStream, Kraken, and SunSpider neutral-shell portfolios, every pinned case
must become runnable and each candidate/QuickJS-NG ratio must be at most 0.50x;
suite geometric means must therefore also be at most 0.50x. This is explicitly
a **general JavaScript-engine performance**
goal, not permission to optimize only the repository's internal benchmark
shapes. The pinned external JetStream 3 JavaScript subset, Kraken 1.1, and
SunSpider 1.0 neutral shell ports are an independent anti-overfitting boundary:
the campaign cannot complete unless the improvement generalizes there too.
Optimization priorities must therefore come from general engine mechanisms and
independent external profiles; broad-micro is a regression guard, not a target
whose case shapes may dictate runtime design.

## Scope

- Allowed paths: first establish `benchmarks/`, `tools/benchmark/`, benchmark
  scripts/tests/docs; later units may change `qjs-runtime` with focused tests.
- Forbidden paths: `third_party/`, benchmark-only engine shortcuts, weakened
  checksums, reduced iteration work, hidden case selection, or Test262
  regressions.
- Runtime changes must optimize a general mechanism (for example allocation,
  representation, dispatch, property access, compilation, or GC) and remain
  justified without referring to an internal case ID, expected iteration
  count, checksum, or workload source path. An internal-only win that leaves
  the corresponding external workloads unchanged or materially worse is not
  campaign progress.
- Owner boundary: serialize manifest/protocol changes and broad runtime
  architecture changes on the main branch; one measured change per commit.

## General Optimization Acceptance Rule

This campaign accepts performance work only when the implementation is a
general engine mechanism and the evidence shows that it is not merely fitted
to broad-micro. Every runtime unit must therefore include both the complete
broad portfolio and the pinned external preview before it is closed. A unit is
rejected as campaign progress when its benefit depends on an internal case
shape, benchmark identity, fixed iteration count, expected checksum, or source
path; when it reduces comparable external coverage; or when a nominal internal
win leaves the relevant external workloads materially worse without an
explained, independently measured tradeoff. Focused microbenchmarks may locate
a bottleneck, but they cannot define the runtime semantics or serve as the
unit's only acceptance evidence.

External results remain informational neutral-shell measurements rather than
official suite scores. That limitation does not make them optional: their role
inside T018 is the mandatory independent check that an optimization improves
ordinary JavaScript mechanisms beyond the repository's own benchmark shapes.

## References

- `AGENTS.md`
- `docs/architecture.md`
- `docs/benchmarking.md`
- `docs/harness.md`
- `tasks/archive/T016-environment-model-rewrite.md`

## Portfolio Contract

Broad v2 contains 25 critical cases across eight families: call (6), binding
(5), property (3), array (3), control (2), builtin (2), string (1), and
allocation (3). The seven historical T016 cases remain as a trace cohort; 18
shape and subsystem holdouts prevent the historical exact-loop trace fast path
from standing in for general engine performance.

The authoritative ratio is candidate wall ns/op divided by pinned QuickJS-NG
wall ns/op. Acceptance requires all of the following on a complete, healthy,
same-host run:

- overall geometric-mean ratio <= 0.50;
- every one of the 25 case ratios <= 0.50;
- every critical family ratio <= 0.50;
- no invalid block, failed linearity probe, checksum mismatch, or timer-limited
  case;
- focused tests plus `scripts/check.sh` pass, preserving Test262 behavior;
- a second independent run confirms the final result before completion.

The target is a campaign acceptance criterion. Existing hosted previews remain
informational and non-gating until T017 M6/M7 fixed-hardware qualification is
complete.

## External Generalization Contract

Every trusted `main` push already publishes a pinned, execution-only external
preview. Each runtime unit must compare candidate, the exact preceding base,
and QuickJS-NG in the same external run; cross-run candidate-duration movement
is diagnostic only and cannot accept a unit because hosted-runner drift is not
controlled across runs. These neutral shell ports are not official JetStream,
Kraken, or SunSpider scores, and incomplete suites have no suite score. They
are still the campaign's independent anti-overfitting evidence because their source,
adapter, case inventory, engine revisions, outer wall timer, and per-case
results do not depend on the broad-micro workload.

Completion additionally requires a repeatable final external preview in which:

- every pinned external case is runnable and comparable; timeouts, unsupported
  cases, and reduced coverage cannot manufacture a better ratio;
- every external case is <= 0.50x qjs-rust/QuickJS-NG and the diagnostic
  geometric mean is <= 0.50x for each of the three pinned suites;
- the winning mechanisms are general runtime changes, with no benchmark-name,
  file-path, iteration-count, or checksum specialization;
- an independent rerun confirms the final external result alongside the broad
  portfolio confirmation and the full correctness gate.

The external preview remains informational in CI and cannot become an official
suite claim. These thresholds are T018 completion guards, not a statement that
an incomplete neutral shell port is an upstream suite score.

## Milestones

- [x] B1 freeze broad v2 workload, exact case/family inventory, manifest,
  protocol hashes, hosted preview contract, and documentation.
- [x] B2 record the first complete broad v2 three-role local baseline and
  identify the largest family/case gaps without excluding weak cases.
- [ ] B3 optimize structural bottlenecks in separately verified commits,
  recording broad and external generalization evidence for each pushed unit.
- [ ] B4 reach <= 0.50x overall, in every critical family, and in every one of
  the 25 broad cases.
- [ ] B5 make every pinned external case runnable, then reach <= 0.50x for each
  case and each suite geometric mean with no coverage reduction.
- [ ] B6 independently confirm both internal and external results and run the
  full correctness gate.

## Verification

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/benchmark/tests -v
./scripts/performance-policy-audit.sh
./scripts/check.sh
```

How to run the complete candidate/base/QuickJS-NG comparison, the external
preview and the reports: [the performance workflow](../docs/performance-workflow.md)
and `docs/benchmarking.md`, "Running".

## Where the rest lives

- Selection and acceptance of units: `docs/performance-workflow.md` and
  `tasks/T022-performance-priority-controller.md`.
- Durable measurement rules: `docs/performance-knowledge.md`.
- Frozen unit plans: `tasks/performance-units/`.
- Closed unit records (`T019` to `T032`) and experiment logs:
  `tasks/archive/`. The broad v1/v2 baselines, the external generalization
  reset and the unit log through 2026-07-29 (Units 1-92 and the dated entries
  that followed) are preserved verbatim in
  [`archive/T018-broad-performance-log.md`](archive/T018-broad-performance-log.md).
  They are historical evidence bound to their own revisions, not current
  ratios or priorities.
