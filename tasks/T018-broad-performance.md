# T018: Broad Performance Campaign

- Status: active. This is the performance campaign contract.
- Implementation: partial; B1-B2 complete, B3-B6 open. Both batch-1 leaves
  are present on `integration/batch-1`, but promotion is inconclusive.
- Verified at: `af4d0d3c`, 2026-10-12, locked 30-block comparison against
  exact queue candidate `e54e33eb` and NG `f7830186`; all lanes complete;
  exact-candidate CI burndown passes all 42672 configured cases with zero gaps.
- Evidence: standing and hashes below; unit decisions in [T037](T037-integration-batch-1.md).
  Raw target/ artifacts are local, not retained.
- Unresolved: only audio-dft is imprecise; it is a declared global-sink control.
  All three target gates pass; neither unit has retained promotion evidence.
- Next action: follow T037's precision/split follow-up; future profiling uses
  the fresh measured-candidate queue below, preserving existing frozen plans.

## Standing at af4d0d3c

Candidate `af4d0d3c81aad98721661689cf7224d2c17b9781`, base
`e54e33eb9371c5cb4627bd8e590a648085a7fa26`, NG `f7830186`.
Same-host diagnostic, not fixed-hardware qualification; promotion metric cycles.
30 valid internal blocks, complete external comparisons, no wall-time divergence.
Preflight was quiet; a five-second monitor observed zero compiler processes.
Bundle: `target/comparison/batch1-formal-af4d0d3c81aad98721661689cf7224d2c17b9781/summary.json`;
SHA-256 `90fe2a3ad29ef7d035195f8f36c3dc3e12d9706f0ae9f5b9a91ef3c174af9bf7` seals all nine artifact hashes.
Queue: `target/comparison/batch1-af4d0d3c-queue.json`;
SHA-256 `2882db837308dca8db304e3d0de0b32efcb4326332fe218393020c2f9e275f26`.
The queue binds measured af4d0d3c; later documentation/ledger commits must not
be substituted into its source identity, even with unchanged runtime bytes.

| Lane | Cases | Wall / NG | Cycles / NG |
| --- | ---: | ---: | ---: |
| Broad | 25 | 0.925548 | 0.923025 |
| Sentinel | 6 | 0.867605 | 0.865307 |
| jetstream3-js-subset | 5 | 0.978939 | 0.974527 |
| kraken-1.1 | 14 | 0.604874 | 0.599738 |
| sunspider-1.0 | 26 | 0.938011 | 0.932327 |

[Full case standing](archive/T018-af4d0d3c-case-standing.md) lists every external
case over 1.10, sentinel over 1.0 and broad case over 1.5 in wall or cycles.

All >3% changes use this run's paired candidate/base comparison; no regression
exceeds 3%. Beyond the two external targets, top-level call (the global plan's
second target) and untargeted 3d-morph improved.

| Case | Wall / base | Cycles / base |
| --- | ---: | ---: |
| `broad/top_level_function_call` | 0.309404 | 0.309658 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.947675 | 0.947281 |
| `external/sunspider-1.0/3d-morph` | 0.776775 | 0.756201 |
| `external/sunspider-1.0/bitops-bitwise-and` | 0.130456 | 0.116990 |

Readiness is **inconclusive**, not failed measurement: audio-dft cycles/base
0.996205, CI [0.906182, 1.040874], relative half-width 9.9344% >3%, all 30 blocks
valid; wall/base 0.978284, CI [0.892233, 1.042806], half-width 9.6445%.
Every other case passes precision and its applicable gate. Audio-dft is a
declared global control and a class promotion portfolio control; unrelated
noise does not explain this batch's status. The new queue is usable for selection.

## Standing at e54e33eb

[Historical baseline](archive/T018-e54e33eb-standing.md), preserving immutable log links.

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
