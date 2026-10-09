# T018: Broad Performance Campaign

- Status: active. This is the performance campaign contract.
- Implementation: partial. Milestones B1-B2 are complete; B3-B6 are open.
- Verified at: `d1313d52`, 2026-10-09, one formal 30-block three-engine
  comparison against base `7a83b568` and the pinned QuickJS-NG
  (`scripts/perf-compare.sh`, same-host diagnostic, not a fixed-hardware
  claim). Results are under "Standing at d1313d52".
- Evidence: the numbers below. The bundle
  (`target/comparison/template-removal-d1313d52/`, `summary.json` SHA-256
  `6b2c98f4...c71b6cf`) is a local artifact, not retained. Earlier results
  are in `tasks/archive/` and apply only to their own revisions.
- Unresolved: none of the completion thresholds below is recorded as met.
  The broad lane produced no ratios at `d1313d52`: `many_locals_call` and
  `property_read` failed the linearity check on the candidate, so the run's
  decision readiness is "inconclusive".
- Next action: generate a queue for the current revision
  (`docs/performance-workflow.md`, step 1) and select a unit through
  `tasks/T022-performance-priority-controller.md`. Explain the two
  non-linear broad cases first; no broad result can be read until they pass.

This file holds the contract and the last formal standing. It does not
record units or select work.

## Standing at d1313d52

Candidate `d1313d52` is the first revision without the loop-template plans;
base `7a83b568` is the last with them. Ratios are paired medians over 30
blocks, lower is faster.

| Lane | Candidate / QuickJS-NG | Candidate / base |
| --- | --- | --- |
| Broad (25 cases) | no ratios (linearity failed, see above) | no ratios |
| Sentinel `recursive_call_tree` | 0.742 | 0.994 |
| Sentinel `prototype_method_call` | 1.002 | 1.004 |
| Sentinel `polymorphic_call_site` | 0.820 | 1.007 |
| Sentinel `capturing_closure_call` | 0.741 | 1.000 |
| Sentinel `heterogeneous_property_read` | 1.022 | 1.022 |
| Sentinel `string_key_map_churn` | 0.919 | 1.015 |
| External, JetStream 3 subset (5) | 0.981 | 1.015 |
| External, Kraken 1.1 (14) | 0.606 | 1.126 |
| External, SunSpider 1.0 (26) | 1.024 | 1.161 |

External cases slower than QuickJS-NG by more than 10%:
`bitops-bitwise-and` 4.357, `date-format-tofte` 1.415, `hash-map` 1.351,
`cdjs` 1.344, `3d-raytrace` 1.328, `access-binary-trees` 1.291,
`string-validate-input` 1.238, `crypto-md5` 1.235,
`raytrace-public-class-fields` 1.177, `date-format-xparb` 1.146.

External cases that moved against the base by more than 3%:
`bitops-bitwise-and` 22.069, `audio-dft` 5.258, `bitops-nsieve-bits` 2.055,
`crypto-md5` 1.049, `hash-map` 1.032. The first three ran on removed
template plans; `audio-dft` (0.786) and `bitops-nsieve-bits` (0.854) are
still faster than QuickJS-NG.

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
